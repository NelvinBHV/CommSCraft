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

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::os::fd::BorrowedFd;

use wayland_client::globals::{registry_queue_init, BindError, GlobalListContents};
use wayland_client::protocol::{wl_buffer, wl_compositor, wl_registry, wl_surface};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::{
    self, ZwpLinuxBufferParamsV1,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::{
    self, ZwpLinuxDmabufV1,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

/// Pictures waiting for the compositor before new ones are skipped (design: one on the screen, one
/// or two on their way).
pub const MAX_IN_FLIGHT: usize = 4;
/// Buffers kept for reuse. A decoder cycles through a small pool of surfaces.
const MAX_BUFFERS: usize = 64;

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
}

/// The window.
pub struct Window {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    dmabuf: ZwpLinuxDmabufV1,
    surface: wl_surface::WlSurface,
    _xdg_surface: xdg_surface::XdgSurface,
    _toplevel: xdg_toplevel::XdgToplevel,
    /// Buffers by key, for the size they were made for.
    buffers: HashMap<u64, wl_buffer::WlBuffer>,
    size: (u32, u32),
}

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
        surface.commit();
        let mut state = State {
            formats: HashSet::new(),
            wm_base,
            configured: false,
            closed: false,
            failed: false,
            in_flight: HashMap::new(),
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
            _xdg_surface: xdg,
            _toplevel: toplevel,
            buffers: HashMap::new(),
            size: (0, 0),
        })
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
