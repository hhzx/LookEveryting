//! System and bundled CJK font loading for egui.

use egui::{Context, FontData, FontDefinitions, FontFamily};

const CJK_FONT_ID: &str = "cjk";

/// Install fonts with CJK fallback into the egui context. Call once at startup.
pub fn install(ctx: &Context) {
    let Some(bytes) = load_cjk_font_bytes() else {
        eprintln!("LookEveryting: no CJK font found; Chinese text may not render");
        return;
    };

    let mut fonts = FontDefinitions::default();
    let mut font_data = FontData::from_owned(bytes);
    font_data.index = 0;

    fonts.font_data.insert(CJK_FONT_ID.to_owned(), font_data.into());

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(CJK_FONT_ID.to_owned());
    }

    ctx.set_fonts(fonts);
}

fn load_cjk_font_bytes() -> Option<Vec<u8>> {
    for path in system_cjk_font_paths() {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                return Some(bytes);
            }
        }
    }
    None
}

fn system_cjk_font_paths() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        let fonts = std::path::Path::new(&windir).join("Fonts");
        [
            "msyh.ttc",
            "msyhbd.ttc",
            "simhei.ttf",
            "simsun.ttc",
            "msjh.ttc",
        ]
        .into_iter()
        .map(|name| fonts.join(name))
        .collect()
    }

    #[cfg(target_os = "macos")]
    {
        [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
    }

    #[cfg(target_os = "linux")]
    {
        [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_system_cjk_font_on_supported_platforms() {
        let paths = system_cjk_font_paths();
        if paths.is_empty() {
            return;
        }
        let found = paths.iter().any(|p| p.exists());
        assert!(
            found,
            "expected at least one CJK font in {:?}",
            paths
        );
    }
}
