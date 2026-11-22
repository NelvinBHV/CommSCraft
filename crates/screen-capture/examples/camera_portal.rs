//! Reads a few pictures from the camera through the Camera portal and prints their format
//! (manual test, issue #257): `camera_portal [COUNT]`.

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(10);
    println!(
        "camera present: {}",
        commscraft_screen_capture::camera_present()
    );
    let capture = match commscraft_screen_capture::start_camera_portal(640, 480, 30) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let start = std::time::Instant::now();
    for i in 0..count {
        match capture
            .frames
            .recv_timeout(std::time::Duration::from_secs(5))
        {
            Ok(f) => println!(
                "{i}: {}x{} stride {} {:?} {} bytes after {} ms",
                f.width,
                f.height,
                f.stride,
                f.pixels,
                f.data.len(),
                start.elapsed().as_millis()
            ),
            Err(_) => {
                eprintln!("no picture within 5 s");
                std::process::exit(2);
            }
        }
    }
}
