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
//! subsurface: dragging it moves the window, the cross at its right end closes it.

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
            pixels.extend_from_slice(if cross { &cream } else { &ink });
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
        if let Some(bar) = self.bar.as_mut().filter(|_| !self.state.server_side) {
            if bar.width != width && bar.draw(width, &qh).is_ok() {
                self.state.bar_width = width;
                self.xdg_surface
                    .set_window_geometry(0, -(BAR as i32), w, h + BAR as i32);
            }
        }
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
        // Our own bar above the picture unless the compositor draws one.
        if let Some(bar) = self.bar.as_mut().filter(|_| !self.state.server_side) {
            if bar.width != frame.width && bar.draw(frame.width, &qh).is_ok() {
                self.state.bar_width = frame.width;
                self.xdg_surface
                    .set_window_geometry(0, -(BAR as i32), width, height + BAR as i32);
            }
        }
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
                surface, surface_x, ..
            } => {
                state.on_bar = state.bar.as_ref() == Some(&surface.id());
                state.pointer_x = surface_x;
            }
            wl_pointer::Event::Leave { .. } => state.on_bar = false,
            wl_pointer::Event::Motion { surface_x, .. } => state.pointer_x = surface_x,
            wl_pointer::Event::Button {
                serial,
                button: BTN_LEFT,
                state: WEnum::Value(wl_pointer::ButtonState::Pressed),
                ..
            } if state.on_bar => {
                if state.pointer_x >= f64::from(state.bar_width.saturating_sub(BAR)) {
                    state.closed = true;
                } else {
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
        if let xdg_toplevel::Event::Close = event {
            state.closed = true;
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
    }
}
