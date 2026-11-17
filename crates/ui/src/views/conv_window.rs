//! Windows of their own: a conversation (chat, participants, meeting content, the bar at the
//! bottom), a call (state, video, controls), the notifications of incoming calls, invitations and
//! messages, the contact card and the about box.

use crate::app::{AddressFor, CommsUi, NewChat, UiIntent, Win};
use crate::model::{
    Call, CallKind, CallState, ConfCommand, ConvKey, ConvState, Gate, Participant, VideoState,
};
use crate::theme::{self, Palette};
use crate::views::icons::{self, Icon};
use crate::widgets;
use egui::{Align, Frame, Layout, Margin, RichText, Ui, ViewportBuilder, ViewportId};

/// The viewport of a window (for screenshots of a window of its own).
pub fn viewport_of(w: Win) -> ViewportId {
    match w {
        Win::Conv(k) => ViewportId::from_hash_of(("conv", k.0)),
        Win::Call(id) => ViewportId::from_hash_of(("call", id)),
        Win::Options => ViewportId::from_hash_of("options"),
        Win::About => ViewportId::from_hash_of("about"),
        Win::Card => ViewportId::from_hash_of("card"),
    }
}

/// A window of its own; embedded (tests, systems without separate windows) an egui window
/// beside the main window. Returns whether the user closed it.
fn window(
    ctx: &egui::Context,
    id: ViewportId,
    builder: ViewportBuilder,
    title: &str,
    size: [f32; 2],
    pos: [f32; 2],
    add: impl FnOnce(&mut Ui),
) -> bool {
    if ctx.embed_viewports() {
        let mut open = true;
        egui::Window::new(title)
            .id(egui::Id::unique(id))
            .default_pos(pos)
            .default_size(size)
            .collapsible(false)
            .open(&mut open)
            .show(ctx, add);
        return !open;
    }
    let mut add = Some(add);
    ctx.show_viewport_immediate(id, builder, |ui, _class| {
        if let Some(f) = add.take() {
            f(ui);
        }
        ui.ctx().input(|i| i.viewport().close_requested())
    })
}

/// Shows every open window; a window the user closed is dropped.
pub fn show_all(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    let wins = app.windows.clone();
    for (i, w) in wins.into_iter().enumerate() {
        let (id, title, size) = match w {
            Win::Conv(k) => {
                let Some(c) = app.state.conversation(k) else {
                    app.windows.retain(|x| *x != w);
                    continue;
                };
                (
                    ViewportId::from_hash_of(("conv", k.0)),
                    c.title.clone(),
                    [620.0, 560.0],
                )
            }
            Win::Call(id) => {
                let Some(c) = app.state.calls.iter().find(|c| c.id == id) else {
                    app.windows.retain(|x| *x != w);
                    continue;
                };
                (
                    ViewportId::from_hash_of(("call", id)),
                    app.state.name_of(&c.peer),
                    [460.0, 420.0],
                )
            }
            Win::Options => (
                ViewportId::from_hash_of("options"),
                app.t.options_title.to_string(),
                [760.0, 560.0],
            ),
            Win::About => (
                ViewportId::from_hash_of("about"),
                app.t.about.to_string(),
                [360.0, 220.0],
            ),
            Win::Card => (
                ViewportId::from_hash_of("card"),
                app.t.contact_card.to_string(),
                [360.0, 300.0],
            ),
        };
        let builder = ViewportBuilder::default()
            .with_title(format!("{title} – CommSCraft"))
            .with_inner_size(size)
            .with_min_inner_size([320.0, 220.0])
            .with_app_id("commscraft");
        let builder = if app.prefs.always_on_top {
            builder.with_window_level(egui::WindowLevel::AlwaysOnTop)
        } else {
            builder
        };
        let focus = app.focus_window == Some(w);
        let pos = [440.0 + 24.0 * i as f32, 10.0 + 24.0 * i as f32];
        let closed = window(ctx, id, builder, &title, size, pos, |ui| {
            theme::apply(ui.ctx(), pal);
            if focus {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if app.screenshot.as_ref().is_some_and(|(x, _)| *x == w) {
                if let Some((_, cb)) = app.screenshot.take() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Screenshot(cb));
                }
            }
            match w {
                Win::Conv(k) => conversation(app, ui, pal, k),
                Win::Call(id) => call(app, ui, pal, id),
                Win::Options => crate::views::options::show(app, ui, pal),
                Win::About => about(app, ui, pal),
                Win::Card => card(app, ui, pal),
            }
        });
        if focus {
            app.focus_window = None;
        }
        if closed {
            app.close_window(w);
        }
    }
    notifications(app, ctx, pal);
}

fn bar_button(
    ui: &mut Ui,
    t: &crate::text::Texts,
    pal: &Palette,
    icon: Icon,
    label: &str,
    gate: &Gate,
    active: bool,
) -> egui::Response {
    let r = icons::button(
        ui,
        icon,
        label,
        44.0,
        active,
        gate.usable(),
        pal.muted,
        pal.primary,
    );
    match gate {
        Gate::Unavailable(why) => r.on_hover_text(format!("{label} – {}", t.reason(*why))),
        Gate::Preview => r.on_hover_text(format!("{label} – {}", t.preview_hint)),
        Gate::Available => r.on_hover_text(label),
    }
}

fn conversation(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    let Some(c) = app.state.conversation(key).cloned() else {
        return;
    };
    if let Some(m) = app.state.conversation_mut(key) {
        m.unread = 0;
    }
    let caps = app.state.capabilities.clone();
    let idle = app.state.active_call().is_none();
    // The bar at the bottom, below the composer.
    egui::Panel::bottom(("conv-bar", key.0))
        .frame(
            Frame::new()
                .fill(pal.surface_subtle)
                .inner_margin(Margin::symmetric(10, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let open = app.participants_open.contains(&key);
                if bar_button(
                    ui,
                    t,
                    pal,
                    Icon::People,
                    t.participants,
                    &Gate::Available,
                    open,
                )
                .clicked()
                {
                    if open {
                        app.participants_open.remove(&key);
                    } else {
                        app.participants_open.insert(key);
                    }
                }
                ui.add_space(12.0);
                let peer = c.peer.clone();
                // A person: a call; a meeting (group conversation): join its audio or video.
                let meeting = peer.is_none() && c.is_group() && c.state == ConvState::Open;
                let video_gate = if (peer.is_some() || meeting) && idle {
                    caps.video_call.clone()
                } else {
                    Gate::Unavailable(crate::model::Reason::NotImplemented)
                };
                let vlabel = if meeting {
                    t.join_meeting_video
                } else {
                    t.video_call
                };
                if bar_button(ui, t, pal, Icon::Camera, vlabel, &video_gate, false).clicked() {
                    match &peer {
                        Some(p) => app.call(p, true),
                        None => app.emit(UiIntent::JoinMeetingVideo { key }),
                    }
                }
                let call_gate = if (peer.is_some() || meeting) && idle {
                    caps.audio_call.clone()
                } else {
                    Gate::Unavailable(crate::model::Reason::NotImplemented)
                };
                let label = if meeting {
                    t.join_meeting_audio
                } else {
                    t.call
                };
                if bar_button(ui, t, pal, Icon::Phone, label, &call_gate, false).clicked() {
                    match &peer {
                        Some(p) => app.call(p, false),
                        None => app.emit(UiIntent::JoinMeetingAudio { key }),
                    }
                }
                let present =
                    bar_button(ui, t, pal, Icon::Screen, t.present, &Gate::Available, false);
                egui::Popup::menu(&present).show(|ui| present_menu(app, ui, key));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let more = bar_button(
                        ui,
                        t,
                        pal,
                        Icon::More,
                        t.more_options,
                        &Gate::Available,
                        false,
                    );
                    egui::Popup::menu(&more).show(|ui| more_menu(app, ui, key));
                });
            });
        });
    // Someone shares in the meeting: say so and offer to view it.
    if let (Some(sharer), true) = (c.sharer.clone(), c.is_group() && c.state == ConvState::Open) {
        egui::Panel::top(("conv-sharing", key.0))
            .frame(
                Frame::new()
                    .fill(pal.primary_soft)
                    .inner_margin(Margin::symmetric(10, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    widgets::ellipsis(
                        ui,
                        RichText::new(format!("{sharer} {}", t.shares_screen)).color(pal.ink),
                    );
                    if ui.button(t.view_shared).clicked() {
                        app.emit(UiIntent::ViewMeetingScreen { key });
                    }
                });
            });
    }
    if app.participants_open.contains(&key) || c.content.is_some() {
        egui::Panel::right(("conv-side", key.0))
            .default_size(230.0)
            .frame(
                Frame::new()
                    .fill(pal.surface)
                    .inner_margin(Margin::same(10)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if app.participants_open.contains(&key) {
                        ui.label(
                            RichText::new(t.participants)
                                .family(theme::bold())
                                .color(pal.ink),
                        );
                        if c.participants.is_empty() {
                            let who = c
                                .peer
                                .clone()
                                .map(|p| app.state.name_of(&p))
                                .unwrap_or_default();
                            ui.label(
                                RichText::new(format!("{who}\n{}", app.state.me.name))
                                    .color(pal.ink),
                            );
                        }
                        let me = app.state.me.uri.clone();
                        // Presenters manage the meeting (the focus checks it again).
                        let presenter = c.participants.iter().any(|p| p.uri == me && p.presenter);
                        let waiting: Vec<_> = c
                            .participants
                            .iter()
                            .filter(|p| p.waiting)
                            .cloned()
                            .collect();
                        if presenter && !waiting.is_empty() {
                            ui.label(
                                RichText::new(format!("{} ({})", t.lobby, waiting.len()))
                                    .size(theme::text::SMALL)
                                    .color(pal.muted),
                            );
                            for p in waiting.iter().take(100) {
                                let name = if p.name.is_empty() {
                                    app.state.name_of(&p.uri)
                                } else {
                                    p.name.clone()
                                };
                                ui.horizontal(|ui| {
                                    widgets::ellipsis(ui, RichText::new(&name).color(pal.ink));
                                    if ui.small_button(t.admit).clicked() {
                                        app.emit(UiIntent::ConferenceCommand {
                                            key,
                                            command: ConfCommand::Admit(vec![p.uri.clone()]),
                                        });
                                    }
                                    if ui.small_button(t.deny).clicked() {
                                        app.emit(UiIntent::ConferenceCommand {
                                            key,
                                            command: ConfCommand::Deny(p.uri.clone()),
                                        });
                                    }
                                });
                            }
                            if waiting.len() > 1 && ui.small_button(t.admit_all).clicked() {
                                app.emit(UiIntent::ConferenceCommand {
                                    key,
                                    command: ConfCommand::Admit(Vec::new()),
                                });
                            }
                            ui.separator();
                        }
                        for p in c.participants.iter().filter(|p| !p.waiting).take(100) {
                            let name = if p.name.is_empty() {
                                app.state.name_of(&p.uri)
                            } else {
                                p.name.clone()
                            };
                            let manage = presenter && p.uri != me;
                            ui.horizontal(|ui| {
                                let presence = app.state.contacts.get(&p.uri).map(|k| k.presence);
                                app.avatar(ui, Some(p.uri.as_str()), &name, 22.0, presence, pal);
                                let r = ui.add(
                                    egui::Label::new(RichText::new(&name).color(pal.ink))
                                        .truncate()
                                        .sense(if manage {
                                            egui::Sense::click()
                                        } else {
                                            egui::Sense::hover()
                                        }),
                                );
                                if manage {
                                    r.context_menu(|ui| participant_menu(app, ui, key, p));
                                }
                                if p.presenter {
                                    ui.label(
                                        RichText::new(t.presenter)
                                            .size(theme::text::SMALL)
                                            .color(pal.muted),
                                    );
                                }
                                if p.muted == Some(true) {
                                    ui.label(
                                        RichText::new(t.muted_mark)
                                            .size(theme::text::SMALL)
                                            .color(pal.muted),
                                    );
                                }
                            });
                        }
                        ui.add_space(10.0);
                    }
                    if c.is_group() && c.state == ConvState::Open {
                        crate::views::content::block(app, ui, pal, key);
                    }
                });
            });
    }
    crate::views::chats::conversation_body(app, ui, pal, key);
}

/// Present: what can be shown to the others (sharing our own screen is not built).
fn present_menu(app: &mut CommsUi, ui: &mut Ui, key: ConvKey) {
    let t = app.t;
    let not_built = Gate::Unavailable(crate::model::Reason::NotImplemented);
    for label in [
        t.share_desktop,
        t.share_window,
        t.present_slides,
        t.whiteboard_new,
        t.poll_new,
    ] {
        widgets::gated(ui, label, &not_built, t, &app.palette(), |ui, l, en| {
            ui.add_enabled(en, egui::Button::new(l))
        });
    }
    ui.separator();
    let is_group = app.state.conversation(key).is_some_and(|c| c.is_group());
    let gate = app.state.capabilities.meeting_content.clone();
    if ui
        .add_enabled(is_group && gate.usable(), egui::Button::new(t.content_show))
        .clicked()
    {
        app.emit(UiIntent::JoinContent { key });
        ui.close();
    }
}

/// A presenter's commands for one participant.
fn participant_menu(app: &mut CommsUi, ui: &mut Ui, key: ConvKey, p: &Participant) {
    let t = app.t;
    let mut cmd = |ui: &mut Ui, label: &str, command: ConfCommand| {
        if ui.button(label).clicked() {
            app.emit(UiIntent::ConferenceCommand { key, command });
            ui.close();
        }
    };
    let user = p.uri.clone();
    if p.presenter {
        cmd(
            ui,
            t.make_attendee,
            ConfCommand::Role {
                user: user.clone(),
                presenter: false,
            },
        );
    } else {
        cmd(
            ui,
            t.make_presenter,
            ConfCommand::Role {
                user: user.clone(),
                presenter: true,
            },
        );
    }
    match p.muted {
        Some(true) => cmd(
            ui,
            t.unmute_user,
            ConfCommand::MuteUser {
                user: user.clone(),
                mute: false,
            },
        ),
        Some(false) => cmd(
            ui,
            t.mute_user,
            ConfCommand::MuteUser {
                user: user.clone(),
                mute: true,
            },
        ),
        None => {}
    }
    ui.separator();
    cmd(ui, t.remove_from_meeting, ConfCommand::Remove(user));
}

fn more_menu(app: &mut CommsUi, ui: &mut Ui, key: ConvKey) {
    let t = app.t;
    let Some(c) = app.state.conversation(key).cloned() else {
        return;
    };
    let can_invite = c.is_group() && c.state == ConvState::Open;
    if can_invite
        && ui
            .add_enabled(
                app.state.capabilities.group_chat.usable(),
                egui::Button::new(t.invite),
            )
            .clicked()
    {
        app.new_chat = Some(NewChat {
            text: String::new(),
            purpose: AddressFor::Invite(key),
        });
        app.focus_main();
        ui.close();
    }
    if let Some(id) = app.favorite_id(key) {
        let fav = app.prefs.favorites.contains(&id);
        if ui
            .button(if fav { t.unfavorite } else { t.favorite })
            .clicked()
        {
            app.toggle_favorite(id);
            ui.close();
        }
    }
    // A presenter's meeting commands.
    let me = app.state.me.uri.clone();
    if c.is_group()
        && c.state == ConvState::Open
        && c.participants.iter().any(|p| p.uri == me && p.presenter)
    {
        ui.separator();
        for (label, command) in [
            (t.mute_audience, ConfCommand::MuteAudience(true)),
            (t.unmute_audience, ConfCommand::MuteAudience(false)),
            (t.lock_meeting, ConfCommand::Lock(true)),
            (t.unlock_meeting, ConfCommand::Lock(false)),
        ] {
            if ui.button(label).clicked() {
                app.emit(UiIntent::ConferenceCommand { key, command });
                ui.close();
            }
        }
    }
    ui.separator();
    if !c.is_room()
        && matches!(c.state, ConvState::Open | ConvState::Opening)
        && ui.button(t.end_conversation).clicked()
    {
        app.emit(UiIntent::EndConversation { key });
        ui.close();
    }
}

fn call(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, id: u32) {
    let t = app.t;
    let Some(call) = app.state.calls.iter().find(|c| c.id == id).cloned() else {
        return;
    };
    let caps = app.state.capabilities.clone();
    let connected = matches!(call.state, CallState::Connected { .. });
    // The audio of a meeting is named like its conversation.
    let name = match call.conference.and_then(|k| app.state.conversation(k)) {
        Some(c) => c.title.clone(),
        None => app.state.name_of(&call.peer),
    };
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .fill(pal.navigation_bg)
                .inner_margin(Margin::same(16)),
        )
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                app.avatar(ui, Some(call.peer.as_str()), &name, 96.0, None, pal);
                ui.label(
                    RichText::new(&name)
                        .size(theme::text::HEADING)
                        .family(theme::bold())
                        .color(pal.ink),
                );
                let state = if call.kind == CallKind::Screen && call.state == CallState::Incoming {
                    t.screen_offered.to_string()
                } else {
                    crate::views::calls::state_text(app, call.state, call.held, call.kind)
                };
                ui.label(RichText::new(state).color(if connected && !call.held {
                    pal.success_text
                } else {
                    pal.muted
                }));
                let video = match &call.video {
                    VideoState::None => None,
                    VideoState::Starting => Some(t.video_starting.to_string()),
                    VideoState::Window => Some(t.video_window.to_string()),
                    VideoState::Failed(why) => Some(format!("{}: {why}", t.video_failed)),
                };
                if let Some(v) = video {
                    ui.label(RichText::new(v).size(theme::text::SMALL).color(pal.muted));
                }
                if call.demo {
                    ui.label(
                        RichText::new(t.demo_call)
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                }
                ui.add_space(16.0);
                if call.state == CallState::Incoming {
                    incoming_buttons(app, ui, pal, &call);
                } else {
                    controls(app, ui, pal, &call, connected, &caps);
                }
            });
        });
}

fn incoming_buttons(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, call: &Call) {
    let t = app.t;
    let caps = app.state.capabilities.clone();
    ui.horizontal(|ui| {
        if call.kind == CallKind::Screen {
            if ui
                .add_enabled(caps.video_call.usable(), egui::Button::new(t.view_screen))
                .clicked()
            {
                app.emit(UiIntent::ViewScreen { id: call.id });
            }
        } else {
            if widgets::primary(ui, t.answer, caps.audio_call.usable(), pal).clicked() {
                app.emit(UiIntent::AnswerCall { id: call.id });
            }
            if call.offers_video
                && ui
                    .add_enabled(caps.video_call.usable(), egui::Button::new(t.answer_video))
                    .clicked()
            {
                app.emit(UiIntent::AnswerVideo { id: call.id });
            }
        }
        if widgets::danger(ui, t.decline, pal).clicked() {
            app.emit(UiIntent::HangUp { id: call.id });
        }
    });
}

fn controls(
    app: &mut CommsUi,
    ui: &mut Ui,
    pal: &Palette,
    call: &Call,
    connected: bool,
    caps: &crate::model::Capabilities,
) {
    let t = app.t;
    ui.horizontal(|ui| {
        if call.kind != CallKind::Screen {
            if ui
                .add_enabled(
                    connected,
                    egui::Button::new(if call.muted { t.unmute } else { t.mute })
                        .selected(call.muted),
                )
                .clicked()
            {
                app.emit(UiIntent::Mute {
                    id: call.id,
                    muted: !call.muted,
                });
            }
            // Holding and transferring belong to calls with a person, not to a meeting's audio.
            let person = call.conference.is_none();
            if person
                && ui
                    .add_enabled(
                        connected && caps.hold.usable(),
                        egui::Button::new(if call.held { t.resume } else { t.hold })
                            .selected(call.held),
                    )
                    .clicked()
            {
                app.emit(UiIntent::Hold {
                    id: call.id,
                    hold: !call.held,
                });
            }
            if person {
                // To a person (unattended), or to the party of another call (attended: the
                // consultation call is replaced).
                let others: Vec<(u32, String)> = app
                    .state
                    .calls
                    .iter()
                    .filter(|o| {
                        o.id != call.id
                            && o.conference.is_none()
                            && matches!(o.state, CallState::Connected { .. })
                    })
                    .map(|o| (o.id, app.state.name_of(&o.peer)))
                    .collect();
                let b = ui.add_enabled(
                    connected && caps.transfer.usable(),
                    egui::Button::new(t.transfer),
                );
                egui::Popup::menu(&b).show(|ui| {
                    if ui.button(t.transfer_to_person).clicked() {
                        app.new_chat = Some(NewChat {
                            text: String::new(),
                            purpose: AddressFor::Transfer(call.id),
                        });
                        app.focus_main();
                        ui.close();
                    }
                    for (target, name) in &others {
                        if ui
                            .button(format!("{} {name}", t.transfer_to_call))
                            .clicked()
                        {
                            app.emit(UiIntent::TransferToCall {
                                id: call.id,
                                target: *target,
                            });
                            ui.close();
                        }
                    }
                });
            }
            let pad = ui.add_enabled(connected, egui::Button::new(t.dial_pad));
            egui::Popup::menu(&pad).show(|ui| {
                egui::Grid::new(("dtmf", call.id)).show(ui, |ui| {
                    for row in [
                        ["1", "2", "3"],
                        ["4", "5", "6"],
                        ["7", "8", "9"],
                        ["*", "0", "#"],
                    ] {
                        for k in row {
                            if ui.add_sized([40.0, 32.0], egui::Button::new(k)).clicked() {
                                app.emit(UiIntent::Dtmf {
                                    id: call.id,
                                    digits: k.to_string(),
                                });
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        }
    });
    ui.add_space(12.0);
    let label = if call.kind == CallKind::Screen {
        t.stop_viewing
    } else {
        t.hang_up
    };
    if widgets::danger(ui, label, pal).clicked() {
        app.emit(UiIntent::HangUp { id: call.id });
    }
}

/// Incoming calls, invitations and new messages, stacked in one small window.
fn notifications(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    let t = app.t;
    let calls: Vec<Call> = app
        .state
        .calls
        .iter()
        .filter(|c| c.state == CallState::Incoming)
        .cloned()
        .collect();
    let invitations = app.state.invitations.clone();
    let messages: Vec<(ConvKey, String, String)> = app
        .state
        .conversations
        .iter()
        .filter(|c| {
            c.unread > 0
                && !app.windows.contains(&Win::Conv(c.key))
                && !app.toasts_seen.contains(&(c.key, c.messages.len()))
        })
        .map(|c| {
            (
                c.key,
                c.title.clone(),
                c.last_text().map(|m| m.text.clone()).unwrap_or_default(),
            )
        })
        .collect();
    if calls.is_empty() && invitations.is_empty() && messages.is_empty() {
        return;
    }
    let builder = ViewportBuilder::default()
        .with_title(t.notifications.to_string())
        .with_inner_size([
            340.0,
            60.0 + 90.0 * (calls.len() + invitations.len() + messages.len()).min(4) as f32,
        ])
        .with_window_level(egui::WindowLevel::AlwaysOnTop)
        .with_app_id("commscraft");
    let title = t.notifications.to_string();
    window(
        ctx,
        ViewportId::from_hash_of("notify"),
        builder,
        &title,
        [340.0, 300.0],
        [1000.0, 520.0],
        |ui| {
            theme::apply(ui.ctx(), pal);
            egui::CentralPanel::default()
                .frame(
                    Frame::new()
                        .fill(pal.surface)
                        .inner_margin(Margin::same(10)),
                )
                .show(ui, |ui| {
                    for c in &calls {
                        let name = app.state.name_of(&c.peer);
                        let what = match c.kind {
                            CallKind::Screen => t.screen_offered,
                            _ if c.offers_video => t.incoming_video_call,
                            _ => t.incoming_call,
                        };
                        ui.label(RichText::new(&name).family(theme::bold()).color(pal.ink));
                        ui.label(RichText::new(what).color(pal.muted));
                        incoming_buttons(app, ui, pal, c);
                        ui.separator();
                    }
                    for (i, inv) in invitations.iter().enumerate() {
                        ui.label(
                            RichText::new(app.state.name_of(&inv.from))
                                .family(theme::bold())
                                .color(pal.ink),
                        );
                        ui.label(RichText::new(t.invites_to_meeting).color(pal.muted));
                        ui.horizontal(|ui| {
                            if widgets::primary(ui, t.join, true, pal).clicked() {
                                app.emit(UiIntent::AcceptInvitation { index: i });
                            }
                            if ui.button(t.decline).clicked() {
                                app.emit(UiIntent::DeclineInvitation { index: i });
                            }
                        });
                        ui.separator();
                    }
                    for (k, title, text) in &messages {
                        ui.label(RichText::new(title).family(theme::bold()).color(pal.ink));
                        widgets::ellipsis(ui, RichText::new(text).color(pal.muted));
                        ui.horizontal(|ui| {
                            if widgets::primary(ui, t.open, true, pal).clicked() {
                                app.select_conversation(*k);
                            }
                            if ui.button(t.ignore).clicked() {
                                let n = app.state.conversation(*k).map_or(0, |c| c.messages.len());
                                app.toasts_seen.insert((*k, n));
                            }
                        });
                        ui.separator();
                    }
                });
        },
    );
}

fn about(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::same(16)),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("CommSCraft")
                    .size(theme::text::HEADING)
                    .family(theme::bold())
                    .color(pal.ink),
            );
            ui.label(format!("{} {}", app.t.version, env!("CARGO_PKG_VERSION")));
            ui.label(RichText::new(app.t.about_body).color(pal.muted));
        });
}

fn card(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let Some(uri) = app.card.clone() else {
        return;
    };
    let c = app.state.contacts.get(&uri).cloned();
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::same(16)),
        )
        .show(ui, |ui| {
            let name = c
                .as_ref()
                .map_or_else(|| app.state.name_of(&uri), |c| c.display_name().to_string());
            ui.horizontal(|ui| {
                app.avatar(
                    ui,
                    Some(uri.as_str()),
                    &name,
                    56.0,
                    c.as_ref().map(|c| c.presence),
                    pal,
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&name)
                            .size(theme::text::NAME + 2.0)
                            .family(theme::bold())
                            .color(pal.ink),
                    );
                    if let Some(c) = &c {
                        ui.label(RichText::new(t.presence(c.presence)).color(pal.muted));
                    }
                });
            });
            ui.add_space(8.0);
            if let Some(c) = &c {
                for (label, v) in [
                    (t.function, &c.title),
                    (t.department, &c.department),
                    (t.company, &c.company),
                    (t.phone_label, &c.phone),
                    (t.note_label, &c.note),
                ] {
                    if !v.is_empty() {
                        ui.label(RichText::new(format!("{label}: {v}")).color(pal.ink));
                    }
                }
            }
            ui.label(RichText::new(uri.trim_start_matches("sip:")).color(pal.muted));
            ui.add_space(8.0);
            ui.horizontal(|ui| crate::views::main_window::contact_menu(app, ui, &uri));
        });
}
