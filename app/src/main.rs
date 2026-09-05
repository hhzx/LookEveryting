mod app;
mod image_viewport;
mod loader;
mod thumbnails;
mod ui;
mod video_thread;

use std::path::PathBuf;

use cap_core::ThemePreference;
use cap_ui::Theme;
use eframe::egui;

use app::LookApp;

fn main() -> eframe::Result<()> {
    let startup_paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([480.0, 360.0])
            .with_title("LookEveryting"),
        ..Default::default()
    };
    eframe::run_native(
        "LookEveryting",
        options,
        Box::new(move |cc| {
            let mut app = LookEverytingApp::new(cc);
            for path in startup_paths {
                if path.exists() {
                    app.inner.open_path(path);
                }
            }
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}

struct LookEverytingApp {
    inner: LookApp,
    theme: Theme,
    /// Last applied `(preference, resolved_dark)`.
    applied: (ThemePreference, bool),
}

impl LookEverytingApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cap_ui::install_fonts(&cc.egui_ctx);
        let system_dark = detect_system_dark(&cc.egui_ctx);
        let mut app = Self {
            inner: LookApp::new(),
            theme: Theme::dark(),
            applied: (ThemePreference::Dark, true),
        };
        let pref = app.inner.settings.theme;
        let resolved = resolve_dark(pref, system_dark);
        app.theme = Theme::from_preference(pref, system_dark);
        app.theme.install(&cc.egui_ctx);
        app.applied = (pref, resolved);
        app
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let pref = self.inner.settings.theme;
        let system_dark = detect_system_dark(ctx);
        let resolved = resolve_dark(pref, system_dark);
        let key = (pref, resolved);
        if key == self.applied {
            return;
        }
        self.theme = Theme::from_preference(pref, system_dark);
        self.theme.install(ctx);
        self.applied = key;
    }
}

fn resolve_dark(pref: ThemePreference, system_dark: bool) -> bool {
    match pref {
        ThemePreference::Dark => true,
        ThemePreference::Light => false,
        ThemePreference::System => system_dark,
    }
}

/// Prefer egui's reported system theme; fall back to Windows registry, else dark.
fn detect_system_dark(ctx: &egui::Context) -> bool {
    if let Some(theme) = ctx.system_theme() {
        return theme == egui::Theme::Dark;
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(dark) = windows_apps_use_dark() {
            return dark;
        }
    }
    true
}

#[cfg(target_os = "windows")]
fn windows_apps_use_dark() -> Option<bool> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .ok()?;
    let light: u32 = key.get_value("AppsUseLightTheme").ok()?;
    Some(light == 0)
}

impl eframe::App for LookEverytingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_theme(ctx);
        self.inner.poll_loader();
        self.inner.poll_video(ctx);
        self.inner.tick_slideshow();
        self.inner.tick_animation(ctx);
        self.inner.tick_zoom_tween();
        ui::draw(&mut self.inner, ctx);
        self.inner.flush_settings_if_dirty();
        let animating = matches!(
            &self.inner.media,
            Some(app::LoadedMedia::Image {
                animation: Some(_),
                ..
            })
        );
        let ms = if self.inner.is_loading()
            || self.inner.last_interaction.elapsed().as_millis() < 500
            || self.inner.video_is_playing()
            || self.inner.slideshow_active
            || animating
        {
            6
        } else {
            16
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }
}
