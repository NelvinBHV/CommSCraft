//! VA-API hardware frames through FFmpeg's hardware context: the only module of
//! CommSCraft with `unsafe` code. It creates a VA-API device and a pool of VA-API surfaces,
//! attaches the pool to an encoder context and uploads NV12 pictures into surfaces; for decoding
//! it gives a decoder context the device, maps decoded surfaces to DMA-BUF descriptors for the
//! window and downloads them where no such window exists. Everything else stays in the safe
//! binding.
//!
//! Rules: every `unsafe` block names why it is sound; pointers come only from FFmpeg's
//! own allocation functions and are checked for null; reference-counted buffers are released
//! exactly once in `Drop`; return codes are checked before anything is used; no pointer leaves
//! this module.
#![allow(unsafe_code)]

use std::os::fd::BorrowedFd;
use std::ptr;

use ff::ffi;
use ffmpeg_next as ff;

use crate::Error;

/// The default VA-API device: a reference the caller owns and must hand on or release.
fn create_device() -> Result<*mut ffi::AVBufferRef, Error> {
    let mut device: *mut ffi::AVBufferRef = ptr::null_mut();
    // SAFETY: `device` is a valid out-pointer; a null device name and options select the default
    // VA-API render node; on failure FFmpeg leaves `device` null.
    let r = unsafe {
        ffi::av_hwdevice_ctx_create(
            &mut device,
            ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
            ptr::null(),
            ptr::null_mut(),
            0,
        )
    };
    if r < 0 || device.is_null() {
        return Err(Error::NoHardware(format!("no VA-API device ({r})")));
    }
    Ok(device)
}

/// Gives a decoder context (not yet opened) the VA-API device. FFmpeg's default format choice then
/// takes VA-API if the device can decode the stream, and its software path otherwise.
pub(crate) fn attach_decoder_device(ctx: &mut ff::codec::context::Context) -> Result<(), Error> {
    let device = create_device()?;
    // SAFETY: `ctx` is a live codec context owned by the caller and not yet opened; the field
    // takes ownership of our only reference, and FFmpeg releases it with the context.
    unsafe {
        let c = ctx.as_mut_ptr();
        if !(*c).hw_device_ctx.is_null() {
            ffi::av_buffer_unref(&mut (*c).hw_device_ctx);
        }
        (*c).hw_device_ctx = device;
    }
    Ok(())
}

/// A software copy of a decoded VA-API surface, in the format the driver prefers.
pub(crate) fn download(surface: &ff::frame::Video) -> Result<ff::frame::Video, Error> {
    if surface.format() != ff::format::Pixel::VAAPI {
        return Err(Error::BadParameter("download needs a VA-API surface"));
    }
    let mut picture = ff::frame::Video::empty();
    // SAFETY: `picture` is an empty, owned AVFrame, for which FFmpeg allocates the buffers itself;
    // `surface` is a valid hardware frame that the decoder returned.
    let r = unsafe { ffi::av_hwframe_transfer_data(picture.as_mut_ptr(), surface.as_ptr(), 0) };
    if r < 0 {
        return Err(Error::NoHardware(format!(
            "download from the GPU failed ({r})"
        )));
    }
    Ok(picture)
}

/// A decoded surface as a DMA-BUF picture. Every number read from FFmpeg's descriptor is checked;
/// the file descriptors are duplicated, so that nothing refers to the descriptor afterwards.
pub(crate) fn map_dmabuf(surface: &ff::frame::Video) -> Result<crate::DmaBuf, Error> {
    if surface.format() != ff::format::Pixel::VAAPI {
        return Err(Error::BadParameter("mapping needs a VA-API surface"));
    }
    let mut mapped = ff::frame::Video::empty();
    // SAFETY: `mapped` is an empty, owned AVFrame whose format we set to ask for DRM PRIME;
    // `surface` is a valid hardware frame that the decoder returned. `av_hwframe_map` makes
    // `mapped` hold a reference to the surface.
    let r = unsafe {
        let m = mapped.as_mut_ptr();
        (*m).format = ffi::AVPixelFormat::AV_PIX_FMT_DRM_PRIME as i32;
        ffi::av_hwframe_map(m, surface.as_ptr(), ffi::AV_HWFRAME_MAP_READ as i32)
    };
    if r < 0 {
        return Err(Error::NoHardware(format!(
            "no DMA-BUF of the surface ({r})"
        )));
    }
    // SAFETY: after a successful map to DRM PRIME, `data[0]` is null or points to the
    // AVDRMFrameDescriptor (FFmpeg's documented layout), which lives as long as `mapped`.
    let descriptor = unsafe {
        let d = (*mapped.as_ptr()).data[0].cast::<ffi::AVDRMFrameDescriptor>();
        d.as_ref()
            .ok_or(Error::NoHardware("DMA-BUF without descriptor".into()))?
    };
    let objects = descriptor
        .objects
        .get(..usize::try_from(descriptor.nb_objects).unwrap_or(0))
        .ok_or(Error::NoHardware("too many DMA-BUF objects".into()))?;
    let layers = descriptor
        .layers
        .get(..usize::try_from(descriptor.nb_layers).unwrap_or(0))
        .ok_or(Error::NoHardware("too many DMA-BUF layers".into()))?;
    let mut planes = Vec::new();
    let mut modifier = None;
    for layer in layers {
        let in_layer = layer
            .planes
            .get(..usize::try_from(layer.nb_planes).unwrap_or(0))
            .ok_or(Error::NoHardware("too many DMA-BUF planes".into()))?;
        for plane in in_layer {
            let object = usize::try_from(plane.object_index)
                .ok()
                .and_then(|i| objects.get(i))
                .ok_or(Error::NoHardware("DMA-BUF plane without object".into()))?;
            if *modifier.get_or_insert(object.format_modifier) != object.format_modifier {
                return Err(Error::NoHardware("mixed DMA-BUF modifiers".into()));
            }
            if object.fd < 0 {
                return Err(Error::NoHardware("DMA-BUF without file".into()));
            }
            // SAFETY: the descriptor owns this open file descriptor while `mapped` lives, and
            // it is only borrowed here to duplicate it.
            let fd = unsafe { BorrowedFd::borrow_raw(object.fd) }
                .try_clone_to_owned()
                .map_err(|e| Error::NoHardware(e.to_string()))?;
            planes.push(crate::DmaBufPlane {
                fd,
                offset: u32::try_from(plane.offset)
                    .map_err(|_| Error::NoHardware("DMA-BUF plane offset".into()))?,
                stride: u32::try_from(plane.pitch)
                    .map_err(|_| Error::NoHardware("DMA-BUF plane pitch".into()))?,
            });
        }
    }
    // One composed NV12 layer, or the two layers R8 and GR88 that make up NV12.
    let fourcc = match layers {
        [one] => one.format,
        [y, uv] if (y.format, uv.format) == (crate::DRM_R8, crate::DRM_GR88) => crate::DRM_NV12,
        _ => 0,
    };
    let modifier = modifier.ok_or(Error::NoHardware("DMA-BUF without planes".into()))?;
    if fourcc != crate::DRM_NV12 || planes.len() != 2 {
        return Err(Error::NoHardware("DMA-BUF is not NV12".into()));
    }
    Ok(crate::DmaBuf::new(
        surface.width(),
        surface.height(),
        fourcc,
        modifier,
        planes,
        mapped,
    ))
}

/// A VA-API device and a pool of NV12 surfaces of one size.
pub(crate) struct VaapiFrames {
    device: *mut ffi::AVBufferRef,
    frames: *mut ffi::AVBufferRef,
}

// SAFETY: the two buffer references are owned by this value alone and are only used through
// `&mut self` or `&self` methods; FFmpeg's buffer reference counting is thread-safe, so moving
// the owner to another thread is sound.
unsafe impl Send for VaapiFrames {}

impl VaapiFrames {
    /// Opens the default VA-API device and a surface pool of `width × height` NV12.
    pub(crate) fn new(width: u32, height: u32) -> Result<VaapiFrames, Error> {
        let w = i32::try_from(width).map_err(|_| Error::BadParameter("width"))?;
        let h = i32::try_from(height).map_err(|_| Error::BadParameter("height"))?;
        let device = create_device()?;
        // From here on `Drop` releases what was allocated.
        let mut this = VaapiFrames {
            device,
            frames: ptr::null_mut(),
        };
        // SAFETY: `device` is a valid device reference created above.
        let frames = unsafe { ffi::av_hwframe_ctx_alloc(this.device) };
        if frames.is_null() {
            return Err(Error::NoHardware("no VA-API frame pool".into()));
        }
        this.frames = frames;
        // SAFETY: `frames` is a freshly allocated, not yet initialised frames context buffer;
        // its `data` points to an `AVHWFramesContext` (FFmpeg's documented layout) that we may
        // fill in before `av_hwframe_ctx_init`.
        let r = unsafe {
            let ctx = (*frames).data.cast::<ffi::AVHWFramesContext>();
            if ctx.is_null() {
                return Err(Error::NoHardware("frame pool without context".into()));
            }
            (*ctx).format = ffi::AVPixelFormat::AV_PIX_FMT_VAAPI;
            (*ctx).sw_format = ffi::AVPixelFormat::AV_PIX_FMT_NV12;
            (*ctx).width = w;
            (*ctx).height = h;
            (*ctx).initial_pool_size = 8;
            ffi::av_hwframe_ctx_init(frames)
        };
        if r < 0 {
            return Err(Error::NoHardware(format!(
                "VA-API frame pool refused ({r})"
            )));
        }
        Ok(this)
    }

    /// Gives an encoder context (not yet opened) a reference to the surface pool.
    pub(crate) fn attach(&self, ctx: &mut ff::codec::context::Context) -> Result<(), Error> {
        // SAFETY: `self.frames` is a valid, initialised frames context; `av_buffer_ref` returns
        // a new reference or null.
        let r = unsafe { ffi::av_buffer_ref(self.frames) };
        if r.is_null() {
            return Err(Error::NoHardware("no reference to the frame pool".into()));
        }
        // SAFETY: `ctx` is a live codec context owned by the caller and not yet opened; the
        // field takes ownership of the new reference, and FFmpeg releases it with the context.
        unsafe {
            let c = ctx.as_mut_ptr();
            if !(*c).hw_frames_ctx.is_null() {
                ffi::av_buffer_unref(&mut (*c).hw_frames_ctx);
            }
            (*c).hw_frames_ctx = r;
        }
        Ok(())
    }

    /// A VA-API surface with the content of an NV12 picture of the pool's size.
    pub(crate) fn upload(&self, nv12: &ff::frame::Video) -> Result<ff::frame::Video, Error> {
        if nv12.format() != ff::format::Pixel::NV12 {
            return Err(Error::BadParameter("upload needs NV12"));
        }
        let mut surface = ff::frame::Video::empty();
        // SAFETY: `surface` is an empty, owned AVFrame; `self.frames` is an initialised pool.
        let r = unsafe { ffi::av_hwframe_get_buffer(self.frames, surface.as_mut_ptr(), 0) };
        if r < 0 {
            return Err(Error::NoHardware(format!("no free VA-API surface ({r})")));
        }
        // SAFETY: `surface` now holds a VA-API surface of the pool; `nv12` is a valid software
        // frame of the pool's software format and size (checked by the caller and above).
        let r = unsafe { ffi::av_hwframe_transfer_data(surface.as_mut_ptr(), nv12.as_ptr(), 0) };
        if r < 0 {
            return Err(Error::NoHardware(format!("upload to the GPU failed ({r})")));
        }
        surface.set_pts(nv12.pts());
        Ok(surface)
    }
}

impl Drop for VaapiFrames {
    fn drop(&mut self) {
        // SAFETY: both pointers are either null or references this value owns; `av_buffer_unref`
        // accepts null and sets the pointer to null, so nothing is released twice.
        unsafe {
            ffi::av_buffer_unref(&mut self.frames);
            ffi::av_buffer_unref(&mut self.device);
        }
    }
}
