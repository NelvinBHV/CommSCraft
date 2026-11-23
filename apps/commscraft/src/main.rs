//! CommSCraft desktop client: the egui window around `commscraft-ui`.
//!
//! Without arguments it shows the sign-in and then works through the session engine (a worker
//! thread, `session.rs`). `commscraft --demo` shows the interface with made-up data and never
//! //! opens a network connection. `--screenshot FILE` saves the first settled frame as PNG and
//! quits (used for the documentation screenshots). `--commands FILE` follows a file of test
//! commands (`script.rs`) that trigger the same actions as the interface.

mod desktop;
mod emoticon_images;
mod fonts;
mod idle;
mod notify;
mod photos;
mod prefs;
mod script;
mod session;
mod spell;
mod tray;
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
                  [--sign-in sip:ADDRESS [--user USER] [--domain DOMAIN] [--server HOST:PORT] [--kerberos] [--tls-dsk] [--compression] [--remember]]
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
            "--compression" => form.compression = true,
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
/// The coloured emoticon pictures (Noto Color Emoji) decoded for the interface.
fn emoticon_pictures() -> Vec<(String, egui::ColorImage)> {
    emoticon_images::IMAGES
        .iter()
        .filter_map(|(emoji, png)| {
            let (w, h, rgba) = decode_png(png)?;
            let size = [usize::try_from(w).ok()?, usize::try_from(h).ok()?];
            Some((
                emoji.to_string(),
                egui::ColorImage::from_rgba_unmultiplied(size, &rgba),
            ))
        })
        .collect()
}

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

/// What the thread of the notification area icon shares with the app.
#[derive(Clone, Default)]
struct TrayControl {
    /// The signed-in worker's command queue (status changes go there directly).
    worker: std::sync::Arc<std::sync::Mutex<Option<std::sync::mpsc::SyncSender<ToWorker>>>>,
    /// "Notifications off", read by the worker's notifications.
    muted: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The window's context while there is a window (to wake it).
    ctx: std::sync::Arc<std::sync::Mutex<Option<egui::Context>>>,
    open: std::sync::Arc<std::sync::atomic::AtomicBool>,
    quit: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Carries out the icon's menu in a thread of its own: a covered or closed window draws no
/// frames (Wayland), so the status, the notifications switch and quitting must not wait for
/// one. Opening and quitting wake the window or the background loop; quitting that finds
/// neither ticking signs out and ends the process after three seconds.
fn spawn_tray_actions(rx: std::sync::mpsc::Receiver<tray::TrayEvent>, c: TrayControl) {
    use std::sync::atomic::Ordering;
    let wake = |c: &TrayControl| {
        if let Some(ctx) = c.ctx.lock().ok().and_then(|g| g.clone()) {
            ctx.request_repaint();
        }
    };
    let _ = std::thread::Builder::new()
        .name("tray-actions".into())
        .spawn(move || {
            for e in rx {
                match e {
                    tray::TrayEvent::Presence(p) => {
                        if let Some(tx) = c.worker.lock().ok().and_then(|g| g.clone()) {
                            let _ = tx.try_send(ToWorker::SetPresence(p));
                        }
                    }
                    tray::TrayEvent::Mute(m) => c.muted.store(m, Ordering::Relaxed),
                    tray::TrayEvent::Open => {
                        c.open.store(true, Ordering::Relaxed);
                        wake(&c);
                    }
                    tray::TrayEvent::Quit => {
                        c.quit.store(true, Ordering::Relaxed);
                        wake(&c);
                        std::thread::sleep(Duration::from_secs(3));
                        if let Some(tx) = c.worker.lock().ok().and_then(|g| g.clone()) {
                            let _ = tx.try_send(ToWorker::SignOut);
                            std::thread::sleep(Duration::from_millis(1500));
                        }
                        std::process::exit(0);
                    }
                }
            }
        });
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
    /// Results of the audio device tests.
    audio_test: (
        std::sync::mpsc::Sender<AppEvent>,
        std::sync::mpsc::Receiver<AppEvent>,
    ),
    worker: Option<session::Worker>,
    worker_done: bool,
    /// The password to keep in the keyring once the sign-in succeeded (the user asked for it).
    pending_store: Option<(String, zeroize::Zeroizing<String>)>,
    pub(crate) ctx: Option<egui::Context>,
    ca: Option<Vec<u8>>,
    started: std::time::Instant,
    screenshot_after: u64,
    /// The desktop session's idle time (status automation).
    idle: Option<idle::Idle>,
    /// The machine activity last handed to the session.
    machine: commscraft_presence::MachineActivity,
    /// A test script's idle time in place of the desktop's (`idle SECONDS`).
    pub(crate) idle_override: Option<u64>,
    /// The desktop's notification service, asked for once (`None` inside: none there); not
    /// used for demos and screenshots. The session worker shows the notifications.
    notifier: Option<Option<std::sync::Arc<notify::Notifier>>>,
    /// What the worker needs to know about the interface for them.
    notes_ui: notify::UiState,
    /// The dictionary being built for this language (spell checking).
    spell_rx: Option<
        std::sync::mpsc::Receiver<Option<std::sync::Arc<dyn commscraft_ui::spell::Speller>>>,
    >,
    spell_lang: Option<Lang>,
    /// The notification area icon, asked for once (`None` inside: no host); not for demos and
    /// screenshots.
    tray: Option<Option<tray::Shared>>,
    /// What the icon shows now.
    tray_shown: Option<tray::Shown>,
    /// The main window was closed into the notification area: the session goes on.
    background: bool,
    /// In the background: something wants the window back (the icon, a notification, a call).
    wants_window: bool,
    /// Quit chosen in the icon's menu.
    quit: bool,
    /// Shared with the thread that carries out the icon's menu.
    tray_control: TrayControl,
}

impl DesktopApp {
    /// Desktop notifications (shown by the session worker): the interface tells it that it is
    /// seen and whether message notifications are quiet; without a notification service the
    /// interface keeps its own notification window.
    fn notes_tick(&mut self) {
        self.notes_tick_seen(true);
    }

    /// As [`Self::notes_tick`]; `seen` false in the background (no window, notifications shown).
    fn notes_tick_seen(&mut self, seen: bool) {
        use std::sync::atomic::Ordering;
        let available = self
            .notifier
            .get_or_insert_with(|| notify::Notifier::start().map(std::sync::Arc::new))
            .is_some();
        self.ui.system_notifications = available;
        if seen {
            self.notes_ui
                .last_frame_ms
                .store(notify::now_ms(), Ordering::Relaxed);
        }
        self.notes_ui
            .messages_quiet
            .store(self.ui.quiet_for_messages(), Ordering::Relaxed);
    }

    /// Spell checking: the dictionary of the interface's language, built in the background;
    /// again when the language changes.
    fn spell_tick(&mut self) {
        if !self.ui.prefs.spell_check {
            return;
        }
        let lang = self.ui.prefs.lang;
        if self.spell_lang != Some(lang) {
            self.spell_lang = Some(lang);
            self.ui.speller = None;
            self.spell_rx = Some(spell::start(lang));
        }
        if let Some(Ok(d)) = self.spell_rx.as_ref().map(|r| r.try_recv()) {
            self.ui.speller = d;
            self.spell_rx = None;
        }
    }

    /// The status automation: idle time against the user's limits; a change goes to the
    /// session, which publishes the machine state again.
    fn idle_tick(&mut self) {
        if self.worker.is_none() {
            self.machine = commscraft_presence::MachineActivity::Active;
            return;
        }
        let idle = self.idle.get_or_insert_with(idle::Idle::start);
        let a = idle::activity(
            self.idle_override.or(idle.seconds()),
            self.ui.prefs.inactive_minutes,
            self.ui.prefs.away_minutes,
        );
        if a != self.machine
            && self
                .worker
                .as_ref()
                .is_some_and(|w| w.send(ToWorker::MachineActivity(a)))
        {
            self.machine = a;
        }
    }

    fn start_demo(&mut self) {
        demo::start(&mut self.ui, now());
    }

    /// Asks the window to draw again, from another thread.
    fn repaint(&self) -> desktop::Repaint {
        let ctx = self.ctx.clone();
        std::sync::Arc::new(move || {
            if let Some(c) = &ctx {
                c.request_repaint();
            }
        })
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
            UiIntent::OpenRecordings => {
                desktop::open_recordings();
                return;
            }
            UiIntent::SendEmail { address } => {
                desktop::send_email(&address);
                return;
            }
            UiIntent::TestSpeaker { device } => {
                desktop::test_speaker(device, self.audio_test.0.clone(), self.repaint());
                return;
            }
            UiIntent::TestMicrophone { device } => {
                desktop::test_microphone(device, self.audio_test.0.clone(), self.repaint());
                return;
            }
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
            self.notes_tick();
            let notes = self.notifier.clone().flatten().map(|n| {
                notify::WorkerNotes::new(n, self.notes_ui.clone(), self.ui.prefs.lang.texts())
            });
            self.worker = Some(session::Worker::start(
                form,
                self.ca.clone(),
                repaint,
                notes,
            ));
            // The worker starts with the defaults: the user's settings at once.
            let p = &self.ui.prefs;
            if let Some(w) = &self.worker {
                w.send(ToWorker::Settings {
                    camera: p.camera.trim().to_string(),
                    sounds: p.sounds,
                    quiet_busy: p.quiet_sounds_busy,
                    save_history: p.save_history,
                    audio_device: p.audio_device.clone(),
                    mic_device: p.mic_device.clone(),
                    message_style: commscraft_im::html::Style {
                        bold: p.im_bold,
                        italic: p.im_italic,
                        underline: p.im_underline,
                        color: p.im_color,
                    },
                });
            }
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
            UiIntent::ShareScreen { key, windows } => {
                self.share_screen(key, windows);
                return;
            }
            UiIntent::ResetStatus => Some(ToWorker::ResetStatus),
            UiIntent::SetNote(n) => Some(ToWorker::SetNote(n)),
            UiIntent::StartConversation { key, peer } => {
                Some(ToWorker::StartConversation { key, peer })
            }
            UiIntent::SetUrgent { key, urgent } => Some(ToWorker::SetUrgent {
                key,
                session: session_of(key),
                urgent,
            }),
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
            UiIntent::JoinMeetingLink { link } => Some(ToWorker::JoinMeetingLink(link)),
            UiIntent::RateCall { id, stars } => Some(ToWorker::RateCall { id, stars }),
            UiIntent::LoadHistory => Some(ToWorker::LoadHistory),
            UiIntent::OpenHistory { id } => Some(ToWorker::OpenHistory { id }),
            UiIntent::TransferToCall { id, target } => {
                Some(ToWorker::TransferToCall { id, target })
            }
            UiIntent::JoinMeetingAudio { key } => {
                session_of(key).map(|session| ToWorker::JoinMeetingAudio { session })
            }
            UiIntent::JoinMeetingVideo { key } => {
                session_of(key).map(|session| ToWorker::JoinMeetingVideo { session })
            }
            UiIntent::ViewMeetingScreen { key } => {
                session_of(key).map(|session| ToWorker::ViewMeetingScreen { session })
            }
            UiIntent::ScheduleMeeting {
                subject,
                minutes,
                access,
                join,
            } => Some(ToWorker::ScheduleMeeting {
                subject,
                minutes,
                access,
                join,
            }),
            UiIntent::ChangeMeeting {
                conference_id,
                subject,
                minutes,
                access,
            } => Some(ToWorker::ChangeMeeting {
                conference_id,
                subject,
                minutes,
                access,
            }),
            UiIntent::CancelMeeting { conference_id } => {
                Some(ToWorker::CancelMeeting { conference_id })
            }
            UiIntent::ConferenceCommand { key, command } => {
                session_of(key).map(|session| ToWorker::ConferenceCommand { session, command })
            }
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
                if self.ui.prefs.autostart != desktop::autostart_enabled() {
                    if let Err(e) = desktop::set_autostart(self.ui.prefs.autostart) {
                        eprintln!("autostart not changed: {e}");
                        self.ui.prefs.autostart = desktop::autostart_enabled();
                    }
                }
                self.lang = self.ui.prefs.lang;
                let p = &self.ui.prefs;
                Some(ToWorker::Settings {
                    camera: p.camera.trim().to_string(),
                    sounds: p.sounds,
                    quiet_busy: p.quiet_sounds_busy,
                    save_history: p.save_history,
                    audio_device: p.audio_device.clone(),
                    mic_device: p.mic_device.clone(),
                    message_style: commscraft_im::html::Style {
                        bold: p.im_bold,
                        italic: p.im_italic,
                        underline: p.im_underline,
                        color: p.im_color,
                    },
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
            UiIntent::CopyContact { uri, group } => {
                Some(ToWorker::Contacts(ContactChange::Copy { uri, group }))
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
            UiIntent::Quit
            | UiIntent::Minimize
            | UiIntent::OpenRecordings
            | UiIntent::FocusMain
            | UiIntent::SendEmail { .. }
            | UiIntent::TestSpeaker { .. }
            | UiIntent::TestMicrophone { .. } => None,
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
        self.ui.audio_devices = desktop::sound_devices();
        self.ui.set_emoticon_images(emoticon_pictures());
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

    /// Sharing the own screen with the conversation's peer: the desktop's dialog runs in a
    /// thread of its own (it waits for the user), then the capture goes to the worker.
    fn share_screen(&mut self, key: commscraft_ui::model::ConvKey, windows: bool) {
        let Some(peer) = self.ui.state.conversation(key).and_then(|c| c.peer.clone()) else {
            return;
        };
        let Some(tx) = self.worker.as_ref().map(|w| w.sender()) else {
            return;
        };
        let notices = self.audio_test.0.clone();
        let failed = self.ui.t.share_failed;
        let repaint = self.ctx.clone();
        let _ = std::thread::Builder::new()
            .name("share-portal".into())
            .spawn(move || {
                let result = commscraft_screen_capture::start_portal(windows)
                    .map_err(|e| e.to_string())
                    .and_then(|capture| {
                        tx.try_send(ToWorker::ShareScreen {
                            to: peer,
                            capture: session::ScreenCapture(capture),
                        })
                        .map_err(|e| e.to_string())
                    });
                if let Err(e) = result {
                    let _ = notices.send(AppEvent::Notice(
                        NoticeKind::Warning,
                        format!("{failed}: {e}"),
                        now(),
                    ));
                    if let Some(c) = repaint {
                        c.request_repaint();
                    }
                }
            });
    }

    /// The notification area icon: started once, kept in line with the own status, its
    /// choices carried out.
    fn tray_tick(&mut self, ctx: Option<&egui::Context>) {
        let t = self.ui.t;
        let signed_in = self.worker.is_some()
            && matches!(
                self.ui.state.connection,
                Connection::Connected | Connection::Degraded(_)
            );
        let presence = self.ui.state.me.presence;
        let shown = tray::Shown {
            presence,
            muted: self
                .tray_control
                .muted
                .load(std::sync::atomic::Ordering::Relaxed),
            signed_in,
            open: t.tray_open.to_string(),
            statuses: tray::STATUSES
                .iter()
                .map(|p| t.presence(*p).to_string())
                .collect(),
            mute: t.tray_mute.to_string(),
            quit: t.tray_quit.to_string(),
            status_text: if signed_in {
                t.presence(presence).to_string()
            } else {
                t.presence(commscraft_ui::model::Presence::Offline)
                    .to_string()
            },
        };
        let tray = self
            .tray
            .get_or_insert_with(|| {
                tray::Tray::start(shown.clone(), decode_png(ICON)).map(std::sync::Arc::new)
            })
            .clone();
        let Some(tray) = tray else { return };
        if self.tray_shown.as_ref() != Some(&shown) {
            tray.show(shown.clone());
            self.tray_shown = Some(shown);
        }
        if let Some(rx) = tray.take_events() {
            spawn_tray_actions(rx, self.tray_control.clone());
        }
        use std::sync::atomic::Ordering;
        if let Ok(mut w) = self.tray_control.worker.lock() {
            *w = self.worker.as_ref().map(|w| w.sender());
        }
        if self.tray_control.open.swap(false, Ordering::Relaxed) {
            match ctx {
                Some(c) => {
                    c.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    c.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                None => self.wants_window = true,
            }
        }
        if self.tray_control.quit.load(Ordering::Relaxed) && !self.quit {
            self.quit = true;
            if let Some(c) = ctx {
                c.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// The window is gone: no context, and the textures of its painter are dropped.
    fn detach(&mut self) {
        self.ctx = None;
        if let Ok(mut c) = self.tray_control.ctx.lock() {
            *c = None;
        }
        self.ui.forget_textures();
    }

    /// A new window: its context (the textures are loaded again on the first frame).
    fn reattach(&mut self, ctx: &egui::Context) {
        self.ctx = Some(ctx.clone());
        if let Ok(mut c) = self.tray_control.ctx.lock() {
            *c = Some(ctx.clone());
        }
        self.ui.forget_textures();
    }

    /// Quitting: signs out (at most three seconds) so the server sees us go at once.
    fn shutdown(&mut self) {
        let Some(w) = &self.worker else { return };
        if !w.send(ToWorker::SignOut) {
            return;
        }
        let until = Instant::now() + Duration::from_secs(3);
        while self.worker.is_some() && Instant::now() < until {
            self.pump_session();
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Whether closing the main window keeps CommSCraft running in the notification area.
    fn closes_to_tray(&self) -> bool {
        !self.quit
            && self.ui.prefs.close_to_tray
            && self.tray.as_ref().is_some_and(|t| t.is_some())
            && self.shots.is_empty()
            && self.close_at.is_none()
    }

    /// One step without a window: the session's events, notifications, the icon.
    fn background_tick(&mut self) {
        self.pump_session();
        self.notes_tick_seen(false);
        self.idle_tick();
        self.tray_tick(None);
        if self.worker_done {
            self.worker_done = false;
            self.worker = None;
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
            if self.background
                && matches!(
                    e,
                    AppEvent::OpenSession { .. } | AppEvent::CallStarted { incoming: true, .. }
                )
            {
                self.wants_window = true;
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
        while let Ok(e) = self.audio_test.1.try_recv() {
            if std::env::var_os("COMMSCRAFT_VERBOSE").is_some() {
                if let AppEvent::MicLevel(l) = &e {
                    eprintln!("microphone level {l:?}");
                }
            }
            self.ui.apply(e);
        }
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
        self.notes_tick();
        self.spell_tick();
        self.idle_tick();
        self.tray_tick(Some(&ctx));
        if ctx.input(|i| i.viewport().close_requested()) && self.closes_to_tray() {
            // The window goes; main() keeps the session running behind the icon.
            self.background = true;
        }
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
    // Received video is shown in the conversation window, as in the Windows client (the decoder
    // stays in its sandboxed process and hands the pictures back).
    commscraft_client::video_window::set_embedded(true);
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
    let embed = args.embed_windows;
    let ctx_for = |cc: &eframe::CreationContext<'_>| {
        cc.egui_ctx.set_fonts(fonts.clone());
        if embed {
            cc.egui_ctx.set_embed_viewports(true);
        }
    };
    // The app outlives its window: closed into the notification area, the session goes on and
    // the window opens again from the icon, a notification or an incoming call.
    let shared: std::rc::Rc<std::cell::RefCell<Option<DesktopApp>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let mut first = Some(move |cc: &eframe::CreationContext<'_>| -> DesktopApp {
        let mut ui = CommsUi::new(lang, offset);
        ui.audio_devices = desktop::sound_devices();
        ui.set_emoticon_images(emoticon_pictures());
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
        let quiet_desktop = args.demo || args.screenshot.is_some();
        let tray_control = TrayControl::default();
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
            audio_test: std::sync::mpsc::channel(),
            shot_since: None,
            worker: None,
            worker_done: false,
            pending_store: None,
            ctx: Some(cc.egui_ctx.clone()),
            ca,
            started: std::time::Instant::now(),
            screenshot_after: args.screenshot_after,
            idle: None,
            machine: commscraft_presence::MachineActivity::Active,
            idle_override: None,
            notifier: quiet_desktop.then_some(None),
            spell_rx: None,
            spell_lang: None,
            tray: quiet_desktop.then_some(None),
            tray_shown: None,
            background: false,
            wants_window: false,
            quit: false,
            tray_control: tray_control.clone(),
            notes_ui: notify::UiState {
                last_frame_ms: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                messages_quiet: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                muted: tray_control.muted.clone(),
            },
        };
        if let Ok(mut c) = tray_control.ctx.lock() {
            *c = Some(cc.egui_ctx.clone());
        }
        if args.demo {
            app.start_demo();
        } else if let Some(form) = args.sign_in {
            app.handle(UiIntent::SignIn(form));
        }
        app
    });
    loop {
        let (sh, make) = (shared.clone(), first.take());
        let result = eframe::run_native(
            "CommSCraft",
            options.clone(),
            Box::new(move |cc| {
                ctx_for(cc);
                let mut slot = sh.borrow_mut();
                match (slot.as_mut(), make) {
                    (Some(app), _) => app.reattach(&cc.egui_ctx),
                    (None, Some(make)) => *slot = Some(make(cc)),
                    (None, None) => {}
                }
                drop(slot);
                Ok(Box::new(Shell(sh)))
            }),
        );
        if let Err(e) = result {
            eprintln!("commscraft: {e}");
            std::process::exit(1);
        }
        let mut slot = shared.borrow_mut();
        let Some(app) = slot.as_mut() else { break };
        if !app.background || app.quit {
            break;
        }
        app.detach();
        while !app.quit && !app.wants_window {
            app.background_tick();
            std::thread::sleep(Duration::from_millis(200));
        }
        if app.quit {
            break;
        }
        app.background = false;
        app.wants_window = false;
    }
    // Quitting signs out as closing the window always did.
    let mut slot = shared.borrow_mut();
    if let Some(app) = slot.as_mut() {
        app.shutdown();
    }
}

/// The window's view of the app that outlives it.
struct Shell(std::rc::Rc<std::cell::RefCell<Option<DesktopApp>>>);

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(app) = self.0.borrow_mut().as_mut() {
            eframe::App::ui(app, ui, frame);
        }
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
