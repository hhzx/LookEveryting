mod app;
mod image_viewport;
mod loader;
mod thumbnails;
mod ui;
mod video_thread;

use std::path::PathBuf;

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
}

impl LookEverytingApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cap_ui::install_fonts(&cc.egui_ctx);
        let theme = Theme::dark();
        theme.install(&cc.egui_ctx);
        Self {
            inner: LookApp::new(),
            theme,
        }
    }
}

impl eframe::App for LookEverytingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.inner.poll_loader();
        self.inner.poll_video(ctx);
        ui::draw(&mut self.inner, ctx);
        self.inner.flush_settings_if_dirty();
        let ms = if self.inner.is_loading()
            || self.inner.last_interaction.elapsed().as_millis() < 500
            || self.inner.video_is_playing()
        {
            6
        } else {
            16
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }
}
