//! A video window on Wayland that shows GPU pictures without copying them: the picture's DMA-BUF
//! goes to the compositor (`zwp_linux_dmabuf_v1`), which puts it on the screen.
//!
//! Evidence: decoding on the GPU costs little, copying the pictures back costs more than decoding
//! on the CPU (measured, evidence file 23), so only this path makes GPU decoding worthwhile.
//!
//! Security by design: no `unsafe` code and no C library; picture sizes are bounded by the decoder;
//! only format and modifier pairs the compositor announced are offered, so it has no reason to end
//! the connection; at most [`MAX_IN_FLIGHT`] pictures wait for the compositor, later ones are
//! skipped, and the cache of buffers is bounded. Every failure is an error, and the caller
//! falls back to the window of the software path.
//!
//! The title bar: the compositor draws one where it offers server-side decorations
//! (`xdg-decoration`); otherwise, as on GNOME, the window draws a plain bar above the picture as a
//! subsurface: dragging it moves the window, a double click or the square next to the cross
//! maximizes it (and back), the cross at its right end closes it.
//!
//! Size: the compositor scales the picture to the window (`wp_viewporter`, no copy), keeping its
//! aspect ratio; the window can be resized by dragging its bottom right corner, maximized, or
//! resized by the compositor's own means.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::os::fd::{AsFd, BorrowedFd};

use wayland_client::globals::{registry_queue_init, BindError, GlobalList, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::{
    self, ZwpLinuxBufferParamsV1,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::{
    self, ZwpLinuxDmabufV1,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    zxdg_toplevel_decoration_v1::{self, ZxdgToplevelDecorationV1},
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

/// Pictures waiting for the compositor before new ones are skipped (design: one on the screen, one
/// or two on their way).
pub const MAX_IN_FLIGHT: usize = 4;
/// Buffers kept for reuse. A decoder cycles through a small pool of surfaces.
const MAX_BUFFERS: usize = 64;
/// Height of the title bar, and width of its close area at the right end.
pub const BAR: u32 = 28;
/// Key of the bar's buffer: none of the pictures uses it.
const BAR_KEY: u64 = u64::MAX;
/// Left mouse button (input-event-codes.h).
const BTN_LEFT: u32 = 0x110;
/// The corner of the picture that resizes the window when dragged (pixels).
pub const GRIP: u32 = 24;
/// Two presses on the bar within this time are a double click (milliseconds).
const DOUBLE_CLICK_MS: u32 = 400;
/// The smallest picture shown (design: the bar's buttons still fit).
const MIN_SIDE: u32 = 120;

/// The size a picture is shown in: as large as fits into the window's content area (`area`,
/// the bar not counted), keeping the aspect ratio; the picture's own size while the compositor
/// leaves the size to us (`None` or a zero side).
pub fn fit(picture: (u32, u32), area: Option<(u32, u32)>) -> (u32, u32) {
    let (pw, ph) = (picture.0.max(1), picture.1.max(1));
    let Some((aw, ah)) = area.filter(|(w, h)| *w > 0 && *h > 0) else {
        return (pw, ph);
    };
    let (aw, ah) = (aw.max(MIN_SIDE), ah.max(MIN_SIDE));
    // Width-bound or height-bound, in 64 bits (no overflow).
    let by_width = (aw, (u64::from(aw) * u64::from(ph) / u64::from(pw)) as u32);
    if by_width.1 <= ah {
        (by_width.0, by_width.1.max(1))
    } else {
        (
            ((u64::from(ah) * u64::from(pw) / u64::from(ph)) as u32).max(1),
            ah,
        )
    }
}

/// The bar's pixels, ARGB8888 in little-endian byte order: the CommSCraft ink colour with a cream
/// cross in the close area.
pub fn draw_bar(width: u32) -> Vec<u8> {
    let (ink, cream) = ([0x2a, 0x24, 0x10, 0xff], [0xde, 0xf4, 0xff, 0xff]);
    let close_from = width.saturating_sub(BAR);
    let mut pixels = Vec::with_capacity((width * BAR * 4) as usize);
    for y in 0..BAR {
        for x in 0..width {
            // The cross: both diagonals of the close area, 2 pixels wide, with a margin of 9.
            let cross = x.checked_sub(close_from).is_some_and(|cx| {
                (9..BAR - 9).contains(&cx)
                    && (9..BAR - 9).contains(&y)
                    && (cx.abs_diff(y) <= 1 || (BAR - 1 - cx).abs_diff(y) <= 1)
            });
            // The maximize square left of it: an outline, 2 pixels wide, with a margin of 9.
            let square = width >= 2 * BAR
                && x.checked_sub(close_from.saturating_sub(BAR))
                    .filter(|_| x < close_from)
                    .is_some_and(|sx| {
                        let inside = (9..BAR - 9).contains(&sx) && (9..BAR - 9).contains(&y);
                        let edge = !(11..BAR - 11).contains(&sx) || !(11..BAR - 11).contains(&y);
                        inside && edge
                    });
            pixels.extend_from_slice(if cross || square { &cream } else { &ink });
        }
    }
    pixels
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No Wayland session or a compositor without what is needed.
    NoCompositor(String),
    /// The compositor does not take this picture format.
    Unsupported,
    /// The connection failed or the compositor refused a buffer.
    Connection(String),
    /// The user closed the window.
    Closed,
}

/// One plane of a picture.
#[derive(Debug, Clone, Copy)]
pub struct Plane<'a> {
    pub fd: BorrowedFd<'a>,
    pub offset: u32,
    pub stride: u32,
}

/// A picture in GPU memory.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    /// DRM format code.
    pub fourcc: u32,
    pub modifier: u64,
    pub planes: &'a [Plane<'a>],
    /// Identifies the memory: the same key means the same picture buffer.
    pub key: u64,
}

struct State {
    formats: HashSet<(u32, u64)>,
    wm_base: xdg_wm_base::XdgWmBase,
    configured: bool,
    closed: bool,
    failed: bool,
    /// What keeps the pictures alive that the compositor still uses, by key.
    in_flight: HashMap<u64, Box<dyn Any>>,
    /// The compositor draws the title bar.
    server_side: bool,
    pointer: Option<wl_pointer::WlPointer>,
    /// The pointer is over our bar, at this x.
    on_bar: bool,
    pointer_x: f64,
    bar: Option<wayland_client::backend::ObjectId>,
    bar_width: u32,
    /// A press on the bar: the window is to be moved (serial of the press).
    move_request: Option<u32>,
    /// The picture's surface, and the pointer on it.
    picture: Option<wayland_client::backend::ObjectId>,
    on_picture: bool,
    pointer_y: f64,
    /// The size the picture is shown in.
    shown: (u32, u32),
    /// A press on the corner: the window is to be resized (serial of the press).
    resize_request: Option<u32>,
    /// Maximize (true) or restore (false), asked from the bar.
    maximize_request: Option<bool>,
    maximized: bool,
    /// The window size the compositor asked for (0: ours to choose).
    asked: (i32, i32),
    /// The time of the last press on the bar (for the double click).
    last_bar_press: Option<u32>,
}

/// Our own title bar.
struct Bar {
    surface: wl_surface::WlSurface,
    subsurface: wl_subsurface::WlSubsurface,
    shm: wl_shm::WlShm,
    pool: Option<(wl_shm_pool::WlShmPool, wl_buffer::WlBuffer, std::fs::File)>,
    width: u32,
}

/// The window.
pub struct Window {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    dmabuf: ZwpLinuxDmabufV1,
    surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
    toplevel: xdg_toplevel::XdgToplevel,
    seat: Option<wl_seat::WlSeat>,
    bar: Option<Bar>,
    _decoration: Option<ZxdgToplevelDecorationV1>,
    viewport: Option<wp_viewport::WpViewport>,
    /// The size of the last picture, and the window size its layout was made for.
    picture_size: (u32, u32),
    laid_out_for: (i32, i32),
    /// Buffers by key, for the size they were made for.
    buffers: HashMap<u64, wl_buffer::WlBuffer>,
    size: (u32, u32),
    /// Shared memory for pictures from memory ([`Window::show_pixels`]) and their next key.
    shm: Option<wl_shm::WlShm>,
    next_pixels_key: u64,
}

/// A picture from memory while the compositor reads it: buffer, pool and memfd go together and
/// are destroyed when the compositor releases the buffer.
struct PixelsBuffer {
    buffer: wl_buffer::WlBuffer,
    pool: wl_shm_pool::WlShmPool,
    _file: std::fs::File,
}

impl Drop for PixelsBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.pool.destroy();
    }
}

/// Keys of pictures from memory: apart from the inodes that key GPU buffers.
const PIXELS_KEY_BASE: u64 = 1 << 62;
/// Largest side of a picture from memory (design: the shared screens of window sharing).
pub const MAX_PIXELS_SIDE: u32 = 8192;

fn connection_error(e: impl std::fmt::Display) -> Error {
    Error::Connection(e.to_string())
}

fn bind_error(e: BindError) -> Error {
    Error::NoCompositor(e.to_string())
}

impl Window {
    /// Whether this is a Wayland session at all.
    pub fn in_wayland_session() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    pub fn open(title: &str) -> Result<Window, Error> {
        let conn = Connection::connect_to_env().map_err(|e| Error::NoCompositor(e.to_string()))?;
        let (globals, mut queue) =
            registry_queue_init::<State>(&conn).map_err(|e| Error::NoCompositor(e.to_string()))?;
        let qh = queue.handle();
        let compositor: wl_compositor::WlCompositor =
            globals.bind(&qh, 4..=4, ()).map_err(bind_error)?;
        let wm_base: xdg_wm_base::XdgWmBase = globals.bind(&qh, 1..=1, ()).map_err(bind_error)?;
        // Version 3 announces format and modifier pairs as events.
        let dmabuf: ZwpLinuxDmabufV1 = globals.bind(&qh, 3..=3, ()).map_err(bind_error)?;
        let surface = compositor.create_surface(&qh, ());
        let xdg = wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg.get_toplevel(&qh, ());
        toplevel.set_title(title.to_string());
        toplevel.set_app_id("commscraft".to_string());
        // A frame from the compositor where it offers one.
        let decoration = globals
            .bind::<ZxdgDecorationManagerV1, _, _>(&qh, 1..=1, ())
            .ok()
            .map(|m| {
                let d = m.get_toplevel_decoration(&toplevel, &qh, ());
                d.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
                d
            });
        let seat: Option<wl_seat::WlSeat> = globals.bind(&qh, 1..=5, ()).ok();
        let shm: Option<wl_shm::WlShm> = globals.bind(&qh, 1..=1, ()).ok();
        let bar = Bar::new(&globals, &compositor, &surface, &qh);
        let viewport = globals
            .bind::<wp_viewporter::WpViewporter, _, _>(&qh, 1..=1, ())
            .ok()
            .map(|v| v.get_viewport(&surface, &qh, ()));
        surface.commit();
        let mut state = State {
            formats: HashSet::new(),
            wm_base,
            configured: false,
            closed: false,
            failed: false,
            in_flight: HashMap::new(),
            server_side: false,
            pointer: None,
            on_bar: false,
            pointer_x: 0.0,
            bar: bar.as_ref().map(|b| b.surface.id()),
            bar_width: 0,
            move_request: None,
            picture: Some(surface.id()),
            on_picture: false,
            pointer_y: 0.0,
            shown: (0, 0),
            resize_request: None,
            maximize_request: None,
            maximized: false,
            asked: (0, 0),
            last_bar_press: None,
        };
        // The formats, then the first configure, which must come before the first picture.
        for _ in 0..4 {
            queue.roundtrip(&mut state).map_err(connection_error)?;
            if state.configured {
                break;
            }
        }
        if !state.configured {
            return Err(Error::NoCompositor("window not configured".into()));
        }
        Ok(Window {
            conn,
            queue,
            state,
            dmabuf,
            surface,
            xdg_surface: xdg,
            toplevel,
            seat,
            bar,
            _decoration: decoration,
            viewport,
            picture_size: (0, 0),
            laid_out_for: (0, 0),
            buffers: HashMap::new(),
            size: (0, 0),
            shm,
            next_pixels_key: PIXELS_KEY_BASE,
        })
    }

    /// Shows a picture from memory: `xrgb` holds `height` rows of `width` pixels in XRGB8888
    /// (little-endian: blue, green, red, unused), written into a memfd with ordinary writes and
    /// handed to the compositor through `wl_shm` (shared screens over RDP, function 43).
    /// `Ok(false)`: the compositor is behind, the picture was skipped.
    pub fn show_pixels(&mut self, width: u32, height: u32, xrgb: &[u8]) -> Result<bool, Error> {
        self.pump()?;
        let stride = width.checked_mul(4).ok_or(Error::Unsupported)?;
        let len = (stride as usize)
            .checked_mul(height as usize)
            .ok_or(Error::Unsupported)?;
        if width == 0
            || height == 0
            || width > MAX_PIXELS_SIDE
            || height > MAX_PIXELS_SIDE
            || xrgb.len() != len
        {
            return Err(Error::Unsupported);
        }
        if self.state.in_flight.len() >= MAX_IN_FLIGHT {
            return Ok(false);
        }
        let shm = self.shm.as_ref().ok_or(Error::Unsupported)?;
        let size = i32::try_from(len).map_err(|_| Error::Unsupported)?;
        let (w, h, s) = (
            i32::try_from(width).map_err(|_| Error::Unsupported)?,
            i32::try_from(height).map_err(|_| Error::Unsupported)?,
            i32::try_from(stride).map_err(|_| Error::Unsupported)?,
        );
        let fd = rustix::fs::memfd_create("commscraft-picture", rustix::fs::MemfdFlags::CLOEXEC)
            .map_err(connection_error)?;
        let mut file = std::fs::File::from(fd);
        file.write_all(xrgb).map_err(connection_error)?;
        let qh = self.queue.handle();
        let key = self.next_pixels_key;
        self.next_pixels_key = self.next_pixels_key.wrapping_add(1).max(PIXELS_KEY_BASE);
        let pool = shm.create_pool(file.as_fd(), size, &qh, ());
        let buffer = pool.create_buffer(0, w, h, s, wl_shm::Format::Xrgb8888, &qh, key);
        self.layout((width, height), &qh);
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, w, h);
        self.surface.commit();
        self.state.in_flight.insert(
            key,
            Box::new(PixelsBuffer {
                buffer,
                pool,
                _file: file,
            }),
        );
        self.pump()?;
        Ok(true)
    }

    /// The size the picture is shown in, our own bar above it unless the compositor draws one,
    /// and the window geometry around both.
    fn layout(&mut self, picture: (u32, u32), qh: &QueueHandle<State>) {
        self.picture_size = picture;
        self.laid_out_for = self.state.asked;
        let own_bar = self.bar.is_some() && !self.state.server_side;
        let (aw, ah) = self.state.asked;
        let area = (self.viewport.is_some() && aw > 0 && ah > 0).then(|| {
            let bar = if own_bar { BAR } else { 0 };
            (aw as u32, (ah as u32).saturating_sub(bar))
        });
        let shown = fit(picture, area);
        if let Some(v) = &self.viewport {
            let (w, h) = (shown.0.min(i32::MAX as u32), shown.1.min(i32::MAX as u32));
            v.set_destination(w as i32, h as i32);
        }
        let (w, h) = (shown.0 as i32, shown.1 as i32);
        if let Some(bar) = self.bar.as_mut().filter(|_| own_bar) {
            if bar.width != shown.0 && bar.draw(shown.0, qh).is_ok() {
                self.state.bar_width = shown.0;
            }
            if self.state.shown != shown {
                self.xdg_surface
                    .set_window_geometry(0, -(BAR as i32), w, h + BAR as i32);
            }
        } else if self.state.shown != shown {
            self.xdg_surface.set_window_geometry(0, 0, w, h);
        }
        self.state.shown = shown;
    }

    /// Handles the user's input and the compositor's requests while no picture comes.
    pub fn idle(&mut self) -> Result<(), Error> {
        self.pump()
    }

    /// Handles what the compositor sent, without waiting.
    fn pump(&mut self) -> Result<(), Error> {
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(connection_error)?;
        // WouldBlock only means there is nothing to send or read now.
        let _ = self.conn.flush();
        if let Some(guard) = self.queue.prepare_read() {
            let _ = guard.read();
        }
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(connection_error)?;
        if let (Some(serial), Some(seat)) = (self.state.move_request.take(), self.seat.as_ref()) {
            self.toplevel._move(seat, serial);
            let _ = self.conn.flush();
        }
        if let (Some(serial), Some(seat)) = (self.state.resize_request.take(), self.seat.as_ref()) {
            self.toplevel
                .resize(seat, serial, xdg_toplevel::ResizeEdge::BottomRight);
            let _ = self.conn.flush();
        }
        match self.state.maximize_request.take() {
            Some(true) => self.toplevel.set_maximized(),
            Some(false) => self.toplevel.unset_maximized(),
            None => {}
        }
        // A new window size: the shown picture follows at once (a still screen sends none).
        if self.state.asked != self.laid_out_for && self.picture_size != (0, 0) {
            let qh = self.queue.handle();
            self.layout(self.picture_size, &qh);
            self.surface.commit();
            let _ = self.conn.flush();
        }
        if let Some(e) = self.conn.protocol_error() {
            return Err(Error::Connection(e.to_string()));
        }
        if self.state.failed {
            return Err(Error::Connection("buffer refused".into()));
        }
        if self.state.closed {
            return Err(Error::Closed);
        }
        Ok(())
    }

    /// Shows a picture; `keep` stays alive until the compositor is done with it. `Ok(false)`: the
    /// compositor is behind, the picture was skipped.
    pub fn show(&mut self, frame: &Frame<'_>, keep: Box<dyn Any>) -> Result<bool, Error> {
        self.pump()?;
        if !self.state.formats.contains(&(frame.fourcc, frame.modifier)) {
            return Err(Error::Unsupported);
        }
        if self.state.in_flight.len() >= MAX_IN_FLIGHT
            || self.state.in_flight.contains_key(&frame.key)
        {
            return Ok(false);
        }
        let size = (frame.width, frame.height);
        let width = i32::try_from(frame.width).map_err(|_| Error::Unsupported)?;
        let height = i32::try_from(frame.height).map_err(|_| Error::Unsupported)?;
        if self.size != size || self.buffers.len() >= MAX_BUFFERS {
            for (key, b) in std::mem::take(&mut self.buffers) {
                if self.state.in_flight.contains_key(&key) {
                    // The compositor still uses it; its release finds the object gone with the
                    // connection at the latest. Only the handle is dropped here.
                    continue;
                }
                b.destroy();
            }
            self.size = size;
        }
        let qh = self.queue.handle();
        if !self.buffers.contains_key(&frame.key) {
            let params = self.dmabuf.create_params(&qh, ());
            for (index, p) in (0u32..).zip(frame.planes) {
                params.add(
                    p.fd,
                    index,
                    p.offset,
                    p.stride,
                    (frame.modifier >> 32) as u32,
                    frame.modifier as u32,
                );
            }
            let buffer = params.create_immed(
                width,
                height,
                frame.fourcc,
                zwp_linux_buffer_params_v1::Flags::empty(),
                &qh,
                frame.key,
            );
            params.destroy();
            self.buffers.insert(frame.key, buffer);
        }
        self.layout((frame.width, frame.height), &qh);
        let Some(buffer) = self.buffers.get(&frame.key) else {
            return Ok(false);
        };
        self.surface.attach(Some(buffer), 0, 0);
        self.surface.damage_buffer(0, 0, width, height);
        self.surface.commit();
        self.state.in_flight.insert(frame.key, keep);
        self.pump()?;
        Ok(true)
    }
}

impl Bar {
    fn new(
        globals: &GlobalList,
        compositor: &wl_compositor::WlCompositor,
        parent: &wl_surface::WlSurface,
        qh: &QueueHandle<State>,
    ) -> Option<Bar> {
        let shm: wl_shm::WlShm = globals.bind(qh, 1..=1, ()).ok()?;
        let sub: wl_subcompositor::WlSubcompositor = globals.bind(qh, 1..=1, ()).ok()?;
        let surface = compositor.create_surface(qh, ());
        let subsurface = sub.get_subsurface(&surface, parent, qh, ());
        subsurface.set_position(0, -(BAR as i32));
        Some(Bar {
            surface,
            subsurface,
            shm,
            pool: None,
            width: 0,
        })
    }

    /// Draws the bar for a picture of this width. The pixels go into a memfd with ordinary writes.
    fn draw(&mut self, width: u32, qh: &QueueHandle<State>) -> std::io::Result<()> {
        let pixels = draw_bar(width);
        let size = i32::try_from(pixels.len()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        let stride = i32::try_from(width * 4).map_err(|_| std::io::ErrorKind::InvalidInput)?;
        let fd = rustix::fs::memfd_create("commscraft-bar", rustix::fs::MemfdFlags::CLOEXEC)?;
        let mut file = std::fs::File::from(fd);
        file.write_all(&pixels)?;
        let pool = self.shm.create_pool(file.as_fd(), size, qh, ());
        let buffer = pool.create_buffer(
            0,
            stride / 4,
            BAR as i32,
            stride,
            wl_shm::Format::Argb8888,
            qh,
            BAR_KEY,
        );
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, stride / 4, BAR as i32);
        self.surface.commit();
        if let Some((old_pool, old_buffer, _)) = self.pool.replace((pool, buffer, file)) {
            old_buffer.destroy();
            old_pool.destroy();
        }
        self.width = width;
        Ok(())
    }
}

impl Drop for Bar {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                state.on_bar = state.bar.as_ref() == Some(&surface.id());
                state.on_picture = state.picture.as_ref() == Some(&surface.id());
                state.pointer_x = surface_x;
                state.pointer_y = surface_y;
            }
            wl_pointer::Event::Leave { .. } => {
                state.on_bar = false;
                state.on_picture = false;
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_x = surface_x;
                state.pointer_y = surface_y;
            }
            wl_pointer::Event::Button {
                serial,
                time,
                button: BTN_LEFT,
                state: WEnum::Value(wl_pointer::ButtonState::Pressed),
                ..
            } if state.on_bar => {
                let close_from = f64::from(state.bar_width.saturating_sub(BAR));
                let double = state
                    .last_bar_press
                    .is_some_and(|t| time.wrapping_sub(t) <= DOUBLE_CLICK_MS);
                state.last_bar_press = Some(time);
                if state.pointer_x >= close_from {
                    state.closed = true;
                } else if state.pointer_x >= close_from - f64::from(BAR) || double {
                    state.last_bar_press = None;
                    state.maximize_request = Some(!state.maximized);
                } else {
                    state.move_request = Some(serial);
                }
            }
            wl_pointer::Event::Button {
                serial,
                button: BTN_LEFT,
                state: WEnum::Value(wl_pointer::ButtonState::Pressed),
                ..
            } if state.on_picture => {
                let (w, h) = state.shown;
                if state.pointer_x >= f64::from(w.saturating_sub(GRIP))
                    && state.pointer_y >= f64::from(h.saturating_sub(GRIP))
                {
                    state.resize_request = Some(serial);
                } else {
                    // Dragging the picture moves the window too.
                    state.move_request = Some(serial);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ZxdgToplevelDecorationV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event {
            state.server_side = mode == WEnum::Value(zxdg_toplevel_decoration_v1::Mode::ServerSide);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            state.wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            state.configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Close => state.closed = true,
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                state.asked = (width.max(0), height.max(0));
                // The states are 32-bit values in native byte order (xdg-shell).
                state.maximized = states
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|c| u32::from_ne_bytes(*c) == xdg_toplevel::State::Maximized as u32);
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwpLinuxDmabufV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwpLinuxDmabufV1,
        event: zwp_linux_dmabuf_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_linux_dmabuf_v1::Event::Modifier {
            format,
            modifier_hi,
            modifier_lo,
        } = event
        {
            let modifier = (u64::from(modifier_hi) << 32) | u64::from(modifier_lo);
            state.formats.insert((format, modifier));
        }
    }
}

impl Dispatch<ZwpLinuxBufferParamsV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwpLinuxBufferParamsV1,
        event: zwp_linux_buffer_params_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_linux_buffer_params_v1::Event::Failed = event {
            state.failed = true;
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, u64> for State {
    fn event(
        state: &mut Self,
        _: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        key: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            state.in_flight.remove(key);
        }
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_subcompositor::WlSubcompositor);
delegate_noop!(State: ignore wl_subsurface::WlSubsurface);
delegate_noop!(State: ignore ZxdgDecorationManagerV1);
delegate_noop!(State: ignore wp_viewporter::WpViewporter);
delegate_noop!(State: ignore wp_viewport::WpViewport);

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar: ink everywhere, the cream cross only in the close area, in the given width.
    #[test]
    fn bar_pixels() {
        let p = draw_bar(100);
        assert_eq!(p.len(), 100 * BAR as usize * 4);
        let at =
            |x: u32, y: u32| &p[((y * 100 + x) * 4) as usize..((y * 100 + x) * 4 + 4) as usize];
        assert_eq!(at(0, 0), [0x2a, 0x24, 0x10, 0xff]);
        assert_eq!(at(50, 14), [0x2a, 0x24, 0x10, 0xff]);
        assert_eq!(
            at(100 - BAR + 14, 14),
            [0xde, 0xf4, 0xff, 0xff],
            "centre of the cross"
        );
        assert_eq!(at(100 - BAR + 2, 2), [0x2a, 0x24, 0x10, 0xff], "margin");
        assert!(
            draw_bar(10).len() == 10 * BAR as usize * 4,
            "narrower than the close area"
        );
        // The maximize square: its outline cream, its middle ink.
        assert_eq!(
            at(100 - 2 * BAR + 9, 14),
            [0xde, 0xf4, 0xff, 0xff],
            "square edge"
        );
        assert_eq!(
            at(100 - 2 * BAR + 14, 14),
            [0x2a, 0x24, 0x10, 0xff],
            "square middle"
        );
    }

    /// The picture fills the window as far as its aspect ratio allows.
    #[test]
    fn pictures_fit_the_window() {
        assert_eq!(fit((850, 532), None), (850, 532));
        assert_eq!(fit((850, 532), Some((0, 0))), (850, 532));
        assert_eq!(fit((850, 532), Some((1700, 2000))), (1700, 1064));
        assert_eq!(fit((850, 532), Some((3000, 1064))), (1700, 1064));
        assert_eq!(fit((1920, 1080), Some((960, 1000))), (960, 540));
        // Never below the smallest size, never zero, no overflow.
        assert_eq!(fit((850, 532), Some((10, 10))), (120, 75));
        assert_eq!(fit((0, 0), Some((100, 100))), (120, 120));
        let big = fit((8192, 8192), Some((u32::MAX / 2, u32::MAX / 2)));
        assert_eq!(big.0, big.1);
    }
}
