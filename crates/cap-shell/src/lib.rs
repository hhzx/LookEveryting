//! Register LookEveryting as a default handler for supported file types.

use std::path::Path;

use cap_core::FileAssociations;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

const PROG_ID: &str = "LookEveryting";

/// Apply file associations according to user settings.
pub fn apply_file_associations(exe: &Path, prefs: &FileAssociations) -> Result<(), ShellError> {
    #[cfg(windows)]
    {
        return windows::apply(exe, prefs);
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, prefs);
        Err(ShellError::UnsupportedPlatform)
    }
}

/// Remove all LookEveryting file associations.
pub fn clear_file_associations() -> Result<(), ShellError> {
    #[cfg(windows)]
    {
        return windows::clear();
    }
    #[cfg(not(windows))]
    {
        Err(ShellError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
mod windows {
    use std::path::Path;

    use cap_core::{extensions_for, FileAssociations, MediaKind};
    use winreg::enums::*;
    use winreg::RegKey;

    use super::{ShellError, PROG_ID};

    pub fn apply(exe: &Path, prefs: &FileAssociations) -> Result<(), ShellError> {
        let exe = exe
            .canonicalize()
            .map_err(ShellError::Io)?
            .to_string_lossy()
            .to_string();
        let command = format!("\"{exe}\" \"%1\"");

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = hkcu
            .open_subkey_with_flags("Software\\Classes", KEY_READ | KEY_WRITE)
            .map_err(ShellError::Io)?;

        let (prog, _) = classes.create_subkey(PROG_ID)?;
        prog.set_value("", &"LookEveryting Media Viewer")?;
        let (icon, _) = prog.create_subkey("DefaultIcon")?;
        icon.set_value("", &format!("{exe},0"))?;
        let (shell, _) = prog.create_subkey("shell")?;
        let (open, _) = shell.create_subkey("open")?;
        let (cmd, _) = open.create_subkey("command")?;
        cmd.set_value("", &command)?;

        let mut extensions: Vec<&'static str> = Vec::new();
        if prefs.images {
            extensions.extend_from_slice(extensions_for(MediaKind::Image));
        }
        if prefs.videos {
            extensions.extend_from_slice(extensions_for(MediaKind::Video));
        }
        if prefs.models {
            extensions.extend_from_slice(extensions_for(MediaKind::Model));
        }

        for ext in extensions {
            let key_name = format!(".{ext}");
            let (ext_key, _) = classes.create_subkey(&key_name)?;
            ext_key.set_value("", &PROG_ID)?;
            let (open_with, _) = ext_key.create_subkey("OpenWithProgids")?;
            open_with.set_value(PROG_ID, &"")?;
        }

        notify_shell();
        Ok(())
    }

    pub fn clear() -> Result<(), ShellError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = hkcu
            .open_subkey_with_flags("Software\\Classes", KEY_READ | KEY_WRITE)
            .map_err(ShellError::Io)?;
        classes.delete_subkey_all(PROG_ID).ok();

        for kind in [
            MediaKind::Image,
            MediaKind::Video,
            MediaKind::Model,
        ] {
            for ext in extensions_for(kind) {
                let key_name = format!(".{ext}");
                if let Ok(ext_key) = classes.open_subkey_with_flags(&key_name, KEY_READ | KEY_WRITE)
                {
                    let current: String = ext_key.get_value("").unwrap_or_default();
                    if current == PROG_ID {
                        ext_key.delete_value("").ok();
                    }
                    ext_key.delete_subkey_all("OpenWithProgids").ok();
                }
            }
        }

        notify_shell();
        Ok(())
    }

    fn notify_shell() {
        #[link(name = "shell32")]
        extern "system" {
            fn SHChangeNotify(
                event_id: i32,
                flags: u32,
                item1: *const std::ffi::c_void,
                item2: *const std::ffi::c_void,
            );
        }
        const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
        const SHCNF_IDLIST: u32 = 0x0000;
        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, std::ptr::null(), std::ptr::null());
        }
    }
}

/// Path to the running executable.
pub fn current_exe() -> Result<std::path::PathBuf, ShellError> {
    std::env::current_exe().map_err(ShellError::Io)
}
