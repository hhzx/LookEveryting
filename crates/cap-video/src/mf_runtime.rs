//! Process-wide Media Foundation initialization (once per process).

use std::sync::Once;

#[cfg(windows)]
use windows::Win32::Media::MediaFoundation::{MFStartup, MF_VERSION, MFSTARTUP_LITE};
#[cfg(windows)]
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

static INIT: Once = Once::new();

/// Ensure COM and Media Foundation are initialized on the calling thread.
pub fn ensure_initialized() {
    INIT.call_once(|| {
        #[cfg(windows)]
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let _ = MFStartup(MF_VERSION, MFSTARTUP_LITE);
        }
    });
}
