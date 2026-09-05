//! egui UI rendering for LookEveryting.

use std::time::Instant;

use cap_core::{all_supported_filter, ThemePreference};
use cap_i18n::Locale;
use cap_ui::colors::{Palette, Semantic};
use cap_ui::layout::LayoutMode;
use cap_ui::spacing::{component, space};
use cap_ui::widgets::{ghost_button, icon_button, paint_floating_panel, panel_frame, titlebar_frame};
use cap_viewer::{draw_mesh_viewport_ex, MeshDrawOpts, ViewportBg};
use egui::{pos2, vec2, Align2, Color32, Frame, RichText, Sense, Ui, Vec2};

use crate::app::{ErrorAction, LoadedMedia, LookApp};
use crate::image_viewport::interact_image_viewport;
use crate::thumbnails::{draw_thumbnail_strip, ensure_thumbnails};

pub fn draw(app: &mut LookApp, ctx: &egui::Context) {
    app.maybe_hide_toolbar(ctx);
    app.ensure_texture(ctx);
    app.tick_video(ctx);
    ensure_thumbnails(app, ctx);
    update_drag_hover(app, ctx);

    let screen = ctx.screen_rect();
    let layout = app.layout_mode(screen.width());
    let show_strip = !app.folder_files.is_empty() && !app.is_fullscreen();
    let fullscreen = app.is_fullscreen();

    if !fullscreen {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(component::TITLEBAR_HEIGHT)
            .frame(titlebar_frame(Palette::SURFACE))
            .show(ctx, |ui| title_bar(app, ui));
    }

    if layout != LayoutMode::Compact && !fullscreen {
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

    if app.info_open && layout == LayoutMode::Spacious && !fullscreen {
        egui::SidePanel::right("info")
            .resizable(false)
            .exact_width(component::INFO_PANEL_WIDTH)
            .frame(panel_frame(Palette::SURFACE))
            .show(ctx, |ui| info_panel(app, ui));
    }

    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(Semantic::BG_VIEWPORT))
        .show(ctx, |ui| {
            if app.drag_hover {
                ui.painter().rect_stroke(
                    ui.max_rect().shrink(4.0),
                    8.0,
                    egui::Stroke::new(2.0_f32, Palette::ACCENT),
                    egui::StrokeKind::Outside,
                );
            }
            viewport(app, ui);
            draw_status_bar(app, ui);
            if app.toolbar_visible {
                draw_floating_toolbar_overlay(app, ui);
            }
        });

    if show_strip {
        egui::TopBottomPanel::bottom("thumbnails")
            .exact_height(component::THUMBNAIL_STRIP_HEIGHT)
            .frame(panel_frame(Palette::SURFACE))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(app.i18n.t("browse-thumbnails"))
                        .size(11.0)
                        .color(Semantic::FG_MUTED),
                );
                let strip_h = component::THUMBNAIL_STRIP_SIZE + 4.0;
                ui.set_height(strip_h);
                // Vertical mouse wheel → horizontal scroll while hovering the strip.
                let strip_id = ui.id().with("thumb_scroll_area");
                let strip_rect = ui.available_rect_before_wrap();
                if ui.rect_contains_pointer(strip_rect) {
                    ui.ctx().input_mut(|i| {
                        let dy = i.smooth_scroll_delta.y + i.raw_scroll_delta.y;
                        if dy.abs() > f32::EPSILON {
                            i.smooth_scroll_delta.x -= dy;
                            i.smooth_scroll_delta.y = 0.0;
                            i.raw_scroll_delta.x -= i.raw_scroll_delta.y;
                            i.raw_scroll_delta.y = 0.0;
                        }
                    });
                }
                egui::ScrollArea::horizontal()
                    .id_salt(strip_id)
                    .auto_shrink([false; 2])
                    .max_height(strip_h)
                    .drag_to_scroll(true)
                    .show(ui, |ui| {
                        ui.set_height(strip_h);
                        draw_thumbnail_strip(app, ui);
                    });
            });
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

    if app.shortcuts_open {
        egui::Window::new(app.i18n.t("shortcuts-title"))
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| shortcuts_panel(app, ui));
    }

    if app.rename_open {
        egui::Window::new(app.i18n.t("rename-title"))
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 40.0))
            .show(ctx, |ui| rename_panel(app, ui));
    }

    if let Some(err) = app.error.clone() {
        egui::Window::new(app.i18n.t("toast-open-failed"))
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, -80.0))
            .show(ctx, |ui| {
                ui.label(&err);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let actions = app.error_actions.clone();
                    for action in actions {
                        match action {
                            ErrorAction::OpenExternally => {
                                if ui.button(app.i18n.t("error-open-system")).clicked() {
                                    app.open_current_externally();
                                    app.clear_error();
                                }
                            }
                            ErrorAction::Dismiss => {
                                if ui.button(app.i18n.t("common-close")).clicked() {
                                    app.clear_error();
                                }
                            }
                        }
                    }
                    if app.error_actions.is_empty()
                        && ui.button(app.i18n.t("common-close")).clicked()
                    {
                        app.clear_error();
                    }
                });
            });
    }

    handle_shortcuts(app, ctx);
    handle_dropped_files(app, ctx);
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
                if ui.selectable_label(selected, text).clicked() && !selected {
                    app.navigate_to_index(idx);
                }
            }
        });
    }
}

fn viewport(app: &mut LookApp, ui: &mut Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, Semantic::BG_VIEWPORT);

    let image_draw = match &app.media {
        Some(LoadedMedia::Image {
            texture: Some(tex),
            width,
            height,
            native_width,
            native_height,
            full_res_loading,
            ..
        }) => Some((
            *width,
            *height,
            *native_width,
            *native_height,
            *full_res_loading,
            tex.id(),
        )),
        _ => None,
    };

    if let Some((width, height, native_width, native_height, loading_full, tex_id)) = image_draw {
        paint_checkerboard(ui.painter(), rect);
        let img_size = vec2(width as f32, height as f32);
        let capped = width != native_width || height != native_height;
        let _ = interact_image_viewport(app, ui, rect, img_size);
        let scale = app.display_scale(rect.size(), img_size);
        let size = img_size * scale;
        let center = rect.center() + app.pan;
        let img_rect = egui::Rect::from_center_size(center, size);
        ui.painter().image(
            tex_id,
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        let pct = (scale * 100.0).round();
        ui.painter().text(
            rect.left_top() + vec2(12.0, 8.0),
            Align2::LEFT_TOP,
            format!("{pct:.0}%"),
            egui::FontId::monospace(11.0),
            Semantic::FG_MUTED,
        );
        if loading_full {
            ui.painter().text(
                rect.left_top() + vec2(12.0, 26.0),
                Align2::LEFT_TOP,
                app.i18n.t("media-loading-full"),
                egui::FontId::proportional(11.0),
                Palette::ACCENT,
            );
        } else if capped {
            ui.painter().text(
                rect.left_top() + vec2(12.0, 26.0),
                Align2::LEFT_TOP,
                app.i18n.t("image-scaled-hint"),
                egui::FontId::proportional(11.0),
                Semantic::FG_MUTED,
            );
        }
        ui.painter().text(
            rect.left_bottom() + vec2(12.0, -12.0),
            Align2::LEFT_BOTTOM,
            "滚轮缩放 · 中键/放大后左键拖拽 · 双击适应 · 0/1/F/R · F11 全屏",
            egui::FontId::proportional(11.0),
            Semantic::FG_MUTED,
        );
        return;
    }

    match &mut app.media {
        None => empty_state(app, ui, rect),
        Some(LoadedMedia::Loading { .. }) => {
            draw_loading_or_held(app, ui, rect);
        }
        Some(LoadedMedia::Image { .. }) => {
            draw_loading_or_held(app, ui, rect);
        }
        Some(LoadedMedia::Video {
            info,
            path,
            texture,
            playing,
            ready,
            duration_secs,
            position_secs,
            position_fraction,
            subtitles,
        }) => {
            let subtitle = if app.settings.show_subtitles {
                subtitles
                    .active_at(*position_secs)
                    .map(str::to_string)
            } else {
                None
            };
            let args = (
                info.clone(),
                path.clone(),
                texture.clone(),
                *playing,
                *ready,
                *duration_secs,
                *position_secs,
                *position_fraction,
                subtitle,
            );
            draw_video_view(app, ui, rect, args);
            draw_play_flash(app, ui, rect);
        }
        Some(LoadedMedia::Model {
            info,
            path,
            wireframe,
            bg,
            mesh,
            camera,
        }) => {
            if let Some(mesh) = mesh.as_ref() {
                let vert_count = mesh.vertices.len();
                let tri_count = if mesh.indices.is_empty() {
                    mesh.vertices.len() / 3
                } else {
                    mesh.indices.len() / 3
                };
                let upload = if app.mesh_upload_pending {
                    app.mesh_upload_pending = false;
                    Some(std::sync::Arc::clone(mesh))
                } else {
                    None
                };
                draw_mesh_viewport_ex(
                    ui,
                    rect,
                    mesh,
                    camera,
                    *wireframe,
                    *bg,
                    MeshDrawOpts {
                        gpu_available: app.gpu_mesh,
                        mesh_to_upload: upload,
                        clear_gpu_mesh: false,
                    },
                );
                let hud = format!(
                    "{} · {}",
                    app.i18n
                        .t("model-hud-verts")
                        .replace("{count}", &vert_count.to_string()),
                    app.i18n
                        .t("model-hud-tris")
                        .replace("{count}", &tri_count.to_string()),
                );
                ui.painter().text(
                    rect.left_top() + vec2(12.0, 8.0),
                    Align2::LEFT_TOP,
                    hud,
                    egui::FontId::monospace(11.0),
                    Semantic::FG_MUTED,
                );
                let hint = app.i18n.t("model-hint");
                ui.painter().text(
                    rect.left_bottom() + vec2(12.0, -8.0),
                    Align2::LEFT_BOTTOM,
                    hint,
                    egui::FontId::proportional(11.0),
                    Semantic::FG_MUTED,
                );
            } else {
                let info = info.clone();
                let path = path.clone();
                draw_placeholder(
                    ui,
                    rect,
                    app.i18n.t("mode-model"),
                    &format!("{}\n{}\n{}", info.format, info.notes, path.display()),
                    Color32::from_rgb(0x22, 0xC5, 0x5E),
                );
            }
        }
    }
}

fn draw_video_view(
    app: &mut LookApp,
    ui: &mut Ui,
    rect: egui::Rect,
    (
        info,
        path,
        texture,
        playing,
        player_ready,
        duration_secs,
        position_secs,
        position_fraction,
        subtitle,
    ): (
        cap_video::VideoInfo,
        std::path::PathBuf,
        Option<egui::TextureHandle>,
        bool,
        bool,
        f32,
        f32,
        f32,
        Option<String>,
    ),
) {
    ui.painter().rect_filled(rect, 0.0, Color32::BLACK);

    let mut frame_bottom = rect.bottom();
    if let Some(tex) = texture.as_ref() {
        let avail = rect.size();
        let [tw, th] = tex.size();
        let img_size = vec2(tw as f32, th as f32);
        let scale = (avail.x / img_size.x).min(avail.y / img_size.y);
        let size = img_size * scale;
        let img_rect = egui::Rect::from_center_size(rect.center(), size);
        frame_bottom = img_rect.bottom();
        ui.painter().image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        let center = rect.center();
        let (label, color) = if !player_ready {
            (app.i18n.t("video-loading"), Semantic::FG_MUTED)
        } else {
            (
                app.error.as_deref().unwrap_or(app.i18n.t("video-failed")),
                Palette::DANGER,
            )
        };
        ui.painter().text(
            center,
            Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            color,
        );
    }

    if let Some(text) = subtitle.as_deref() {
        draw_subtitle_overlay(ui, rect, frame_bottom, text);
    }

    let controls_h = 72.0;
    let controls_y = rect.bottom() - controls_h;
    let bar_margin = 24.0;
    let bar_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + bar_margin, controls_y + 28.0),
        egui::pos2(rect.right() - bar_margin, controls_y + 40.0),
    );

    if duration_secs > 0.0 && ui.is_rect_visible(bar_rect) {
        ui.painter()
            .rect_filled(bar_rect, 4.0, Palette::SURFACE_RAISED);
        let fill_w = bar_rect.width() * position_fraction.clamp(0.0, 1.0);
        if fill_w > 0.0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(bar_rect.min, vec2(fill_w, bar_rect.height())),
                4.0,
                Palette::ACCENT,
            );
        }
        if let (Some(a), Some(b)) = (app.ab_a, app.ab_b) {
            if duration_secs > 0.0 && b > a {
                let ax = bar_rect.left() + bar_rect.width() * (a / duration_secs).clamp(0.0, 1.0);
                let bx = bar_rect.left() + bar_rect.width() * (b / duration_secs).clamp(0.0, 1.0);
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(ax, bar_rect.top() - 2.0),
                        egui::pos2(bx, bar_rect.bottom() + 2.0),
                    ),
                    2.0,
                    Color32::from_rgba_unmultiplied(80, 200, 120, 90),
                );
            }
        } else if let Some(a) = app.ab_a {
            if duration_secs > 0.0 {
                let ax = bar_rect.left() + bar_rect.width() * (a / duration_secs).clamp(0.0, 1.0);
                ui.painter().line_segment(
                    [egui::pos2(ax, bar_rect.top() - 4.0), egui::pos2(ax, bar_rect.bottom() + 4.0)],
                    egui::Stroke::new(2.0_f32, Color32::from_rgb(80, 200, 120)),
                );
            }
        }
        let seek_resp = ui.allocate_rect(bar_rect, Sense::click_and_drag());
        if seek_resp.clicked() || seek_resp.dragged() {
            if let Some(pos) = seek_resp.interact_pointer_pos() {
                let frac = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                app.seek_video(frac, ui.ctx());
                let preview_secs = duration_secs * frac;
                draw_seek_bubble(ui, pos, &format_time(preview_secs));
            }
        } else if seek_resp.hovered() {
            if let Some(pos) = seek_resp.hover_pos() {
                let frac = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                let preview_secs = duration_secs * frac;
                draw_seek_bubble(ui, pos, &format_time(preview_secs));
            }
        }

        let time_label = app
            .i18n
            .t("video-time")
            .replace("{current}", &format_time(position_secs))
            .replace("{total}", &format_time(duration_secs));
        ui.painter().text(
            bar_rect.left_bottom() + vec2(0.0, 14.0),
            Align2::LEFT_TOP,
            time_label,
            egui::FontId::monospace(11.0),
            Semantic::FG_MUTED,
        );
    }

    let play_label = if playing {
        app.i18n.t("video-pause").to_string()
    } else {
        app.i18n.t("video-play").to_string()
    };
    let btn_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, controls_y + 12.0),
        vec2(120.0, 32.0),
    );
    let resp = ui.allocate_rect(btn_rect, Sense::click());
    if resp.clicked() {
        app.toggle_video_playback();
    }
    if ui.is_rect_visible(btn_rect) {
        let bg = if resp.hovered() {
            Palette::SURFACE_RAISED
        } else {
            Palette::ACCENT_MUTED
        };
        ui.painter().rect_filled(btn_rect, 6.0, bg);
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            &play_label,
            egui::FontId::proportional(14.0),
            Semantic::FG_PRIMARY,
        );
    }

    ui.painter().text(
        rect.left_top() + vec2(12.0, 8.0),
        Align2::LEFT_TOP,
        format!("{} · {}", info.format, info.file_size_label()),
        egui::FontId::monospace(11.0),
        Semantic::FG_MUTED,
    );
    ui.painter().text(
        rect.left_bottom() + vec2(12.0, -8.0),
        Align2::LEFT_BOTTOM,
        path.display().to_string(),
        egui::FontId::monospace(11.0),
        Semantic::FG_MUTED,
    );
}

fn draw_subtitle_overlay(ui: &mut Ui, viewport: egui::Rect, frame_bottom: f32, text: &str) {
    let font = egui::FontId::proportional(18.0);
    let galley = ui.painter().layout(
        text.to_string(),
        font,
        Color32::WHITE,
        viewport.width() * 0.86,
    );
    let pad = vec2(12.0, 6.0);
    let size = galley.size() + pad * 2.0;
    let y = (frame_bottom - 28.0).min(viewport.bottom() - 88.0);
    let center = egui::pos2(viewport.center().x, y);
    let box_rect = egui::Rect::from_center_size(center, size);
    ui.painter()
        .rect_filled(box_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 160));
    ui.painter()
        .galley(box_rect.min + pad, galley, Color32::WHITE);
}

fn format_time(secs: f32) -> String {
    let total = secs.max(0.0) as u32;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn draw_seek_bubble(ui: &mut Ui, pointer: egui::Pos2, label: &str) {
    let font = egui::FontId::monospace(12.0);
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        Semantic::FG_PRIMARY,
    );
    let pad = vec2(8.0, 4.0);
    let size = galley.size() + pad * 2.0;
    let center = pointer + vec2(0.0, -22.0);
    let bubble = egui::Rect::from_center_size(center, size);
    ui.painter()
        .rect_filled(bubble, 4.0, Palette::SURFACE_OVERLAY);
    ui.painter().galley(bubble.min + pad, galley, Semantic::FG_PRIMARY);
}

fn format_system_time(time: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let Ok(dur) = time.duration_since(UNIX_EPOCH) else {
        return "—".into();
    };
    let secs = dur.as_secs() as i64;
    // Format as UTC YYYY-MM-DD HH:MM without chrono dependency.
    const DAY: i64 = 86_400;
    let days = secs.div_euclid(DAY);
    let tod = secs.rem_euclid(DAY) as u32;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {hour:02}:{min:02}")
}

/// Howard Hinnant's civil_from_days (proleptic Gregorian).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

fn draw_status_bar(app: &LookApp, ui: &mut Ui) {
    let rect = ui.max_rect();
    let bar_h = 22.0;
    let bar_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - bar_h),
        egui::pos2(rect.right(), rect.bottom()),
    );
    if !ui.is_rect_visible(bar_rect) {
        return;
    }
    ui.painter()
        .rect_filled(bar_rect, 0.0, Color32::from_rgba_unmultiplied(0x11, 0x11, 0x13, 210));

    let mut parts = Vec::new();
    if !app.counter_label().is_empty() {
        parts.push(app.counter_label());
    }
    match &app.media {
        Some(LoadedMedia::Image {
            native_width,
            native_height,
            width,
            height,
            ..
        }) => {
            let label = app
                .i18n
                .t("status-resolution")
                .replace("{width}", &native_width.to_string())
                .replace("{height}", &native_height.to_string());
            parts.push(label);
            if width != native_width || height != native_height {
                parts.push(format!("({width}×{height})"));
            }
        }
        Some(LoadedMedia::Video { info, .. }) => {
            if info.width > 0 && info.height > 0 {
                parts.push(format!("{}×{}", info.width, info.height));
            }
            if info.duration_secs > 0.0 {
                parts.push(format_time(info.duration_secs));
            }
        }
        _ => {}
    }
    if parts.is_empty() {
        return;
    }
    ui.painter().text(
        bar_rect.left_center() + vec2(12.0, 0.0),
        Align2::LEFT_CENTER,
        parts.join(" · "),
        egui::FontId::monospace(11.0),
        Semantic::FG_MUTED,
    );
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
}

fn handle_dropped_files(app: &mut LookApp, ctx: &egui::Context) {
    let paths: Vec<std::path::PathBuf> = ctx.input(|i| {
        i.raw.dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if let Some(path) = paths.into_iter().next() {
        app.open_path(path);
    }
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

fn draw_floating_toolbar_overlay(app: &mut LookApp, ui: &mut Ui) {
    let parent = ui.max_rect();
    let width = parent.width() - component::FLOATING_TOOLBAR_MARGIN * 2.0;
    let bar_size = vec2(width.max(200.0), component::TOOLBAR_HEIGHT);
    let bar_rect = egui::Rect::from_min_size(
        pos2(
            parent.left() + component::FLOATING_TOOLBAR_MARGIN,
            parent.bottom() - component::TOOLBAR_HEIGHT - component::FLOATING_TOOLBAR_MARGIN,
        ),
        bar_size,
    );
    paint_floating_panel(ui, bar_rect);
    ui.allocate_ui_at_rect(bar_rect, |ui| floating_toolbar(app, ui, parent.size()));
}

fn floating_toolbar(app: &mut LookApp, ui: &mut Ui, viewport_size: Vec2) {
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
            Some(LoadedMedia::Image { width, height, .. }) => {
                let img_size = vec2(*width as f32, *height as f32);
                if icon_button(ui, "Fit", app.i18n.t("toolbar-fit")).clicked() {
                    app.fit_image();
                    app.touch();
                }
                if icon_button(ui, "1:1", app.i18n.t("toolbar-actual-size")).clicked() {
                    app.actual_size_image();
                    app.touch();
                }
                if icon_button(ui, "Win", app.i18n.t("toolbar-window-fit")).clicked() {
                    app.toggle_window_fit(ui.ctx());
                }
                if icon_button(ui, "−", app.i18n.t("toolbar-zoom-out")).clicked() {
                    app.zoom_image(viewport_size, img_size, 1.0 / 1.15);
                    app.touch();
                }
                if icon_button(ui, "+", app.i18n.t("toolbar-zoom-in")).clicked() {
                    app.zoom_image(viewport_size, img_size, 1.15);
                    app.touch();
                }
            }
                Some(LoadedMedia::Video { .. }) => {
                    let label = if app.video_is_playing() {
                        app.i18n.t("video-pause")
                    } else {
                        app.i18n.t("video-play")
                    };
                    if icon_button(ui, "▶", label).clicked() {
                        app.toggle_video_playback();
                        app.touch();
                    }
                    if icon_button(ui, "⏮", app.i18n.t("video-frame-prev")).clicked() {
                        app.step_video_frame(false, ui.ctx());
                        app.touch();
                    }
                    if icon_button(ui, "⏭", app.i18n.t("video-frame-next")).clicked() {
                        app.step_video_frame(true, ui.ctx());
                        app.touch();
                    }
                    let mute_label = if app.muted {
                        app.i18n.t("unmute")
                    } else {
                        app.i18n.t("mute")
                    };
                    if icon_button(ui, if app.muted { "🔇" } else { "🔊" }, mute_label).clicked()
                    {
                        app.toggle_mute();
                    }
                    let vol_before = app.volume;
                    ui.add(
                        egui::Slider::new(&mut app.volume, 0.0..=1.0)
                            .show_value(false)
                            .text(app.i18n.t("volume")),
                    );
                    if (app.volume - vol_before).abs() > f32::EPSILON {
                        app.push_volume();
                    }
                    let rate_label = format!("{:.2}x", app.playback_rate);
                    if ui
                        .add(egui::Button::new(rate_label).min_size(vec2(48.0, 28.0)))
                        .on_hover_text(app.i18n.t("video-rate-cycle"))
                        .clicked()
                    {
                        app.cycle_playback_rate();
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
                if icon_button(ui, "◎", "Background").clicked() {
                    if let Some(LoadedMedia::Model { bg, .. }) = &mut app.media {
                        *bg = match *bg {
                            ViewportBg::Solid => ViewportBg::Gradient,
                            ViewportBg::Gradient => ViewportBg::Solid,
                        };
                    }
                    app.touch();
                }
                if icon_button(ui, "↗", app.i18n.t("common-open")).clicked() {
                    app.open_model_externally();
                    app.touch();
                }
            }
            None | Some(LoadedMedia::Loading { .. }) => {}
        }
        if icon_button(ui, "ℹ", app.i18n.t("toolbar-info")).clicked() {
            app.info_open = !app.info_open;
            app.touch();
        }
        let slide_label = if app.slideshow_active {
            app.i18n.t("slideshow-stop")
        } else {
            app.i18n.t("slideshow-start")
        };
        if icon_button(ui, "▶▶", slide_label).clicked() {
            app.toggle_slideshow();
        }
        if icon_button(ui, "?", app.i18n.t("shortcuts-title")).clicked() {
            app.shortcuts_open = !app.shortcuts_open;
            app.touch();
        }
        if icon_button(ui, "Aa", app.i18n.t("rename-title")).clicked() {
            app.rename_open = !app.rename_open;
            app.touch();
        }
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
        Some(LoadedMedia::Image {
            width,
            height,
            native_width,
            native_height,
            ..
        }) => {
            if let Some(meta) = &app.image_meta {
                ui.label(format!(
                    "{} × {} ({:.2} MP)",
                    meta.width,
                    meta.height,
                    meta.megapixels()
                ));
                ui.label(format!("{} · {}", meta.format, meta.file_size_label()));
                ui.label(format!("Modified: {}", format_system_time(meta.modified)));
                if let Some(make) = &meta.camera_make {
                    ui.label(format!("Make: {make}"));
                }
                if let Some(model) = &meta.camera_model {
                    ui.label(format!("Model: {model}"));
                }
                if let Some(dt) = &meta.datetime {
                    ui.label(format!("Taken: {dt}"));
                }
            } else {
                ui.label(format!(
                    "{} × {} ({:.2} MP)",
                    native_width,
                    native_height,
                    (*native_width as f64 * *native_height as f64 / 1_000_000.0)
                ));
            }
            if width != native_width || height != native_height {
                ui.label(format!("Display buffer: {width} × {height}"));
            }
        }
        Some(LoadedMedia::Loading { .. }) => {
            ui.label(app.i18n.t("media-loading"));
        }
        Some(LoadedMedia::Video { info, duration_secs, .. }) => {
            ui.label(format!("{} · {}", info.format, info.file_size_label()));
            let dur = (*duration_secs).max(info.duration_secs);
            if dur > 0.0 {
                ui.label(format!("Duration: {}", format_time(dur)));
            }
            if info.width > 0 {
                ui.label(format!("{}×{}", info.width, info.height));
            }
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
    ui.label(app.i18n.t("settings-theme"));
    let theme_label = match app.settings.theme {
        ThemePreference::Dark => app.i18n.t("settings-theme-dark"),
        ThemePreference::Light => app.i18n.t("settings-theme-light"),
        ThemePreference::System => app.i18n.t("settings-theme-system"),
    };
    egui::ComboBox::from_id_salt("theme")
        .selected_text(theme_label)
        .show_ui(ui, |ui| {
            let options = [
                (ThemePreference::Dark, "settings-theme-dark"),
                (ThemePreference::Light, "settings-theme-light"),
                (ThemePreference::System, "settings-theme-system"),
            ];
            for (pref, key) in options {
                if ui
                    .selectable_label(app.settings.theme == pref, app.i18n.t(key))
                    .clicked()
                {
                    app.settings.theme = pref;
                    let _ = cap_core::save_settings(&app.settings);
                }
            }
        });

    ui.add_space(8.0);
    ui.checkbox(
        &mut app.settings.toolbar_auto_hide,
        app.i18n.t("settings-toolbar-auto-hide"),
    );
    ui.checkbox(
        &mut app.settings.show_subtitles,
        app.i18n.t("settings-show-subtitles"),
    );
    ui.checkbox(
        &mut app.settings.prefer_hw_decode,
        app.i18n.t("settings-prefer-hw-decode"),
    );
    ui.label(
        RichText::new(app.i18n.t("settings-prefer-hw-decode-hint"))
            .size(11.0)
            .color(Semantic::FG_MUTED),
    );

    ui.add_space(12.0);
    ui.separator();
    ui.label(RichText::new(app.i18n.t("settings-associations")).strong());
    ui.label(
        RichText::new(app.i18n.t("settings-associations-hint"))
            .size(11.0)
            .color(Semantic::FG_MUTED),
    );
    ui.checkbox(
        &mut app.settings.file_associations.images,
        app.i18n.t("settings-assoc-images"),
    );
    ui.checkbox(
        &mut app.settings.file_associations.videos,
        app.i18n.t("settings-assoc-videos"),
    );
    ui.checkbox(
        &mut app.settings.file_associations.models,
        app.i18n.t("settings-assoc-models"),
    );
    if ui.button(app.i18n.t("settings-assoc-apply")).clicked() {
        app.apply_file_associations();
    }
    if let Some(msg) = &app.association_message {
        ui.label(RichText::new(msg).size(11.0).color(Semantic::FG_SECONDARY));
    }

    ui.add_space(8.0);
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
    let viewport_size = ctx.screen_rect().size();
    let img_size = app.media.as_ref().and_then(|m| m.image_size()).unwrap_or(Vec2::ZERO);
    let is_image = img_size != Vec2::ZERO;
    let is_video = matches!(&app.media, Some(LoadedMedia::Video { .. }));
    let is_model = matches!(&app.media, Some(LoadedMedia::Model { .. }));

    ctx.input(|i| {
        if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
            open_file_dialog(app);
        }
        // Video: Left/Right = ±5s seek; Up/Down navigate files. Non-video: arrows navigate.
        if i.key_pressed(egui::Key::ArrowLeft) {
            if is_video && !i.modifiers.shift {
                app.seek_video_by(-5.0, ctx);
            } else if !is_video {
                app.navigate(-1);
            }
        }
        if i.key_pressed(egui::Key::ArrowRight) {
            if is_video && !i.modifiers.shift {
                app.seek_video_by(5.0, ctx);
            } else if !is_video {
                app.navigate(1);
            }
        }
        if i.key_pressed(egui::Key::ArrowUp) {
            app.navigate(-1);
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            app.navigate(1);
        }
        if i.key_pressed(egui::Key::PageUp) {
            app.navigate(-1);
        }
        if i.key_pressed(egui::Key::PageDown) {
            app.navigate(1);
        }
        if i.key_pressed(egui::Key::Home) && !app.folder_files.is_empty() {
            app.navigate_to_index(0);
        }
        if i.key_pressed(egui::Key::End) && !app.folder_files.is_empty() {
            app.navigate_to_index(app.folder_files.len() - 1);
        }
        if is_video && i.key_pressed(egui::Key::T) {
            app.cycle_audio_track();
        }
        if is_video && i.key_pressed(egui::Key::A) && !i.modifiers.shift {
            app.mark_ab_a();
        }
        if is_video && i.key_pressed(egui::Key::B) && !i.modifiers.shift {
            app.mark_ab_b();
        }
        if is_video && i.key_pressed(egui::Key::A) && i.modifiers.shift {
            app.clear_ab_loop();
        }
        if is_video && i.key_pressed(egui::Key::OpenBracket) {
            app.playback_rate = (app.playback_rate - 0.25).max(0.5);
            app.video_engine.set_rate(app.playback_rate);
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::CloseBracket) {
            app.playback_rate = (app.playback_rate + 0.25).min(2.0);
            app.video_engine.set_rate(app.playback_rate);
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::V) {
            app.settings.show_subtitles = !app.settings.show_subtitles;
            let _ = cap_core::save_settings(&app.settings);
        }
        if is_video && i.key_pressed(egui::Key::Space) {
            app.toggle_video_playback();
        }
        if !is_video && i.key_pressed(egui::Key::Space) {
            app.toggle_slideshow();
        }
        if is_video && i.key_pressed(egui::Key::M) {
            app.toggle_mute();
        }
        if i.key_pressed(egui::Key::Questionmark)
            || (i.modifiers.shift && i.key_pressed(egui::Key::Slash))
        {
            app.shortcuts_open = !app.shortcuts_open;
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::Comma) {
            app.step_video_frame(false, ctx);
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::Period) {
            app.step_video_frame(true, ctx);
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.shift {
            app.step_video_frame(false, ctx);
            app.touch();
        }
        if is_video && i.key_pressed(egui::Key::ArrowRight) && i.modifiers.shift {
            app.step_video_frame(true, ctx);
            app.touch();
        }
        if i.key_pressed(egui::Key::I) {
            app.info_open = !app.info_open;
            app.touch();
        }
        if i.key_pressed(egui::Key::F11) {
            app.toggle_fullscreen(ctx);
        }
        if i.key_pressed(egui::Key::Escape) && app.is_fullscreen() {
            app.toggle_fullscreen(ctx);
        } else if i.key_pressed(egui::Key::Escape) {
            if app.shortcuts_open {
                app.shortcuts_open = false;
            } else if app.settings_open {
                app.settings_open = false;
            } else if app.info_open {
                app.info_open = false;
            } else if app.slideshow_active {
                app.slideshow_active = false;
            }
        }
        if is_image && (i.key_pressed(egui::Key::Num0) || i.key_pressed(egui::Key::F)) {
            app.fit_image();
            app.touch();
        }
        if is_image && i.key_pressed(egui::Key::Num1) {
            app.actual_size_image();
            app.touch();
        }
        if is_image && i.key_pressed(egui::Key::W) && !i.modifiers.ctrl {
            app.toggle_window_fit(ctx);
        }
        if i.key_pressed(egui::Key::F2) {
            app.rename_open = !app.rename_open;
            app.touch();
        }
        if is_image && i.key_pressed(egui::Key::R) && !i.modifiers.ctrl {
            app.reset_image_view();
            app.touch();
        }
        if is_image && i.key_pressed(egui::Key::R) && i.modifiers.ctrl {
            app.rotate_image_cw();
        }
        if is_image && i.key_pressed(egui::Key::H) && !i.modifiers.shift {
            app.flip_image(true);
        }
        if is_image && i.key_pressed(egui::Key::H) && i.modifiers.shift {
            app.flip_image(false);
        }
        if is_model && i.key_pressed(egui::Key::R) {
            app.reset_model_camera();
            app.touch();
        }
        if is_image && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            app.zoom_image(viewport_size, img_size, 1.15);
            app.touch();
        }
        if is_image && i.key_pressed(egui::Key::Minus) {
            app.zoom_image(viewport_size, img_size, 1.0 / 1.15);
            app.touch();
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
            app.settings_open = !app.settings_open;
        }
    });
}

fn update_drag_hover(app: &mut LookApp, ctx: &egui::Context) {
    app.drag_hover = ctx.input(|i| !i.raw.hovered_files.is_empty());
}

fn rename_panel(app: &mut LookApp, ui: &mut Ui) {
    ui.label(
        RichText::new(app.i18n.t("rename-hint"))
            .size(12.0)
            .color(Semantic::FG_MUTED),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(app.i18n.t("rename-pattern"));
        ui.text_edit_singleline(&mut app.rename_pattern);
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button(app.i18n.t("rename-apply")).clicked() {
            app.apply_batch_rename();
        }
        if ui.button(app.i18n.t("common-close")).clicked() {
            app.rename_open = false;
        }
    });
    if let Some(msg) = &app.rename_message {
        ui.label(RichText::new(msg).size(11.0).color(Semantic::FG_SECONDARY));
    }
}

fn shortcuts_panel(app: &mut LookApp, ui: &mut Ui) {
    ui.label(RichText::new(app.i18n.t("help-hint")).color(Semantic::FG_MUTED).size(12.0));
    ui.separator();
    let rows = [
        ("← → ↑ ↓", "Navigate files (video ←→ = ±5s)"),
        ("Shift+← →", "Frame step (video)"),
        ("[ ]", "Playback speed"),
        ("A / B", "A-B loop markers"),
        ("Shift+A", "Clear A-B loop"),
        ("T", "Cycle audio track"),
        ("Space", "Slideshow / Play-Pause"),
        ("V", "Toggle subtitles (.srt)"),
        ("M", "Mute (video)"),
        ("F / 0", "Fit"),
        ("1", "Actual size 100%"),
        ("R", "Reset view / camera"),
        ("Ctrl+R", "Rotate image 90°"),
        ("H / Shift+H", "Flip H / V"),
        ("F11", "Fullscreen"),
        ("I", "Info panel"),
        ("? ", "This help"),
        ("Esc", "Close overlays / exit fullscreen"),
    ];
    for (key, desc) in rows {
        ui.horizontal(|ui| {
            ui.label(RichText::new(key).monospace().strong());
            ui.label(desc);
        });
    }
    ui.add_space(8.0);
    if ui.button(app.i18n.t("common-close")).clicked() {
        app.shortcuts_open = false;
    }
}

fn draw_loading_or_held(app: &LookApp, ui: &mut Ui, rect: egui::Rect) {
    if let Some(held) = &app.held_frame {
        let avail = rect.size();
        let scale = (avail.x / held.size.x)
            .min(avail.y / held.size.y)
            .max(0.01);
        let size = held.size * scale;
        let img_rect = egui::Rect::from_center_size(rect.center(), size);
        ui.painter().image(
            held.texture.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::from_white_alpha(200),
        );
        // Corner spinner only — never cover the held frame.
        draw_corner_spinner(app, ui, rect);
        return;
    }
    draw_loading_state(app, ui, rect);
}

fn draw_corner_spinner(app: &LookApp, ui: &mut Ui, rect: egui::Rect) {
    let t = app.load_started.elapsed().as_secs_f32();
    let angle = t * 4.0;
    let origin = rect.right_top() + vec2(-28.0, 28.0);
    for i in 0..8 {
        let a = angle + i as f32 * std::f32::consts::FRAC_PI_4;
        let alpha = (255.0 * (1.0 - i as f32 / 8.0)).max(40.0) as u8;
        let p = origin + vec2(a.cos(), a.sin()) * 8.0;
        ui.painter().circle_filled(
            p,
            2.0,
            Color32::from_rgba_unmultiplied(
                Palette::ACCENT.r(),
                Palette::ACCENT.g(),
                Palette::ACCENT.b(),
                alpha,
            ),
        );
    }
    ui.ctx().request_repaint();
}

fn draw_play_flash(app: &LookApp, ui: &mut Ui, rect: egui::Rect) {
    let Some(until) = app.play_flash_until else {
        return;
    };
    if Instant::now() >= until {
        return;
    }
    let label = if app.video_is_playing() { "▶" } else { "⏸" };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(48.0),
        Color32::from_white_alpha(180),
    );
    ui.ctx().request_repaint();
}

fn draw_loading_state(app: &LookApp, ui: &mut Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, Semantic::BG_VIEWPORT);
    let center = rect.center();
    let t = app.load_started.elapsed().as_secs_f32();
    let angle = t * 4.0;
    let r = 14.0;
    for i in 0..8 {
        let a = angle + i as f32 * std::f32::consts::FRAC_PI_4;
        let alpha = (255.0 * (1.0 - i as f32 / 8.0)).max(40.0) as u8;
        let p = center + vec2(a.cos(), a.sin()) * r;
        ui.painter().circle_filled(
            p,
            2.5,
            Color32::from_rgba_unmultiplied(
                Palette::ACCENT.r(),
                Palette::ACCENT.g(),
                Palette::ACCENT.b(),
                alpha,
            ),
        );
    }
    ui.painter().text(
        center + vec2(0.0, 28.0),
        Align2::CENTER_CENTER,
        app.i18n.t("media-loading"),
        egui::FontId::proportional(15.0),
        Semantic::FG_MUTED,
    );
    ui.ctx().request_repaint();
}

fn paint_checkerboard(painter: &egui::Painter, rect: egui::Rect) {
    // Fast two-tone fill — avoids hundreds of small rects per frame.
    let light = Color32::from_rgb(0x2A, 0x2D, 0x35);
    let dark = Color32::from_rgb(0x22, 0x25, 0x2C);
    painter.rect_filled(rect, 0.0, dark);
    let cell = 16.0;
    let cols = (rect.width() / cell).ceil() as i32;
    let rows = (rect.height() / cell).ceil() as i32;
    for row in (0..rows).step_by(2) {
        for col in (0..cols).step_by(2) {
            let min = rect.min + vec2(col as f32 * cell, row as f32 * cell);
            let size = vec2(
                cell.min(rect.right() - min.x),
                cell.min(rect.bottom() - min.y),
            );
            painter.rect_filled(egui::Rect::from_min_size(min, size), 0.0, light);
            let min2 = min + vec2(cell, cell);
            if min2.x < rect.right() && min2.y < rect.bottom() {
                let size2 = vec2(
                    cell.min(rect.right() - min2.x),
                    cell.min(rect.bottom() - min2.y),
                );
                painter.rect_filled(egui::Rect::from_min_size(min2, size2), 0.0, light);
            }
        }
    }
}
