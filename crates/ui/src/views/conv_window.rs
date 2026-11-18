//! Windows of their own: a conversation (chat, participants, meeting content, the bar at the
//! bottom), a call (state, video, controls), the notifications of incoming calls, invitations and
//! messages, the contact card and the about box.

use crate::app::{AddressFor, CommsUi, NewChat, UiIntent, Win};
use crate::model::{
    Call, CallKind, CallState, ConfCommand, ConvKey, ConvState, Gate, MeetingAccess, Participant,
    Presenters, VideoState,
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
    // As in the Windows client: the open conversations as tabs of one window (an option).
    let convs: Vec<ConvKey> = wins
        .iter()
        .filter_map(|w| match w {
            Win::Conv(k) if app.state.conversation(*k).is_some() => Some(*k),
            _ => None,
        })
        .collect();
    app.windows
        .retain(|w| !matches!(w, Win::Conv(k) if app.state.conversation(*k).is_none()));
    let tabbed = app.prefs.tabbed_conversations && !convs.is_empty();
    if tabbed {
        tabbed_window(app, ctx, pal, &convs);
    }
    for (i, w) in wins.into_iter().enumerate() {
        if tabbed && matches!(w, Win::Conv(_)) {
            continue;
        }
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

/// The conversations as tabs of one window: the list on the left (from two on), the chosen one
/// on the right; closing the window with several tabs asks which to close.
fn tabbed_window(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette, convs: &[ConvKey]) {
    let t = app.t;
    if let Some(Win::Conv(k)) = app.focus_window {
        if convs.contains(&k) {
            app.selected_conv = Some(k);
        }
    }
    let Some(&last) = convs.last() else {
        return;
    };
    let current = app
        .selected_conv
        .filter(|k| convs.contains(k))
        .unwrap_or(last);
    app.selected_conv = Some(current);
    let title = app
        .state
        .conversation(current)
        .map(|c| c.title.clone())
        .unwrap_or_default();
    let focus = matches!(app.focus_window, Some(Win::Conv(_)));
    let size = [if convs.len() > 1 { 820.0 } else { 620.0 }, 560.0];
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
    let several = convs.len() > 1;
    let ask = several && !app.prefs.always_close_all_tabs;
    let closed = window(
        ctx,
        ViewportId::from_hash_of("conversations"),
        builder,
        &title,
        size,
        [440.0, 10.0],
        |ui| {
            theme::apply(ui.ctx(), pal);
            if focus {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            if app
                .screenshot
                .as_ref()
                .is_some_and(|(x, _)| matches!(x, Win::Conv(_)))
            {
                if let Some((_, cb)) = app.screenshot.take() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Screenshot(cb));
                }
            }
            if ask && ui.ctx().input(|i| i.viewport().close_requested()) {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                app.close_tabs_question = true;
            }
            if several {
                egui::Panel::left("conv-tabs")
                    .exact_size(210.0)
                    .resizable(false)
                    .frame(Frame::new().fill(pal.surface).inner_margin(Margin::same(4)))
                    .show(ui, |ui| tab_list(app, ui, pal, convs, current));
            }
            conversation(app, ui, pal, current);
            if app.close_tabs_question {
                close_tabs_dialog(app, ui, current, convs);
            }
        },
    );
    if focus {
        app.focus_window = None;
    }
    if closed {
        if ask {
            app.close_tabs_question = true;
        } else {
            for k in convs {
                app.close_window(Win::Conv(*k));
            }
        }
    }
    let _ = t;
}

/// The tab list: picture, name, the last message, the unread count, a close button.
fn tab_list(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, convs: &[ConvKey], current: ConvKey) {
    let t = app.t;
    let unread_any = convs
        .iter()
        .any(|k| app.state.conversation(*k).is_some_and(|c| c.unread > 0));
    egui::Panel::bottom("conv-tabs-foot")
        .frame(Frame::new().fill(if unread_any {
            pal.warning.gamma_multiply(0.35)
        } else {
            pal.surface
        }))
        .show(ui, |ui| {
            if unread_any {
                ui.label(
                    RichText::new(t.unread_upper)
                        .size(theme::text::SMALL)
                        .color(pal.ink),
                );
            }
        });
    egui::ScrollArea::vertical().show(ui, |ui| {
        for &k in convs {
            let Some(c) = app.state.conversation(k).cloned() else {
                continue;
            };
            let selected = k == current;
            let (rect, resp) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 48.0), egui::Sense::click());
            if selected {
                ui.painter().rect_filled(rect, 0.0, pal.primary_soft);
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 0.0, pal.surface_subtle);
            }
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink2(egui::vec2(6.0, 6.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            app.avatar(&mut child, c.peer.as_deref(), &c.title, 30.0, None, pal);
            let preview = c
                .last_text()
                .map(|m| m.text.lines().next().unwrap_or("").to_string())
                .unwrap_or_default();
            child.vertical(|ui| {
                ui.set_width((ui.available_width() - 30.0).max(40.0));
                let name = RichText::new(&c.title).color(pal.ink);
                widgets::ellipsis(
                    ui,
                    if c.unread > 0 {
                        name.family(theme::bold())
                    } else {
                        name
                    },
                );
                if !preview.is_empty() {
                    widgets::ellipsis(
                        ui,
                        RichText::new(&preview)
                            .size(theme::text::SMALL)
                            .color(pal.muted),
                    );
                }
            });
            child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if (selected || resp.hovered())
                    && ui
                        .add(egui::Button::new("×").frame(false))
                        .on_hover_text(t.close_tab)
                        .clicked()
                {
                    app.close_window(Win::Conv(k));
                } else if c.unread > 0 {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 9.0, pal.error);
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        c.unread.min(99).to_string(),
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                }
            });
            let title = c.title.clone();
            resp.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::Tab, true, selected, &title)
            });
            if resp.clicked() {
                app.selected_conv = Some(k);
            }
        }
    });
}

/// "Close all tabs or the current one?", with "always close all tabs".
fn close_tabs_dialog(app: &mut CommsUi, ui: &mut Ui, current: ConvKey, convs: &[ConvKey]) {
    let t = app.t;
    egui::Modal::new(egui::Id::unique("close-tabs")).show(ui.ctx(), |ui| {
        ui.set_width(380.0);
        ui.label(RichText::new(t.close_tabs_question).size(theme::text::HEADING));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.close_all_tabs).clicked() {
                for k in convs {
                    app.close_window(Win::Conv(*k));
                }
                app.close_tabs_question = false;
                app.emit(UiIntent::PrefsChanged);
            }
            if ui.button(t.close_current_tab).clicked() {
                app.close_window(Win::Conv(current));
                app.close_tabs_question = false;
                app.emit(UiIntent::PrefsChanged);
            }
        });
        ui.separator();
        ui.checkbox(
            &mut app.prefs.always_close_all_tabs,
            t.always_close_all_tabs,
        );
    });
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
    // A call with the IM area hidden: the stage fills the window (the compact call view).
    if app.im_hidden.contains(&key) {
        if let Some(call) = app
            .state
            .calls
            .iter()
            .find(|cl| {
                cl.state != CallState::Incoming && app.state.conversation_of_call(cl) == Some(key)
            })
            .cloned()
        {
            egui::CentralPanel::default()
                .frame(Frame::new().fill(STAGE).inner_margin(Margin::same(10)))
                .show(ui, |ui| stage(app, ui, pal, &call, &c.title));
            return;
        }
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
                        None if app.prefs.meeting_audio_ask => app.meeting_audio = Some(key),
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
    // A call of this conversation: the stage above the IM, as in the Windows client.
    if let Some(call) = app
        .state
        .calls
        .iter()
        .find(|cl| {
            cl.state != CallState::Incoming && app.state.conversation_of_call(cl) == Some(key)
        })
        .cloned()
    {
        let height = (ui.available_height() * 0.62).max(260.0);
        egui::Panel::top(("conv-stage", key.0))
            .resizable(true)
            .default_size(height)
            .frame(Frame::new().fill(STAGE).inner_margin(Margin::same(10)))
            .show(ui, |ui| stage(app, ui, pal, &call, &c.title));
    }
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
                        // As in the Windows client: presenters and attendees, each with
                        // what they take part in (IM, audio, video, sharing).
                        let shown: Vec<Participant> = c
                            .participants
                            .iter()
                            .filter(|p| !p.waiting)
                            .take(100)
                            .cloned()
                            .collect();
                        let groups = [(t.presenters, true), (t.attendees, false)];
                        for (heading, role) in groups {
                            let members: Vec<&Participant> = shown
                                .iter()
                                .filter(|p| p.presenter == role || !c.is_group() && role)
                                .collect();
                            if members.is_empty() {
                                continue;
                            }
                            if c.is_group() {
                                ui.label(
                                    RichText::new(format!("{heading} ({})", members.len()))
                                        .size(theme::text::SMALL)
                                        .color(pal.muted),
                                );
                            }
                            for p in members {
                                participant_row(app, ui, pal, key, p, presenter && p.uri != me);
                            }
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

/// One participant: picture, name (a presenter's menu when we present) and the symbols of
/// what they take part in.
fn participant_row(
    app: &mut CommsUi,
    ui: &mut Ui,
    pal: &Palette,
    key: ConvKey,
    p: &Participant,
    manage: bool,
) {
    let t = app.t;
    let name = if p.name.is_empty() {
        app.state.name_of(&p.uri)
    } else {
        p.name.clone()
    };
    ui.horizontal(|ui| {
        let presence = app.state.contacts.get(&p.uri).map(|k| k.presence);
        app.avatar(ui, Some(p.uri.as_str()), &name, 22.0, presence, pal);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (icon, on, label, struck) in [
                (Icon::Screen, p.sharing, t.sharing_mark, false),
                (Icon::Camera, p.video, t.video_mark, false),
                (
                    Icon::Mic,
                    p.muted.is_some(),
                    t.audio_mark,
                    p.muted == Some(true),
                ),
                (Icon::Chat, p.in_chat, t.im_mark, false),
            ] {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                let colour = if on { pal.primary } else { pal.border };
                icons::draw(ui.painter(), rect, icon, colour);
                if struck {
                    ui.painter().line_segment(
                        [rect.left_top(), rect.right_bottom()],
                        egui::Stroke::new(1.5, colour),
                    );
                }
                resp.on_hover_text(label);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
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
            });
        });
    });
}

/// Who skips the lobby, who becomes a presenter, phone callers, announcements – as the Windows
/// client's meeting options.
pub fn meeting_options_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some((key, mut v)) = app.meeting_options.take() else {
        return;
    };
    let mut keep = true;
    egui::Modal::new(egui::Id::unique("meeting-options")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.label(RichText::new(t.meeting_options).size(theme::text::HEADING));
        ui.add_space(6.0);
        ui.label(t.meeting_access);
        for (a, label) in [
            (MeetingAccess::Invited, t.access_invited),
            (MeetingAccess::Organisation, t.access_organisation),
            (MeetingAccess::Anyone, t.access_anyone),
        ] {
            ui.radio_value(&mut v.access, a, label);
        }
        ui.checkbox(&mut v.phone_bypass, t.phone_bypass);
        ui.checkbox(&mut v.announcements, t.announce_entries);
        ui.add_space(6.0);
        ui.label(t.who_presents);
        for (pr, label) in [
            (Presenters::Nobody, t.presenters_organizer),
            (Presenters::Organisation, t.access_organisation),
            (Presenters::Everyone, t.presenters_everyone),
        ] {
            ui.radio_value(&mut v.presenters, pr, label);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.ok).clicked() {
                let old = app.state.conversation(key).and_then(|c| c.meeting);
                app.emit(UiIntent::ConferenceCommand {
                    key,
                    command: ConfCommand::Options {
                        access: v.access,
                        presenters: v.presenters,
                        phone_bypass: v.phone_bypass,
                    },
                });
                if old.map(|o| o.announcements) != Some(v.announcements) {
                    app.emit(UiIntent::ConferenceCommand {
                        key,
                        command: ConfCommand::Announcements(v.announcements),
                    });
                }
                keep = false;
            }
            if ui.button(t.cancel).clicked() {
                keep = false;
            }
        });
    });
    if keep {
        app.meeting_options = Some((key, v));
    }
}

/// The meeting's link and organizer, as the Windows client's *Meeting Entry Info*. The organizer
/// is the address part of the focus URI (inference: the form of all [MS-CONFBAS] examples,
/// `sip:organizer;gruu;opaque=app:conf:focus:id:…`).
pub fn entry_info_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some(key) = app.entry_info else {
        return;
    };
    let Some(focus) = app.state.conversation(key).and_then(|c| c.focus.clone()) else {
        app.entry_info = None;
        return;
    };
    let organizer_uri = focus.split(';').next().unwrap_or("").to_string();
    let organizer = app.state.name_of(&organizer_uri);
    let link = app
        .state
        .meetings
        .iter()
        .find(|m| m.focus.eq_ignore_ascii_case(&focus))
        .and_then(|m| m.join_url.clone());
    let mut open = true;
    egui::Modal::new(egui::Id::unique("entry-info")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(RichText::new(t.meeting_entry_info).size(theme::text::HEADING));
        ui.add_space(6.0);
        let shown_link = link.clone().unwrap_or_else(|| focus.clone());
        ui.label(RichText::new(t.meeting_link_label).strong());
        ui.add(egui::Label::new(&shown_link).wrap());
        ui.add_space(4.0);
        ui.label(RichText::new(t.organizer).strong());
        ui.label(&organizer);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.copy_all_info).clicked() {
                ui.ctx().copy_text(format!(
                    "{}: {shown_link}\n{}: {organizer}",
                    t.meeting_link_label, t.organizer
                ));
            }
            if ui.button(t.close).clicked() {
                open = false;
            }
        });
    });
    if !open {
        app.entry_info = None;
    }
}

/// How to join a meeting's audio, as the Windows client's *Join Meeting Audio*: the computer's
/// audio, the meeting calls a phone number, or no audio; *Don't ask again*.
pub fn meeting_audio_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some(key) = app.meeting_audio else {
        return;
    };
    let id = egui::Id::unique("meeting-audio-choice");
    let mut choice: u8 = ctx.data(|d| d.get_temp(id)).unwrap_or(0);
    let mut done = None;
    egui::Modal::new(egui::Id::unique("meeting-audio")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.label(RichText::new(t.join_meeting_audio_title).size(theme::text::HEADING));
        ui.add_space(6.0);
        ui.radio_value(&mut choice, 0, t.audio_computer);
        ui.horizontal(|ui| {
            ui.radio_value(&mut choice, 1, t.audio_call_me);
            ui.add_enabled(
                choice == 1,
                egui::TextEdit::singleline(&mut app.prefs.call_me_number)
                    .desired_width(160.0)
                    .char_limit(32)
                    .hint_text("+49 …"),
            );
        });
        ui.radio_value(&mut choice, 2, t.audio_none);
        ui.add_space(4.0);
        let mut never = !app.prefs.meeting_audio_ask;
        if ui.checkbox(&mut never, t.dont_ask_again).changed() {
            app.prefs.meeting_audio_ask = !never;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let ok = choice != 1 || !app.prefs.call_me_number.trim().is_empty();
            if ui.add_enabled(ok, egui::Button::new(t.ok)).clicked() {
                done = Some(choice);
            }
            if ui.button(t.cancel).clicked() {
                done = Some(2);
            }
        });
    });
    ctx.data_mut(|d| d.insert_temp(id, choice));
    if let Some(c) = done {
        match c {
            0 => app.emit(UiIntent::JoinMeetingAudio { key }),
            1 => app.emit(UiIntent::ConferenceCommand {
                key,
                command: ConfCommand::DialOut(app.prefs.call_me_number.trim().to_string()),
            }),
            _ => {}
        }
        app.emit(UiIntent::PrefsChanged);
        app.meeting_audio = None;
    }
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
    if c.is_group() && c.focus.is_some() && ui.button(t.meeting_entry_info).clicked() {
        app.entry_info = Some(key);
        ui.close();
    }
    // A presenter's meeting commands.
    let me = app.state.me.uri.clone();
    if c.is_group()
        && c.state == ConvState::Open
        && c.participants.iter().any(|p| p.uri == me && p.presenter)
    {
        ui.separator();
        if ui.button(t.meeting_options_dots).clicked() {
            let v = c.meeting.unwrap_or_default();
            app.meeting_options = Some((key, v));
            ui.close();
        }
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

/// The dark stage of a call in its conversation window.
const STAGE: egui::Color32 = egui::Color32::from_rgb(0x16, 0x18, 0x1c);

/// The texture of a call's latest received picture.
fn video_texture(app: &CommsUi, ui: &Ui, call: u32) -> Option<egui::TextureHandle> {
    let f = app.state.video.get(&call)?;
    let mut cache = app.video_textures.borrow_mut();
    if let Some((seq, tex)) = cache.get_mut(&call) {
        if *seq != f.seq {
            tex.set(
                egui::ColorImage::from_rgba_unmultiplied(
                    [f.width as usize, f.height as usize],
                    &f.rgba,
                ),
                egui::TextureOptions::LINEAR,
            );
            *seq = f.seq;
        }
        return Some(tex.clone());
    }
    let tex = ui.ctx().load_texture(
        format!("video-{call}"),
        egui::ColorImage::from_rgba_unmultiplied([f.width as usize, f.height as usize], &f.rgba),
        egui::TextureOptions::LINEAR,
    );
    cache.insert(call, (f.seq, tex.clone()));
    Some(tex)
}

/// The stage: the received video (or the peer's picture), name, state and the call controls.
fn stage(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, call: &Call, title: &str) {
    let t = app.t;
    let caps = app.state.capabilities.clone();
    let connected = matches!(call.state, CallState::Connected { .. });
    let name = if call.conference.is_some() {
        title.to_string()
    } else {
        app.state.name_of(&call.peer)
    };
    let light = egui::Color32::from_gray(235);
    let state = crate::views::calls::state_text(app, call.state, call.held, call.kind);
    // State on the right, the name in what is left (shortened in a narrow window).
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(&state).size(theme::text::SMALL).color(light));
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&name).size(theme::text::HEADING).color(light))
                        .truncate(),
                );
            });
        });
    });
    let controls_h = 46.0;
    let area = egui::vec2(
        ui.available_width(),
        (ui.available_height() - controls_h).max(80.0),
    );
    let (rect, _) = ui.allocate_exact_size(area, egui::Sense::hover());
    match video_texture(app, ui, call.id) {
        Some(tex) => {
            // The picture as large as fits, aspect kept, centred.
            let size = tex.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            let shown = egui::Rect::from_center_size(rect.center(), size * scale);
            ui.painter().image(
                tex.id(),
                shown,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            ui.painter().text(
                shown.left_bottom() + egui::vec2(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                &name,
                egui::FontId::proportional(15.0),
                light,
            );
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(66));
        }

        None => {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            child.add_space((rect.height() / 2.0 - 70.0).max(0.0));
            app.avatar(&mut child, Some(call.peer.as_str()), &name, 96.0, None, pal);
            let note = match &call.video {
                VideoState::Starting => Some(t.video_starting.to_string()),
                VideoState::Failed(why) => Some(format!("{}: {why}", t.video_failed)),
                _ => None,
            };
            if let Some(n) = note {
                child.label(RichText::new(n).size(theme::text::SMALL).color(light));
            }
        }
    }
    // The self view: our own camera picture in the lower right corner, as in the Windows client.
    if let Some(tex) = video_texture(app, ui, call.id | crate::model::SELF_VIEW) {
        let size = tex.size_vec2();
        let w = (rect.width() * 0.22).clamp(80.0, 240.0);
        let shown_size = egui::vec2(w, w * size.y / size.x.max(1.0));
        let shown = egui::Rect::from_min_size(
            rect.right_bottom() - shown_size - egui::vec2(10.0, 10.0),
            shown_size,
        );
        ui.painter().image(
            tex.id(),
            shown,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        ui.painter().rect_stroke(
            shown,
            0.0,
            egui::Stroke::new(1.0, light),
            egui::StrokeKind::Outside,
        );
    }
    ui.horizontal_centered(|ui| stage_controls(app, ui, pal, call, connected, &caps));
}

/// A round button of the stage: filled circle, white symbol; `None` colour: the theme's.
fn round_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    fill: egui::Color32,
    enabled: bool,
    struck: bool,
) -> egui::Response {
    let size = egui::vec2(40.0, 40.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let fill = if !enabled {
        fill.gamma_multiply(0.35)
    } else if resp.hovered() {
        fill.gamma_multiply(1.15)
    } else {
        fill
    };
    ui.painter().circle_filled(rect.center(), 20.0, fill);
    icons::draw(ui.painter(), rect.shrink(11.0), icon, egui::Color32::WHITE);
    if struck {
        ui.painter().line_segment(
            [
                rect.left_top() + egui::vec2(11.0, 11.0),
                rect.right_bottom() - egui::vec2(11.0, 11.0),
            ],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
    }
    let label = label.to_string();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &label));
    resp.on_hover_text(label)
}

/// The call controls menu of the stage: hold, transfer (a person, or another call's party),
/// dial pad – submenus, as nested pop-ups would close each other.
fn call_menu(
    app: &mut CommsUi,
    ui: &mut Ui,
    call: &Call,
    connected: bool,
    caps: &crate::model::Capabilities,
) {
    let t = app.t;
    let person = call.conference.is_none();
    if person
        && ui
            .add_enabled(
                connected && caps.hold.usable(),
                egui::Button::new(if call.held { t.resume } else { t.hold }),
            )
            .clicked()
    {
        app.emit(UiIntent::Hold {
            id: call.id,
            hold: !call.held,
        });
        ui.close();
    }
    if person {
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
        ui.add_enabled_ui(connected && caps.transfer.usable(), |ui| {
            ui.menu_button(t.transfer, |ui| {
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
        });
    }
    ui.add_enabled_ui(connected, |ui| {
        ui.menu_button(t.dial_pad, |ui| {
            egui::Grid::new(("dtmf-menu", call.id)).show(ui, |ui| {
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
    });
}

/// The stage's buttons as in the Windows client: IM, microphone, hang up (red), and the call
/// controls (hold, transfer, dial pad).
fn stage_controls(
    app: &mut CommsUi,
    ui: &mut Ui,
    pal: &Palette,
    call: &Call,
    connected: bool,
    caps: &crate::model::Capabilities,
) {
    let t = app.t;
    let blue = pal.primary;
    let red = egui::Color32::from_rgb(0xd9, 0x2b, 0x2b);
    let grey = egui::Color32::from_gray(90);
    ui.add_space(4.0);
    if let Some(key) = app.state.conversation_of_call(call) {
        let hidden = app.im_hidden.contains(&key);
        if round_button(ui, Icon::Chat, t.im_pane, grey, true, hidden).clicked() {
            if hidden {
                app.im_hidden.remove(&key);
            } else {
                app.im_hidden.insert(key);
            }
        }
    }
    ui.add_space(12.0);
    if call.kind != CallKind::Screen
        && round_button(
            ui,
            Icon::Mic,
            if call.muted { t.unmute } else { t.mute },
            blue,
            connected,
            call.muted,
        )
        .clicked()
    {
        app.emit(UiIntent::Mute {
            id: call.id,
            muted: !call.muted,
        });
    }
    let hang = if call.kind == CallKind::Screen {
        t.stop_viewing
    } else {
        t.hang_up
    };
    if round_button(ui, Icon::HangUp, hang, red, true, false).clicked() {
        app.emit(UiIntent::HangUp { id: call.id });
    }
    ui.add_space(12.0);
    if call.kind != CallKind::Screen {
        let b = round_button(ui, Icon::Phone, t.call_controls, grey, connected, false);
        egui::Popup::menu(&b).show(|ui| call_menu(app, ui, call, connected, caps));
    }
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
    // The call window has its own hang-up button; the stage's menu does not need one.
    if app.state.conversation_of_call(call).is_none() {
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
