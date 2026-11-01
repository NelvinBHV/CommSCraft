use super::*;

/// A WAV file built here: format tag, channels, rate, bits, interleaved samples (as written).
fn wav(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8], extra_chunk: bool) -> Vec<u8> {
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&tag.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    let block = channels * bits / 8;
    fmt.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    fmt.extend_from_slice(&block.to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    if tag == 0xfffe {
        fmt.extend_from_slice(&22u16.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        fmt.extend_from_slice(&0u32.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&PCM_GUID_TAIL);
    }
    let mut body = b"WAVE".to_vec();
    if extra_chunk {
        // An odd-sized LIST chunk with its pad byte.
        body.extend_from_slice(b"LIST");
        body.extend_from_slice(&3u32.to_le_bytes());
        body.extend_from_slice(b"abc\0");
    }
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    body.extend_from_slice(&fmt);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(data);
    let mut f = b"RIFF".to_vec();
    f.extend_from_slice(&(body.len() as u32).to_le_bytes());
    f.extend_from_slice(&body);
    f
}

fn le(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

#[test]
fn mono_stereo_24bit_extensible() {
    let mono = wav(1, 1, 16000, 16, &le(&[100, -200, 300]), false);
    assert_eq!(
        parse_wav(&mono),
        Ok(Pcm {
            rate: 16000,
            samples: vec![100, -200, 300]
        })
    );
    // Stereo is averaged; chunks before fmt are skipped (with their pad byte).
    let stereo = wav(1, 2, 44100, 16, &le(&[100, 300, -100, -300]), true);
    assert_eq!(parse_wav(&stereo).unwrap().samples, vec![200, -200]);
    // 24 bits: the upper 16 bits.
    let d24 = [0x00, 0x34, 0x12, 0xff, 0xff, 0xff];
    assert_eq!(
        parse_wav(&wav(1, 1, 8000, 24, &d24, false))
            .unwrap()
            .samples,
        vec![0x1234, -1]
    );
    // WAVE_FORMAT_EXTENSIBLE with the PCM sub-format.
    let ext = wav(0xfffe, 1, 48000, 16, &le(&[7]), false);
    assert_eq!(parse_wav(&ext).unwrap().samples, vec![7]);
}

#[test]
fn hostile_files() {
    assert_eq!(parse_wav(b""), Err(Error::NotWav));
    assert_eq!(parse_wav(b"RIFF\0\0\0\0WAVX"), Err(Error::NotWav));
    assert_eq!(parse_wav(&vec![0; MAX_FILE + 1]), Err(Error::TooLarge));
    // Not PCM (format 3 float, 85 MP3), odd channel counts, rates and sizes.
    assert_eq!(
        parse_wav(&wav(3, 1, 16000, 16, &le(&[1]), false)),
        Err(Error::NotPcm)
    );
    assert_eq!(
        parse_wav(&wav(85, 1, 16000, 16, &le(&[1]), false)),
        Err(Error::NotPcm)
    );
    assert!(matches!(
        parse_wav(&wav(1, 6, 16000, 16, &le(&[1; 6]), false)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        parse_wav(&wav(1, 1, 4000, 16, &le(&[1]), false)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        parse_wav(&wav(1, 1, 16000, 8, &[1], false)),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        parse_wav(&wav(1, 1, 16000, 16, &[], false)),
        Err(Error::NoData)
    );
    // A chunk claiming more than the file holds.
    let mut f = wav(1, 1, 16000, 16, &le(&[1, 2]), false);
    let fmt_len_at = 16;
    f.splice(fmt_len_at..fmt_len_at + 4, 0xffff_fff0u32.to_le_bytes());
    assert_eq!(parse_wav(&f), Err(Error::Truncated));
    // A data chunk longer than the file (streamed writer): what is there is read.
    let mut f = wav(1, 1, 16000, 16, &le(&[5, 6]), false);
    let n = f.len();
    f.splice(n - 8..n - 4, 1000u32.to_le_bytes());
    assert_eq!(parse_wav(&f).unwrap().samples, vec![5, 6]);
    // Too long: 11 minutes at 8 kHz is under the size limit but over the duration limit.
    let long = vec![0u8; 2 * 8000 * 660];
    assert_eq!(
        parse_wav(&wav(1, 1, 8000, 16, &long, false)),
        Err(Error::TooLong)
    );
}

#[test]
fn resample_normalize_loop() {
    let pcm = Pcm {
        rate: 16000,
        samples: (0..1600).map(|i| (i * 10) as i16).collect(),
    };
    assert_eq!(resample(&pcm, 8000).len(), 800);
    assert_eq!(resample(&pcm, 48000).len(), 4800);
    assert_eq!(resample(&pcm, 16000), pcm.samples);
    let mut s = vec![100, -400, 200];
    normalize(&mut s);
    assert_eq!(s.iter().map(|x| x.abs()).max(), Some(PEAK as i16));
    let mut quiet = vec![0; 10];
    normalize(&mut quiet);
    assert_eq!(quiet, vec![0; 10]);
    // The loop goes on seamlessly across its end; the fade silences the ends.
    let mut l = Looper::new(vec![1000; 300], 8000, true);
    let a = l.next_frame(160);
    let b = l.next_frame(160);
    assert_eq!((a.len(), b.len()), (160, 160));
    assert_eq!(a[0], 0, "faded in");
    assert_eq!(b[139], 0, "faded out at the loop end");
    assert_eq!(b[140], 0, "starts faded in again");
    assert_eq!(Looper::new(vec![], 8000, true).next_frame(4), vec![0; 4]);
}

#[test]
fn generated_tones() {
    for rate in [8000, 16000] {
        let ring = ringtone(rate);
        assert_eq!(ring.len(), rate as usize * 3, "a 3 s cycle");
        assert!(ring.iter().any(|s| s.abs() > 10_000), "audible");
        assert!(
            ring[ring.len() - 1] == 0 && ring[0].abs() < 1000,
            "quiet at the ends"
        );
        let n = notification(rate);
        assert_eq!(n.len(), rate as usize * 330 / 1000);
        assert!(n.iter().all(|s| s.abs() <= PEAK as i16));
    }
}
