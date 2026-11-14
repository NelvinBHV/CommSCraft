//! The video window in a process of its own (`commscraft-cli video-window ID gpu|cpu`): it decodes
//! the received video and shows it; the client hands it access units over a pipe.
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
fn raw_main(title: &str) -> i32 {
    let mut input = std::io::stdin().lock();
    let mut wayland: Option<Window> = None;
    let mut use_wayland = Window::in_wayland_session();
    let mut window: Option<vf::Display> = None;
    let mut size = (0, 0);
    let mut closed = false;
    while let Ok(Some(unit)) = read_bounded(&mut input, MAX_RAW_UNIT) {
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

/// Read-only for the helper: programs, libraries, drivers, system configuration, and what the kernel
/// tells about devices.
const READ_ONLY: [&str; 6] = ["/usr", "/lib", "/lib64", "/etc", "/sys", "/proc"];
/// The GPU's render devices: open, read, write and control.
const DEVICES: [&str; 1] = ["/dev/dri"];
/// Single device files libraries read.
const DEVICE_FILES: [&str; 3] = ["/dev/null", "/dev/zero", "/dev/urandom"];

/// How far the sandbox could be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Full,
    /// The kernel knows only part of the rules.
    Partial,
    /// The kernel has no Landlock: the helper runs without it.
    None,
}

/// Restricts this process for good (Landlock, no new privileges): reading only the system paths
/// above and the window's runtime directory (Wayland socket, X authority file), the GPU devices in
/// full, no other file access, no TCP. Unix sockets, which the window needs, are not restricted.
/// A kernel without Landlock leaves the process as it is.
pub fn sandbox() -> Result<Sandbox, landlock::RulesetError> {
    use landlock::{
        path_beneath_rules, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, ABI,
    };
    let abi = ABI::V5;
    let read = AccessFs::from_read(abi);
    let mut readable: Vec<String> = READ_ONLY.iter().map(|p| p.to_string()).collect();
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        readable.push(runtime);
    }
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .handle_access(AccessNet::from_all(abi))?
        .create()?
        .add_rules(path_beneath_rules(&readable, read))?
        .add_rules(path_beneath_rules(DEVICES, AccessFs::from_all(abi)))?
        .add_rules(path_beneath_rules(DEVICE_FILES, read | AccessFs::WriteFile))?
        .restrict_self()?;
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
fn syscall_program() -> Result<seccompiler::BpfProgram, String> {
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
    rules.insert(i64::from(libc::SYS_socket), vec![other_family]);
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
pub fn syscall_filter() -> Result<(), String> {
    let program = syscall_program()?;
    seccompiler::apply_filter_all_threads(&program).map_err(|e| e.to_string())
}

/// The helper process. Messages for the user go to standard output, one per line.
pub fn helper_main(mut args: impl Iterator<Item = String>) -> i32 {
    // Before any received video is read. A sandbox that cannot be set up is reported; the helper
    // still runs, as it did before the sandbox existed (design: the call matters more).
    match sandbox() {
        Ok(Sandbox::Full) => {}
        Ok(Sandbox::Partial) => println!("video window: sandbox only partly available"),
        Ok(Sandbox::None) => println!("video window: no sandbox (kernel without Landlock)"),
        Err(e) => println!("video window: no sandbox ({e})"),
    }
    if let Err(e) = syscall_filter() {
        println!("video window: no system call filter ({})", clean_notice(&e));
    }
    let id = args.next().unwrap_or_default();
    let mode = args.next();
    let title = format!("CommSCraft – call {}", clean_notice(&id));
    if mode.as_deref() == Some("raw") {
        return raw_main(&title);
    }
    let mut surfaces = mode.as_deref() == Some("gpu") && Window::in_wayland_session();
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
    let mut size = (0, 0);
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
            let mode = match (raw, gpu) {
                (true, _) => "raw",
                (false, true) => "gpu",
                (false, false) => "cpu",
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

    /// The system call filter, applied to one thread only: starting a program and IP sockets are
    /// refused, Unix sockets still work; other threads are not affected.
    #[test]
    fn syscall_filter_refuses_what_the_window_never_needs() {
        let program = syscall_program().unwrap();
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
            if sandbox().unwrap() == Sandbox::None {
                println!("SANDBOX-UNAVAILABLE");
                return;
            }
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
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "video_window::tests::sandbox_keeps_the_users_files_out",
                "--nocapture",
            ])
            .env(CHILD, &secret)
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
