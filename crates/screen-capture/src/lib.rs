//! Capturing the own screen for sharing it (function 43): the desktop's ScreenCast portal
//! (`org.freedesktop.portal.ScreenCast`, XDG Desktop Portal) lets the user pick a screen or a
//! window in the desktop's own dialog and hands over a PipeWire stream; this crate reads its
//! frames. Adapter crate: D-Bus through `ashpd` (zbus 6), PipeWire through `pipewire` (the FFI
//! stays in pipewire-rs; this crate has no `unsafe`).
//!
//! The camera goes the same way (issue #257): the Camera portal hands over a PipeWire remote
//! with the cameras the user allowed, so the Flatpak needs no access to `/dev/video*`.
//!
//! Security by design: nothing is captured without the user's choice in the portal dialog; the
//! portal session ends with [`Capture`]; frames go through a bounded channel (a slow encoder
//! drops frames instead of piling them up); sizes are checked against [`MAX_SIDE`].

use std::os::fd::OwnedFd;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};

use pipewire as pw;
use pw::spa;

/// Largest frame side taken (design: 8K).
pub const MAX_SIDE: u32 = 7680;
/// Frames waiting for the encoder at most.
const QUEUE: usize = 2;

/// The pixel layout of a frame: 4-byte RGB (screens), YUYV 4:2:2 with 2 bytes per pixel
/// (cameras).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pixels {
    Bgrx,
    Rgbx,
    Bgra,
    Rgba,
    Yuy2,
}

impl Pixels {
    /// Bytes per pixel.
    pub fn bytes(self) -> u32 {
        match self {
            Pixels::Yuy2 => 2,
            _ => 4,
        }
    }
}

/// What a stream is asked for: a screen (4-byte RGB, any size) or a camera (YUYV at a size
/// and rate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    Screen,
    Camera { width: u32, height: u32, fps: u32 },
}

/// One captured frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Bytes per row.
    pub stride: u32,
    pub pixels: Pixels,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum Error {
    /// The portal is missing or refused (the user cancelled the dialog).
    Portal(String),
    PipeWire(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Portal(e) => write!(f, "desktop portal: {e}"),
            Error::PipeWire(e) => write!(f, "PipeWire: {e}"),
        }
    }
}

/// What to capture.
pub enum Source {
    /// A stream the portal handed over: its PipeWire remote and node.
    Portal { remote: OwnedFd, node: u32 },
    /// A node of the user's PipeWire session (tests: a video test source).
    Node(u32),
    /// A portal's PipeWire remote; the session manager links the stream to its default node
    /// (the Camera portal's remote shows only the allowed cameras).
    Remote(OwnedFd),
}

/// A running capture; dropping it stops the stream (and ends the portal session it holds).
pub struct Capture {
    pub frames: Receiver<Frame>,
    stop: pw::channel::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
    _portal: Option<PortalSession>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Keeps the portal session (and its connection) alive while capturing.
enum PortalSession {
    Screen {
        _proxy: ashpd::desktop::screencast::Screencast,
        _session: ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>,
    },
    Camera {
        _proxy: ashpd::desktop::camera::Camera,
    },
}

/// Whether the desktop's Camera portal reports a camera (false without portal).
pub fn camera_present() -> bool {
    futures_lite::future::block_on(async {
        match ashpd::desktop::camera::Camera::new().await {
            Ok(c) => c.is_present().await.unwrap_or(false),
            Err(_) => false,
        }
    })
}

/// Asks the desktop for the camera (the Camera portal; the desktop may ask the user once) and
/// starts reading it as YUYV at the given size and rate.
pub fn start_camera_portal(width: u32, height: u32, fps: u32) -> Result<Capture, Error> {
    use ashpd::desktop::camera::{Camera, CameraAccessOptions, OpenPipeWireRemoteOptions};
    let (proxy, remote) = futures_lite::future::block_on(async {
        let p = |e: ashpd::Error| Error::Portal(e.to_string());
        let proxy = Camera::new().await.map_err(p)?;
        if !proxy.is_present().await.map_err(p)? {
            return Err(Error::Portal("no camera".into()));
        }
        proxy
            .request_access(CameraAccessOptions::default())
            .await
            .map_err(p)?
            .response()
            .map_err(p)?;
        let remote = proxy
            .open_pipe_wire_remote(OpenPipeWireRemoteOptions::default())
            .await
            .map_err(p)?;
        Ok::<_, Error>((proxy, remote))
    })?;
    let mut c = start_with(Source::Remote(remote), Want::Camera { width, height, fps })?;
    c._portal = Some(PortalSession::Camera { _proxy: proxy });
    Ok(c)
}

/// Asks the desktop for a screen or window (its own dialog) and starts reading it.
pub fn start_portal(windows: bool) -> Result<Capture, Error> {
    use ashpd::desktop::screencast::{
        CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, SourceType,
        StartCastOptions,
    };
    let (proxy, session, remote, node) = futures_lite::future::block_on(async {
        let p = |e: ashpd::Error| Error::Portal(e.to_string());
        let proxy = Screencast::new().await.map_err(p)?;
        let session = proxy.create_session(Default::default()).await.map_err(p)?;
        let sources = if windows {
            SourceType::MONITOR | SourceType::WINDOW
        } else {
            SourceType::MONITOR
        };
        proxy
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_cursor_mode(CursorMode::EMBEDDED)
                    .set_sources(sources)
                    .set_multiple(false),
            )
            .await
            .map_err(p)?
            .response()
            .map_err(p)?;
        let streams = proxy
            .start(&session, None, StartCastOptions::default())
            .await
            .map_err(p)?
            .response()
            .map_err(p)?;
        let node = streams
            .streams()
            .first()
            .map(|s| s.pipe_wire_node_id())
            .ok_or_else(|| Error::Portal("no stream chosen".into()))?;
        let remote = proxy
            .open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default())
            .await
            .map_err(p)?;
        Ok::<_, Error>((proxy, session, remote, node))
    })?;
    let mut c = start(Source::Portal { remote, node })?;
    c._portal = Some(PortalSession::Screen {
        _proxy: proxy,
        _session: session,
    });
    Ok(c)
}

/// Starts reading a PipeWire screen stream in a thread of its own.
pub fn start(source: Source) -> Result<Capture, Error> {
    start_with(source, Want::Screen)
}

/// Starts reading a PipeWire video stream (screen or camera) in a thread of its own.
pub fn start_with(source: Source, want: Want) -> Result<Capture, Error> {
    let (tx, frames) = std::sync::mpsc::sync_channel::<Frame>(QUEUE);
    let (stop, stop_rx) = pw::channel::channel::<()>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), Error>>();
    let thread = std::thread::Builder::new()
        .name("screen-capture".into())
        .spawn(move || {
            if let Err(e) = run(source, want, tx, stop_rx, &ready_tx) {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| Error::PipeWire(e.to_string()))?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(Capture {
            frames,
            stop,
            thread: Some(thread),
            _portal: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(Error::PipeWire("capture thread ended".into())),
    }
}

/// The format of the stream as negotiated.
#[derive(Default)]
struct Negotiated {
    info: spa::param::video::VideoInfoRaw,
    pixels: Option<Pixels>,
}

fn pixels_of(f: spa::param::video::VideoFormat) -> Option<Pixels> {
    use spa::param::video::VideoFormat as V;
    match f {
        V::BGRx => Some(Pixels::Bgrx),
        V::RGBx => Some(Pixels::Rgbx),
        V::BGRA => Some(Pixels::Bgra),
        V::RGBA => Some(Pixels::Rgba),
        V::YUY2 => Some(Pixels::Yuy2),
        _ => None,
    }
}

fn run(
    source: Source,
    want: Want,
    tx: SyncSender<Frame>,
    stop: pw::channel::Receiver<()>,
    ready: &std::sync::mpsc::Sender<Result<(), Error>>,
) -> Result<(), Error> {
    let e = |x: pw::Error| Error::PipeWire(x.to_string());
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(e)?;
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(e)?;
    let (core, target) = match source {
        Source::Portal { remote, node } => {
            (context.connect_fd_rc(remote, None).map_err(e)?, Some(node))
        }
        Source::Node(node) => (context.connect_rc(None).map_err(e)?, Some(node)),
        Source::Remote(remote) => (context.connect_fd_rc(remote, None).map_err(e)?, None),
    };
    let (category, role) = match want {
        Want::Screen => ("Capture", "Screen"),
        Want::Camera { .. } => ("Capture", "Camera"),
    };
    let stream = pw::stream::StreamBox::new(
        &core,
        "commscraft-screen",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => category,
            *pw::keys::MEDIA_ROLE => role,
        },
    )
    .map_err(e)?;
    let _listener = stream
        .add_local_listener_with_user_data(Negotiated::default())
        .param_changed(|stream, n, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            if n.info.parse(param).is_ok() {
                n.pixels = pixels_of(n.info.format());
                // Buffers we can map (memory or memfd): a camera node otherwise offers DMA-BUFs
                // we do not import, and the buffer negotiation fails.
                if let Some(b) = buffers_pod() {
                    if let Some(pod) = spa::pod::Pod::from_bytes(&b) {
                        let _ = stream.update_params(&mut [pod]);
                    }
                }
            }
        })
        .process(move |stream, n| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let (Some(pixels), size) = (n.pixels, n.info.size()) else {
                return;
            };
            let (w, h) = (size.width, size.height);
            if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
                return;
            }
            let datas = buffer.datas_mut();
            let Some(d) = datas.first_mut() else { return };
            let chunk = d.chunk();
            let stride = u32::try_from(chunk.stride())
                .unwrap_or(0)
                .max(w * pixels.bytes());
            let offset = chunk.offset() as usize;
            let len = (stride as usize).saturating_mul(h as usize);
            let Some(bytes) = d
                .data()
                .and_then(|b| b.get(offset..offset.saturating_add(len)))
            else {
                return;
            };
            let frame = Frame {
                width: w,
                height: h,
                stride,
                pixels,
                data: bytes.to_vec(),
            };
            // A full queue drops the frame (the encoder is behind).
            if let Err(TrySendError::Disconnected(_)) = tx.try_send(frame) {}
        })
        .register()
        .map_err(e)?;
    let format = match want {
        Want::Screen => format_pod(),
        Want::Camera { width, height, fps } => camera_pod(width, height, fps),
    }
    .ok_or_else(|| Error::PipeWire("format".into()))?;
    let pod = spa::pod::Pod::from_bytes(&format).ok_or_else(|| Error::PipeWire("pod".into()))?;
    stream
        .connect(
            spa::utils::Direction::Input,
            target,
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut [pod],
        )
        .map_err(e)?;
    let ml = mainloop.clone();
    let _stop = stop.attach(mainloop.loop_(), move |()| ml.quit());
    let _ = ready.send(Ok(()));
    mainloop.run();
    Ok(())
}

/// The formats we take: 4-byte RGB layouts in any size and rate (the encoder converts).
fn format_pod() -> Option<Vec<u8>> {
    use spa::param::format::{FormatProperties as F, MediaSubtype, MediaType};
    use spa::param::video::VideoFormat as V;
    let obj = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(F::MediaType, Id, MediaType::Video),
        spa::pod::property!(F::MediaSubtype, Id, MediaSubtype::Raw),
        spa::pod::property!(
            F::VideoFormat,
            Choice,
            Enum,
            Id,
            V::BGRx,
            V::BGRx,
            V::RGBx,
            V::BGRA,
            V::RGBA
        ),
        spa::pod::property!(
            F::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: MAX_SIDE,
                height: MAX_SIDE
            }
        ),
        spa::pod::property!(
            F::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 15, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 60, denom: 1 }
        ),
    );
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .ok()
    .map(|(c, _)| c.into_inner())
}

/// The format a camera is asked for: uncompressed YUYV (no JPEG decoding of camera data), the
/// size and rate given preferred, others within bounds accepted (the encoder scales).
fn camera_pod(width: u32, height: u32, fps: u32) -> Option<Vec<u8>> {
    use spa::param::format::{FormatProperties as F, MediaSubtype, MediaType};
    use spa::param::video::VideoFormat as V;
    let obj = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(F::MediaType, Id, MediaType::Video),
        spa::pod::property!(F::MediaSubtype, Id, MediaSubtype::Raw),
        spa::pod::property!(F::VideoFormat, Id, V::YUY2),
        spa::pod::property!(
            F::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle { width, height },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: MAX_SIDE,
                height: MAX_SIDE
            }
        ),
        spa::pod::property!(
            F::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: fps, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 120, denom: 1 }
        ),
    );
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .ok()
    .map(|(c, _)| c.into_inner())
}

/// The buffer types taken: plain memory or memfd, both mapped by PipeWire (MAP_BUFFERS).
fn buffers_pod() -> Option<Vec<u8>> {
    use spa::pod::{ChoiceValue, Object, Property, PropertyFlags, Value};
    use spa::utils::{Choice, ChoiceEnum, ChoiceFlags};
    let mask = (1i32 << spa::sys::SPA_DATA_MemPtr) | (1i32 << spa::sys::SPA_DATA_MemFd);
    let obj = Object {
        type_: spa::utils::SpaTypes::ObjectParamBuffers.as_raw(),
        id: spa::param::ParamType::Buffers.as_raw(),
        properties: vec![Property {
            key: spa::sys::SPA_PARAM_BUFFERS_dataType,
            flags: PropertyFlags::empty(),
            value: Value::Choice(ChoiceValue::Int(Choice(
                ChoiceFlags::empty(),
                ChoiceEnum::Flags {
                    default: mask,
                    flags: vec![mask],
                },
            ))),
        }],
    };
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &Value::Object(obj),
    )
    .ok()
    .map(|(c, _)| c.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_pods_serialize() {
        assert!(format_pod().is_some_and(|p| !p.is_empty()));
        assert!(camera_pod(640, 360, 15).is_some_and(|p| !p.is_empty()));
        assert!(buffers_pod().is_some_and(|p| !p.is_empty()));
        assert_eq!(Pixels::Yuy2.bytes(), 2);
        assert_eq!(Pixels::Bgrx.bytes(), 4);
    }
}
