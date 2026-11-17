//! The content of a meeting in the details of its group conversation (function 44): what is
//! presented and by whom, whiteboards, slides, notes, files and polls with voting.

use crate::app::{CommsUi, UiIntent};
use crate::content::{ContentItem, ContentKind, ContentState, MeetingContent};
use crate::model::{ConvKey, Gate};
use crate::theme::{self, Palette};
use crate::widgets;
use egui::{RichText, Ui};

fn kind_word(app: &CommsUi, k: ContentKind) -> &'static str {
    let t = app.t;
    match k {
        ContentKind::Whiteboard => t.content_whiteboard,
        ContentKind::Slides => t.content_slides,
        ContentKind::Poll => t.content_poll,
        ContentKind::Notes => t.content_notes,
        ContentKind::File => t.content_file,
        ContentKind::Other => t.content_other,
    }
}

/// The block in the details panel.
pub fn block(app: &mut CommsUi, ui: &mut Ui, pal: &Palette, key: ConvKey) {
    let t = app.t;
    widgets::rule(ui, pal);
    ui.add_space(10.0);
    ui.label(
        RichText::new(t.content_title)
            .family(crate::theme::bold())
            .color(pal.ink),
    );
    let content = app.state.conversation(key).and_then(|c| c.content.clone());
    let gate = app.state.capabilities.meeting_content.clone();
    match content {
        None => {
            if let Some(r) = widgets::gated(ui, t.content_show, &gate, t, pal, |ui, l, en| {
                ui.add_enabled(en, egui::Button::new(l))
            }) {
                if r.on_hover_text(t.content_show_tip).clicked() {
                    app.emit(UiIntent::JoinContent { key });
                }
            }
        }
        Some(m) => shown(app, ui, pal, key, &m, gate.usable()),
    }
    ui.add_space(14.0);
}

fn shown(
    app: &mut CommsUi,
    ui: &mut Ui,
    pal: &Palette,
    key: ConvKey,
    m: &MeetingContent,
    usable: bool,
) {
    let t = app.t;
    let small = |s: String| {
        RichText::new(s)
            .size(theme::text::SMALL + 0.5)
            .color(pal.muted)
    };
    match &m.state {
        ContentState::Connecting => {
            ui.label(small(t.content_connecting.to_string()));
            return;
        }
        ContentState::Ended(why) => {
            ui.label(small(format!("{}: {why}", t.content_ended)));
            if usable && ui.button(t.content_show).clicked() {
                app.emit(UiIntent::JoinContent { key });
            }
            return;
        }
        ContentState::Connected => {}
    }
    if !m.presenter.is_empty() {
        ui.label(small(format!("{}: {}", t.content_presenter, m.presenter)));
    }
    if m.items.is_empty() {
        ui.label(small(t.content_none.to_string()));
    }
    for item in m.items.iter().take(50) {
        ui.add_space(6.0);
        entry(app, ui, pal, key, item, m.presented == item.id, usable);
    }
    ui.add_space(6.0);
    ui.label(small(t.content_not_drawn.to_string()));
    if ui.button(t.content_leave).clicked() {
        app.emit(UiIntent::LeaveContent { key });
    }
}

fn entry(
    app: &mut CommsUi,
    ui: &mut Ui,
    pal: &Palette,
    key: ConvKey,
    item: &ContentItem,
    presented: bool,
    usable: bool,
) {
    let t = app.t;
    let kind = kind_word(app, item.kind);
    let title = if item.title.is_empty() {
        kind.to_string()
    } else {
        format!("{kind}: {}", item.title)
    };
    ui.horizontal_wrapped(|ui| {
        widgets::ellipsis(ui, RichText::new(title).color(pal.ink));
        if presented {
            ui.label(
                RichText::new(t.content_presented)
                    .size(theme::text::SMALL)
                    .color(pal.success_text),
            );
        }
    });
    let mut facts = Vec::new();
    if let Some(n) = item.slides {
        facts.push(format!("{n} {}", t.content_slide_count));
    }
    if !item.slide_at.is_empty() {
        facts.push(item.slide_at.clone());
    }
    if item.annotations > 0 || item.kind == ContentKind::Whiteboard {
        facts.push(format!(
            "{} {}",
            item.annotations,
            if item.annotations == 1 {
                t.content_annotation
            } else {
                t.content_annotations
            }
        ));
    }
    if let Some((name, size)) = &item.file {
        facts.push(format!("{name} ({})", crate::format::bytes(*size)));
    }
    if !facts.is_empty() {
        ui.label(
            RichText::new(facts.join(" · "))
                .size(theme::text::SMALL)
                .color(pal.muted),
        );
    }
    let Some(p) = &item.poll else {
        return;
    };
    if !p.question.is_empty() {
        ui.label(RichText::new(&p.question).color(pal.ink));
    }
    ui.label(
        RichText::new(if p.open {
            t.content_poll_open
        } else {
            t.content_poll_closed
        })
        .size(theme::text::SMALL)
        .color(pal.muted),
    );
    let gate = if usable {
        Gate::Available
    } else {
        app.state.capabilities.meeting_content.clone()
    };
    for (i, choice) in p.choices.iter().enumerate() {
        if choice.is_empty() {
            continue;
        }
        let index = i as i64;
        let count = p.results.get(i).copied().unwrap_or(0);
        let mine = p.my_vote == Some(index);
        let label = if mine {
            format!("{choice} · {count} · {}", t.content_your_vote)
        } else {
            format!("{choice} · {count}")
        };
        if let Some(r) = widgets::gated(ui, &label, &gate, t, pal, |ui, l, en| {
            ui.add_enabled(en && p.open, egui::Button::new(l).selected(mine))
        }) {
            if r.clicked() {
                app.emit(UiIntent::Vote {
                    key,
                    content: item.id,
                    choice: index,
                });
            }
        }
    }
}
