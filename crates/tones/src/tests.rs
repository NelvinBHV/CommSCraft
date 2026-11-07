use super::*;
use std::f64::consts::PI;

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

/// The strongest step between neighbouring samples inside the sound and at its loop seam.
fn steps(s: &[i16]) -> (i32, i32) {
    let inside = s
        .windows(2)
        .map(|w| (i32::from(w[0]) - i32::from(w[1])).abs())
        .max()
        .unwrap();
    let seam = (i32::from(s[s.len() - 1]) - i32::from(s[0])).abs();
    (inside, seam)
}

fn sine(rate: u32, hz: f64, samples: usize, level: f64) -> Pcm {
    Pcm {
        rate,
        samples: (0..samples)
            .map(|i| (level * (2.0 * PI * hz * i as f64 / f64::from(rate)).sin()) as i16)
            .collect(),
    }
}

fn rms(s: &[i16]) -> f64 {
    (s.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
}

#[test]
fn resampling_is_band_limited() {
    // A tone below the new Nyquist frequency keeps its level.
    let pass = sine(44100, 1000.0, 44100, 10000.0);
    let out = resample(&pass, 16000);
    assert_eq!(out.len(), 16000);
    let middle = &out[1000..15000];
    assert!(
        (rms(middle) / rms(&pass.samples) - 1.0).abs() < 0.01,
        "{}",
        rms(middle)
    );
    // 12 kHz does not fold back into 16 kHz sampling (it would appear at 4 kHz).
    let stop = sine(44100, 12000.0, 44100, 10000.0);
    let out = resample(&stop, 16000);
    assert!(rms(&out[1000..15000]) < 50.0, "{}", rms(&out[1000..15000]));
    // Equal rates change nothing; upward works too.
    assert_eq!(resample(&pass, 44100), pass.samples);
    assert_eq!(resample(&pass, 48000).len(), 48000);
    assert_eq!(
        resample(
            &Pcm {
                rate: 8000,
                samples: vec![]
            },
            16000
        ),
        Vec::<i16>::new()
    );
}

#[test]
fn looped_resampling_joins_without_a_jump() {
    // 100 whole cycles of a tone: the seam is no stronger than any other step.
    let tone = sine(44100, 441.0, 10000, 10000.0);
    let out = resample_loop(&tone, 16000);
    assert_eq!(out.len(), 3628, "a whole number of samples per cycle");
    let (inside, seam) = steps(&out);
    assert!(seam <= inside, "{seam} > {inside}");
}

#[test]
fn volume_and_levels() {
    let mut s = vec![100, -400, 200];
    normalize(&mut s);
    assert_eq!(s.iter().map(|x| x.abs()).max(), Some(PEAK as i16));
    let mut quiet = vec![0; 10];
    normalize(&mut quiet);
    assert_eq!(quiet, vec![0; 10]);
    let mut v = vec![1000, -1000];
    set_volume(&mut v, 50);
    assert_eq!(v, vec![500, -500]);
    set_volume(&mut v, 100);
    assert_eq!(v, vec![500, -500]);
    set_volume(&mut v, 500);
    assert_eq!(v, vec![500, -500], "never louder than the sound");
    set_volume(&mut v, 0);
    assert_eq!(v, vec![0, 0]);
}

#[test]
fn looper_goes_on_seamlessly() {
    // The fade silences the ends of a user's file.
    let mut l = Looper::new(vec![1000; 300], 8000, true);
    let a = l.next_frame(160);
    let b = l.next_frame(160);
    assert_eq!((a.len(), b.len()), (160, 160));
    assert_eq!(a[0], 0, "faded in");
    assert_eq!(b[140], 0, "starts faded in again");
    assert_eq!(Looper::new(vec![], 8000, true).next_frame(4), vec![0; 4]);
    // Without a fade: frame after frame exactly the sound, over the end of the loop.
    let sound: Vec<i16> = (0..7).collect();
    let mut l = Looper::new(sound.clone(), 8000, false);
    let played: Vec<i16> = (0..5).flat_map(|_| l.next_frame(3)).collect();
    let expected: Vec<i16> = sound.iter().cycle().take(15).copied().collect();
    assert_eq!(played, expected);
}

/// #82, #83: exactly the bars of the approved scores at 112 BPM, in a sample-exact cycle.
fn beats(p: &Pcm) -> f64 {
    p.samples.len() as f64 / f64::from(p.rate) * 112.0 / 60.0
}

#[test]
fn the_approved_sound_pack() {
    let pulse = soundpack::pulse().unwrap();
    assert_eq!((pulse.rate, pulse.samples.len()), (44100, 2_268_000));
    assert!((beats(&pulse) - 96.0).abs() < 1e-9, "24 bars of 4/4");
    let call = soundpack::beat_call().unwrap();
    assert_eq!((call.rate, call.samples.len()), (44100, 189_000));
    assert!((beats(&call) - 8.0).abs() < 1e-9, "2 bars of 4/4");
    let signal = soundpack::signal_message().unwrap();
    assert_eq!((signal.rate, signal.samples.len()), (44100, 52_920));
    // The masters join without a click, also at the rates of the audio codecs.
    for looped in [&pulse, &call] {
        let (inside, seam) = steps(&looped.samples);
        assert!(seam <= inside, "master: {seam} > {inside}");
        for rate in [8000, 16000] {
            let out = resample_loop(looped, rate);
            let want = looped.samples.len() as f64 * f64::from(rate) / 44100.0;
            assert!((out.len() as f64 - want).abs() <= 0.5, "{}", out.len());
            let (inside, seam) = steps(&out);
            assert!(seam <= inside, "{rate} Hz: {seam} > {inside}");
            // Not silent, and about as loud as the master (it has little above 4 kHz).
            assert!(rms(&out) > rms(&looped.samples) * 0.7, "{rate} Hz");
        }
    }
    // The notification ends in silence (played once, no click at the end).
    assert_eq!(signal.samples.last(), Some(&0));
}
