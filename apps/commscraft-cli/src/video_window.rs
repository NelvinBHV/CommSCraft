//! The video window in a process of its own (`commscraft-cli video-window ID gpu|cpu`): it decodes
//! the received video and shows it; the client hands it access units over a pipe.
//!
//! Why a process: on an integrated AMD GPU with VCN 4 a GPU encoder changing next to a GPU decoder
//! in the same process hangs the video engine, in separate processes it does not (measured, evidence
//! file 23). A remote peer decides when the encoder changes.
//!
//! Security by design: received video is the most exposed input of the client. A decoder or driver
//! failing on it now ends this process and not the call; the client starts it again on the CPU. It
//! gets no credentials and only the environment a window needs. What it reports is treated as
//! untrusted: bounded, printable, limited in number.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use commscraft_video_ffmpeg as vf;
use commscraft_video_wayland::{Error as WindowError, Frame, Plane, Window};

use crate::video_call::FromVideo;

/// The command of the helper process.
pub const COMMAND: &str = "video-window";
/// What a window needs from the environment.
const ENVIRONMENT: [&str; 4] = [
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "DISPLAY",
    "XAUTHORITY",
];
/// Longest message of the helper that is passed on, and how many of them.
const MAX_NOTICE: usize = 200;
const MAX_NOTICES: usize = 16;
/// Starts of the helper after it failed: the first again on the CPU.
const MAX_RESTARTS: u32 = 2;

/// One access unit on the pipe: its length, then its bytes.
fn write_unit(out: &mut impl Write, unit: &[u8]) -> std::io::Result<()> {
    let length = u32::try_from(unit.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    out.write_all(&length.to_be_bytes())?;
    out.write_all(unit)?;
    out.flush()
}

/// The next access unit; `None` at the end of the pipe. A length above the decoder's limit ends the
/// stream rather than allocating it.
fn read_unit(input: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut length = [0u8; 4];
    match input.read_exact(&mut length) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > vf::MAX_ACCESS_UNIT {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut unit = vec![0; length];
    input.read_exact(&mut unit)?;
    Ok(Some(unit))
}

/// A message of the helper as it may be shown.
fn clean_notice(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_control())
        .take(MAX_NOTICE)
        .collect()
}

/// A GPU picture in the window; the window keeps it until the compositor is done with it.
fn show_surface(window: &mut Window, surface: vf::DmaBuf) -> Result<(), WindowError> {
    let surface = Rc::new(surface);
    // The memory's inode tells whether the window knows this buffer already.
    let first = surface.planes.first().ok_or(WindowError::Unsupported)?;
    let key = first
        .fd
        .try_clone()
        .and_then(|fd| std::fs::File::from(fd).metadata())
        .map_err(|e| WindowError::Connection(e.to_string()))?
        .ino();
    let planes: Vec<Plane<'_>> = surface
        .planes
        .iter()
        .map(|p| Plane {
            fd: p.fd.as_fd(),
            offset: p.offset,
            stride: p.stride,
        })
        .collect();
    let frame = Frame {
        width: surface.width,
        height: surface.height,
        fourcc: surface.fourcc,
        modifier: surface.modifier,
        planes: &planes,
        key,
    };
    window.show(&frame, Box::new(Rc::clone(&surface)))?;
    Ok(())
}

/// The helper process. Messages for the user go to standard output, one per line.
pub fn helper_main(mut args: impl Iterator<Item = String>) -> i32 {
    let id = args.next().unwrap_or_default();
    let mut surfaces = args.next().as_deref() == Some("gpu") && Window::in_wayland_session();
    let title = format!("CommSCraft – call {}", clean_notice(&id));
    let mut decoder = match vf::init().and_then(|()| vf::Decoder::with_hardware(surfaces)) {
        Ok(d) => d,
        Err(e) => {
            println!("no video decoder: {e:?}");
            return 2;
        }
    };
    let mut input = std::io::stdin().lock();
    let mut surface_window: Option<Window> = None;
    let mut window: Option<vf::Display> = None;
    let mut failed = false;
    let mut announced = false;
    let mut closed = false;
    while let Ok(Some(unit)) = read_unit(&mut input) {
        let decoded = if surfaces {
            decoder.decode_to_surfaces(&unit)
        } else {
            decoder
                .decode(&unit)
                .map(|p| p.into_iter().map(vf::Decoded::Picture).collect())
        };
        let Ok(decoded) = decoded else {
            continue;
        };
        for d in decoded {
            let p = match d {
                vf::Decoded::Surface(s) => {
                    if closed {
                        continue;
                    }
                    if !announced {
                        announced = true;
                        println!(
                            "receiving video {}x{}, decoded on the GPU, shown directly",
                            s.width, s.height
                        );
                    }
                    let shown = match surface_window.as_mut() {
                        Some(w) => show_surface(w, s),
                        None => Window::open(&title).and_then(|mut w| {
                            let r = show_surface(&mut w, s);
                            surface_window = Some(w);
                            r
                        }),
                    };
                    match shown {
                        Ok(()) => {}
                        Err(WindowError::Closed) => {
                            closed = true;
                            surface_window = None;
                        }
                        Err(e) => {
                            // The next pictures come through the copy, to the other window.
                            println!("no direct video window: {e:?}");
                            surface_window = None;
                            surfaces = false;
                        }
                    }
                    continue;
                }
                vf::Decoded::Picture(p) => p,
            };
            if !announced {
                announced = true;
                println!(
                    "receiving video {}x{}, decoded on the {}",
                    p.width,
                    p.height,
                    match decoder.backend() {
                        vf::Backend::Vaapi => "GPU, copied back",
                        vf::Backend::Software => "CPU",
                    }
                );
            }
            if window.as_ref().is_none() && !failed {
                match vf::Display::open(&title, p.width, p.height) {
                    Ok(w) => window = Some(w),
                    Err(e) => {
                        failed = true;
                        println!("no video window: {e:?}");
                    }
                }
            }
            if let Some(w) = window.as_mut() {
                if w.show(&p).is_err() {
                    // Another size: a new window.
                    window = vf::Display::open(&title, p.width, p.height).ok();
                }
            }
        }
    }
    0
}

/// GPU decoding where the pictures can go to the screen directly: copying them back would cost more
/// than decoding on the CPU (measured, evidence file 23). `COMMSCRAFT_VIDEO_DECODE=cpu` asks for the
/// CPU.
fn gpu_wanted() -> bool {
    Window::in_wayland_session()
        && std::env::var("COMMSCRAFT_VIDEO_DECODE").map_or(true, |v| v.trim() != "cpu")
}

fn start(id: u32, gpu: bool) -> std::io::Result<Child> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(COMMAND)
        .arg(id.to_string())
        .arg(if gpu { "gpu" } else { "cpu" })
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    for name in ENVIRONMENT {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command.spawn()
}

/// Passes the helper's messages on.
fn forward_notices(child: &mut Child, events: &Sender<FromVideo>, id: u32) {
    let Some(stdout) = child.stdout.take() else {
        return;
    };
    let events = events.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut left = MAX_NOTICES;
        loop {
            let mut line = Vec::new();
            // A line is read only up to a bound, the rest of it counts as the next line.
            match (&mut reader)
                .take((MAX_NOTICE * 4) as u64)
                .read_until(b'\n', &mut line)
            {
                Ok(n) if n > 0 => {}
                _ => return,
            }
            if left > 0 {
                left -= 1;
                let text = clean_notice(&String::from_utf8_lossy(&line));
                let _ = events.send(FromVideo::Notice(id, text));
            }
        }
    });
}

/// Hands access units to the helper until it or the stream ends. Returns the input, and whether the
/// stream has ended.
fn feed(
    mut stdin: ChildStdin,
    input: Receiver<Vec<u8>>,
    stop: &AtomicBool,
) -> (Receiver<Vec<u8>>, bool) {
    loop {
        if stop.load(Ordering::Relaxed) {
            return (input, true);
        }
        match input.recv_timeout(Duration::from_millis(100)) {
            Ok(unit) => {
                if write_unit(&mut stdin, &unit).is_err() {
                    return (input, false);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return (input, true),
        }
    }
}

/// The client's side: starts the helper and feeds it. A helper that fails is started again, on the
/// CPU, and the peer is asked for a new picture; a helper still failing then ends the received video
/// and nothing else.
pub fn spawn(input: Receiver<Vec<u8>>, stop: Arc<AtomicBool>, events: Sender<FromVideo>, id: u32) {
    std::thread::spawn(move || {
        let notice = |text: String| {
            let _ = events.send(FromVideo::Notice(id, text));
        };
        let mut gpu = gpu_wanted();
        let mut input = Some(input);
        let mut restarts = 0;
        loop {
            let mut child = match start(id, gpu) {
                Ok(c) => c,
                Err(e) => {
                    notice(format!("no video window process: {e}"));
                    return;
                }
            };
            forward_notices(&mut child, &events, id);
            let (Some(stdin), Some(units)) = (child.stdin.take(), input.take()) else {
                let _ = child.kill();
                let _ = child.wait();
                return;
            };
            let feeder = {
                let stop = stop.clone();
                std::thread::spawn(move || feed(stdin, units, &stop))
            };
            let status = loop {
                if stop.load(Ordering::Relaxed) {
                    let _ = child.kill();
                }
                match child.try_wait() {
                    Ok(Some(s)) => break Some(s),
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => break None,
                }
            };
            let Ok((units, ended)) = feeder.join() else {
                return;
            };
            if ended || stop.load(Ordering::Relaxed) || status.is_some_and(|s| s.success()) {
                return;
            }
            restarts += 1;
            if restarts > MAX_RESTARTS {
                notice("the video window keeps failing: no more received video".into());
                return;
            }
            notice(if gpu {
                "the video window failed: decoding on the CPU from here on".into()
            } else {
                "the video window failed: starting it again".into()
            });
            gpu = false;
            input = Some(units);
            let _ = events.send(FromVideo::PictureUpdate(id));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What goes over the pipe; hostile lengths are refused before allocating.
    #[test]
    fn units_on_the_pipe() {
        let mut pipe = Vec::new();
        write_unit(&mut pipe, b"abc").unwrap();
        write_unit(&mut pipe, b"").unwrap();
        let mut r = pipe.as_slice();
        assert_eq!(read_unit(&mut r).unwrap(), Some(b"abc".to_vec()));
        assert_eq!(read_unit(&mut r).unwrap(), Some(Vec::new()));
        assert_eq!(read_unit(&mut r).unwrap(), None);
        let too_long = ((vf::MAX_ACCESS_UNIT + 1) as u32).to_be_bytes();
        assert!(read_unit(&mut too_long.as_slice()).is_err());
        assert!(
            read_unit(&mut [0u8, 0, 0, 9, 1].as_slice()).is_err(),
            "cut off"
        );
    }

    /// A message cannot fake a line of the client or fill the screen.
    #[test]
    fn notices_are_cleaned() {
        assert_eq!(
            clean_notice("ok\r\n* signed out\x1b[2J\n"),
            "ok* signed out[2J"
        );
        assert_eq!(clean_notice(&"x".repeat(1000)).len(), MAX_NOTICE);
    }
}
