//! Windows of their own: a conversation (chat, participants, meeting content, the bar at the
//! bottom), a call (state, video, controls), the notifications of incoming calls, invitations and
//! messages, the contact card and the about box.

use crate::app::{AddressFor, CommsUi, NewChat, UiIntent, Win};
use crate::model::{
    Call, CallKind, CallState, ConfCommand, ConvKey, ConvState, Conversation, Gate, MeetingAccess,
    Participant, Presenters, VideoState,
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
    // Where the main window is (known on desktops that tell; not under Wayland).
    let main_rect = ctx.input(|i| i.viewport().outer_rect);
    if !wins.contains(&Win::Card) {
        app.card_dock = None;
    }
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
        // The contact card docks to the right of the main window, as in the Windows client.
        let builder = match (w, main_rect) {
            (Win::Card, Some(r)) => {
                let at = r.right_top() + egui::vec2(4.0, 0.0);
                if app.card_dock.is_some_and(|p| p.distance(at) > 1.0) {
                    ctx.send_viewport_cmd_to(id, egui::ViewportCommand::OuterPosition(at));
                }
                app.card_dock = Some(at);
                builder
                    .with_position(at)
                    .with_inner_size([360.0, r.height().clamp(300.0, 900.0)])
            }
            _ => builder,
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
                    crate::views::chats::emoticon_line(
                        app,
                        ui,
                        &preview,
                        theme::text::SMALL,
                        pal.muted,
                    );
                }
            });
            child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if (selected || resp.hovered())
                    && crate::views::icons::button(
                        ui,
                        Icon::Close,
                        t.close_tab,
                        18.0,
                        false,
                        true,
                        pal.muted,
                        pal.primary,
                    )
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
    let Some(c) = app.state.conversation(key).cloned() else {
        return;
    };
    if let Some(m) = app.state.conversation_mut(key) {
        m.unread = 0;
    }
    // A call: as in the Windows client the IM is a pane left of the stage, hidden at first
    // unless the conversation already has messages; hidden, the stage fills the window.
    let active = app
        .state
        .calls
        .iter()
        .find(|cl| {
            cl.state != CallState::Incoming && app.state.conversation_of_call(cl) == Some(key)
        })
        .cloned();
    if let Some(call) = &active {
        if app.im_call_seen.insert(call.id) && c.messages.is_empty() {
            app.im_hidden.insert(key);
        }
        // As the Windows client: the window grows by the pane when it opens and shrinks again
        // when it closes (embedded windows keep their size).
        let shown = !app.im_hidden.contains(&key);
        if shown != app.im_widened.contains(&key) && !ui.ctx().embed_viewports() {
            // The window's own size (egui reports no inner rectangle for these viewports).
            let r = ui.ctx().content_rect();
            let w = if shown {
                r.width() + IM_PANE_WIDTH
            } else {
                (r.width() - IM_PANE_WIDTH).max(340.0)
            };
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, r.height())));
            if shown {
                app.im_widened.insert(key);
            } else {
                app.im_widened.remove(&key);
            }
        }
    }
    // The call ended: the window goes back to its chat size.
    if active.is_none() && app.im_widened.remove(&key) && !ui.ctx().embed_viewports() {
        let r = ui.ctx().content_rect();
        let w = (r.width() - IM_PANE_WIDTH).max(340.0);
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, r.height())));
    }
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
            // The participants take the pane at the left, as in the Windows client.
            side_panel(app, ui, pal, key, &c);
            egui::CentralPanel::default()
                .frame(Frame::new().fill(STAGE).inner_margin(Margin::same(10)))
                .show(ui, |ui| stage(app, ui, pal, &call, &c.title));
            return;
        }
    }
    if let Some(call) = active {
        // The left pane shows the participants or the IM (one at a time, as in the Windows
        // client).
        if app.participants_open.contains(&key) {
            side_panel(app, ui, pal, key, &c);
            egui::CentralPanel::default()
                .frame(Frame::new().fill(STAGE).inner_margin(Margin::same(10)))
                .show(ui, |ui| stage(app, ui, pal, &call, &c.title));
            return;
        }
        side_panel(app, ui, pal, key, &c);
        let width = IM_PANE_WIDTH.min(ui.available_width() * 0.5);
        egui::Panel::left(("conv-im", key.0))
            .resizable(true)
            .default_size(width)
            .frame(Frame::new().fill(pal.surface))
            .show(ui, |ui| {
                im_pane_head(app, ui, pal, key);
                conv_bar(app, ui, pal, key, &c);
                sharing_banner(app, ui, pal, key, &c);
                crate::views::chats::conversation_im(app, ui, pal, key);
            });
        egui::CentralPanel::default()
            .frame(Frame::new().fill(STAGE).inner_margin(Margin::same(10)))
            .show(ui, |ui| stage(app, ui, pal, &call, &c.title));
        return;
    }
    conv_bar(app, ui, pal, key, &c);
    sharing_banner(app, ui, pal, key, &c);
    side_panel(app, ui, pal, key, &c);
    crate::views::chats::conversation_body(app, ui, pal, key);
}

/// The IM pane's width beside a call's stage (design: as wide as the Windows client's).
const IM_PANE_WIDTH: f32 = 320.0;

/// The head of the IM pane in a call: its title and a button that hides it.
fn im_pane_head(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    egui::Panel::top(("conv-im-head", key.0))
        .frame(Frame::new().inner_margin(Margin::symmetric(12, 8)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t.conversation_heading)
                        .family(theme::bold())
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let resp = crate::views::icons::button(
                        ui,
                        Icon::Close,
                        t.im_pane,
                        22.0,
                        false,
                        true,
                        pal.muted,
                        pal.primary,
                    );
                    if resp.on_hover_text(t.im_pane).clicked() {
                        app.im_hidden.insert(key);
                    }
                });
            });
        });
}

/// The bar of a conversation: participants, recording, video, call, present, more.
fn conv_bar(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey, c: &Conversation) {
    let t = app.t;
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
            // As in the Windows client: outlined round buttons at the sides (participants,
            // more), the filled blue ones for video, call and present in the middle.
            ui.horizontal(|ui| {
                let x0 = ui.cursor().min.x;
                let full = ui.available_width();
                let open = app.participants_open.contains(&key);
                if icons::round(
                    ui,
                    Icon::People,
                    t.participants,
                    40.0,
                    pal.primary,
                    open,
                    true,
                )
                .clicked()
                {
                    if open {
                        app.participants_open.remove(&key);
                    } else {
                        app.participants_open.insert(key);
                    }
                }
                // Someone records the meeting ([MS-CONFBAS] client-recording in the roster).
                let recorders: Vec<String> = c
                    .participants
                    .iter()
                    .filter(|p| p.recording)
                    .map(|p| p.name.clone())
                    .collect();
                if !recorders.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("● {}", t.being_recorded)).color(pal.error))
                        .on_hover_text(recorders.join(", "));
                }
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
                // The three in the middle of the bar.
                let used = ui.cursor().min.x - x0;
                let middle = 3.0 * 40.0 + 2.0 * ui.spacing().item_spacing.x;
                ui.add_space(((full - middle) / 2.0 - used).max(8.0));
                let reason = |g: &Gate| match g {
                    Gate::Unavailable(r) => Some(t.reason(*r).to_string()),
                    _ => None,
                };
                let v = icons::round(
                    ui,
                    Icon::Camera,
                    vlabel,
                    40.0,
                    pal.primary,
                    true,
                    video_gate.usable(),
                );
                let v = match reason(&video_gate) {
                    Some(why) => v.on_disabled_hover_text(why),
                    None => v,
                };
                if v.clicked() {
                    match &peer {
                        Some(p) => app.call(p, true),
                        None => app.emit(UiIntent::JoinMeetingVideo { key }),
                    }
                }
                let a = icons::round(
                    ui,
                    Icon::Phone,
                    label,
                    40.0,
                    pal.primary,
                    true,
                    call_gate.usable(),
                );
                let a = match reason(&call_gate) {
                    Some(why) => a.on_disabled_hover_text(why),
                    None => a,
                };
                if a.clicked() {
                    match &peer {
                        Some(p) => app.call(p, false),
                        None if app.prefs.meeting_audio_ask => app.meeting_audio = Some(key),
                        None => app.emit(UiIntent::JoinMeetingAudio { key }),
                    }
                }
                let present =
                    icons::round(ui, Icon::Screen, t.present, 40.0, pal.primary, true, true);
                egui::Popup::menu(&present).show(|ui| present_menu(app, ui, key));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let more = icons::round(
                        ui,
                        Icon::More,
                        t.more_options,
                        40.0,
                        pal.primary,
                        false,
                        true,
                    );
                    egui::Popup::menu(&more).show(|ui| more_menu(app, ui, key));
                });
            });
        });
}

/// Someone shares in the meeting: say so and offer to view it.
fn sharing_banner(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey, c: &Conversation) {
    let t = app.t;
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
}

/// Participants and meeting content at the right.
fn side_panel(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey, c: &Conversation) {
    let t = app.t;
    if app.participants_open.contains(&key) || c.content.is_some() {
        egui::Panel::left(("conv-side", key.0))
            .resizable(true)
            .default_size(260.0)
            .frame(
                Frame::new()
                    .fill(pal.surface)
                    .inner_margin(Margin::same(10)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if app.participants_open.contains(&key) {
                        // As in the Windows client: "PARTICIPANTS" with a close button.
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(t.participants.to_uppercase())
                                    .family(theme::bold())
                                    .size(theme::text::SMALL)
                                    .color(pal.muted),
                            );
                            ui.add_space((ui.available_width() - 22.0).max(0.0));
                            if icons::button(
                                ui,
                                Icon::Close,
                                t.close,
                                20.0,
                                false,
                                true,
                                pal.muted,
                                pal.primary,
                            )
                            .on_hover_text(t.close)
                            .clicked()
                            {
                                app.participants_open.remove(&key);
                            }
                        });
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
                        // "Invite more people", as at the bottom of the Windows client's list.
                        let gate = app.state.capabilities.group_chat.clone();
                        if ui
                            .add_enabled(gate.usable(), egui::Button::new(t.invite_more))
                            .clicked()
                        {
                            app.new_chat = Some(crate::app::NewChat {
                                text: String::new(),
                                purpose: crate::app::AddressFor::Invite(key),
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

fn theme_muted(ui: &Ui) -> egui::Color32 {
    ui.visuals().weak_text_color()
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
    let meeting = app
        .state
        .meetings
        .iter()
        .find(|m| m.focus.eq_ignore_ascii_case(&focus))
        .cloned();
    let link = meeting.as_ref().and_then(|m| m.join_url.clone());
    // The conference ID for callers by phone: the meeting's, else the focus URI's `id:` part
    // (the form of the [MS-CONFBAS] examples, inference).
    let conference_id = meeting
        .map(|m| m.conference_id)
        .filter(|id| !id.is_empty())
        .or_else(|| {
            focus
                .rsplit_once("focus:id:")
                .map(|(_, id)| id.chars().take(32).collect::<String>())
        });
    let dial_in = app.state.dial_in.clone();
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
        // As the Windows client: how to join by phone.
        if let Some(d) = &dial_in {
            ui.add_space(4.0);
            ui.label(RichText::new(t.dial_in).strong());
            if !d.region.is_empty() {
                ui.label(RichText::new(&d.region).color(theme_muted(ui)));
            }
            for (number, toll_free) in &d.numbers {
                if *toll_free {
                    ui.label(format!("{number}  ({})", t.toll_free));
                } else {
                    ui.label(number);
                }
            }
            if let Some(id) = &conference_id {
                ui.label(format!("{}: {id}", t.conference_id));
            }
            if let Some(page) = &d.page {
                ui.horizontal(|ui| {
                    ui.label(t.more_dial_in_numbers);
                    if ui.small_button(t.copy_link).clicked() {
                        ui.ctx().copy_text(page.clone());
                    }
                });
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.copy_all_info).clicked() {
                let mut info = format!(
                    "{}: {shown_link}\n{}: {organizer}",
                    t.meeting_link_label, t.organizer
                );
                if let Some(d) = &dial_in {
                    info.push_str(&format!("\n{}:", t.dial_in));
                    for (number, _) in &d.numbers {
                        info.push_str(&format!("\n  {number}"));
                    }
                    if let Some(id) = &conference_id {
                        info.push_str(&format!("\n{}: {id}", t.conference_id));
                    }
                    if let Some(page) = &d.page {
                        info.push_str(&format!("\n{}: {page}", t.more_dial_in_numbers));
                    }
                }
                ui.ctx().copy_text(info);
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

/// "This ends the meeting for everyone. Continue?", as the Windows client asks.
pub fn end_meeting_dialog(app: &mut CommsUi, ctx: &egui::Context) {
    let t = app.t;
    let Some(key) = app.end_meeting else {
        return;
    };
    let mut done = false;
    egui::Modal::new(egui::Id::unique("end-meeting")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.label(RichText::new(t.end_meeting).size(theme::text::HEADING));
        ui.add_space(6.0);
        ui.label(t.end_meeting_question);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(t.ok).clicked() {
                app.emit(UiIntent::ConferenceCommand {
                    key,
                    command: ConfCommand::End,
                });
                done = true;
            }
            if ui.button(t.cancel).clicked() {
                done = true;
            }
        });
    });
    if done {
        app.end_meeting = None;
    }
}

/// Present: sharing the own screen or a window with the other person (1:1; the desktop's
/// dialog picks it); slides, whiteboards and polls are not built.
fn present_menu(app: &mut CommsUi, ui: &mut Ui, key: ConvKey) {
    let t = app.t;
    let not_built = Gate::Unavailable(crate::model::Reason::NotImplemented);
    let peer = app.state.conversation(key).and_then(|c| c.peer.clone());
    let can_share = peer.is_some() && app.state.capabilities.video_call.usable();
    for (label, windows) in [(t.share_desktop, false), (t.share_window, true)] {
        if ui
            .add_enabled(can_share, egui::Button::new(label))
            .clicked()
        {
            app.emit(UiIntent::ShareScreen { key, windows });
            ui.close();
        }
    }
    for label in [t.present_slides, t.whiteboard_new, t.poll_new] {
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
    // Recording the meeting (its audio, on this computer; the others see that we record).
    if c.is_group() && c.focus.is_some() && c.state == ConvState::Open {
        let me = app.state.me.uri.clone();
        let recording = c.participants.iter().any(|p| p.uri == me && p.recording);
        let label = if recording {
            t.stop_recording
        } else {
            t.start_recording
        };
        if ui.button(label).on_hover_text(t.recording_tip).clicked() {
            app.emit(UiIntent::ConferenceCommand {
                key,
                command: ConfCommand::Record(!recording),
            });
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
        if ui.button(t.meeting_options_dots).clicked() {
            let v = c.meeting.unwrap_or_default();
            app.meeting_options = Some((key, v));
            ui.close();
        }
        if ui.button(t.end_meeting_dots).clicked() {
            app.end_meeting = Some(key);
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
    // As the Windows client's "more" menu: recordings, IM text size, font, help.
    ui.separator();
    if ui.button(t.manage_recordings).clicked() {
        app.emit(UiIntent::OpenRecordings);
        ui.close();
    }
    ui.menu_button(t.im_text_size, |ui| {
        for (label, size) in [
            (t.size_small, 13.0),
            (t.size_medium, 15.0),
            (t.size_large, 18.0),
        ] {
            if ui
                .radio((app.prefs.chat_text_size - size).abs() < 0.5, label)
                .clicked()
            {
                app.prefs.chat_text_size = size;
                app.emit(UiIntent::PrefsChanged);
                ui.close();
            }
        }
    });
    if ui.button(t.change_font_dots).clicked() {
        app.open_options(crate::views::options::IM);
        ui.close();
    }
    if ui.button(t.menu_help).clicked() {
        app.open_window(crate::app::Win::About);
        ui.close();
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
    // As in the Windows client: the participants at the left, the state (with the call's
    // length) and the add-people button at the right; the name is on the picture.
    let key = app.state.conversation_of_call(call);
    let count = key
        .and_then(|k| app.state.conversation(k))
        .map_or(2, |c| c.participants.len().max(2));
    ui.horizontal(|ui| {
        let open = key.is_some_and(|k| app.participants_open.contains(&k));
        if icons::round(
            ui,
            Icon::People,
            t.participants,
            34.0,
            light,
            open,
            key.is_some(),
        )
        .clicked()
        {
            if let Some(k) = key {
                if open {
                    app.participants_open.remove(&k);
                } else {
                    app.participants_open.insert(k);
                }
            }
        }
        ui.label(
            RichText::new(t.participants_count(count))
                .size(theme::text::HEADING - 2.0)
                .color(light),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let gate = app.state.capabilities.group_chat.clone();
            let add = icons::round(
                ui,
                Icon::AddPerson,
                t.invite,
                34.0,
                light,
                false,
                gate.usable() && key.is_some(),
            );
            if add.clicked() {
                if let Some(k) = key {
                    app.new_chat = Some(crate::app::NewChat {
                        text: String::new(),
                        purpose: crate::app::AddressFor::Invite(k),
                    });
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(&state).size(theme::text::SMALL).color(light));
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
            child.label(RichText::new(&name).size(theme::text::HEADING).color(light));
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
    let light = egui::Color32::from_gray(225);
    let key = app.state.conversation_of_call(call);
    // As in the Windows client: the chat button outlined at the left; video, microphone,
    // present (blue) and hang up (red) in the middle; call controls and more outlined at the
    // right.
    let x0 = ui.cursor().min.x;
    let full = ui.available_width();
    ui.add_space(4.0);
    if let Some(key) = key {
        let hidden = app.im_hidden.contains(&key);
        if icons::round(ui, Icon::Chat, t.im_pane, 40.0, light, !hidden, true).clicked() {
            if hidden {
                app.im_hidden.remove(&key);
            } else {
                app.im_hidden.insert(key);
            }
        }
    }
    let screen = call.kind.is_screen();
    let middle_count = if screen { 1.0 } else { 4.0 };
    let middle = middle_count * 40.0 + (middle_count - 1.0) * ui.spacing().item_spacing.x;
    let used = ui.cursor().min.x - x0;
    ui.add_space(((full - middle) / 2.0 - used).max(8.0));
    if !screen {
        // Video is part of the call from its start (a video call); adding it later is not
        // built yet.
        let video = call.kind == CallKind::Video;
        icons::round(
            ui,
            Icon::Camera,
            if video { t.video_call } else { t.add_video },
            40.0,
            blue,
            true,
            false,
        )
        .on_disabled_hover_text(t.reason(crate::model::Reason::NotImplemented));
        if round_button(
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
        let present = icons::round(ui, Icon::Screen, t.present, 40.0, blue, true, key.is_some());
        if let Some(key) = key {
            egui::Popup::menu(&present).show(|ui| present_menu(app, ui, key));
        }
    }
    let hang = match call.kind {
        CallKind::Screen => t.stop_viewing,
        CallKind::Sharing => t.stop_presenting,
        _ => t.hang_up,
    };
    if round_button(ui, Icon::HangUp, hang, red, true, false).clicked() {
        app.emit(UiIntent::HangUp { id: call.id });
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(4.0);
        if let Some(key) = key {
            let more = icons::round(ui, Icon::More, t.more_options, 40.0, light, false, true);
            egui::Popup::menu(&more).show(|ui| more_menu(app, ui, key));
        }
        if !screen {
            let b = icons::round(
                ui,
                Icon::Phone,
                t.call_controls,
                40.0,
                light,
                false,
                connected,
            );
            egui::Popup::menu(&b).show(|ui| call_menu(app, ui, call, connected, caps));
        }
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
        if !call.kind.is_screen() {
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
        let label = match call.kind {
            CallKind::Screen => t.stop_viewing,
            CallKind::Sharing => t.stop_presenting,
            _ => t.hang_up,
        };
        if widgets::danger(ui, label, pal).clicked() {
            app.emit(UiIntent::HangUp { id: call.id });
        }
    }
}

/// Incoming calls, invitations and new messages, stacked in one small window.
fn notifications(app: &mut CommsUi, ctx: &egui::Context, pal: &Palette) {
    if app.system_notifications {
        return;
    }
    let t = app.t;
    let quiet = app.quiet_for_messages();
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
            !quiet
                && c.unread > 0
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

/// The contact card as in the Windows client: picture, name and state, buttons for IM, call,
/// video and e-mail, then the sections Contact, Organisation and Membership; *More* holds the
/// rest of the contact menu. It sits beside the main window (where the desktop lets windows be
/// placed).
fn card(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let Some(uri) = app.card.clone() else {
        return;
    };
    let c = app.state.contacts.get(&uri).cloned();
    let caps = app.state.capabilities.clone();
    let idle = app.state.active_call().is_none();
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
            let address = uri.trim_start_matches("sip:").to_string();
            ui.horizontal(|ui| {
                app.avatar(
                    ui,
                    Some(uri.as_str()),
                    &name,
                    64.0,
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
                        if !c.note.is_empty() {
                            ui.label(RichText::new(&c.note).italics().color(pal.muted));
                        }
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if bar_button(ui, t, pal, Icon::Chat, t.send_im, &caps.chat, false).clicked() {
                    app.chat_with(&uri);
                }
                let call_gate = if idle {
                    caps.audio_call.clone()
                } else {
                    Gate::Unavailable(crate::model::Reason::NotImplemented)
                };
                if bar_button(ui, t, pal, Icon::Phone, t.call, &call_gate, false).clicked() {
                    app.call(&uri, false);
                }
                let video_gate = if idle {
                    caps.video_call.clone()
                } else {
                    Gate::Unavailable(crate::model::Reason::NotImplemented)
                };
                if bar_button(ui, t, pal, Icon::Camera, t.video_call, &video_gate, false).clicked()
                {
                    app.call(&uri, true);
                }
                if ui.button(t.send_email).clicked() {
                    app.emit(UiIntent::SendEmail {
                        address: address.clone(),
                    });
                }
                let more = bar_button(
                    ui,
                    t,
                    pal,
                    Icon::More,
                    t.more_options,
                    &Gate::Available,
                    false,
                );
                egui::Popup::menu(&more)
                    .show(|ui| crate::views::main_window::contact_menu(app, ui, &uri));
            });
            ui.separator();
            let section = |ui: &mut Ui, title: &str, rows: &[(&str, &str)]| {
                if rows.iter().all(|(_, v)| v.is_empty()) {
                    return;
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(title)
                        .size(theme::text::SMALL)
                        .family(theme::bold())
                        .color(pal.muted),
                );
                for (label, v) in rows.iter().filter(|(_, v)| !v.is_empty()) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*label).color(pal.muted));
                        ui.add(egui::Label::new(RichText::new(*v).color(pal.ink)).wrap());
                    });
                }
            };
            let (title, department, company, phone, group) =
                c.as_ref().map_or_else(Default::default, |c| {
                    (
                        c.title.clone(),
                        c.department.clone(),
                        c.company.clone(),
                        c.phone.clone(),
                        c.group.clone(),
                    )
                });
            let state = c
                .as_ref()
                .map(|c| t.presence(c.presence).to_string())
                .unwrap_or_default();
            section(
                ui,
                t.card_contact,
                &[
                    (t.card_state, &state),
                    (t.card_im, &address),
                    (t.phone_label, &phone),
                ],
            );
            section(
                ui,
                t.card_organisation,
                &[
                    (t.function, &title),
                    (t.department, &department),
                    (t.company, &company),
                ],
            );
            section(ui, t.card_membership, &[(t.card_group, &group)]);
        });
}
