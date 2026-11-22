//! The video window in a process of its own (`<client> video-window ID gpu|cpu`, the command-line
//! tool or the desktop client): it decodes the received video and shows it; the client hands it
//! access units over a pipe.
//!
//! Why a process: on an integrated AMD GPU with VCN 4 a GPU encoder changing next to a GPU decoder
//! in the same process hangs the video engine, in separate processes it does not (measured, evidence
//! file 23). A remote peer decides when the encoder changes.
//!
//! Security by design: received video is the most exposed input of the client. A decoder or driver
//! failing on it now ends this process and not the call; the client starts it again on the CPU. It
//! gets no credentials and only the environment a window needs. Before it reads any video it puts
//! itself into a Landlock sandbox ([`sandbox`]): it may read the system's files and use the GPU, but
//! not read the user's files, write files, open TCP connections or gain privileges. A system call
//! filter ([`syscall_filter`]) then takes away what a taken-over decoder could still use: starting
//! programs, network sockets, debugging other processes, kernel interfaces. What it reports is
//! treated as untrusted: bounded, printable, limited in number.
//!
//! Fail closed (issue #144): without the system call filter or without Landlock the helper
//! decodes nothing ([`confinement`]); a Landlock the kernel knows only in part is reported and
//! taken. `COMMSCRAFT_VIDEO_UNCONFINED=1` in the client's environment allows decoding without
//! them anyway, by the user's explicit choice.

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
/// The helper's exit code when it refuses to decode without its confinement (not restarted).
const EXIT_UNCONFINED: i32 = 3;
/// The variable by which the user allows decoding without confinement.
const UNCONFINED: &str = "COMMSCRAFT_VIDEO_UNCONFINED";

/// One access unit on the pipe: its length, then its bytes.
fn write_unit(out: &mut impl Write, unit: &[u8]) -> std::io::Result<()> {
    let length = u32::try_from(unit.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    out.write_all(&length.to_be_bytes())?;
    out.write_all(unit)?;
    out.flush()
}

/// The next access unit; `None` at the end of the pipe. A length above the decoder's limit ends the
/// stream rather than allocating it (the helper reads with this limit through [`read_units`]).
#[cfg(test)]
fn read_unit(input: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    read_bounded(input, vf::MAX_ACCESS_UNIT)
}

/// The largest shared screen sent to the window as it is; larger ones are scaled down (design:
/// bounds the pipe's units).
pub const MAX_RAW_WIDTH: u32 = 2560;
pub const MAX_RAW_HEIGHT: u32 = 1600;
/// The largest raw picture on the pipe: its size header and the pixels.
const MAX_RAW_UNIT: usize = 8 + MAX_RAW_WIDTH as usize * MAX_RAW_HEIGHT as usize * 4;

/// A raw picture on the pipe (shared windows over RDP, function 43): width and height (big
/// endian), then the pixels in BGRX order (XRGB8888 little-endian, what Wayland shows).
pub fn raw_unit(width: u32, height: u32, bgrx: &[u8]) -> Vec<u8> {
    let mut unit = Vec::with_capacity(8 + bgrx.len());
    unit.extend_from_slice(&width.to_be_bytes());
    unit.extend_from_slice(&height.to_be_bytes());
    unit.extend_from_slice(bgrx);
    unit
}

/// Width, height and pixels of a raw unit, if size and length agree.
fn raw_of(unit: &[u8]) -> Option<(u32, u32, &[u8])> {
    let width = u32::from_be_bytes(unit.get(..4)?.try_into().ok()?);
    let height = u32::from_be_bytes(unit.get(4..8)?.try_into().ok()?);
    if width == 0 || height == 0 || width > MAX_RAW_WIDTH || height > MAX_RAW_HEIGHT {
        return None;
    }
    let pixels = unit.get(8..)?;
    (pixels.len() == width as usize * height as usize * 4).then_some((width, height, pixels))
}

/// The size of a `w × h` picture scaled down to fit `max_w × max_h`, aspect kept.
fn fit(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let (mut ow, mut oh) = (w, h);
    if ow > max_w {
        oh = (u64::from(oh) * u64::from(max_w) / u64::from(ow)) as u32;
        ow = max_w;
    }
    if oh > max_h {
        ow = (u64::from(ow) * u64::from(max_h) / u64::from(oh)) as u32;
        oh = max_h;
    }
    (ow, oh)
}

/// The RGBA screen as BGRX pixels no larger than the raw maximum (aspect kept, nearest
/// sampling); `None` when the data does not fit the size.
pub fn scaled_bgrx(w: u16, h: u16, rgba: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h) = (u32::from(w), u32::from(h));
    if w == 0 || h == 0 || rgba.len() != w as usize * h as usize * 4 {
        return None;
    }
    let (ow, oh) = fit(w, h, MAX_RAW_WIDTH, MAX_RAW_HEIGHT);
    if ow == 0 || oh == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(ow as usize * oh as usize * 4);
    for y in 0..oh {
        let sy = (u64::from(y) * u64::from(h) / u64::from(oh)) as usize;
        for x in 0..ow {
            let sx = (u64::from(x) * u64::from(w) / u64::from(ow)) as usize;
            let i = (sy * w as usize + sx) * 4;
            match rgba.get(i..i + 3) {
                Some([r, g, b]) => out.extend_from_slice(&[*b, *g, *r, 0xff]),
                _ => out.extend_from_slice(&[0, 0, 0, 0xff]),
            }
        }
    }
    Some((ow, oh, out))
}

/// BGRX pixels as an I420 picture for the software window, no larger than `max_w × max_h`
/// (aspect kept, nearest sampling, even sides); BT.601 limited range, chroma from each 2×2
/// block's top left pixel. `None` for a size the window cannot show.
fn i420_of_bgrx(w: u32, h: u32, bgrx: &[u8], max_w: u32, max_h: u32) -> Option<vf::Picture> {
    if w == 0 || h == 0 || bgrx.len() != w as usize * h as usize * 4 {
        return None;
    }
    let (ow, oh) = fit(w, h, max_w, max_h);
    let (ow, oh) = (ow & !1, oh & !1);
    if ow < 16 || oh < 16 {
        return None;
    }
    let pixel = |x: u32, y: u32| -> (i32, i32, i32) {
        let sx = (u64::from(x) * u64::from(w) / u64::from(ow)) as usize;
        let sy = (u64::from(y) * u64::from(h) / u64::from(oh)) as usize;
        let i = (sy * w as usize + sx) * 4;
        match bgrx.get(i..i + 3) {
            Some([b, g, r]) => (i32::from(*r), i32::from(*g), i32::from(*b)),
            _ => (0, 0, 0),
        }
    };
    let clamp = |v: i32| v.clamp(0, 255) as u8;
    let (uw, uh) = (ow / 2, oh / 2);
    let mut data = Vec::with_capacity(vf::Picture::i420_len(ow, oh));
    for y in 0..oh {
        for x in 0..ow {
            let (r, g, b) = pixel(x, y);
            data.push(clamp(((66 * r + 129 * g + 25 * b + 128) >> 8) + 16));
        }
    }
    let mut v_plane = Vec::with_capacity((uw * uh) as usize);
    for y in 0..uh {
        for x in 0..uw {
            let (r, g, b) = pixel(x * 2, y * 2);
            data.push(clamp(((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128));
            v_plane.push(clamp(((112 * r - 94 * g - 18 * b + 128) >> 8) + 128));
        }
    }
    data.extend(v_plane);
    Some(vf::Picture {
        width: ow,
        height: oh,
        data,
    })
}

/// The raw mode of the helper: pictures as they come, shown in the Wayland window from shared
/// memory, else in the software window as I420.
/// Largest picture handed back to an embedding client (design: the conversation window's
/// stage; a 640×360 RGBA picture is 0.9 MB).
pub const EMBED_WIDTH: u32 = 640;
pub const EMBED_HEIGHT: u32 = 360;
/// Pictures handed back per second at most (design).
const EMBED_FPS: u64 = 15;
/// The header line of a picture handed back: `FRAME <width> <height> <bytes>`, then the RGBA
/// bytes. Every other line is a notice.
const FRAME_TAG: &str = "FRAME ";

/// The client shows the received video itself (in the conversation window) instead of the
/// helper's own window; set once by the desktop client.
static EMBEDDED: AtomicBool = AtomicBool::new(false);

/// Received video goes back to the client as pictures (the desktop client's conversation
/// window) instead of into a window of its own.
pub fn set_embedded(on: bool) {
    EMBEDDED.store(on, Ordering::Relaxed);
}

/// Whether received video (and the self view) go back to the client as pictures.
pub fn embedded() -> bool {
    EMBEDDED.load(Ordering::Relaxed)
}

/// The self view's size: a small picture in the corner of the stage.
pub const SELF_VIEW_WIDTH: u32 = 320;
pub const SELF_VIEW_HEIGHT: u32 = 180;

/// An I420 picture as RGBA no larger than `max_w × max_h` (aspect kept, nearest sampling;
/// BT.601 limited range). `None` for a picture whose data does not fit its size.
pub fn rgba_of_i420(p: &vf::Picture, max_w: u32, max_h: u32) -> Option<(u32, u32, Vec<u8>)> {
    let (w, h) = (p.width, p.height);
    if w == 0 || h == 0 || p.data.len() < vf::Picture::i420_len(w, h) {
        return None;
    }
    let (ow, oh) = fit(w, h, max_w, max_h);
    if ow == 0 || oh == 0 {
        return None;
    }
    let (wu, hu) = (w as usize, h as usize);
    let (y_plane, rest) = p.data.split_at(wu * hu);
    let (u_plane, v_plane) = rest.split_at((wu / 2) * (hu / 2));
    let mut out = Vec::with_capacity(ow as usize * oh as usize * 4);
    for oy in 0..oh {
        let sy = (u64::from(oy) * u64::from(h) / u64::from(oh)) as usize;
        for ox in 0..ow {
            let sx = (u64::from(ox) * u64::from(w) / u64::from(ow)) as usize;
            let c = (sy / 2) * (wu / 2) + (sx / 2).min(wu / 2 - 1);
            let yv = i32::from(*y_plane.get(sy * wu + sx).unwrap_or(&16)) - 16;
            let u = i32::from(*u_plane.get(c).unwrap_or(&128)) - 128;
            let v = i32::from(*v_plane.get(c).unwrap_or(&128)) - 128;
            let r = (298 * yv + 409 * v + 128) >> 8;
            let g = (298 * yv - 100 * u - 208 * v + 128) >> 8;
            let b = (298 * yv + 516 * u + 128) >> 8;
            out.extend_from_slice(&[
                r.clamp(0, 255) as u8,
                g.clamp(0, 255) as u8,
                b.clamp(0, 255) as u8,
                0xff,
            ]);
        }
    }
    Some((ow, oh, out))
}

/// Writes one picture back to the client (embedded mode), at most [`EMBED_FPS`] a second.
fn send_frame(last: &mut Option<std::time::Instant>, w: u32, h: u32, rgba: &[u8]) {
    let now = std::time::Instant::now();
    if last.is_some_and(|t| now.duration_since(t) < Duration::from_millis(1000 / EMBED_FPS)) {
        return;
    }
    *last = Some(now);
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{FRAME_TAG}{w} {h} {}", rgba.len());
    let _ = out.write_all(rgba);
    let _ = out.flush();
}

/// Embedded mode: decode and hand the pictures back, no window.
fn embed_main(raw: bool) -> i32 {
    let mut last = None;
    if raw {
        let units = read_units(MAX_RAW_UNIT);
        let mut size = (0, 0);
        while let Ok(unit) = units.recv() {
            let Some((width, height, pixels)) = raw_of(&unit) else {
                continue;
            };
            if size != (width, height) {
                println!("the shared window is {width}x{height}");
                size = (width, height);
            }
            // BGRX to RGBA, made smaller for the stage (a shared window keeps more detail).
            let (ow, oh) = fit(width, height, EMBED_WIDTH * 2, EMBED_HEIGHT * 2);
            let mut out = Vec::with_capacity(ow as usize * oh as usize * 4);
            for oy in 0..oh {
                let sy = (u64::from(oy) * u64::from(height) / u64::from(oh.max(1))) as usize;
                for ox in 0..ow {
                    let sx = (u64::from(ox) * u64::from(width) / u64::from(ow.max(1))) as usize;
                    let i = (sy * width as usize + sx) * 4;
                    match pixels.get(i..i + 3) {
                        Some([b, g, r]) => out.extend_from_slice(&[*r, *g, *b, 0xff]),
                        _ => out.extend_from_slice(&[0, 0, 0, 0xff]),
                    }
                }
            }
            send_frame(&mut last, ow, oh, &out);
        }
        return 0;
    }
    let mut decoder = match vf::init().and_then(|()| vf::Decoder::with_hardware(false)) {
        Ok(d) => d,
        Err(e) => {
            println!("no video decoder: {e:?}");
            return 2;
        }
    };
    let units = read_units(vf::MAX_ACCESS_UNIT);
    let mut size = (0, 0);
    while let Ok(unit) = units.recv() {
        let Ok(pictures) = decoder.decode(&unit) else {
            continue;
        };
        for p in pictures {
            if size == (0, 0) {
                println!(
                    "receiving video {}x{}, decoded on the {}",
                    p.width,
                    p.height,
                    match decoder.backend() {
                        vf::Backend::Vaapi => "GPU, copied back",
                        vf::Backend::Software => "CPU",
                    }
                );
            } else if size != (p.width, p.height) {
                println!("the video is now {}x{}", p.width, p.height);
            }
            size = (p.width, p.height);
            if let Some((w, h, rgba)) = rgba_of_i420(&p, EMBED_WIDTH, EMBED_HEIGHT) {
                send_frame(&mut last, w, h, &rgba);
            }
        }
    }
    0
}

fn raw_main(title: &str) -> i32 {
    let units = read_units(MAX_RAW_UNIT);
    let mut wayland: Option<Window> = None;
    let mut use_wayland = Window::in_wayland_session();
    let mut window: Option<vf::Display> = None;
    let mut size = (0, 0);
    let mut closed = false;
    while let Some(unit) = next_unit(&units, wayland.as_mut(), &mut closed) {
        let Some((width, height, pixels)) = raw_of(&unit) else {
            continue;
        };
        if size != (width, height) {
            println!("the shared window is {width}x{height}");
            size = (width, height);
            window = None;
        }
        if closed {
            continue;
        }
        if use_wayland {
            let shown = match wayland.as_mut() {
                Some(w) => w.show_pixels(width, height, pixels),
                None => Window::open(title).and_then(|mut w| {
                    let r = w.show_pixels(width, height, pixels);
                    wayland = Some(w);
                    r
                }),
            };
            match shown {
                Ok(_) => continue,
                Err(WindowError::Closed) => {
                    closed = true;
                    continue;
                }
                Err(e) => {
                    println!("no Wayland window: {e:?}");
                    wayland = None;
                    use_wayland = false;
                }
            }
        }
        let Some(p) = i420_of_bgrx(width, height, pixels, vf::MAX_WIDTH, vf::MAX_HEIGHT) else {
            continue;
        };
        if window.is_none() {
            match vf::Display::open(title, p.width, p.height) {
                Ok(w) => window = Some(w),
                Err(e) => {
                    println!("no window: {e:?}");
                    return 2;
                }
            }
        }
        if let Some(w) = window.as_mut() {
            if w.show(&p).is_err() {
                window = None;
            }
        }
    }
    0
}

/// How often the window handles the user's input while no picture comes (a still screen).
const IDLE: Duration = Duration::from_millis(50);

/// The units of the pipe, read by a thread of their own into a bounded queue (two units: the
/// pipe and the client wait when the window is behind).
fn read_units(max: usize) -> Receiver<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::sync_channel(2);
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        while let Ok(Some(unit)) = read_bounded(&mut input, max) {
            if tx.send(unit).is_err() {
                return;
            }
        }
    });
    rx
}

/// The next unit; meanwhile the window follows the user (resize, maximize, move, close).
/// `None` at the end of the pipe.
fn next_unit(
    units: &Receiver<Vec<u8>>,
    mut window: Option<&mut Window>,
    closed: &mut bool,
) -> Option<Vec<u8>> {
    loop {
        match units.recv_timeout(IDLE) {
            Ok(unit) => return Some(unit),
            Err(RecvTimeoutError::Disconnected) => return None,
            Err(RecvTimeoutError::Timeout) => {
                if let Some(w) = window.as_deref_mut().filter(|_| !*closed) {
                    if w.idle() == Err(WindowError::Closed) {
                        *closed = true;
                    }
                }
            }
        }
    }
}

fn read_bounded(input: &mut impl Read, max: usize) -> std::io::Result<Option<Vec<u8>>> {
    let mut length = [0u8; 4];
    match input.read_exact(&mut length) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > max {
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

/// Read-only for the helper: programs, libraries, drivers and system configuration.
const READ_ONLY: [&str; 4] = ["/usr", "/lib", "/lib64", "/etc"];
/// Of `/proc` only the helper itself and what the C library and decoders read about the system
/// (issue #207: no other process's files): `/proc/self` is resolved to the helper's own
/// directory when the rules are made, its threads lie beneath it.
const PROC_READ: [&str; 4] = ["/proc/self", "/proc/sys", "/proc/cpuinfo", "/proc/meminfo"];
/// What the kernel tells about devices: the GPU drivers of a window need it.
const SYS_READ: [&str; 1] = ["/sys"];
/// The GPU's render devices: open, read, write and control.
const DEVICES: [&str; 1] = ["/dev/dri"];
/// Single device files libraries read.
const DEVICE_FILES: [&str; 3] = ["/dev/null", "/dev/zero", "/dev/urandom"];

/// What the helper is started for, which decides how tightly it is confined (issue #207).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Pictures go back over standard output, decoded on the CPU: no window, no GPU, no
    /// socket, nothing of the session's runtime directory.
    Embedded,
    /// A window of its own: the GPU devices, `/sys`, Unix sockets to the Wayland or X server and
    /// the X authority file – but not the rest of the runtime directory.
    Window,
}

impl Profile {
    /// The profile for the helper's mode argument.
    pub fn of_mode(mode: Option<&str>) -> Profile {
        match mode {
            Some(m) if m.starts_with("embed") => Profile::Embedded,
            _ => Profile::Window,
        }
    }
}

/// How far the sandbox could be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Full,
    /// The kernel knows only part of the rules.
    Partial,
    /// The kernel has no Landlock: the helper runs without it.
    None,
}

/// The files the helper may read in a profile (paths that do not exist are left out by the rules).
fn readable_paths(profile: Profile) -> Vec<String> {
    let mut readable: Vec<String> = READ_ONLY
        .iter()
        .chain(PROC_READ.iter())
        .map(|p| p.to_string())
        .collect();
    if profile == Profile::Window {
        readable.extend(SYS_READ.iter().map(|p| p.to_string()));
        // Of the session: only the X authority file. The Wayland socket needs no file access
        // (Landlock does not govern connecting to a Unix socket).
        if let Ok(x) = std::env::var("XAUTHORITY") {
            if !x.is_empty() {
                readable.push(x);
            }
        }
    }
    readable
}

/// Restricts this process for good (Landlock, no new privileges): reading only the paths of
/// [`readable_paths`], in the window profile the GPU devices in full, no other file access, no
/// TCP, no signals to processes outside the sandbox; in the embedded profile also no abstract
/// Unix sockets (its system call filter refuses every socket). A kernel without Landlock leaves
/// the process as it is; scopes the kernel does not know make the sandbox partial.
pub fn sandbox(profile: Profile) -> Result<Sandbox, landlock::RulesetError> {
    use landlock::{
        path_beneath_rules, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, Scope, ABI,
    };
    let abi = ABI::V6;
    let read = AccessFs::from_read(abi);
    let readable = readable_paths(profile);
    let scopes = match profile {
        Profile::Embedded => Scope::from_all(abi),
        // X clients reach the server through an abstract socket first.
        Profile::Window => Scope::Signal.into(),
    };
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .handle_access(AccessNet::from_all(abi))?
        .scope(scopes)?
        .create()?
        .add_rules(path_beneath_rules(&readable, read))?
        .add_rules(path_beneath_rules(DEVICE_FILES, read | AccessFs::WriteFile))?;
    let ruleset = if profile == Profile::Window {
        ruleset.add_rules(path_beneath_rules(DEVICES, AccessFs::from_all(abi)))?
    } else {
        ruleset
    };
    let status = ruleset.restrict_self()?;
    Ok(match status.ruleset {
        RulesetStatus::FullyEnforced => Sandbox::Full,
        RulesetStatus::PartiallyEnforced => Sandbox::Partial,
        RulesetStatus::NotEnforced => Sandbox::None,
    })
}

/// System calls the video window never needs and a taken-over decoder could misuse: starting
/// programs, reading or changing other processes, kernel modules, namespaces and mounts, BPF and
/// performance counters, kernel key rings, io_uring (its operations are not seen by seccomp),
/// file handles that bypass path checks, the clock, swap, reboot.
const DENIED_SYSCALLS: &[libc::c_long] = &[
    libc::SYS_execve,
    libc::SYS_execveat,
    libc::SYS_ptrace,
    libc::SYS_process_vm_readv,
    libc::SYS_process_vm_writev,
    libc::SYS_process_madvise,
    libc::SYS_pidfd_getfd,
    libc::SYS_kcmp,
    libc::SYS_init_module,
    libc::SYS_finit_module,
    libc::SYS_delete_module,
    libc::SYS_kexec_load,
    libc::SYS_kexec_file_load,
    libc::SYS_mount,
    libc::SYS_umount2,
    libc::SYS_pivot_root,
    libc::SYS_chroot,
    libc::SYS_unshare,
    libc::SYS_setns,
    libc::SYS_fsopen,
    libc::SYS_fsmount,
    libc::SYS_move_mount,
    libc::SYS_open_tree,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_userfaultfd,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
    libc::SYS_open_by_handle_at,
    libc::SYS_name_to_handle_at,
    libc::SYS_fanotify_init,
    libc::SYS_personality,
    libc::SYS_swapon,
    libc::SYS_swapoff,
    libc::SYS_reboot,
    libc::SYS_settimeofday,
    libc::SYS_clock_settime,
    libc::SYS_acct,
    libc::SYS_quotactl,
];
/// Socket families the window may open: only local (Unix) sockets for the Wayland or X server.
/// Every other family (IP, raw packets, netlink, …) is refused.
const ALLOWED_SOCKET_FAMILY: libc::c_long = libc::AF_UNIX as libc::c_long;

/// The system call filter of the helper (seccomp-BPF): the calls of [`DENIED_SYSCALLS`] and sockets
/// other than Unix sockets fail with EPERM, everything else is allowed. A deny list on top of
/// Landlock (design: the GPU drivers' system calls differ by driver and version, an allow list
/// would break video on some systems).
// `c_long` is `i64` on the 64-bit targets seccompiler knows; the conversion keeps other widths
// compiling.
#[allow(clippy::useless_conversion)]
fn syscall_program(profile: Profile) -> Result<seccompiler::BpfProgram, String> {
    use seccompiler::{
        SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
        SeccompRule, TargetArch,
    };
    let arch = TargetArch::try_from(std::env::consts::ARCH).map_err(|e| e.to_string())?;
    let mut rules: std::collections::BTreeMap<i64, Vec<SeccompRule>> = DENIED_SYSCALLS
        .iter()
        .map(|&n| (i64::from(n), Vec::new()))
        .collect();
    let other_family = SeccompCondition::new(
        0,
        SeccompCmpArgLen::Dword,
        SeccompCmpOp::Ne,
        ALLOWED_SOCKET_FAMILY as u64,
    )
    .and_then(|c| SeccompRule::new(vec![c]))
    .map_err(|e| e.to_string())?;
    match profile {
        // Embedded: no socket at all (an empty rule list matches every call).
        Profile::Embedded => {
            rules.insert(i64::from(libc::SYS_socket), Vec::new());
            rules.insert(i64::from(libc::SYS_socketpair), Vec::new());
        }
        Profile::Window => {
            rules.insert(i64::from(libc::SYS_socket), vec![other_family]);
        }
    }
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|e| e.to_string())?;
    filter
        .try_into()
        .map_err(|e: seccompiler::BackendError| e.to_string())
}

/// Applies [`syscall_program`] to every thread of this process, for good.
pub fn syscall_filter(profile: Profile) -> Result<(), String> {
    let program = syscall_program(profile)?;
    seccompiler::apply_filter_all_threads(&program).map_err(|e| e.to_string())
}

/// Whether the helper may decode with the confinement it got (issue #144): the messages to
/// report, or why it refuses. The system call filter and Landlock are both required; a partial
/// Landlock is taken; `allow_unconfined` (the user's explicit choice) takes anything.
fn confinement(
    landlock: Result<Sandbox, String>,
    filter: Result<(), String>,
    allow_unconfined: bool,
) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let mut missing = Vec::new();
    match landlock {
        Ok(Sandbox::Full) => {}
        Ok(Sandbox::Partial) => notes.push("video window: sandbox only partly available".into()),
        Ok(Sandbox::None) => missing.push("no sandbox (kernel without Landlock)".to_string()),
        Err(e) => missing.push(format!("no sandbox ({})", clean_notice(&e))),
    }
    if let Err(e) = filter {
        missing.push(format!("no system call filter ({})", clean_notice(&e)));
    }
    if missing.is_empty() {
        return Ok(notes);
    }
    if allow_unconfined {
        notes.extend(
            missing
                .into_iter()
                .map(|m| format!("video window: {m}, decoding anyway ({UNCONFINED})")),
        );
        return Ok(notes);
    }
    Err(format!(
        "video window: {}; received video is not decoded unconfined (set {UNCONFINED}=1 to allow it)",
        missing.join(", ")
    ))
}

/// The helper process. Messages for the user go to standard output, one per line.
pub fn helper_main(mut args: impl Iterator<Item = String>) -> i32 {
    // The arguments are only strings; the profile follows from the mode (issue #207).
    let id = args.next().unwrap_or_default();
    let mode = args.next();
    let profile = Profile::of_mode(mode.as_deref());
    // Before any received video is read (issue #144: fail closed).
    let allow = std::env::var(UNCONFINED).is_ok_and(|v| v.trim() == "1");
    match confinement(
        sandbox(profile).map_err(|e| e.to_string()),
        syscall_filter(profile),
        allow,
    ) {
        Ok(notes) => notes.iter().for_each(|n| println!("{n}")),
        Err(refusal) => {
            println!("{refusal}");
            return EXIT_UNCONFINED;
        }
    }
    let title = format!("CommSCraft – call {}", clean_notice(&id));
    if mode.as_deref() == Some("raw") {
        return raw_main(&title);
    }
    if let Some(m) = mode.as_deref().filter(|m| m.starts_with("embed")) {
        return embed_main(m == "embed-raw");
    }
    let mut surfaces = mode.as_deref() == Some("gpu") && Window::in_wayland_session();
    let mut decoder = match vf::init().and_then(|()| vf::Decoder::with_hardware(surfaces)) {
        Ok(d) => d,
        Err(e) => {
            println!("no video decoder: {e:?}");
            return 2;
        }
    };
    let units = read_units(vf::MAX_ACCESS_UNIT);
    let mut surface_window: Option<Window> = None;
    let mut window: Option<vf::Display> = None;
    let mut failed = false;
    let mut announced = false;
    let mut closed = false;
    let mut size = (0, 0);
    while let Some(unit) = next_unit(&units, surface_window.as_mut(), &mut closed) {
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
                    } else if size != (s.width, s.height) {
                        println!("the video is now {}x{}", s.width, s.height);
                    }
                    size = (s.width, s.height);
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
            if announced && size != (p.width, p.height) {
                println!("the video is now {}x{}", p.width, p.height);
            }
            size = (p.width, p.height);
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

fn start(id: u32, mode: &str) -> std::io::Result<Child> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(COMMAND)
        .arg(id.to_string())
        .arg(mode)
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    for name in ENVIRONMENT.iter().chain(std::iter::once(&UNCONFINED)) {
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
            // A picture handed back (embedded mode): its size is checked before it is read;
            // anything that does not fit ends the reading (the helper is not trusted).
            if let Some(head) = line.strip_prefix(FRAME_TAG.as_bytes()) {
                let text = String::from_utf8_lossy(head);
                let mut n = text.split_whitespace().map(|v| v.parse::<usize>().ok());
                let (Some(Some(w)), Some(Some(h)), Some(Some(len))) =
                    (n.next(), n.next(), n.next())
                else {
                    return;
                };
                let max = (EMBED_WIDTH as usize * 2) * (EMBED_HEIGHT as usize * 2) * 4;
                if w == 0 || h == 0 || len != w * h * 4 || len > max {
                    return;
                }
                let mut rgba = vec![0; len];
                if reader.read_exact(&mut rgba).is_err() {
                    return;
                }
                // Dropped when the session loop is behind (issue #253).
                if crate::backlog::FRAMES.enter()
                    && events
                        .send(FromVideo::Frame(id, w as u32, h as u32, rgba))
                        .is_err()
                {
                    crate::backlog::FRAMES.leave();
                }
                continue;
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
    spawn_mode(input, stop, events, id, false);
}

/// The same for raw pictures ([`raw_unit`]): a shared window over RDP (function 43).
pub fn spawn_raw(
    input: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromVideo>,
    id: u32,
) {
    spawn_mode(input, stop, events, id, true);
}

fn spawn_mode(
    input: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
    events: Sender<FromVideo>,
    id: u32,
    raw: bool,
) {
    std::thread::spawn(move || {
        let notice = |text: String| {
            let _ = events.send(FromVideo::Notice(id, text));
        };
        let mut gpu = gpu_wanted();
        let mut input = Some(input);
        let mut restarts = 0;
        loop {
            let embedded = EMBEDDED.load(Ordering::Relaxed);
            let mode = match (embedded, raw, gpu) {
                (true, true, _) => "embed-raw",
                (true, false, _) => "embed",
                (false, true, _) => "raw",
                (false, false, true) => "gpu",
                (false, false, false) => "cpu",
            };
            let mut child = match start(id, mode) {
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
            // It refused to decode without its confinement: starting it again would not help.
            if status.and_then(|s| s.code()) == Some(EXIT_UNCONFINED) {
                notice("no received video: the video window cannot be confined".into());
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
    #[test]
    fn an_i420_picture_becomes_bounded_rgba() {
        // 4×2 mid grey (Y 126, neutral chroma) → grey, alpha opaque; too little data → None.
        let p = vf::Picture {
            width: 4,
            height: 2,
            data: [vec![126u8; 8], vec![128u8; 2], vec![128u8; 2]].concat(),
        };
        let (w, h, rgba) = rgba_of_i420(&p, 640, 360).unwrap();
        assert_eq!((w, h), (4, 2));
        assert_eq!(rgba.len(), 4 * 2 * 4);
        assert_eq!(&rgba[..4], &[128, 128, 128, 255]);
        let big = vf::Picture {
            width: 1280,
            height: 720,
            data: vec![16; vf::Picture::i420_len(1280, 720)],
        };
        let (w, h, _) = rgba_of_i420(&big, EMBED_WIDTH, EMBED_HEIGHT).unwrap();
        assert!(w <= EMBED_WIDTH && h <= EMBED_HEIGHT);
        let short = vf::Picture {
            width: 4,
            height: 2,
            data: vec![0; 3],
        };
        assert!(rgba_of_i420(&short, 640, 360).is_none());
    }

    use super::*;

    /// The system call filter, applied to one thread only: starting a program and IP sockets are
    /// refused, Unix sockets still work; other threads are not affected.
    #[test]
    fn syscall_filter_refuses_what_the_window_never_needs() {
        let program = syscall_program(Profile::Window).unwrap();
        assert!(!program.is_empty());
        let filtered = std::thread::spawn(move || {
            seccompiler::apply_filter(&program).unwrap();
            let exec = std::process::Command::new("/bin/true").status();
            let udp = std::net::UdpSocket::bind("127.0.0.1:0");
            let tcp = std::net::TcpListener::bind("127.0.0.1:0");
            let unix = std::os::unix::net::UnixDatagram::unbound();
            (exec.is_err(), udp.is_err(), tcp.is_err(), unix.is_ok())
        })
        .join()
        .unwrap();
        assert_eq!(filtered, (true, true, true, true));
        // This thread has no filter.
        assert!(std::net::UdpSocket::bind("127.0.0.1:0").is_ok());
    }

    /// Issue #207: the embedded helper may open no socket at all, not even a Unix socket.
    #[test]
    fn embedded_helper_gets_no_socket() {
        let program = syscall_program(Profile::Embedded).unwrap();
        let filtered = std::thread::spawn(move || {
            seccompiler::apply_filter(&program).unwrap();
            let unix = std::os::unix::net::UnixDatagram::unbound();
            let pair = std::os::unix::net::UnixStream::pair();
            let udp = std::net::UdpSocket::bind("127.0.0.1:0");
            (unix.is_err(), pair.is_err(), udp.is_err())
        })
        .join()
        .unwrap();
        assert_eq!(filtered, (true, true, true));
        assert_eq!(Profile::of_mode(Some("embed")), Profile::Embedded);
        assert_eq!(Profile::of_mode(Some("embed-raw")), Profile::Embedded);
        assert_eq!(Profile::of_mode(Some("gpu")), Profile::Window);
        assert_eq!(Profile::of_mode(None), Profile::Window);
    }

    /// Shared screens: RGBA becomes BGRX, kept when it fits, scaled down with the aspect kept;
    /// data that does not fit the size is refused.
    #[test]
    fn shared_screen_pixels() {
        let rgba: Vec<u8> = std::iter::repeat_n([10u8, 20, 30, 255], 4)
            .flatten()
            .collect();
        let (w, h, px) = scaled_bgrx(2, 2, &rgba).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(px.get(..4), Some(&[30u8, 20, 10, 255][..]));
        let big = vec![0u8; 3840 * 2160 * 4];
        let (w, h, px) = scaled_bgrx(3840, 2160, &big).unwrap();
        assert_eq!((w, h), (2560, 1440));
        assert_eq!(px.len(), 2560 * 1440 * 4);
        assert!(scaled_bgrx(2, 2, &rgba[..15]).is_none());
    }

    /// The software window's I420 (BT.601 limited range): white, black, pure red; sizes made
    /// even and fitted.
    #[test]
    fn shared_screen_i420() {
        let solid = |r: u8, g: u8, b: u8| {
            let px: Vec<u8> = std::iter::repeat_n([b, g, r, 255], 16 * 16)
                .flatten()
                .collect();
            let p = i420_of_bgrx(16, 16, &px, 1920, 1088).unwrap();
            (p.data[0], p.data[256], p.data[256 + 64])
        };
        assert_eq!(solid(255, 255, 255), (235, 128, 128));
        assert_eq!(solid(0, 0, 0), (16, 128, 128));
        assert_eq!(solid(255, 0, 0), (82, 90, 240));
        let px = vec![0u8; 2560 * 1440 * 4];
        let p = i420_of_bgrx(2560, 1440, &px, 1920, 1088).unwrap();
        assert_eq!((p.width, p.height), (1920, 1080));
        let px = vec![0u8; 801 * 601 * 4];
        let p = i420_of_bgrx(801, 601, &px, 1920, 1088).unwrap();
        assert_eq!((p.width, p.height), (800, 600));
        assert!(i420_of_bgrx(8, 8, &[0; 256], 1920, 1088).is_none());
    }

    /// Issue #144: without the system call filter or Landlock nothing is decoded unless the user
    /// allows it; a partial Landlock is taken with a note.
    #[test]
    fn confinement_fails_closed() {
        let ok = || Ok(());
        let no_filter = || Err("EINVAL".to_string());
        assert_eq!(confinement(Ok(Sandbox::Full), ok(), false), Ok(vec![]));
        assert_eq!(
            confinement(Ok(Sandbox::Partial), ok(), false).map(|n| n.len()),
            Ok(1)
        );
        assert!(confinement(Ok(Sandbox::Full), no_filter(), false).is_err());
        assert!(confinement(Ok(Sandbox::None), ok(), false).is_err());
        assert!(confinement(Err("ENOSYS".into()), ok(), false).is_err());
        let refusal = confinement(Ok(Sandbox::None), no_filter(), false).unwrap_err();
        assert!(refusal.contains("Landlock") && refusal.contains("system call filter"));
        assert!(refusal.contains(UNCONFINED));
        // Allowed by the user: decoded, with both gaps reported.
        assert_eq!(
            confinement(Ok(Sandbox::None), no_filter(), true).map(|n| n.len()),
            Ok(2)
        );
    }

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
        // Raw pictures: size and length must agree, the size is bounded.
        let px = vec![7u8; 16 * 16 * 4];
        let unit = raw_unit(16, 16, &px);
        assert_eq!(raw_of(&unit), Some((16, 16, &px[..])));
        assert_eq!(raw_of(&unit[..unit.len() - 1]), None);
        let mut huge = unit.clone();
        huge.splice(..4, (MAX_RAW_WIDTH + 1).to_be_bytes());
        assert_eq!(raw_of(&huge), None);
    }

    /// The sandbox (checked in a child process, since it cannot be undone): the user's files and TCP
    /// are out of reach, the system's files are not.
    #[test]
    fn sandbox_keeps_the_users_files_out() {
        const CHILD: &str = "COMMSCRAFT_SANDBOX_TEST_FILE";
        if let Ok(path) = std::env::var(CHILD) {
            let listener = std::env::var("COMMSCRAFT_SANDBOX_TEST_PORT").unwrap();
            let runtime = std::env::var("COMMSCRAFT_SANDBOX_TEST_RUNTIME").unwrap();
            if sandbox(Profile::Window).unwrap() == Sandbox::None {
                println!("SANDBOX-UNAVAILABLE");
                return;
            }
            // Issue #207: nothing of the session's runtime directory, no other process.
            assert!(std::fs::read(&runtime).is_err(), "the runtime directory");
            assert!(
                std::fs::read_to_string("/proc/1/status").is_err(),
                "another process"
            );
            assert!(
                std::fs::read_to_string("/proc/self/status").is_ok(),
                "itself"
            );
            assert!(
                std::fs::read(&path).is_err(),
                "a file outside the system paths"
            );
            assert!(std::fs::write("/usr/commscraft-sandbox-test", b"x").is_err());
            assert!(
                std::fs::read("/etc/hostname").is_ok() || std::fs::read("/etc/os-release").is_ok()
            );
            assert!(
                std::net::TcpStream::connect(listener.as_str()).is_err(),
                "TCP"
            );
            println!("SANDBOX-OK");
            return;
        }
        let dir = std::env::temp_dir().join(format!("commscraft-sandbox-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let secret = dir.join("secret");
        std::fs::write(&secret, b"password").unwrap();
        // A stand-in for a file in the runtime directory the helper would see.
        let runtime = dir.join("runtime");
        std::fs::create_dir_all(&runtime).unwrap();
        let runtime_file = runtime.join("bus-token");
        std::fs::write(&runtime_file, b"token").unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "video_window::tests::sandbox_keeps_the_users_files_out",
                "--nocapture",
            ])
            .env(CHILD, &secret)
            .env("COMMSCRAFT_SANDBOX_TEST_RUNTIME", &runtime_file)
            // The old rules made all of it readable; it must stay out of reach now.
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "COMMSCRAFT_SANDBOX_TEST_PORT",
                listener.local_addr().unwrap().to_string(),
            )
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(
            text.contains("SANDBOX-OK") || text.contains("SANDBOX-UNAVAILABLE"),
            "{text}"
        );
        eprintln!(
            "sandbox: {}",
            if text.contains("SANDBOX-OK") {
                "enforced"
            } else {
                "not available here"
            }
        );
    }

    /// Issue #207: the embedded helper still decodes under its tighter confinement – the real
    /// `helper_main` in a child process, fed an H.264 stream on standard input, answers with
    /// pictures on standard output (or reports that the kernel has no sandbox).
    #[test]
    fn embedded_helper_decodes_under_its_confinement() {
        const CHILD: &str = "COMMSCRAFT_EMBED_TEST_CHILD";
        if std::env::var(CHILD).is_ok() {
            std::process::exit(helper_main(
                ["1".to_string(), "embed".to_string()].into_iter(),
            ));
        }
        let Ok(mut enc) = vf::init().and_then(|()| {
            vf::Encoder::new(vf::EncoderConfig {
                width: 320,
                height: 180,
                fps: 15,
                bitrate: 300_000,
                idr_interval: 30,
                hardware: false,
            })
        }) else {
            eprintln!("no H.264 encoder here; skipping");
            return;
        };
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "video_window::tests::embedded_helper_decodes_under_its_confinement",
            ])
            .env(CHILD, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for i in 0..20u8 {
            let p = vf::Picture::filled(320, 180, 16 + i * 8, 128, 128);
            for unit in enc.encode(&p, i == 0).unwrap() {
                write_unit(&mut stdin, &unit).unwrap();
            }
            std::thread::sleep(Duration::from_millis(70));
        }
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        // (The helper's notices go through println!, which the test harness captures.)
        if out.status.code() == Some(EXIT_UNCONFINED) {
            eprintln!("no Landlock here; the helper refused as it should");
            return;
        }
        assert!(
            out.stdout.windows(14).any(|w| w == b"FRAME 320 180 "),
            "{:?} {}",
            out.status,
            String::from_utf8_lossy(&out.stdout)
                .chars()
                .take(300)
                .collect::<String>()
        );
    }

    /// Writes a short H.264 stream in the pipe format to `COMMSCRAFT_UNITS_OUT` (manual check of
    /// the helper with a real window: `commscraft video-window 1 < file`).
    #[test]
    #[ignore]
    fn write_units_for_a_manual_check() {
        let path = std::env::var("COMMSCRAFT_UNITS_OUT").unwrap();
        vf::init().unwrap();
        let mut enc = vf::Encoder::new(vf::EncoderConfig {
            width: 320,
            height: 180,
            fps: 15,
            bitrate: 300_000,
            idr_interval: 30,
            hardware: false,
        })
        .unwrap();
        let mut out = Vec::new();
        for i in 0..60u8 {
            let p = vf::Picture::filled(320, 180, 16 + (i % 30) * 6, 128, 128);
            for unit in enc.encode(&p, i == 0).unwrap() {
                write_unit(&mut out, &unit).unwrap();
            }
        }
        std::fs::write(path, out).unwrap();
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
