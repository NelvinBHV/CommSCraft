//! Reads a PipeWire video node for two seconds and prints what arrived:
//! `capture_node NODE_ID` (a test source: `gst-launch-1.0 videotestsrc ! pipewiresink`).
fn main() {
    let node = std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<u32>().ok())
        .unwrap_or(0);
    let c = match commscraft_screen_capture::start(commscraft_screen_capture::Source::Node(node)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let end = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut n = 0;
    let mut last = None;
    while std::time::Instant::now() < end {
        if let Ok(f) = c.frames.recv_timeout(std::time::Duration::from_millis(200)) {
            n += 1;
            last = Some((f.width, f.height, f.stride, f.pixels, f.data.len()));
        }
    }
    println!("{n} frames, last {last:?}");
}
