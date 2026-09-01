//! egui UI rendering for LookEveryting.

use cap_core::all_supported_filter;
use cap_i18n::Locale;
use cap_ui::colors::{Palette, Semantic};
use cap_ui::layout::LayoutMode;
use cap_ui::spacing::{component, space};
use cap_ui::widgets::{ghost_button, icon_button, paint_floating_panel, panel_frame, titlebar_frame};
use egui::{pos2, vec2, Align2, Color32, Frame, RichText, Sense, Ui};

use crate::app::{LoadedMedia, LookApp};

pub fn draw(app: &mut LookApp, ctx: &egui::Context) {
    app.maybe_hide_toolbar();
    app.ensure_texture(ctx);

    let screen = ctx.screen_rect();
    let layout = app.layout_mode(screen.width());

    egui::TopBottomPanel::top("titlebar")
        .exact_height(component::TITLEBAR_HEIGHT)
        .frame(titlebar_frame(Palette::SURFACE))
        .show(ctx, |ui| title_bar(app, ui));

    if layout != LayoutMode::Compact {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(if layout == LayoutMode::Spacious {
                component::SIDEBAR_WIDTH
            } else {
                component::SIDEBAR_COLLAPSED
            })
            .frame(panel_frame(Palette::SURFACE))
            .show(ctx, |ui| sidebar(app, ui, layout));
    }

    if app.info_open && layout == LayoutMode::Spacious {
        egui::SidePanel::right("info")
            .resizable(false)
            .exact_width(component::INFO_PANEL_WIDTH)
            .frame(panel_frame(Palette::SURFACE))
            .show(ctx, |ui| info_panel(app, ui));
    }

    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(Semantic::BG_VIEWPORT))
        .show(ctx, |ui| viewport(app, ui));

    if app.toolbar_visible {
        egui::TopBottomPanel::bottom("toolbar")
            .exact_height(component::TOOLBAR_HEIGHT + component::FLOATING_TOOLBAR_MARGIN * 2.0)
            .frame(Frame::NONE)
            .show(ctx, |ui| floating_toolbar(app, ui));
    }

    if app.info_open && layout != LayoutMode::Spacious {
        egui::Window::new(app.i18n.t("panel-file-info"))
            .collapsible(false)
            .resizable(true)
            .default_width(component::DRAWER_WIDTH)
            .show(ctx, |ui| info_panel(app, ui));
    }

    if app.settings_open {
        egui::Window::new(app.i18n.t("settings-title"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| settings_panel(app, ui));
    }

    if let Some(err) = app.error.clone() {
        egui::Window::new(app.i18n.t("toast-open-failed"))
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, -80.0))
            .show(ctx, |ui| {
                ui.label(err);
                if ui.button(app.i18n.t("common-close")).clicked() {
                    app.error = None;
                }
            });
    }

    handle_shortcuts(app, ctx);
}

fn title_bar(app: &mut LookApp, ui: &mut Ui) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = space::S2;
        ui.label(RichText::new("◇").color(Palette::ACCENT).size(16.0));
        ui.label(
            RichText::new(app.i18n.t("app-title"))
                .color(Semantic::FG_SECONDARY)
                .size(13.0),
        );
        if !app.file_name().is_empty() {
            ui.separator();
            ui.label(
                RichText::new(app.file_name())
                    .color(Semantic::FG_PRIMARY)
                    .size(13.0),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = space::S2;
            if ghost_button(ui, "⚙")
                .on_hover_text(app.i18n.t("settings-title"))
                .clicked()
            {
                app.settings_open = !app.settings_open;
                app.touch();
            }
            if ghost_button(ui, app.i18n.t("common-open")).clicked() {
                open_file_dialog(app);
            }
        });
    });
}

fn sidebar(app: &mut LookApp, ui: &mut Ui, layout: LayoutMode) {
    ui.spacing_mut().item_spacing.y = space::S1;
    if ghost_button(ui, app.i18n.t("common-open")).clicked() {
        open_file_dialog(app);
    }
    if ghost_button(ui, app.i18n.t("toolbar-info")).clicked() {
        app.info_open = !app.info_open;
        app.touch();
    }
    ui.add_space(space::S2);
    ui.separator();
    ui.add_space(space::S2);
    if layout == LayoutMode::Spacious {
        ui.label(
            RichText::new(app.i18n.t("browse-grid"))
                .color(Semantic::FG_MUTED)
                .size(11.0),
        );
        egui::ScrollArea::vertical().show(ui, |ui| {
            let files = app.folder_files.clone();
            for (idx, path) in files.iter().enumerate() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?");
                let selected = idx == app.current_index;
                let text = if selected {
                    RichText::new(name).color(Palette::ACCENT)
                } else {
                    RichText::new(name).color(Semantic::FG_SECONDARY)
                };
                if ui.selectable_label(selected, text).clicked() {
                    app.open_path(path.clone());
                }
            }
        });
    }
}

fn viewport(app: &mut LookApp, ui: &mut Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, Semantic::BG_VIEWPORT);

    match &app.media {
        None => empty_state(app, ui, rect),
        Some(LoadedMedia::Image { texture, decoded }) => {
            if let Some(tex) = texture {
                let avail = rect.size();
                let img_size = vec2(decoded.width as f32, decoded.height as f32);
                let scale = if app.fit_mode {
                    (avail.x / img_size.x).min(avail.y / img_size.y)
                } else {
                    app.zoom
                };
                let size = img_size * scale;
                let center = rect.center() + app.pan;
                let img_rect = egui::Rect::from_center_size(center, size);
                ui.put(
                    img_rect,
                    egui::Image::new(tex).fit_to_exact_size(size).sense(Sense::drag()),
                );
                if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
                    app.touch();
                }
            }
        }
        Some(LoadedMedia::Video { info, path }) => {
            draw_placeholder(
                ui,
                rect,
                app.i18n.t("mode-video"),
                &format!(
                    "{} · {}\n{}",
                    info.format,
                    info.file_size_label(),
                    path.display()
                ),
                Palette::ACCENT,
            );
        }
        Some(LoadedMedia::Model { info, path, wireframe }) => {
            let mode = if *wireframe {
                app.i18n.t("model-wireframe")
            } else {
                app.i18n.t("model-solid")
            };
            draw_placeholder(
                ui,
                rect,
                app.i18n.t("mode-model"),
                &format!(
                    "{} · {} · {}\n{} triangles\n{}",
                    info.format,
                    mode,
                    path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                    info.triangle_count,
                    info.notes
                ),
                Color32::from_rgb(0x22, 0xC5, 0x5E),
            );
        }
    }
}

fn empty_state(app: &mut LookApp, ui: &mut Ui, rect: egui::Rect) {
    let inner = rect.shrink(24.0);
    ui.allocate_ui_at_rect(inner, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(inner.height() * 0.28);
            ui.label(RichText::new("🖼").size(48.0).color(Semantic::FG_MUTED));
            ui.add_space(12.0);
            ui.label(
                RichText::new(app.i18n.t("empty-title"))
                    .size(18.0)
                    .color(Semantic::FG_SECONDARY),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(app.i18n.t("empty-subtitle"))
                    .size(13.0)
                    .color(Semantic::FG_MUTED),
            );
            ui.add_space(16.0);
            if ghost_button(ui, app.i18n.t("empty-open-button")).clicked() {
                open_file_dialog(app);
            }
        });
    });

    ui.ctx().input(|i| {
        if !i.raw.dropped_files.is_empty() {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    app.open_path(path.clone());
                }
            }
        }
    });
}

fn draw_placeholder(ui: &mut Ui, rect: egui::Rect, title: &str, body: &str, accent: Color32) {
    let center = rect.center();
    ui.painter().text(
        center + vec2(0.0, -30.0),
        Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(22.0),
        accent,
    );
    ui.painter().text(
        center + vec2(0.0, 20.0),
        Align2::CENTER_CENTER,
        body,
        egui::FontId::proportional(13.0),
        Semantic::FG_SECONDARY,
    );
}

fn floating_toolbar(app: &mut LookApp, ui: &mut Ui) {
    let width = ui.available_width() - component::FLOATING_TOOLBAR_MARGIN * 2.0;
    let bar_size = vec2(width, component::TOOLBAR_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), bar_size.y + component::FLOATING_TOOLBAR_MARGIN),
        Sense::hover(),
    );
    let bar_rect = egui::Rect::from_min_size(
        pos2(
            rect.left() + component::FLOATING_TOOLBAR_MARGIN,
            rect.bottom() - component::TOOLBAR_HEIGHT - component::FLOATING_TOOLBAR_MARGIN,
        ),
        bar_size,
    );
    paint_floating_panel(ui, bar_rect);

    ui.allocate_ui_at_rect(bar_rect, |ui| {
        ui.horizontal_centered(|ui| {
            if icon_button(ui, "◀", app.i18n.t("toolbar-prev")).clicked() {
                app.navigate(-1);
            }
            if icon_button(ui, "▶", app.i18n.t("toolbar-next")).clicked() {
                app.navigate(1);
            }
            ui.label(
                RichText::new(app.counter_label())
                    .color(Semantic::FG_MUTED)
                    .size(12.0),
            );
            ui.separator();
            match &app.media {
                Some(LoadedMedia::Image { .. }) => {
                    if icon_button(ui, "Fit", app.i18n.t("toolbar-fit")).clicked() {
                        app.fit_mode = true;
                        app.touch();
                    }
                    if icon_button(ui, "1:1", app.i18n.t("toolbar-actual-size")).clicked() {
                        app.fit_mode = false;
                        app.zoom = 1.0;
                        app.touch();
                    }
                    if icon_button(ui, "−", app.i18n.t("toolbar-zoom-out")).clicked() {
                        app.fit_mode = false;
                        app.zoom = (app.zoom * 0.85).max(0.05);
                        app.touch();
                    }
                    if icon_button(ui, "+", app.i18n.t("toolbar-zoom-in")).clicked() {
                        app.fit_mode = false;
                        app.zoom = (app.zoom * 1.15).min(20.0);
                        app.touch();
                    }
                }
                Some(LoadedMedia::Video { .. }) => {
                    if icon_button(ui, "▶", app.i18n.t("video-play")).clicked() {
                        app.play_video_externally();
                        app.touch();
                    }
                }
                Some(LoadedMedia::Model { .. }) => {
                    if icon_button(ui, "3D", app.i18n.t("model-solid")).clicked() {
                        if let Some(LoadedMedia::Model { wireframe, .. }) = &mut app.media {
                            *wireframe = false;
                        }
                        app.touch();
                    }
                    if icon_button(ui, "▦", app.i18n.t("model-wireframe")).clicked() {
                        if let Some(LoadedMedia::Model { wireframe, .. }) = &mut app.media {
                            *wireframe = true;
                        }
                        app.touch();
                    }
                    if icon_button(ui, "↗", "Open").clicked() {
                        app.open_model_externally();
                        app.touch();
                    }
                }
                None => {}
            }
            if icon_button(ui, "ℹ", app.i18n.t("toolbar-info")).clicked() {
                app.info_open = !app.info_open;
                app.touch();
            }
        });
    });
}

fn info_panel(app: &mut LookApp, ui: &mut Ui) {
    ui.heading(app.i18n.t("panel-file-info"));
    ui.add_space(8.0);
    if let Some(path) = &app.current_path {
        ui.label(RichText::new(path.display().to_string()).monospace().size(12.0));
    }
    ui.separator();
    match &app.media {
        Some(LoadedMedia::Image { decoded, .. }) => {
            ui.label(format!("{} × {}", decoded.width, decoded.height));
            ui.label(format!("{:.2} MP", decoded.megapixels()));
        }
        Some(LoadedMedia::Video { info, .. }) => {
            ui.label(format!("{} · {}", info.format, info.file_size_label()));
            ui.label(&info.notes);
        }
        Some(LoadedMedia::Model { info, .. }) => {
            ui.heading(app.i18n.t("panel-model-info"));
            ui.label(format!("Meshes: {}", info.mesh_count));
            ui.label(format!("Materials: {}", info.material_count));
            ui.label(format!("Vertices: {}", info.vertex_count));
            ui.label(format!("Triangles: {}", info.triangle_count));
            ui.label(&info.notes);
        }
        None => {
            ui.label(app.i18n.t("empty-title"));
        }
    }
}

fn settings_panel(app: &mut LookApp, ui: &mut Ui) {
    ui.label(app.i18n.t("settings-language"));
    egui::ComboBox::from_id_salt("locale")
        .selected_text(app.i18n.locale().display_name())
        .show_ui(ui, |ui| {
            for locale in Locale::ALL {
                if ui
                    .selectable_label(app.i18n.locale() == *locale, locale.display_name())
                    .clicked()
                {
                    app.i18n.set_locale(*locale);
                    app.settings.locale = locale.id().to_string();
                    let _ = cap_core::save_settings(&app.settings);
                }
            }
        });

    ui.add_space(8.0);
    ui.checkbox(
        &mut app.settings.toolbar_auto_hide,
        app.i18n.t("settings-toolbar-auto-hide"),
    );
    if ui.button(app.i18n.t("common-close")).clicked() {
        app.settings_open = false;
        let _ = cap_core::save_settings(&app.settings);
    }
}

fn open_file_dialog(app: &mut LookApp) {
    let (label, exts) = all_supported_filter();
    let mut dialog = rfd::FileDialog::new().add_filter(label, &exts);
    if let Some(dir) = &app.settings.last_directory {
        dialog = dialog.set_directory(dir);
    }
    if let Some(path) = dialog.pick_file() {
        app.open_path(path);
    }
}

fn handle_shortcuts(app: &mut LookApp, ctx: &egui::Context) {
    ctx.input(|i| {
        if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
            open_file_dialog(app);
        }
        if i.key_pressed(egui::Key::ArrowLeft) {
            app.navigate(-1);
        }
        if i.key_pressed(egui::Key::ArrowRight) {
            app.navigate(1);
        }
        if i.key_pressed(egui::Key::I) {
            app.info_open = !app.info_open;
            app.touch();
        }
        if i.key_pressed(egui::Key::Num0) {
            app.fit_mode = true;
            app.touch();
        }
        if i.key_pressed(egui::Key::Num1) {
            app.fit_mode = false;
            app.zoom = 1.0;
            app.touch();
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
            app.settings_open = !app.settings_open;
        }
    });
}
