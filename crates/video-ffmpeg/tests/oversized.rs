//! Issue #93: a few bytes of H.264 must not make FFmpeg allocate a huge picture. A program of
//! its own, because the memory peak of the process is measured.

#![allow(clippy::unwrap_used)]

use commscraft_video_ffmpeg::{init, Decoder, Error, MAX_HEIGHT, MAX_PIXELS, MAX_WIDTH};

/// Bits of an H.264 parameter set or slice header, with Exp-Golomb codes (H.264 §9.1).
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u32,
}

impl Bits {
    fn u(&mut self, n: u32, v: u32) {
        for i in (0..n).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if (v >> i) & 1 == 1 {
                *self.bytes.last_mut().unwrap() |= 0x80 >> (self.used % 8);
            }
            self.used += 1;
        }
    }

    fn ue(&mut self, v: u32) {
        let code = v + 1;
        let len = 32 - code.leading_zeros();
        self.u(len - 1, 0);
        self.u(len, code);
    }

    /// The NAL unit with its header, stop bit and emulation prevention, Annex B start code.
    fn nal(mut self, header: u8) -> Vec<u8> {
        self.u(1, 1);
        let mut out = vec![0, 0, 0, 1, header];
        let mut zeros = 0;
        for b in self.bytes {
            if zeros >= 2 && b <= 3 {
                out.push(3);
                zeros = 0;
            }
            zeros = if b == 0 { zeros + 1 } else { 0 };
            out.push(b);
        }
        out
    }
}

/// An access unit whose sequence parameter set announces `width` × `height` (multiples of 16):
/// constrained baseline SPS and PPS, then an IDR slice header followed by rubbish – under 100
/// bytes on the wire.
fn announcing(width: u32, height: u32) -> Vec<u8> {
    let mut sps = Bits::default();
    sps.u(8, 66); // profile_idc: baseline
    sps.u(8, 0xc0); // constraint flags
    sps.u(8, 52); // level_idc
    sps.ue(0); // seq_parameter_set_id
    sps.ue(0); // log2_max_frame_num_minus4
    sps.ue(2); // pic_order_cnt_type
    sps.ue(1); // max_num_ref_frames
    sps.u(1, 0); // gaps_in_frame_num_value_allowed_flag
    sps.ue(width / 16 - 1); // pic_width_in_mbs_minus1
    sps.ue(height / 16 - 1); // pic_height_in_map_units_minus1
    sps.u(1, 1); // frame_mbs_only_flag
    sps.u(1, 1); // direct_8x8_inference_flag
    sps.u(1, 0); // frame_cropping_flag
    sps.u(1, 0); // vui_parameters_present_flag
    let mut pps = Bits::default();
    pps.ue(0); // pic_parameter_set_id
    pps.ue(0); // seq_parameter_set_id
    pps.u(2, 0); // entropy_coding_mode_flag, bottom_field_pic_order_in_frame_present_flag
    pps.ue(0); // num_slice_groups_minus1
    pps.ue(0); // num_ref_idx_l0_default_active_minus1
    pps.ue(0); // num_ref_idx_l1_default_active_minus1
    pps.u(3, 0); // weighted_pred_flag, weighted_bipred_idc
    pps.ue(0); // pic_init_qp_minus26
    pps.ue(0); // pic_init_qs_minus26
    pps.ue(0); // chroma_qp_index_offset
    pps.u(3, 0); // deblocking control, constrained intra, redundant pic cnt
    let mut slice = Bits::default();
    slice.ue(0); // first_mb_in_slice
    slice.ue(7); // slice_type: I
    slice.ue(0); // pic_parameter_set_id
    slice.u(4, 0); // frame_num
    slice.ue(0); // idr_pic_id
    slice.u(2, 0); // no_output_of_prior_pics_flag, long_term_reference_flag
    slice.ue(0); // slice_qp_delta
    for _ in 0..4 {
        slice.u(32, 0xa5a5_a5a5);
    }
    let mut au = sps.nal(0x67);
    au.extend(pps.nal(0x68));
    au.extend(slice.nal(0x65));
    au
}

/// The highest resident memory of this process so far, in KiB.
fn peak_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
        .unwrap()
}

/// Sizes above the limit are refused by FFmpeg before it allocates: under 100 bytes announcing
/// 8192x8192 cost more than 300 MB before the limit. Sizes up to the limit still decode, and
/// the decoder stays usable after a refusal.
#[test]
fn oversized_pictures_are_refused_before_allocation() {
    init().unwrap();
    assert_eq!(MAX_PIXELS, u64::from(MAX_WIDTH) * u64::from(MAX_HEIGHT));
    let mut decoder = Decoder::new().unwrap();
    // The limit itself is accepted (rubbish slice, concealed picture).
    assert_eq!(
        decoder
            .decode(&announcing(MAX_WIDTH, MAX_HEIGHT))
            .map(|p| p.len()),
        Ok(1)
    );
    let before = peak_kib();
    for (w, h) in [(1920, 1104), (8192, 8192), (16384, 16384), (16384, 1024)] {
        let au = announcing(w, h);
        assert!(au.len() < 100, "{}", au.len());
        for _ in 0..3 {
            match decoder.decode(&au) {
                Err(Error::Ffmpeg(_)) | Err(Error::BadParameter(_)) | Ok(_) => {}
                other => panic!("{w}x{h}: {other:?}"),
            }
        }
        assert!(
            !matches!(decoder.decode(&au), Ok(p) if p.iter().any(|p| p.width > MAX_WIDTH || p.height > MAX_HEIGHT)),
            "{w}x{h} delivered"
        );
    }
    let grown = peak_kib().saturating_sub(before);
    assert!(grown < 64 * 1024, "memory peak grew by {grown} KiB");
    // Back to a size within the limit.
    assert_eq!(
        decoder.decode(&announcing(640, 480)).map(|p| p.len()),
        Ok(1)
    );
}
