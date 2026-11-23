//! The options window: a list of areas on the left, the area's settings on the right, OK and
//! Cancel. Settings that work change the local preferences; what the client cannot do yet is
//! shown disabled with the reason, never as if it had an effect.

use crate::app::{CommsUi, Prefs, UiIntent};
use crate::model::{Gate, Reason};
use crate::text::{Lang, Texts};
use crate::theme::{self, Palette};
use egui::{Frame, Margin, RichText, Ui};

/// The IM page (font, emoticons, spelling).
pub const IM: usize = 6;
pub const AUDIO: usize = 8;
pub const VIDEO: usize = 9;

fn areas(t: &Texts) -> [&'static str; 13] {
    [
        t.opt_general,
        t.opt_personal,
        t.opt_contacts,
        t.opt_status,
        t.opt_picture,
        t.opt_alerts,
        t.opt_im,
        t.opt_sounds,
        t.opt_audio,
        t.opt_video,
        t.opt_files,
        t.opt_recording,
        t.opt_meetings,
    ]
}

pub fn show(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    // The settings are edited on a copy; OK keeps them, Cancel drops them.
    if app.prefs_draft.is_none() {
        app.prefs_draft = Some(app.prefs.clone());
    }
    egui::Panel::bottom("opt-buttons")
        .frame(
            Frame::new()
                .fill(pal.surface_subtle)
                .inner_margin(Margin::same(10)),
        )
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // As in the Windows client: OK · Cancel · Help.
                if ui.button(t.menu_help).clicked() {
                    app.open_window(crate::app::Win::About);
                }
                if ui.button(t.cancel).clicked() {
                    app.prefs_draft = None;
                    app.close_window(crate::app::Win::Options);
                }
                if ui.button(t.ok).clicked() {
                    if let Some(p) = app.prefs_draft.take() {
                        let lang_changed = p.lang != app.prefs.lang;
                        app.prefs = p;
                        if lang_changed {
                            app.t = app.prefs.lang.texts();
                        }
                        app.emit(UiIntent::PrefsChanged);
                    }
                    app.close_window(crate::app::Win::Options);
                }
            });
        });
    egui::Panel::left("opt-areas")
        .exact_size(190.0)
        .frame(Frame::new().fill(pal.surface).inner_margin(Margin::same(8)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("opt-area-list")
                .show(ui, |ui| {
                    // The chosen area filled in the accent colour with white text, as in the
                    // Windows client's list.
                    for (i, a) in areas(t).iter().enumerate() {
                        let on = app.options_page == i;
                        let text = egui::RichText::new(*a).color(if on {
                            egui::Color32::WHITE
                        } else {
                            pal.ink
                        });
                        let b = egui::Button::new(text)
                            .fill(if on {
                                pal.primary
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(2)
                            .min_size(egui::vec2(ui.available_width(), 24.0));
                        let r = ui.add(b);
                        r.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, *a));
                        if r.clicked() {
                            app.options_page = i;
                        }
                    }
                });
        });
    egui::CentralPanel::default()
        .frame(
            Frame::new()
                .fill(pal.surface)
                .inner_margin(Margin::same(14)),
        )
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let page = app.options_page;
                let me = app.state.me.uri.clone();
                let caps = app.state.capabilities.clone();
                let devices = app.audio_devices.clone();
                let Some(mut p) = app.prefs_draft.take() else {
                    return;
                };
                page_ui(t, ui, pal, page, &mut p, &me, &caps, &devices);
                app.prefs_draft = Some(p);
                if page == 8 {
                    audio_test(app, ui, pal);
                }
                if page == 1 {
                    group(ui, pal, t.opt_keyring, |ui| {
                        if app.stored_password_for.is_some() {
                            if ui.button(t.opt_forget_password).clicked() {
                                app.emit(UiIntent::ForgetPassword);
                            }
                        } else {
                            note(ui, pal, t.opt_no_password_stored);
                        }
                    });
                }
            });
        });
}

fn group(ui: &mut Ui, pal: &Palette, title: &str, add: impl FnOnce(&mut Ui)) {
    Frame::new()
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(4)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).family(theme::bold()).color(pal.ink));
            ui.add_space(4.0);
            add(ui);
        });
    ui.add_space(10.0);
}

/// A setting the client does not have yet: disabled, with the reason.
fn missing(ui: &mut Ui, t: &Texts, label: &str, why: Reason) {
    let mut off = false;
    ui.add_enabled(false, egui::Checkbox::new(&mut off, label))
        .on_disabled_hover_text(t.reason(why));
}

fn note(ui: &mut Ui, pal: &Palette, text: &str) {
    ui.label(
        RichText::new(text)
            .size(theme::text::SMALL)
            .color(pal.muted),
    );
}

/// Test tone and microphone level for the chosen devices, before the settings are kept.
fn audio_test(app: &mut CommsUi, ui: &mut Ui, pal: &Palette) {
    let t = app.t;
    let (speaker, mic) = app
        .prefs_draft
        .as_ref()
        .map(|p| (p.audio_device.clone(), p.mic_device.clone()))
        .unwrap_or_default();
    group(ui, pal, t.opt_audio_levels, |ui| {
        ui.horizontal(|ui| {
            if ui.button(t.test_speaker).clicked() {
                app.emit(UiIntent::TestSpeaker { device: speaker });
            }
            let testing = app.state.mic_level.is_some();
            if ui
                .add_enabled(!testing, egui::Button::new(t.test_microphone))
                .clicked()
            {
                app.emit(UiIntent::TestMicrophone { device: mic });
            }
        });
        if let Some(level) = app.state.mic_level {
            ui.horizontal(|ui| {
                ui.label(t.mic_level);
                ui.add(
                    egui::ProgressBar::new(f32::from(level) / 100.0)
                        .desired_width(220.0)
                        .fill(pal.primary),
                );
            });
        }
        note(ui, pal, t.audio_test_body);
    });
}

#[allow(clippy::too_many_arguments)]
fn page_ui(
    t: &Texts,
    ui: &mut Ui,
    pal: &Palette,
    page: usize,
    p: &mut Prefs,
    me: &str,
    caps: &crate::model::Capabilities,
    devices: &[(String, String)],
) {
    let ni = Reason::NotImplemented;
    match page {
        0 => {
            group(ui, pal, t.opt_conversation_window, |ui| {
                ui.checkbox(&mut p.reopen_conversations, t.opt_reopen_conversations);
            });
            group(ui, pal, t.opt_language, |ui| {
                ui.radio_value(&mut p.lang, Lang::De, "Deutsch");
                ui.radio_value(&mut p.lang, Lang::En, "English");
            });
            group(ui, pal, t.opt_app_window, |ui| {
                ui.checkbox(&mut p.always_on_top, t.always_on_top);
                ui.checkbox(&mut p.high_contrast, t.high_contrast);
                ui.horizontal(|ui| {
                    ui.label(t.scale);
                    ui.add(egui::Slider::new(&mut p.scale, 1.0..=2.0).step_by(0.25))
                        .widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Slider, true, t.scale)
                        });
                });
            });
            group(ui, pal, t.opt_logging, |ui| {
                note(ui, pal, t.opt_logging_body)
            });
        }
        1 => {
            group(ui, pal, t.opt_account, |ui| {
                ui.label(format!(
                    "{}: {}",
                    t.sign_in_address,
                    me.trim_start_matches("sip:")
                ));
                ui.checkbox(&mut p.autostart, t.opt_autostart);
                ui.checkbox(&mut p.close_to_tray, t.opt_close_to_tray);
            });
            group(ui, pal, t.opt_exchange, |ui| {
                note(ui, pal, t.opt_exchange_body)
            });
            group(ui, pal, t.opt_location, |ui| {
                missing(ui, t, t.opt_share_location, ni)
            });
            group(ui, pal, t.opt_pictures, |ui| {
                ui.checkbox(&mut p.show_pictures, t.opt_show_pictures);
            });
        }
        2 => {
            group(ui, pal, t.opt_contacts_display, |ui| {
                ui.radio_value(&mut p.compact_contacts, false, t.opt_two_lines);
                ui.radio_value(&mut p.compact_contacts, true, t.opt_one_line);
            });
            group(ui, pal, t.opt_contacts_order, |ui| {
                ui.radio_value(&mut p.sort_by_availability, false, t.opt_by_name);
                ui.radio_value(&mut p.sort_by_availability, true, t.opt_by_availability);
            });
            group(ui, pal, t.opt_contacts_show, |ui| {
                ui.checkbox(&mut p.away_group, t.opt_away_group);
                ui.checkbox(&mut p.favorites_group, t.opt_favorites_group);
            });
        }
        3 => group(ui, pal, t.opt_status, |ui| {
            // The status automation: inactive, then away (0 minutes: that step is off).
            ui.horizontal(|ui| {
                ui.label(t.opt_inactive_after);
                ui.add(
                    egui::DragValue::new(&mut p.inactive_minutes)
                        .range(0..=360)
                        .suffix(" min"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(t.opt_away_after);
                ui.add(
                    egui::DragValue::new(&mut p.away_minutes)
                        .range(0..=360)
                        .suffix(" min"),
                );
            });
            missing(ui, t, t.opt_dnd_sharing, ni);
            note(ui, pal, t.opt_status_body);
        }),
        4 => group(ui, pal, t.opt_picture, |ui| {
            missing(ui, t, t.opt_show_my_picture, ni)
        }),
        5 => {
            group(ui, pal, t.opt_alerts_general, |ui| {
                ui.checkbox(&mut p.message_alerts, t.opt_message_alerts);
            });
            group(ui, pal, t.opt_alerts_dnd, |ui| {
                ui.checkbox(&mut p.quiet_when_dnd, t.opt_quiet_dnd);
            });
            group(ui, pal, t.opt_alerts_strangers, |ui| {
                missing(ui, t, t.opt_block_strangers, ni)
            });
        }
        6 => group(ui, pal, t.opt_im, |ui| {
            ui.horizontal(|ui| {
                ui.label(t.opt_text_size);
                ui.add(egui::Slider::new(&mut p.chat_text_size, 12.0..=20.0).step_by(1.0))
                    .widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Slider, true, t.opt_text_size)
                    });
            });
            ui.label(RichText::new(t.opt_im_font).family(crate::theme::bold()))
                .on_hover_text(t.opt_im_font_tip);
            ui.horizontal(|ui| {
                ui.checkbox(&mut p.im_bold, t.opt_im_bold);
                ui.checkbox(&mut p.im_italic, t.opt_im_italic);
                ui.checkbox(&mut p.im_underline, t.opt_im_underline);
            });
            ui.horizontal(|ui| {
                ui.label(t.opt_im_color);
                let mut c = p.im_color.unwrap_or([0, 0, 0]);
                if ui.color_edit_button_srgb(&mut c).changed() {
                    p.im_color = (c != [0, 0, 0]).then_some(c);
                }
                if p.im_color.is_some() && ui.button(t.opt_im_color_default).clicked() {
                    p.im_color = None;
                }
            });
            // The server's policy, when it sets one, decides; this is the user's choice.
            ui.checkbox(&mut p.save_history, t.opt_save_history);
            ui.checkbox(&mut p.tabbed_conversations, t.opt_tabbed);
            ui.checkbox(&mut p.spell_check, t.opt_spelling);
            missing(ui, t, t.opt_emoticons, ni);
            missing(ui, t, t.opt_tabs, ni);
        }),
        7 => group(ui, pal, t.opt_sounds, |ui| {
            ui.checkbox(&mut p.sounds, t.opt_play_sounds);
            ui.add_enabled_ui(p.sounds, |ui| {
                ui.checkbox(&mut p.quiet_sounds_busy, t.opt_quiet_busy);
            });
            note(ui, pal, t.opt_sounds_body);
        }),
        8 => group(ui, pal, t.opt_audio, |ui| {
            note(ui, pal, t.audio_devices_body);
            // Speaker and microphone, each the system's default (PipeWire) or a device ALSA
            // names, as on the Windows client's audio device page.
            let pick = |ui: &mut Ui, label: &str, value: &mut String, kind: &str, id: &str| {
                let current = if value.is_empty() {
                    t.audio_default.to_string()
                } else {
                    devices
                        .iter()
                        .find(|(n, _)| n == value)
                        .map_or(value.clone(), |(_, l)| l.clone())
                };
                ui.horizontal(|ui| {
                    let name = ui.label(label);
                    egui::ComboBox::from_id_salt(id)
                        .selected_text(current)
                        .width(320.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(value, String::new(), t.audio_default);
                            for (name, l) in devices.iter().filter(|(_, l)| l.contains(kind)) {
                                ui.selectable_value(value, name.clone(), l);
                            }
                        })
                        .response
                        .labelled_by(name.id);
                });
            };
            pick(
                ui,
                t.opt_speaker,
                &mut p.audio_device,
                "playback",
                "speaker",
            );
            pick(
                ui,
                t.opt_microphone,
                &mut p.mic_device,
                "capture",
                "microphone",
            );
        }),
        9 => group(ui, pal, t.opt_video, |ui| {
            ui.label(t.opt_camera);
            ui.add(
                egui::TextEdit::singleline(&mut p.camera)
                    .hint_text("/dev/video0")
                    .char_limit(200),
            );
            note(ui, pal, t.opt_camera_body);
            if !caps.video_call.usable() {
                note(
                    ui,
                    pal,
                    t.reason(match &caps.video_call {
                        Gate::Unavailable(r) => *r,
                        _ => Reason::NotConnected,
                    }),
                );
            }
        }),
        10 => group(ui, pal, t.opt_files, |ui| {
            missing(ui, t, t.opt_save_files, ni)
        }),
        11 => group(ui, pal, t.opt_recording, |ui| {
            missing(ui, t, t.opt_record, ni)
        }),
        _ => group(ui, pal, t.opt_meetings, |ui| {
            ui.checkbox(&mut p.meeting_participants, t.opt_meeting_participants);
            missing(ui, t, t.opt_meeting_audio, ni);
        }),
    }
}
