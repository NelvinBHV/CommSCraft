//! CommSCraft desktop client: the egui window around `commscraft-ui`.
//!
//! `commscraft --demo` shows the interface with made-up data and never opens a network
//! connection. `--screenshot FILE` saves the first settled frame as PNG and quits (used for the
//! documentation screenshots).

mod fonts;

use commscraft_ui::app::{CommsUi, UiIntent};
use commscraft_ui::demo;
use commscraft_ui::model::{AppEvent, Connection, View};
use commscraft_ui::text::Lang;
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The unchanged brand icon (assets/branding, see BRAND-POLICY.md).
const ICON: &[u8] = include_bytes!("../../../assets/branding/CommSCraft-icon-128.png");

const USAGE: &str =
    "usage: commscraft [--demo] [--lang de|en] [--view chats|contacts|calls|meetings|settings]
                  [--size WIDTHxHEIGHT] [--screenshot FILE.png]";

struct Args {
    demo: bool,
    lang: Option<Lang>,
    view: View,
    size: (f32, f32),
    screenshot: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        demo: false,
        lang: None,
        view: View::Chats,
        size: (1440.0, 900.0),
        screenshot: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--demo" => a.demo = true,
            "--lang" => {
                a.lang = Some(match it.next().as_deref() {
                    Some("de") => Lang::De,
                    Some("en") => Lang::En,
                    _ => return Err(USAGE.into()),
                })
            }
            "--view" => {
                a.view = match it.next().as_deref() {
                    Some("chats") => View::Chats,
                    Some("contacts") => View::Contacts,
                    Some("calls") => View::Calls,
                    Some("meetings") => View::Meetings,
                    Some("settings") => View::Settings,
                    _ => return Err(USAGE.into()),
                }
            }
            "--size" => {
                let v = it.next().ok_or(USAGE)?;
                let (w, h) = v.split_once('x').ok_or(USAGE)?;
                let w: f32 = w.parse().map_err(|_| USAGE)?;
                let h: f32 = h.parse().map_err(|_| USAGE)?;
                if !(640.0..=7680.0).contains(&w) || !(480.0..=4320.0).contains(&h) {
                    return Err("window size out of range".into());
                }
                a.size = (w, h);
            }
            "--screenshot" => a.screenshot = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    Ok(a)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The language from LC_ALL, LC_MESSAGES or LANG.
fn system_lang() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                return Lang::from_locale(&v);
            }
        }
    }
    Lang::En
}

/// The local UTC offset in seconds, from `date +%z` (no time zone database in the program).
fn utc_offset() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output();
    let Ok(out) = out else { return 0 };
    parse_offset(String::from_utf8_lossy(&out.stdout).trim()).unwrap_or(0)
}

fn parse_offset(v: &str) -> Option<i64> {
    let (sign, rest) = match v.as_bytes().first()? {
        b'+' => (1, v.get(1..)?),
        b'-' => (-1, v.get(1..)?),
        _ => return None,
    };
    if rest.len() != 4 || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let h: i64 = rest.get(..2)?.parse().ok()?;
    let m: i64 = rest.get(2..)?.parse().ok()?;
    (h <= 14 && m < 60).then_some(sign * (h * 3600 + m * 60))
}

/// RGBA pixels of a PNG (bounded: the brand icon is 128 × 128).
fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w > 1024 || h > 1024 {
        return None;
    }
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buf).ok()?;
    let data = buf.get(..frame.buffer_size())?.to_vec();
    let rgba = match frame.color_type {
        png::ColorType::Rgba => data,
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|[r, g, b]| [*r, *g, *b, 255])
            .collect(),
        _ => return None,
    };
    Some((w, h, rgba))
}

fn save_png(path: &std::path::Path, image: &egui::ColorImage) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let w = u32::try_from(image.size.first().copied().unwrap_or(0)).map_err(|e| e.to_string())?;
    let h = u32::try_from(image.size.get(1).copied().unwrap_or(0)).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    writer.write_image_data(&bytes).map_err(|e| e.to_string())
}

struct DesktopApp {
    ui: CommsUi,
    lang: Lang,
    screenshot: Option<PathBuf>,
    frames: u32,
    shot_rx: Option<std::sync::mpsc::Receiver<std::sync::Arc<egui::ColorImage>>>,
}

impl DesktopApp {
    fn start_demo(&mut self) {
        demo::start(&mut self.ui, now());
    }

    fn handle(&mut self, intent: UiIntent) {
        if self.ui.demo {
            match intent {
                UiIntent::SignOut => {
                    self.ui = CommsUi::new(self.lang, self.ui.utc_offset);
                    self.ui.logo = None;
                }
                i => demo::handle(&mut self.ui.state, &i, now(), self.lang),
            }
            return;
        }
        match intent {
            UiIntent::StartDemo => self.start_demo(),
            UiIntent::SignIn(_) => self.ui.state.apply(AppEvent::Connection(Connection::Failed(
                "this build has no session adapter yet".into(),
            ))),
            _ => {}
        }
    }
}

impl eframe::App for DesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.ui.logo.is_none() {
            if let Some((w, h, rgba)) = decode_png(ICON) {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [
                        usize::try_from(w).unwrap_or(0),
                        usize::try_from(h).unwrap_or(0),
                    ],
                    &rgba,
                );
                self.ui.logo =
                    Some(ctx.load_texture("brand-icon", img, egui::TextureOptions::LINEAR));
            }
        }
        let intents = self.ui.show(ui, now());
        for i in intents {
            self.handle(i);
        }
        // Call timers and typing timeouts are shown in seconds.
        ctx.request_repaint_after(Duration::from_millis(500));
        if let Some(path) = self.screenshot.clone() {
            self.frames += 1;
            if self.frames == 6 && self.shot_rx.is_none() {
                let (tx, rx) = std::sync::mpsc::channel();
                self.shot_rx = Some(rx);
                let repaint = ctx.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                    egui::ScreenshotCallback::new(move |image| {
                        let _ = tx.send(image);
                        repaint.request_repaint();
                    }),
                ));
            }
            let shot = self.shot_rx.as_ref().and_then(|rx| rx.try_recv().ok());
            if let Some(image) = shot {
                match save_png(&path, &image) {
                    Ok(()) => println!("saved {}", path.display()),
                    Err(e) => eprintln!("screenshot: {e}"),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let lang = args.lang.unwrap_or_else(system_lang);
    let offset = utc_offset();
    let fonts = match fonts::system_fonts() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("commscraft: {e}");
            std::process::exit(1);
        }
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("CommSCraft")
        .with_app_id("commscraft")
        .with_inner_size([args.size.0, args.size.1])
        .with_min_inner_size([960.0, 640.0]);
    if let Some((w, h, rgba)) = decode_png(ICON) {
        viewport = viewport.with_icon(egui::IconData {
            rgba,
            width: w,
            height: h,
        });
    }
    if args.screenshot.is_some() {
        viewport = viewport.with_resizable(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "CommSCraft",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(fonts);
            let mut ui = CommsUi::new(lang, offset);
            ui.set_view(args.view);
            let mut app = DesktopApp {
                ui,
                lang,
                screenshot: args.screenshot,
                frames: 0,
                shot_rx: None,
            };
            if args.demo {
                app.start_demo();
            }
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = result {
        eprintln!("commscraft: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_offsets() {
        assert_eq!(parse_offset("+0100"), Some(3600));
        assert_eq!(parse_offset("-0530"), Some(-19_800));
        assert_eq!(parse_offset("0100"), None);
        assert_eq!(parse_offset("+01"), None);
        assert_eq!(parse_offset("+2500"), None);
    }

    #[test]
    fn the_brand_icon_decodes() {
        let (w, h, rgba) = decode_png(ICON).unwrap();
        assert_eq!((w, h), (128, 128));
        assert_eq!(rgba.len(), 128 * 128 * 4);
    }
}
