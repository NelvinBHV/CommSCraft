//! VA-API hardware frames through FFmpeg's hardware context (decision D-13): the only module of
//! CommSCraft with `unsafe` code. It creates a VA-API device and a pool of VA-API surfaces,
//! attaches the pool to an encoder context and uploads NV12 pictures into surfaces. Everything
//! else stays in the safe binding.
//!
//! Rules (D-13): every `unsafe` block names why it is sound; pointers come only from FFmpeg's
//! own allocation functions and are checked for null; reference-counted buffers are released
//! exactly once in `Drop`; return codes are checked before anything is used; no pointer leaves
//! this module.
#![allow(unsafe_code)]

use std::ptr;

use ff::ffi;
use ffmpeg_next as ff;

use crate::Error;

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
        let mut device: *mut ffi::AVBufferRef = ptr::null_mut();
        // SAFETY: `device` is a valid out-pointer; a null device name and options select the
        // default VA-API render node; on failure FFmpeg leaves `device` null.
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
