//! CommSCraft desktop client: the egui window around `commscraft-ui`.
//!
//! Without arguments it shows the sign-in and then works through the session engine (a worker
//! thread, `session.rs`). `commscraft --demo` shows the interface with made-up data and never
//! //! opens a network connection. `--screenshot FILE` saves the first settled frame as PNG and
//! quits (used for the documentation screenshots). `--commands FILE` follows a file of test
//! commands (`script.rs`) that trigger the same actions as the interface.

mod fonts;
mod photos;
mod prefs;
mod script;
mod session;
mod vault;

use commscraft_ui::app::{CommsUi, UiIntent};
use commscraft_ui::demo;
use commscraft_ui::model::{AppEvent, Connection, NoticeKind, View};
use commscraft_ui::text::Lang;
use eframe::egui;
use session::{ContactChange, ToWorker};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The unchanged brand icon (assets/branding, see BRAND-POLICY.md).
const ICON: &[u8] = include_bytes!("../../../assets/branding/CommSCraft-icon-128.png");

const USAGE: &str =
    "usage: commscraft [--demo] [--lang de|en] [--view chats|contacts|calls|meetings|settings]
                  [--size WIDTHxHEIGHT] [--screenshot FILE.png] [--screenshot-after SECONDS]
                  [--ca-file PEM] [--commands FILE] [--embed-windows]
                  [--sign-in sip:ADDRESS [--user USER] [--domain DOMAIN] [--server HOST:PORT] [--kerberos] [--tls-dsk] [--remember]]
  --ca-file    adds an enterprise CA to the system trust store (never replaces the checks)
  --sign-in    signs in at start; the password is the first line of standard input
  --commands   follows FILE for test commands (see the documentation in docs/ui/README.md)";

struct Args {
    demo: bool,
    /// Windows of their own drawn inside the main window (screenshots of everything).
    embed_windows: bool,
    lang: Option<Lang>,
    view: View,
    size: (f32, f32),
    screenshot: Option<PathBuf>,
    screenshot_after: u64,
    ca_file: Option<PathBuf>,
    sign_in: Option<commscraft_ui::app::SignIn>,
    commands: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        demo: false,
        embed_windows: false,
        lang: None,
        view: View::Contacts,
        size: (420.0, 760.0),
        screenshot: None,
        screenshot_after: 0,
        ca_file: None,
        sign_in: None,
        commands: None,
    };
    let mut form = commscraft_ui::app::SignIn::default();
    let mut scripted = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--demo" => a.demo = true,
            "--embed-windows" => a.embed_windows = true,
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
            "--screenshot-after" => {
                a.screenshot_after = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|v| *v <= 600)
                    .ok_or(USAGE)?
            }
            "--ca-file" => a.ca_file = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "--commands" => a.commands = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "--sign-in" => {
                scripted = true;
                form.address = it.next().ok_or(USAGE)?;
            }
            "--user" => form.user = it.next().ok_or(USAGE)?,
            "--domain" => form.domain = it.next().ok_or(USAGE)?,
            "--server" => form.server = it.next().ok_or(USAGE)?,
            "--kerberos" => form.kerberos = true,
            "--tls-dsk" => form.tls_dsk = true,
            "--remember" => form.remember = true,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if scripted {
        form.address = commscraft_ui::normalise_address(&form.address)
            .ok_or("--sign-in: not a SIP address")?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        form.password = line.trim_end_matches(['\r', '\n']).to_string();
        line.clear();
        // An empty line: the keyring's password (with --remember, for the last address).
        if form.password.is_empty() && !form.remember {
            return Err("--sign-in: no password on standard input".into());
        }
        a.sign_in = Some(form);
    }
    Ok(a)
}

pub(crate) fn now() -> u64 {
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

pub(crate) struct DesktopApp {
    pub(crate) ui: CommsUi,
    lang: Lang,
    /// Screenshots to take, each with "close the window afterwards".
    /// Screenshots to take: file, close afterwards, the window (`None`: the main window).
    pub(crate) shots: std::collections::VecDeque<(PathBuf, bool, Option<commscraft_ui::app::Win>)>,
    /// When the pending screenshot was asked for (given up after a while).
    pub(crate) shot_since: Option<Instant>,
    commands: Option<std::sync::mpsc::Receiver<String>>,
    pub(crate) close_at: Option<std::time::Instant>,
    frames: u32,
    shot_rx: Option<std::sync::mpsc::Receiver<std::sync::Arc<egui::ColorImage>>>,
    worker: Option<session::Worker>,
    worker_done: bool,
    /// The password to keep in the keyring once the sign-in succeeded (the user asked for it).
    pending_store: Option<(String, zeroize::Zeroizing<String>)>,
    ctx: Option<egui::Context>,
    ca: Option<Vec<u8>>,
    started: std::time::Instant,
    screenshot_after: u64,
}

impl DesktopApp {
    fn start_demo(&mut self) {
        demo::start(&mut self.ui, now());
    }

    pub(crate) fn handle(&mut self, intent: UiIntent) {
        // What concerns the main window itself.
        let root = |c: egui::ViewportCommand| {
            if let Some(ctx) = &self.ctx {
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, c);
            }
        };
        match intent {
            UiIntent::Minimize => return root(egui::ViewportCommand::Minimized(true)),
            UiIntent::FocusMain => return root(egui::ViewportCommand::Focus),
            UiIntent::Quit => {
                // Closing the window signs out (the worker sees the window go).
                return root(egui::ViewportCommand::Close);
            }
            _ => {}
        }
        if self.ui.demo {
            match intent {
                UiIntent::SignOut => self.reset(),
                // The demo never writes the user's preferences.
                UiIntent::PrefsChanged => {}
                i => demo::handle(&mut self.ui.state, &i, now(), self.lang),
            }
            return;
        }
        if let UiIntent::StartDemo = intent {
            self.start_demo();
            return;
        }
        if let UiIntent::SignIn(mut form) = intent {
            // The keyring's password when the field stayed empty.
            if form.password.is_empty()
                && self.ui.stored_password_for.as_deref() == Some(form.address.as_str())
            {
                match vault::load(&form.address) {
                    Some(pw) => form.password = pw.to_string(),
                    None => self.ui.state.apply(AppEvent::Notice(
                        NoticeKind::Warning,
                        "the keyring gave no password".into(),
                        now(),
                    )),
                }
            }
            self.pending_store = (form.remember && !form.password.is_empty()).then(|| {
                (
                    form.address.clone(),
                    zeroize::Zeroizing::new(form.password.clone()),
                )
            });
            self.ui.prefs.last_address = form.address.clone();
            let ctx = self.ctx.clone();
            let repaint: Box<dyn Fn() + Send> = Box::new(move || {
                if let Some(c) = &ctx {
                    c.request_repaint();
                }
            });
            self.worker = Some(session::Worker::start(form, self.ca.clone(), repaint));
            return;
        }
        if matches!(
            intent,
            UiIntent::StartGroup { .. }
                | UiIntent::JoinMeeting { .. }
                | UiIntent::AcceptInvitation { .. }
        ) {
            self.ui.expect_group();
        }
        let state = &mut self.ui.state;
        let session_of = |k| state.conversation(k).and_then(|c| c.session);
        let cmd = match intent {
            UiIntent::SetPresence(p) => Some(ToWorker::SetPresence(p)),
            UiIntent::StartConversation { key, peer } => {
                Some(ToWorker::StartConversation { key, peer })
            }
            UiIntent::SendMessage { key, text } => Some(ToWorker::SendText {
                key,
                session: session_of(key),
                text,
            }),
            UiIntent::Typing { key, typing } => Some(ToWorker::Typing {
                key,
                session: session_of(key),
                typing,
            }),
            UiIntent::EndConversation { key } => Some(ToWorker::EndConversation {
                key,
                session: session_of(key),
            }),
            UiIntent::StartGroup { invite } => Some(ToWorker::StartGroup { invite }),
            UiIntent::InviteToGroup { key, peer } => {
                session_of(key).map(|session| ToWorker::InviteToGroup { session, peer })
            }
            UiIntent::JoinMeeting { focus } => Some(ToWorker::JoinConference { focus }),
            UiIntent::JoinContent { key } => {
                session_of(key).map(|session| ToWorker::JoinContent { session })
            }
            UiIntent::LeaveContent { key } => {
                session_of(key).map(|session| ToWorker::LeaveContent { session })
            }
            UiIntent::Vote {
                key,
                content,
                choice,
            } => session_of(key).map(|session| ToWorker::Vote {
                session,
                content,
                choice,
            }),
            UiIntent::AcceptInvitation { index } => {
                if index < state.invitations.len() {
                    let inv = state.invitations.remove(index);
                    Some(ToWorker::JoinConference { focus: inv.focus })
                } else {
                    None
                }
            }
            UiIntent::DeclineInvitation { index } => {
                if index < state.invitations.len() {
                    state.invitations.remove(index);
                }
                None
            }
            UiIntent::RefreshMeetings => Some(ToWorker::RefreshMeetings),
            UiIntent::CallUser { peer } => Some(ToWorker::CallUser { peer }),
            UiIntent::Transfer { id, peer } => Some(ToWorker::Transfer { id, peer }),
            UiIntent::VideoCall { peer } => Some(ToWorker::VideoCall { peer }),
            UiIntent::AnswerVideo { id } => Some(ToWorker::AnswerVideo { id }),
            UiIntent::ViewScreen { id } => Some(ToWorker::ViewScreen { id }),
            UiIntent::LoadRooms => Some(ToWorker::LoadRooms),
            UiIntent::SendToRoom { room, text } => Some(ToWorker::SendToRoom { room, text }),
            UiIntent::AnswerCall { id } => Some(ToWorker::AnswerCall { id }),
            UiIntent::HangUp { id } => Some(ToWorker::HangUp { id }),
            UiIntent::Hold { id, hold } => Some(ToWorker::Hold { id, hold }),
            UiIntent::Mute { id, muted } => Some(ToWorker::Mute { id, muted }),
            UiIntent::SignOut => {
                if self.worker.is_none() {
                    self.reset();
                    return;
                }
                Some(ToWorker::SignOut)
            }
            UiIntent::SearchDirectory { text } => Some(ToWorker::Search { text }),
            UiIntent::AnswerTrust(a) => Some(ToWorker::Trust(a)),
            UiIntent::PrefsChanged => {
                if let Err(e) = prefs::save(&self.ui.prefs) {
                    eprintln!("preferences not saved: {e}");
                }
                self.lang = self.ui.prefs.lang;
                let p = &self.ui.prefs;
                Some(ToWorker::Settings {
                    camera: p.camera.trim().to_string(),
                    sounds: p.sounds,
                    quiet_busy: p.quiet_sounds_busy,
                })
            }
            UiIntent::Dtmf { id, digits } => Some(ToWorker::Dtmf { id, digits }),
            UiIntent::ExpandGroup { email } => Some(ToWorker::ExpandGroup { email }),
            UiIntent::AddContact { uri, name, group } => {
                Some(ToWorker::Contacts(ContactChange::Add { uri, name, group }))
            }
            UiIntent::RemoveContact { uri } => {
                Some(ToWorker::Contacts(ContactChange::Remove { uri }))
            }
            UiIntent::MoveContact { uri, group } => {
                Some(ToWorker::Contacts(ContactChange::Move { uri, group }))
            }
            UiIntent::AddGroup { name } => {
                Some(ToWorker::Contacts(ContactChange::AddGroup { name }))
            }
            UiIntent::RenameGroup { name, to } => {
                Some(ToWorker::Contacts(ContactChange::RenameGroup { name, to }))
            }
            UiIntent::DeleteGroup { name } => {
                Some(ToWorker::Contacts(ContactChange::DeleteGroup { name }))
            }
            UiIntent::SetRelation { uri, relation } => {
                Some(ToWorker::Contacts(ContactChange::Relation {
                    uri,
                    relation,
                }))
            }
            UiIntent::AcknowledgeSubscriber { uri } => {
                Some(ToWorker::Contacts(ContactChange::Acknowledge { uri }))
            }
            UiIntent::ForgetPassword => {
                if let Some(account) = self.ui.stored_password_for.take() {
                    if let Err(e) = vault::forget(&account) {
                        eprintln!("keyring: {e}");
                    }
                }
                None
            }
            UiIntent::Quit | UiIntent::Minimize | UiIntent::FocusMain => None,
            UiIntent::StartDemo | UiIntent::SignIn(_) => None,
        };
        if let Some(c) = cmd {
            let sent = self.worker.as_ref().is_some_and(|w| w.send(c));
            if !sent {
                self.ui.state.apply(AppEvent::Notice(
                    NoticeKind::Error,
                    "the session is not running or busy – the action was not sent".into(),
                    now(),
                ));
            }
        }
    }

    /// Back to the sign-in screen with an empty state (nothing of the last session stays).
    fn reset(&mut self) {
        let logo = self.ui.logo.take();
        let view = self.ui.view();
        let kept = if self.ui.demo {
            prefs::load()
        } else {
            self.ui.prefs.clone()
        };
        self.ui = CommsUi::new(self.lang, self.ui.utc_offset);
        self.ui.logo = logo;
        self.ui.prefs = kept;
        self.ui.set_view(view);
        self.worker = None;
    }

    /// After a successful sign-in: keep the password if asked, remember the address.
    fn after_sign_in(&mut self) {
        if let Some((account, pw)) = self.pending_store.take() {
            match vault::store(&account, &pw) {
                Ok(()) => self.ui.stored_password_for = Some(account),
                Err(e) => self.ui.state.apply(AppEvent::Notice(
                    NoticeKind::Warning,
                    format!("the password could not be kept in the keyring ({e})"),
                    now(),
                )),
            }
        }
        if let Err(e) = prefs::save(&self.ui.prefs) {
            eprintln!("preferences not saved: {e}");
        }
    }

    fn pump_session(&mut self) {
        let Some(w) = &self.worker else { return };
        let mut ended = false;
        for e in w.events(512) {
            if let AppEvent::Connection(c) = &e {
                ended = matches!(c, Connection::SignedOut);
                if matches!(c, Connection::Failed(_) | Connection::Disconnected(_)) {
                    // The worker has stopped; a new sign-in starts a new one.
                    self.worker_done = true;
                }
            }
            if matches!(e, AppEvent::Connection(Connection::Failed(_))) {
                self.pending_store = None;
            }
            let signed_in = matches!(e, AppEvent::SignedIn(_));
            self.ui.apply(e);
            if signed_in {
                self.after_sign_in();
            }
        }
        if ended {
            self.reset();
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
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
        }
        self.pump_session();
        if self.worker_done {
            self.worker_done = false;
            self.worker = None;
        }
        let intents = self.ui.show(ui, now());
        for i in intents {
            self.handle(i);
        }
        // Call timers and typing timeouts are shown in seconds.
        ctx.request_repaint_after(Duration::from_millis(500));
        self.frames += 1;
        if let Some(rx) = &self.commands {
            let lines: Vec<String> = rx.try_iter().take(16).collect();
            for l in lines {
                script::run(self, &l);
            }
        }
        if self
            .close_at
            .is_some_and(|t| t.elapsed() >= Duration::from_secs(5))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let due = self.frames >= 6 && self.started.elapsed().as_secs() >= self.screenshot_after;
        if due && self.shot_rx.is_none() && !self.shots.is_empty() {
            let (tx, rx) = std::sync::mpsc::channel();
            self.shot_rx = Some(rx);
            let repaint = ctx.clone();
            let cb = egui::ScreenshotCallback::new(move |image| {
                let _ = tx.send(image);
                repaint.request_repaint();
            });
            self.shot_since = Some(Instant::now());
            match self.shots.front().and_then(|s| s.2) {
                // A window of its own takes it in its own frame.
                Some(w) => self.ui.screenshot = Some((w, cb)),
                None => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(cb)),
            }
        }
        if self
            .shot_since
            .is_some_and(|t| t.elapsed() > Duration::from_secs(8))
            && self.shot_rx.is_some()
        {
            self.shot_rx = None;
            self.shot_since = None;
            self.ui.screenshot = None;
            if let Some((path, _, _)) = self.shots.pop_front() {
                eprintln!("screenshot: no picture for {}", path.display());
            }
        }
        let shot = self.shot_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(image) = shot {
            self.shot_rx = None;
            if let Some((path, close, _)) = self.shots.pop_front() {
                match save_png(&path, &image) {
                    Ok(()) => println!("saved {}", path.display()),
                    Err(e) => eprintln!("screenshot: {e}"),
                }
                if close {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if !self.shots.is_empty() || self.commands.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn main() {
    // The sandboxed video window process (function 28, function 43) runs from this binary.
    if std::env::args().nth(1).as_deref() == Some(commscraft_client::video_window::COMMAND) {
        std::process::exit(commscraft_client::video_window::helper_main(
            std::env::args().skip(2),
        ));
    }
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    // The command line, then the language chosen in the options, then the system's.
    let lang = args
        .lang
        .or_else(|| {
            (args.screenshot.is_none() && !args.demo)
                .then(prefs::saved_lang)
                .flatten()
        })
        .unwrap_or_else(system_lang);
    let ca = match &args.ca_file {
        Some(p) => match std::fs::read(p) {
            Ok(b) => Some(b),
            Err(e) => {
                eprintln!("commscraft: {}: {e}", p.display());
                std::process::exit(2);
            }
        },
        None => None,
    };
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
        .with_min_inner_size([340.0, 480.0]);
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
            if args.embed_windows {
                cc.egui_ctx.set_embed_viewports(true);
            }
            let mut ui = CommsUi::new(lang, offset);
            ui.set_view(args.view);
            // Screenshots use the defaults, not the user's preferences.
            if args.screenshot.is_none() && !args.demo {
                ui.prefs = prefs::load();
                // The last sign-in address, and whether the keyring holds its password.
                let last = ui.prefs.last_address.clone();
                if !last.is_empty() {
                    ui.set_sign_in_address(last.trim_start_matches("sip:"));
                    if vault::has(&last) {
                        ui.stored_password_for = Some(last);
                    }
                }
            }
            ui.prefs.lang = lang;
            let mut app = DesktopApp {
                ui,
                lang,
                shots: args
                    .screenshot
                    .map(|p| (p, true, None))
                    .into_iter()
                    .collect(),
                commands: args.commands.map(script::follow),
                close_at: None,
                frames: 0,
                shot_rx: None,
                shot_since: None,
                worker: None,
                worker_done: false,
                pending_store: None,
                ctx: Some(cc.egui_ctx.clone()),
                ca,
                started: std::time::Instant::now(),
                screenshot_after: args.screenshot_after,
            };
            if args.demo {
                app.start_demo();
            } else if let Some(form) = args.sign_in {
                app.handle(UiIntent::SignIn(form));
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
