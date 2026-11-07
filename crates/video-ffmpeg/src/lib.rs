//! Video adapter through the system's FFmpeg (function 28): camera capture over
//! V4L2 (libavdevice), H.264 encoding for H.264 UC (libx264 in constrained baseline, no B frames,
//! SPS/PPS before every IDR, IDR on request) and H.264 decoding, all frames as planar YUV 4:2:0.
//!
//! The protocol side (RTP payload, PACSI, stream layout) is `commscraft-h264uc`; this crate only
//! turns pictures into Annex B access units and back.
//!
//! Security by design: decoder input is bounded (an access unit above [`MAX_ACCESS_UNIT`], the
//! limit of `commscraft-h264uc`, is refused before FFmpeg sees it); picture sizes are bounded –
//! the decoder refuses pictures above [`MAX_PIXELS`] before it allocates them; plane accesses are checked against the plane count, because the binding
//! panics on an out-of-range plane; every FFmpeg error is returned, never unwrapped.
//! A decoder on the GPU keeps these limits and moves to the CPU for good when input or driver keep
//! failing or the picture size keeps changing: a hostile stream cannot keep working on the driver.

use std::os::fd::OwnedFd;

use ffmpeg_next as ff;

mod hw;

/// Largest access unit handed to the decoder (as `commscraft-h264uc`).
pub const MAX_ACCESS_UNIT: usize = 2 * 1024 * 1024;
/// Largest picture accepted (design: 1920 × 1088).
pub const MAX_WIDTH: u32 = 1920;
pub const MAX_HEIGHT: u32 = 1088;
/// Largest picture the decoder accepts, in pixels.
pub const MAX_PIXELS: u64 = MAX_WIDTH as u64 * MAX_HEIGHT as u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// FFmpeg reported an error (its message).
    Ffmpeg(String),
    /// No usable H.264 encoder in the system's FFmpeg.
    NoEncoder,
    NoDecoder,
    /// No usable GPU encoder (VA-API device, surface pool or upload failed).
    NoHardware(String),
    /// No window for the picture (no XVideo, no X server).
    NoDisplay(String),
    /// The camera or its V4L2 input format is not available.
    NoCamera(String),
    /// A size or rate outside the supported range.
    BadParameter(&'static str),
    TooLarge,
}

impl From<ff::Error> for Error {
    fn from(e: ff::Error) -> Error {
        Error::Ffmpeg(e.to_string())
    }
}

/// Initialises FFmpeg once (codecs and devices).
pub fn init() -> Result<(), Error> {
    ff::init()?;
    ff::device::register_all();
    Ok(())
}

/// A picture in planar YUV 4:2:0 (I420): Y plane `width × height`, then U and V planes of
/// `(width/2) × (height/2)`, without padding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Picture {
    /// The size of an I420 picture.
    pub fn i420_len(width: u32, height: u32) -> usize {
        let (w, h) = (width as usize, height as usize);
        w * h + 2 * (w / 2) * (h / 2)
    }

    /// A uniformly coloured picture (tests, placeholder before the camera starts).
    pub fn filled(width: u32, height: u32, y: u8, u: u8, v: u8) -> Picture {
        let (w, h) = (width as usize, height as usize);
        let mut data = vec![y; w * h];
        data.extend(std::iter::repeat_n(u, (w / 2) * (h / 2)));
        data.extend(std::iter::repeat_n(v, (w / 2) * (h / 2)));
        Picture {
            width,
            height,
            data,
        }
    }
}

fn check_size(width: u32, height: u32) -> Result<(), Error> {
    if width < 16
        || height < 16
        || width > MAX_WIDTH
        || height > MAX_HEIGHT
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
    {
        return Err(Error::BadParameter("picture size"));
    }
    Ok(())
}

/// Copies an I420 picture into an FFmpeg frame (row by row: the frame has a line stride).
fn to_frame(p: &Picture) -> Result<ff::frame::Video, Error> {
    check_size(p.width, p.height)?;
    if p.data.len() != Picture::i420_len(p.width, p.height) {
        return Err(Error::BadParameter("picture data"));
    }
    let mut f = ff::frame::Video::new(ff::format::Pixel::YUV420P, p.width, p.height);
    if f.planes() < 3 {
        return Err(Error::Ffmpeg("frame without three planes".into()));
    }
    let (w, h) = (p.width as usize, p.height as usize);
    let mut src = p.data.as_slice();
    for (plane, (pw, ph)) in [(w, h), (w / 2, h / 2), (w / 2, h / 2)]
        .into_iter()
        .enumerate()
    {
        let stride = f.stride(plane);
        let dst = f.data_mut(plane);
        for row in 0..ph {
            let (line, rest) = src
                .split_at_checked(pw)
                .ok_or(Error::BadParameter("picture data"))?;
            src = rest;
            dst.get_mut(row * stride..row * stride + pw)
                .ok_or(Error::Ffmpeg("frame smaller than the picture".into()))?
                .copy_from_slice(line);
        }
    }
    Ok(f)
}

/// Copies a YUV 4:2:0 FFmpeg frame into a packed picture.
fn from_frame(f: &ff::frame::Video) -> Result<Picture, Error> {
    if f.format() != ff::format::Pixel::YUV420P || f.planes() < 3 {
        return Err(Error::Ffmpeg("unexpected frame format".into()));
    }
    let (width, height) = (f.width(), f.height());
    check_size(width, height)?;
    let (w, h) = (width as usize, height as usize);
    let mut data = Vec::with_capacity(Picture::i420_len(width, height));
    for (plane, (pw, ph)) in [(w, h), (w / 2, h / 2), (w / 2, h / 2)]
        .into_iter()
        .enumerate()
    {
        let stride = f.stride(plane);
        let src = f.data(plane);
        for row in 0..ph {
            let line = src
                .get(row * stride..row * stride + pw)
                .ok_or(Error::Ffmpeg("frame smaller than its size".into()))?;
            data.extend_from_slice(line);
        }
    }
    Ok(Picture {
        width,
        height,
        data,
    })
}

/// Copies an NV12 frame (what a VA-API surface downloads to) into a packed I420 picture.
fn from_nv12(f: &ff::frame::Video) -> Result<Picture, Error> {
    if f.format() != ff::format::Pixel::NV12 || f.planes() < 2 {
        return Err(Error::Ffmpeg("unexpected frame format".into()));
    }
    let (width, height) = (f.width(), f.height());
    check_size(width, height)?;
    let (w, h) = (width as usize, height as usize);
    let mut data = Vec::with_capacity(Picture::i420_len(width, height));
    let (stride, src) = (f.stride(0), f.data(0));
    for row in 0..h {
        let line = src
            .get(row * stride..row * stride + w)
            .ok_or(Error::Ffmpeg("frame smaller than its size".into()))?;
        data.extend_from_slice(line);
    }
    let (stride, src) = (f.stride(1), f.data(1));
    for offset in [0, 1] {
        for row in 0..h / 2 {
            let line = src
                .get(row * stride..row * stride + w)
                .ok_or(Error::Ffmpeg("frame smaller than its size".into()))?;
            data.extend(line.iter().skip(offset).step_by(2));
        }
    }
    Ok(Picture {
        width,
        height,
        data,
    })
}

/// Encoder settings for one H.264 UC layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    /// Frames per second (the frame rate table of MS-H264PF: 7.5 … 60; whole numbers here).
    pub fps: u32,
    /// Target bit rate in bits per second.
    pub bitrate: u32,
    /// Distance between IDR pictures in frames (besides those requested).
    pub idr_interval: u32,
    /// Try the GPU first (VA-API); without usable hardware libx264 is used.
    pub hardware: bool,
}

/// Where video is encoded or decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// On the CPU (libx264, FFmpeg's H.264 decoder).
    Software,
    /// On the GPU through VA-API.
    Vaapi,
}

enum Path {
    Software,
    Vaapi {
        frames: hw::VaapiFrames,
        to_nv12: ff::software::scaling::Context,
    },
}

/// H.264 encoder (libx264 through FFmpeg): constrained baseline profile (PL-03: the CB bit
/// of the layer description), no B frames, zero latency, Annex B with SPS/PPS repeated before
/// every IDR so a receiver can start at any IDR.
pub struct Encoder {
    enc: ff::encoder::Video,
    config: EncoderConfig,
    pts: i64,
    path: Path,
}

impl Encoder {
    pub fn new(config: EncoderConfig) -> Result<Encoder, Error> {
        check_size(config.width, config.height)?;
        if !(5..=60).contains(&config.fps)
            || !(32_000..=8_000_000).contains(&config.bitrate)
            || !(1..=600).contains(&config.idr_interval)
        {
            return Err(Error::BadParameter("encoder settings"));
        }
        if config.hardware {
            // The GPU if it works, otherwise the CPU – never no video.
            if let Ok(e) = Encoder::new_vaapi(config) {
                return Ok(e);
            }
        }
        let codec = ff::encoder::find_by_name("libx264").ok_or(Error::NoEncoder)?;
        let mut enc = ff::codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()?;
        enc.set_width(config.width);
        enc.set_height(config.height);
        enc.set_format(ff::format::Pixel::YUV420P);
        let fps = i32::try_from(config.fps).map_err(|_| Error::BadParameter("fps"))?;
        enc.set_time_base((1, fps));
        enc.set_frame_rate(Some((fps, 1)));
        enc.set_bit_rate(config.bitrate as usize);
        enc.set_max_b_frames(0);
        enc.set_gop(config.idr_interval);
        let mut opts = ff::Dictionary::new();
        opts.set("profile", "baseline");
        opts.set("preset", "veryfast");
        opts.set("tune", "zerolatency");
        // A requested I picture becomes an IDR (key frame requests of the receiver).
        opts.set("forced-idr", "1");
        opts.set("x264-params", "repeat-headers=1:annexb=1:aud=0");
        let enc = enc.open_as_with(codec, opts)?;
        Ok(Encoder {
            enc,
            config,
            pts: 0,
            path: Path::Software,
        })
    }

    /// `h264_vaapi` with a VA-API surface pool: constrained baseline, no B frames, the
    /// same rate and IDR settings as the software path; SPS/PPS come with every IDR.
    fn new_vaapi(config: EncoderConfig) -> Result<Encoder, Error> {
        let codec = ff::encoder::find_by_name("h264_vaapi").ok_or(Error::NoEncoder)?;
        let frames = hw::VaapiFrames::new(config.width, config.height)?;
        let mut ctx = ff::codec::context::Context::new_with_codec(codec);
        frames.attach(&mut ctx)?;
        let mut enc = ctx.encoder().video()?;
        enc.set_width(config.width);
        enc.set_height(config.height);
        enc.set_format(ff::format::Pixel::VAAPI);
        let fps = i32::try_from(config.fps).map_err(|_| Error::BadParameter("fps"))?;
        enc.set_time_base((1, fps));
        enc.set_frame_rate(Some((fps, 1)));
        enc.set_bit_rate(config.bitrate as usize);
        enc.set_max_b_frames(0);
        enc.set_gop(config.idr_interval);
        let mut opts = ff::Dictionary::new();
        opts.set("profile", "constrained_baseline");
        opts.set("aud", "0");
        let enc = enc.open_as_with(codec, opts)?;
        let to_nv12 = ff::software::scaling::Context::get(
            ff::format::Pixel::YUV420P,
            config.width,
            config.height,
            ff::format::Pixel::NV12,
            config.width,
            config.height,
            ff::software::scaling::Flags::BILINEAR,
        )?;
        Ok(Encoder {
            enc,
            config,
            pts: 0,
            path: Path::Vaapi { frames, to_nv12 },
        })
    }

    /// Whether the GPU or the CPU encodes.
    pub fn backend(&self) -> Backend {
        match self.path {
            Path::Software => Backend::Software,
            Path::Vaapi { .. } => Backend::Vaapi,
        }
    }

    pub fn config(&self) -> EncoderConfig {
        self.config
    }

    /// Encodes one picture; `idr` forces an IDR picture (a key frame request). Returns the
    /// access units that are ready (zero latency: normally exactly one).
    pub fn encode(&mut self, picture: &Picture, idr: bool) -> Result<Vec<Vec<u8>>, Error> {
        if picture.width != self.config.width || picture.height != self.config.height {
            return Err(Error::BadParameter("picture size differs from the encoder"));
        }
        let mut frame = to_frame(picture)?;
        frame.set_pts(Some(self.pts));
        self.pts += 1;
        if let Path::Vaapi { frames, to_nv12 } = &mut self.path {
            let mut nv12 = ff::frame::Video::empty();
            to_nv12.run(&frame, &mut nv12)?;
            nv12.set_pts(frame.pts());
            frame = frames.upload(&nv12)?;
        }
        if idr {
            frame.set_kind(ff::picture::Type::I);
        }
        self.enc.send_frame(&frame)?;
        let mut out = Vec::new();
        let mut packet = ff::Packet::empty();
        while self.enc.receive_packet(&mut packet).is_ok() {
            if let Some(d) = packet.data() {
                out.push(d.to_vec());
            }
        }
        Ok(out)
    }
}

/// DRM format codes (drm_fourcc.h).
const DRM_NV12: u32 = u32::from_le_bytes(*b"NV12");
const DRM_R8: u32 = u32::from_le_bytes(*b"R8  ");
const DRM_GR88: u32 = u32::from_le_bytes(*b"GR88");

/// One plane of a [`DmaBuf`].
#[derive(Debug)]
pub struct DmaBufPlane {
    pub fd: OwnedFd,
    pub offset: u32,
    pub stride: u32,
}

/// A decoded picture in GPU memory, for a window that shows it without copying.
pub struct DmaBuf {
    pub width: u32,
    pub height: u32,
    /// DRM format code.
    pub fourcc: u32,
    pub modifier: u64,
    pub planes: Vec<DmaBufPlane>,
    /// Keeps the surface from being reused while the picture is in use.
    _mapped: ff::frame::Video,
}

impl DmaBuf {
    fn new(
        width: u32,
        height: u32,
        fourcc: u32,
        modifier: u64,
        planes: Vec<DmaBufPlane>,
        mapped: ff::frame::Video,
    ) -> DmaBuf {
        DmaBuf {
            width,
            height,
            fourcc,
            modifier,
            planes,
            _mapped: mapped,
        }
    }
}

/// A decoded picture.
pub enum Decoded {
    /// In memory.
    Picture(Picture),
    /// In GPU memory.
    Surface(DmaBuf),
}

/// Failed access units before a GPU decoder moves to the CPU (design: a stream that starts between
/// two IDR pictures or loses packets fails a few times without any hardware problem).
const MAX_GPU_ERRORS: u32 = 16;
/// Picture size changes before a GPU decoder moves to the CPU (design: each one rebuilds surfaces
/// in the driver; a call changes size now and then, not all the time).
const MAX_GPU_RESIZES: u32 = 32;

fn open_decoder(hardware: bool) -> Result<ff::decoder::Video, Error> {
    let codec = ff::decoder::find(ff::codec::Id::H264).ok_or(Error::NoDecoder)?;
    // Issue #93: FFmpeg itself refuses pictures above our size before it allocates them
    // (AVCodecContext.max_pixels); the check of the decoded frame alone comes too late.
    let mut opts = ff::Dictionary::new();
    opts.set("max_pixels", &MAX_PIXELS.to_string());
    let mut ctx = ff::codec::context::Context::new_with_codec(codec);
    if hardware {
        hw::attach_decoder_device(&mut ctx)?;
        // Pictures on their way to the window keep their surfaces.
        opts.set("extra_hw_frames", "6");
    }
    Ok(ctx.decoder().open_as_with(codec, opts)?.video()?)
}

/// H.264 decoder (FFmpeg's own `h264` decoder, on the GPU through its VA-API path if wanted).
pub struct Decoder {
    dec: ff::decoder::Video,
    /// The context has a VA-API device.
    gpu: bool,
    backend: Backend,
    errors: u32,
    resizes: u32,
    size: Option<(u32, u32)>,
}

impl Decoder {
    /// A decoder on the CPU.
    pub fn new() -> Result<Decoder, Error> {
        Decoder::with_hardware(false)
    }

    /// A decoder on the GPU if `hardware` is set and a device works, otherwise on the CPU.
    pub fn with_hardware(hardware: bool) -> Result<Decoder, Error> {
        let (dec, gpu) = match open_decoder(hardware) {
            Ok(dec) => (dec, hardware),
            Err(_) if hardware => (open_decoder(false)?, false),
            Err(e) => return Err(e),
        };
        Ok(Decoder {
            dec,
            gpu,
            backend: Backend::Software,
            errors: 0,
            resizes: 0,
            size: None,
        })
    }

    /// Where the last picture was decoded (FFmpeg also uses the CPU for a stream the device
    /// cannot decode).
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Decodes one Annex B access unit; returns the pictures that became ready, in memory.
    /// Pictures in another pixel format than YUV 4:2:0 or above the size limit are refused.
    pub fn decode(&mut self, access_unit: &[u8]) -> Result<Vec<Picture>, Error> {
        let decoded = self.decode_as(access_unit, false)?;
        Ok(decoded
            .into_iter()
            .filter_map(|d| match d {
                Decoded::Picture(p) => Some(p),
                Decoded::Surface(_) => None,
            })
            .collect())
    }

    /// Like [`Decoder::decode`], but pictures on the GPU stay there where that works.
    pub fn decode_to_surfaces(&mut self, access_unit: &[u8]) -> Result<Vec<Decoded>, Error> {
        self.decode_as(access_unit, true)
    }

    fn decode_as(&mut self, access_unit: &[u8], surfaces: bool) -> Result<Vec<Decoded>, Error> {
        if access_unit.len() > MAX_ACCESS_UNIT {
            return Err(Error::TooLarge);
        }
        let result = self.decode_checked(access_unit, surfaces);
        if self.gpu {
            if result.is_err() {
                self.errors = self.errors.saturating_add(1);
            }
            if self.errors >= MAX_GPU_ERRORS || self.resizes >= MAX_GPU_RESIZES {
                self.use_cpu()?;
            }
        }
        result
    }

    /// The CPU from here on; it starts with the stream's next IDR picture.
    fn use_cpu(&mut self) -> Result<(), Error> {
        self.dec = open_decoder(false)?;
        self.gpu = false;
        self.backend = Backend::Software;
        Ok(())
    }

    fn decode_checked(
        &mut self,
        access_unit: &[u8],
        surfaces: bool,
    ) -> Result<Vec<Decoded>, Error> {
        self.dec.send_packet(&ff::Packet::copy(access_unit))?;
        let mut out = Vec::new();
        let mut frame = ff::frame::Video::empty();
        while self.dec.receive_frame(&mut frame).is_ok() {
            let (width, height) = (frame.width(), frame.height());
            check_size(width, height)?;
            if self.size.is_some_and(|s| s != (width, height)) {
                self.resizes = self.resizes.saturating_add(1);
            }
            self.size = Some((width, height));
            if frame.format() == ff::format::Pixel::VAAPI {
                self.backend = Backend::Vaapi;
                // A surface that cannot be mapped is still a picture, through the copy.
                let surface = surfaces.then(|| hw::map_dmabuf(&frame).ok()).flatten();
                out.push(match surface {
                    Some(s) => Decoded::Surface(s),
                    None => Decoded::Picture(from_nv12(&hw::download(&frame)?)?),
                });
            } else {
                self.backend = Backend::Software;
                out.push(Decoded::Picture(from_frame(&frame)?));
            }
        }
        Ok(out)
    }
}

/// A camera opened over V4L2 (libavdevice), delivering I420 pictures.
pub struct Camera {
    input: ff::format::context::Input,
    stream: usize,
    decoder: ff::decoder::Video,
    scaler: Option<ff::software::scaling::Context>,
    width: u32,
    height: u32,
}

impl Camera {
    /// Opens `device` (e.g. `/dev/video0`) with the given size and rate in its uncompressed
    /// YUYV format (no JPEG decoding of untrusted camera data needed).
    pub fn open(device: &str, width: u32, height: u32, fps: u32) -> Result<Camera, Error> {
        check_size(width, height)?;
        if !(5..=60).contains(&fps) {
            return Err(Error::BadParameter("fps"));
        }
        if !device.starts_with("/dev/video") || device.contains("..") {
            return Err(Error::BadParameter("camera device"));
        }
        let format = ff::device::input::video()
            .find(|f| f.name() == "video4linux2,v4l2")
            .ok_or(Error::NoCamera("FFmpeg has no V4L2 input".into()))?;
        let mut opts = ff::Dictionary::new();
        opts.set("video_size", &format!("{width}x{height}"));
        opts.set("framerate", &fps.to_string());
        opts.set("input_format", "yuyv422");
        let input = match ff::format::open_with(&device, &format, opts) {
            Ok(ff::format::context::Context::Input(i)) => i,
            Ok(_) => return Err(Error::NoCamera("not an input".into())),
            Err(e) => return Err(Error::NoCamera(e.to_string())),
        };
        let stream = input
            .streams()
            .best(ff::media::Type::Video)
            .ok_or(Error::NoCamera("no video stream".into()))?;
        let index = stream.index();
        let decoder = ff::codec::context::Context::from_parameters(stream.parameters())?
            .decoder()
            .video()?;
        Ok(Camera {
            input,
            stream: index,
            decoder,
            scaler: None,
            width,
            height,
        })
    }

    /// The next picture from the camera (blocks until the device delivers one).
    pub fn next_picture(&mut self) -> Result<Picture, Error> {
        loop {
            let mut packet = ff::Packet::empty();
            packet.read(&mut self.input)?;
            if packet.stream() != self.stream {
                continue;
            }
            self.decoder.send_packet(&packet)?;
            let mut raw = ff::frame::Video::empty();
            if self.decoder.receive_frame(&mut raw).is_err() {
                continue;
            }
            if raw.width() != self.width || raw.height() != self.height {
                return Err(Error::NoCamera(format!(
                    "the camera delivers {}x{}",
                    raw.width(),
                    raw.height()
                )));
            }
            let scaler = match &mut self.scaler {
                Some(s) => s,
                None => self.scaler.insert(ff::software::scaling::Context::get(
                    raw.format(),
                    self.width,
                    self.height,
                    ff::format::Pixel::YUV420P,
                    self.width,
                    self.height,
                    ff::software::scaling::Flags::BILINEAR,
                )?),
            };
            let mut yuv = ff::frame::Video::empty();
            scaler.run(&raw, &mut yuv)?;
            return from_frame(&yuv);
        }
    }
}

/// Scales I420 pictures to another size (swscale), e.g. to what a video source request allows.
pub struct Scaler {
    ctx: ff::software::scaling::Context,
    to: (u32, u32),
}

impl Scaler {
    pub fn new(from: (u32, u32), to: (u32, u32)) -> Result<Scaler, Error> {
        check_size(from.0, from.1)?;
        check_size(to.0, to.1)?;
        let ctx = ff::software::scaling::Context::get(
            ff::format::Pixel::YUV420P,
            from.0,
            from.1,
            ff::format::Pixel::YUV420P,
            to.0,
            to.1,
            ff::software::scaling::Flags::BILINEAR,
        )?;
        Ok(Scaler { ctx, to })
    }

    pub fn scale(&mut self, picture: &Picture) -> Result<Picture, Error> {
        let src = to_frame(picture)?;
        let mut dst = ff::frame::Video::new(ff::format::Pixel::YUV420P, self.to.0, self.to.1);
        self.ctx.run(&src, &mut dst)?;
        from_frame(&dst)
    }
}

/// A window showing pictures through FFmpeg's XVideo output device (libavdevice; under
/// Wayland through Xwayland). Pictures of the window's size only.
pub struct Display {
    output: ff::format::context::Output,
    width: u32,
    height: u32,
    pts: i64,
}

impl Display {
    /// Opens a window of `width × height` with `title` (control characters removed, at most 64
    /// characters). The device writes no file: the output path is `/dev/null`, because the
    /// binding opens the path even for devices.
    pub fn open(title: &str, width: u32, height: u32) -> Result<Display, Error> {
        check_size(width, height)?;
        let mut output = ff::format::output_as("/dev/null", "xv")
            .map_err(|e| Error::NoDisplay(e.to_string()))?;
        let codec = ff::encoder::find(ff::codec::Id::RAWVIDEO).ok_or(Error::NoEncoder)?;
        let mut ctx = ff::codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()?;
        ctx.set_width(width);
        ctx.set_height(height);
        ctx.set_format(ff::format::Pixel::YUV420P);
        ctx.set_time_base((1, 30));
        {
            let mut stream = output.add_stream(codec)?;
            stream.set_time_base((1, 30));
            stream.set_parameters(&ctx);
        }
        let title: String = title.chars().filter(|c| !c.is_control()).take(64).collect();
        let mut opts = ff::Dictionary::new();
        opts.set("window_title", &title);
        output
            .write_header_with(opts)
            .map_err(|e| Error::NoDisplay(e.to_string()))?;
        Ok(Display {
            output,
            width,
            height,
            pts: 0,
        })
    }

    /// Shows one picture (it must have the window's size).
    pub fn show(&mut self, picture: &Picture) -> Result<(), Error> {
        if picture.width != self.width
            || picture.height != self.height
            || picture.data.len() != Picture::i420_len(self.width, self.height)
        {
            return Err(Error::BadParameter("picture size differs from the window"));
        }
        let mut packet = ff::Packet::copy(&picture.data);
        packet.set_stream(0);
        packet.set_pts(Some(self.pts));
        packet.set_dts(Some(self.pts));
        self.pts += 1;
        packet.write(&mut self.output)?;
        Ok(())
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        let _ = self.output.write_trailer();
    }
}

#[cfg(test)]
mod tests;
