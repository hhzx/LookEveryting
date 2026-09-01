mod app;
mod ui;

use cap_ui::Theme;
use eframe::egui;

use app::LookApp;

fn main() -> eframe::Result<()> {
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
        Box::new(|cc| Ok(Box::new(LookEverytingApp::new(cc)))),
    )
}

struct LookEverytingApp {
    inner: LookApp,
    theme: Theme,
}

impl LookEverytingApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
        ui::draw(&mut self.inner, ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
