//! Video adapter through the system's FFmpeg (decision D-12; function 28): camera capture over
//! V4L2 (libavdevice), H.264 encoding for H.264 UC (libx264 in constrained baseline, no B frames,
//! SPS/PPS before every IDR, IDR on request) and H.264 decoding, all frames as planar YUV 4:2:0.
//!
//! The protocol side (RTP payload, PACSI, stream layout) is `commscraft-h264uc`; this crate only
//! turns pictures into Annex B access units and back.
//!
//! Security by design: decoder input is bounded (an access unit above [`MAX_ACCESS_UNIT`], the
//! limit of `commscraft-h264uc`, is refused before FFmpeg sees it); picture
//! sizes are bounded; plane accesses are checked against the plane count, because the binding
//! panics on an out-of-range plane; every FFmpeg error is returned, never unwrapped.

use ffmpeg_next as ff;

mod hw;

/// Largest access unit handed to the decoder (as `commscraft-h264uc`).
pub const MAX_ACCESS_UNIT: usize = 2 * 1024 * 1024;
/// Largest picture accepted (design: 1920 × 1088).
pub const MAX_WIDTH: u32 = 1920;
pub const MAX_HEIGHT: u32 = 1088;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// FFmpeg reported an error (its message).
    Ffmpeg(String),
    /// No usable H.264 encoder in the system's FFmpeg.
    NoEncoder,
    NoDecoder,
    /// No usable GPU encoder (VA-API device, surface pool or upload failed).
    NoHardware(String),
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
    /// Try the GPU first (VA-API, D-13); without usable hardware libx264 is used.
    pub hardware: bool,
}

/// How the encoder works.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// libx264 on the CPU.
    Software,
    /// `h264_vaapi` on the GPU (D-13).
    Vaapi,
}

enum Path {
    Software,
    Vaapi {
        frames: hw::VaapiFrames,
        to_nv12: ff::software::scaling::Context,
    },
}

/// H.264 encoder (libx264 through FFmpeg, D-12): constrained baseline profile (PL-03: the CB bit
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
            // D-13: the GPU if it works, otherwise the CPU – never no video.
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

    /// `h264_vaapi` with a VA-API surface pool (D-13): constrained baseline, no B frames, the
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

/// H.264 decoder (FFmpeg's own `h264` decoder).
pub struct Decoder {
    dec: ff::decoder::Video,
}

impl Decoder {
    pub fn new() -> Result<Decoder, Error> {
        let codec = ff::decoder::find(ff::codec::Id::H264).ok_or(Error::NoDecoder)?;
        let dec = ff::codec::context::Context::new_with_codec(codec)
            .decoder()
            .video()?;
        Ok(Decoder { dec })
    }

    /// Decodes one Annex B access unit; returns the pictures that became ready. Pictures in
    /// another pixel format than YUV 4:2:0 or above the size limit are refused.
    pub fn decode(&mut self, access_unit: &[u8]) -> Result<Vec<Picture>, Error> {
        if access_unit.len() > MAX_ACCESS_UNIT {
            return Err(Error::TooLarge);
        }
        self.dec.send_packet(&ff::Packet::copy(access_unit))?;
        let mut out = Vec::new();
        let mut frame = ff::frame::Video::empty();
        while self.dec.receive_frame(&mut frame).is_ok() {
            out.push(from_frame(&frame)?);
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

#[cfg(test)]
mod tests;
