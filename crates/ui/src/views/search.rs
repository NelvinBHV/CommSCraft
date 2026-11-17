//! The global search: only sources that exist – the contact list, the open conversations by
//! title, and the server's address book search when it is available. Message texts are not
//! searched (no archive, no index).

use crate::app::{CommsUi, UiIntent};
use crate::model::ConvKey;
use crate::theme::{self, Palette};
use crate::widgets;
use egui::{RichText, Ui};

/// Shortest text that is sent to the address book search.
const MIN_DIRECTORY: usize = 3;
/// How long the search text rests before the server is asked (ms).
const SETTLE_MS: u64 = 600;

pub fn results(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let needle = app.search.trim().to_lowercase();
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(t.search_results)
                .size(theme::text::PANEL_TITLE)
                .family(crate::theme::bold())
                .color(pal.ink),
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(t.search_scope)
                .size(theme::text::SMALL)
                .color(pal.muted),
        );
    });
    ui.add_space(6.0);
    let contacts: Vec<String> = app
        .state
        .contacts
        .values()
        .filter(|c| {
            c.name.to_lowercase().contains(&needle) || c.uri.to_lowercase().contains(&needle)
        })
        .take(50)
        .map(|c| c.uri.clone())
        .collect();
    let convs: Vec<(ConvKey, String)> = app
        .state
        .conversations
        .iter()
        .filter(|c| c.title.to_lowercase().contains(&needle))
        .take(30)
        .map(|c| (c.key, c.title.clone()))
        .collect();
    let gate = app.state.capabilities.directory_search.clone();
    // The server is asked once the text has rested for a moment (not on every key).
    if gate.usable() && needle.chars().count() >= MIN_DIRECTORY && app.last_search != needle {
        let time = ui.input(|i| i.time);
        let settle = std::time::Duration::from_millis(SETTLE_MS);
        if app.search_pending.0 != needle {
            app.search_pending = (needle.clone(), time);
            ui.ctx().request_repaint_after(settle);
        } else if (time - app.search_pending.1) * 1000.0 >= SETTLE_MS as f64 {
            app.last_search = needle.clone();
            app.emit(UiIntent::SearchDirectory {
                text: app.search.trim().to_string(),
            });
        } else {
            ui.ctx().request_repaint_after(settle);
        }
    }
    let directory: Vec<String> = app
        .state
        .search_results
        .iter()
        .filter(|c| !app.state.contacts.contains_key(&c.uri))
        .map(|c| c.uri.clone())
        .collect();
    egui::ScrollArea::vertical()
        .id_salt("search")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                widgets::section(ui, t.source_contact_list, pal);
            });
            if contacts.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.nothing_found).color(pal.muted));
                });
            }
            for uri in contacts {
                crate::views::main_window::contact_row_of(app, ui, pal, &uri);
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                widgets::section(ui, t.conversations, pal);
            });
            if convs.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.nothing_found).color(pal.muted));
                });
            }
            for (k, title) in convs {
                if ui
                    .selectable_label(app.selected_conv == Some(k), &title)
                    .clicked()
                {
                    app.select_conversation(k);
                    app.search.clear();
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                widgets::section(ui, t.source_directory, pal);
            });
            match &gate {
                crate::model::Gate::Unavailable(r) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new(t.reason(*r)).color(pal.muted));
                    });
                }
                _ if needle.chars().count() < MIN_DIRECTORY => {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new(t.search_more_letters).color(pal.muted));
                    });
                }
                _ => {
                    if directory.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new(t.nothing_found).color(pal.muted));
                        });
                    }
                    for uri in directory {
                        crate::views::main_window::contact_row_of(app, ui, pal, &uri);
                    }
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t.no_message_search)
                        .size(theme::text::SMALL)
                        .color(pal.muted),
                );
            });
        });
}
