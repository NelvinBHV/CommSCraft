use super::*;

/// Table 1a/1b column 5 at the segment ends and the rows printed in full.
#[test]
fn alaw_decision_values_of_table_1() {
    for (n, x) in [
        (0, 0),
        (1, 2),
        (32, 64),
        (33, 68),
        (48, 128),
        (49, 136),
        (64, 256),
        (65, 272),
        (80, 512),
        (81, 544),
        (96, 1024),
        (97, 1088),
        (112, 2048),
        (113, 2176),
        (127, 3968),
        (128, 4096),
    ] {
        assert_eq!(alaw_x(n), x, "x_{n}");
    }
}

/// Table 1a/1b columns 6–8: character signal before even-bit inversion, quantized value and
/// decoder output value number.
#[test]
fn alaw_characters_and_quantized_values() {
    let rows = [
        (128, 0b1111_1111u8, 4032),
        (113, 0b1111_0000, 2112),
        (97, 0b1110_0000, 1056),
        (81, 0b1101_0000, 528),
        (65, 0b1100_0000, 264),
        (49, 0b1011_0000, 132),
        (33, 0b1010_0000, 66),
        (1, 0b1000_0000, 1),
    ];
    for (n, pos, y) in rows {
        assert_eq!(alaw_y(n), y);
        assert_eq!(alaw_to_uniform(pos ^ 0x55), y as i16, "positive n={n}");
        // Table 1b: the negative character is the positive one without the polarity bit.
        assert_eq!(
            alaw_to_uniform((pos & 0x7f) ^ 0x55),
            -(y as i16),
            "negative n={n}"
        );
        assert_eq!(uniform_to_alaw(y as i16), pos ^ 0x55);
        assert_eq!(uniform_to_alaw(-(y as i16)), (pos & 0x7f) ^ 0x55);
    }
}

#[test]
fn ulaw_decision_values_of_table_2() {
    for (n, x) in [
        (0, 0),
        (1, 1),
        (2, 3),
        (16, 31),
        (17, 35),
        (32, 95),
        (33, 103),
        (48, 223),
        (49, 239),
        (64, 479),
        (65, 511),
        (80, 991),
        (81, 1055),
        (96, 2015),
        (97, 2143),
        (112, 4063),
        (113, 4319),
        (126, 7647),
        (127, 7903),
        (128, 8159),
    ] {
        assert_eq!(ulaw_x(n), x, "x_{n}");
    }
}

/// Table 2a/2b columns 6–8.
#[test]
fn ulaw_characters_and_quantized_values() {
    let rows = [
        (127, 0b1000_0000u8, 8031),
        (112, 0b1000_1111, 4191),
        (96, 0b1001_1111, 2079),
        (80, 0b1010_1111, 1023),
        (64, 0b1011_1111, 495),
        (48, 0b1100_1111, 231),
        (32, 0b1101_1111, 99),
        (16, 0b1110_1111, 33),
        (1, 0b1111_1110, 2),
        (0, 0b1111_1111, 0),
    ];
    for (n, pos, y) in rows {
        assert_eq!(ulaw_y(n), y);
        assert_eq!(ulaw_to_uniform(pos), y as i16, "positive n={n}");
        assert_eq!(ulaw_to_uniform(pos & 0x7f), -(y as i16), "negative n={n}");
        assert_eq!(uniform_to_ulaw(y as i16), pos);
        if y != 0 {
            assert_eq!(uniform_to_ulaw(-(y as i16)), pos & 0x7f);
        }
    }
    // Table 2b row n = 126: character 00000001, −7775.
    assert_eq!(ulaw_to_uniform(0b0000_0001), -7775);
}

fn ulaw_number(code: u8) -> u32 {
    u32::from(0x7f - (code & 0x7f))
}

fn alaw_number(code: u8) -> u32 {
    u32::from((code ^ 0x55) & 0x7f) + 1
}

/// The quantized value of `law` nearest to `v` on a logarithmic scale (`v` > 0), as value
/// number. Recomputed: with the physical scaling of the two laws (A: 4096 ↔ +3.14 dBm0,
/// µ: 8159 ↔ +3.17 dBm0), this rule reproduces Tables 3 and 4 except the two deliberate changes
/// of their Notes – which checks every quantized value of both decoders.
fn nearest(v: f64, alaw: bool) -> u32 {
    let mut best = (f64::MAX, 0u32);
    for code in 0..=255u8 {
        let y = f64::from(if alaw {
            alaw_to_uniform(code)
        } else {
            ulaw_to_uniform(code)
        });
        if y <= 0.0 {
            continue;
        }
        let d = (v / y).ln().abs();
        if d < best.0 {
            best = (
                d,
                if alaw {
                    alaw_number(code)
                } else {
                    ulaw_number(code)
                },
            );
        }
    }
    best.1
}

/// Tables 3 and 4 (µ-A / A-µ conversion).
#[test]
#[allow(clippy::approx_constant)] // Tmax values of G.711 §5
fn conversion_tables_3_and_4() {
    let mu_to_a = 4096.0 / 8159.0 * 10f64.powf((3.17 - 3.14) / 20.0);
    #[rustfmt::skip]
    let table3: [(u8, u32); 88] = [
        (0,1),(1,1),(2,2),(3,2),(4,3),(5,3),(6,4),(7,4),(8,5),(9,5),(10,6),(11,6),(12,7),(13,7),
        (14,8),(15,8),(16,9),(17,10),(18,11),(19,12),(20,13),(21,14),(22,15),(23,16),(24,17),
        (25,18),(26,19),(27,20),(28,21),(29,22),(30,23),(31,24),(32,25),(33,27),(34,29),(35,31),
        (36,33),(37,34),(38,35),(39,36),(40,37),(41,38),(42,39),(43,40),(44,41),(45,42),(46,43),
        (47,44),(48,46),(49,48),(50,49),(51,50),(52,51),(53,52),(54,53),(55,54),(56,55),(57,56),
        (58,57),(59,58),(60,59),(61,60),(62,61),(63,62),(64,64),(65,65),(66,66),(67,67),(68,68),
        (69,69),(70,70),(71,71),(72,72),(73,73),(74,74),(75,75),(76,76),(77,77),(78,78),(79,79),
        (80,81),(81,82),(82,83),(83,84),(84,85),(85,86),(86,87),(87,88),
    ];
    for (mu, a) in table3 {
        let y = f64::from(ulaw_to_uniform(0xff - mu)).max(0.5);
        let got = nearest(y * mu_to_a, true);
        if mu == 80 {
            assert_eq!((got, a), (80, 81), "Note: µ-80 → A-81 instead of A-80");
        } else {
            assert_eq!(got, a, "µ {mu}");
        }
    }
    #[rustfmt::skip]
    let table4: [(u32, u32); 98] = [
        (1,1),(2,3),(3,5),(4,7),(5,9),(6,11),(7,13),(8,15),(9,16),(10,17),(11,18),(12,19),(13,20),
        (14,21),(15,22),(16,23),(17,24),(18,25),(19,26),(20,27),(21,28),(22,29),(23,30),(24,31),
        (25,32),(26,32),(27,33),(28,33),(29,34),(30,34),(31,35),(32,35),(33,36),(34,37),(35,38),
        (36,39),(37,40),(38,41),(39,42),(40,43),(41,44),(42,45),(43,46),(44,47),(45,48),(46,48),
        (47,49),(48,49),(49,50),(50,51),(51,52),(52,53),(53,54),(54,55),(55,56),(56,57),(57,58),
        (58,59),(59,60),(60,61),(61,62),(62,63),(63,64),(64,64),(65,65),(66,66),(67,67),(68,68),
        (69,69),(70,70),(71,71),(72,72),(73,73),(74,74),(75,75),(76,76),(77,77),(78,78),(79,79),
        (80,79),(81,80),(82,81),(83,82),(84,83),(85,84),(86,85),(87,86),(88,87),(89,88),(90,89),
        (91,90),(92,91),(93,92),(94,93),(95,94),(96,95),(97,96),(98,97),
    ];
    for (a, mu) in table4 {
        let y = f64::from(alaw_to_uniform((0x80 | (a - 1) as u8) ^ 0x55));
        let got = nearest(y / mu_to_a, false);
        if a == 80 {
            assert_eq!((got, mu), (80, 79), "Note: A-80 → µ-79 instead of µ-80");
        } else {
            assert_eq!(got, mu, "A {a}");
        }
    }
}

#[allow(clippy::approx_constant)] // 3.14 / 3.17 dBm0 are Tmax values of G.711 §5, not π
fn level_dbm0(samples: &[i16], tmax_peak: f64, tmax_dbm0: f64) -> f64 {
    let p: f64 = samples.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>() / samples.len() as f64;
    // A full-scale sine (peak = virtual decision value) has power Tmax.
    tmax_dbm0 + 10.0 * (p / (tmax_peak * tmax_peak / 2.0)).log10()
}

/// §5 with Tables 5 and 6: the periodic sequences decode to a 1 kHz sine of 0 dBm0.
#[test]
#[allow(clippy::approx_constant)] // Tmax values of G.711 §5
fn digital_milliwatt_tables_5_and_6() {
    let a: Vec<i16> = [0x34u8, 0x21, 0x21, 0x34, 0xb4, 0xa1, 0xa1, 0xb4]
        .iter()
        .map(|&c| alaw_to_uniform(c))
        .collect();
    assert!(a[0] < 0 && a[4] > 0 && a[0] == -a[4] && a[1] == -a[5]);
    assert!(
        level_dbm0(&a, 4096.0, 3.14).abs() < 0.1,
        "{}",
        level_dbm0(&a, 4096.0, 3.14)
    );
    let u: Vec<i16> = [0x1eu8, 0x0b, 0x0b, 0x1e, 0x9e, 0x8b, 0x8b, 0x9e]
        .iter()
        .map(|&c| ulaw_to_uniform(c))
        .collect();
    assert!(
        level_dbm0(&u, 8159.0, 3.17).abs() < 0.1,
        "{}",
        level_dbm0(&u, 8159.0, 3.17)
    );
}

#[test]
fn linear_round_trip_and_bounds() {
    for law in [Law::Alaw, Law::Ulaw] {
        for code in 0..=255u8 {
            let mut pcm = Vec::new();
            law.decode(&[code], &mut pcm);
            let mut back = Vec::new();
            law.encode(&pcm, &mut back);
            if law == Law::Ulaw && code == 0x7f {
                // Negative zero decodes to 0 and encodes as positive zero.
                assert_eq!(back, vec![0xff]);
            } else {
                assert_eq!(back, vec![code], "{law:?} {code:#04x}");
            }
        }
        let mut out = Vec::new();
        law.encode(&[i16::MIN, i16::MAX, 0, -1], &mut out);
        assert_eq!(out.len(), 4);
    }
    assert_eq!((Law::Ulaw.payload_type(), Law::Alaw.payload_type()), (0, 8));
}
