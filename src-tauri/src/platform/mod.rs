use serde::Serialize;
use std::sync::{
    mpsc::{self, Receiver, SyncSender},
    OnceLock,
};

#[derive(Clone, Debug, Serialize)]
pub struct Permissions {
    pub microphone: String,
    pub accessibility: bool,
    pub platform: String,
    pub text_injection: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct DisplayBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemEvent {
    WillSleep,
    ScreensDidSleep,
    DidWake,
    ScreensDidWake,
}

static SYSTEM_EVENTS: OnceLock<SyncSender<SystemEvent>> = OnceLock::new();

/// Subscribe to public macOS sleep and display-sleep notifications.
pub fn subscribe_system_events() -> Result<Receiver<SystemEvent>, String> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err("System sleep notifications are not implemented on this platform.".into());
    }
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = mpsc::sync_channel(8);
        SYSTEM_EVENTS
            .set(tx)
            .map_err(|_| "system event monitor is already installed")?;
        unsafe {
            native::dicta_register_system_monitors();
        }
        Ok(rx)
    }
}

#[no_mangle]
pub extern "C" fn dicta_system_event(kind: i32) {
    let event = match kind {
        0 => SystemEvent::WillSleep,
        1 => SystemEvent::ScreensDidSleep,
        2 => SystemEvent::DidWake,
        3 => SystemEvent::ScreensDidWake,
        _ => return,
    };
    if let Some(sender) = SYSTEM_EVENTS.get() {
        let _ = sender.try_send(event);
    }
}

#[cfg(target_os = "macos")]
mod native {
    use super::{DisplayBounds, Permissions};
    use std::{
        ffi::{c_char, c_void, CStr, CString},
        sync::Arc,
    };

    unsafe extern "C" {
        fn dicta_microphone_status() -> i32;
        fn dicta_request_microphone();
        fn dicta_accessibility_trusted(prompt: bool) -> bool;
        fn dicta_capture_target(
            token: *mut u64,
            pid: *mut i32,
            role: *mut c_char,
            role_len: usize,
            title: *mut c_char,
            title_len: usize,
        ) -> i32;
        fn dicta_release_target(token: u64);
        fn dicta_insert_text(token: u64, pid: i32, text: *const c_char) -> i32;
        fn dicta_copy_recovery(text: *const c_char) -> i32;
        fn dicta_active_display_bounds(bounds: *mut f64) -> i32;
        fn dicta_configure_overlay(window: *mut c_void) -> i32;
        pub(super) fn dicta_register_system_monitors();
        fn dicta_play_sound();
    }

    struct TargetInner {
        token: u64,
    }
    impl Drop for TargetInner {
        fn drop(&mut self) {
            unsafe { dicta_release_target(self.token) }
        }
    }

    #[derive(Clone)]
    pub struct FocusTarget {
        inner: Arc<TargetInner>,
        pub pid: i32,
        pub role: String,
        pub window_title: String,
    }

    fn read_buf(buf: &[c_char]) -> String {
        unsafe { CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned() }
    }
    fn c_string(value: &str) -> Result<CString, String> {
        CString::new(value).map_err(|_| "text contains an invalid NUL character".into())
    }

    pub fn permissions() -> Permissions {
        let microphone = match unsafe { dicta_microphone_status() } {
            3 => "granted",
            2 | 1 => "denied",
            0 => "undetermined",
            _ => "unavailable",
        };
        let accessibility = unsafe { dicta_accessibility_trusted(false) };
        Permissions {
            microphone: microphone.into(),
            accessibility,
            platform: "macos".into(),
            text_injection: accessibility,
        }
    }

    pub fn request_permission(kind: &str) -> Result<(), String> {
        match kind {
            "microphone" => {
                unsafe { dicta_request_microphone() };
                Ok(())
            }
            "accessibility" => {
                unsafe { dicta_accessibility_trusted(true) };
                Ok(())
            }
            _ => Err("permission kind must be microphone or accessibility".into()),
        }
    }

    pub fn capture_target() -> Result<FocusTarget, String> {
        let mut token = 0;
        let mut pid = 0;
        let mut role = [0 as c_char; 128];
        let mut title = [0 as c_char; 512];
        let status = unsafe {
            dicta_capture_target(
                &mut token,
                &mut pid,
                role.as_mut_ptr(),
                role.len(),
                title.as_mut_ptr(),
                title.len(),
            )
        };
        if status != 0 {
            return Err(match status {
                1 => "Accessibility permission is required to identify the text field.",
                2 => "No safe editable field is focused.",
                3 => "Dicta è attivo: avvia con la scorciatoia nell’app in cui vuoi scrivere, oppure copia il testo da Dicta",
                4 => "The focused field is secure.",
                _ => "Could not identify the focused application.",
            }
            .into());
        }
        Ok(FocusTarget {
            inner: Arc::new(TargetInner { token }),
            pid,
            role: read_buf(&role),
            window_title: read_buf(&title),
        })
    }

    pub fn insert_text(target: &FocusTarget, text: &str) -> Result<(), String> {
        let text = c_string(text)?;
        let status = unsafe { dicta_insert_text(target.inner.token, target.pid, text.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(match status {
                1 => "Accessibility permission was revoked.",
                2 => "The original application is no longer active.",
                3 => "The original text field is no longer focused.",
                4 => "Text insertion is disabled for this field.",
                5 => "The target field became secure.",
                _ => "macOS could not insert text into the original field.",
            }
            .into())
        }
    }

    pub fn copy_recovery_text(text: &str) -> Result<(), String> {
        let text = c_string(text)?;
        if unsafe { dicta_copy_recovery(text.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err("Could not copy the recovery text to the clipboard.".into())
        }
    }

    pub fn active_display_bounds() -> Result<DisplayBounds, String> {
        let mut b = [0.0; 4];
        if unsafe { dicta_active_display_bounds(b.as_mut_ptr()) } != 0 {
            return Err("Could not determine the active display.".into());
        }
        Ok(DisplayBounds {
            x: b[0],
            y: b[1],
            width: b[2],
            height: b[3],
        })
    }

    /// Configure a Tauri NSWindow to join Spaces and fullscreen spaces.
    ///
    /// # Safety
    /// `window` must be a live `NSWindow` pointer owned by the calling Tauri window.
    pub unsafe fn configure_overlay(window: *mut c_void) -> Result<(), String> {
        if window.is_null() || unsafe { dicta_configure_overlay(window) } != 0 {
            Err("Could not configure the overlay for macOS Spaces.".into())
        } else {
            Ok(())
        }
    }

    pub fn play_sound() {
        unsafe { dicta_play_sound() }
    }
}

#[cfg(target_os = "macos")]
pub use native::{
    active_display_bounds, capture_target, copy_recovery_text, insert_text, permissions,
    request_permission, FocusTarget,
};

#[cfg(target_os = "macos")]
pub use native::configure_overlay;

#[cfg(target_os = "macos")]
pub use native::play_sound;

#[cfg(not(target_os = "macos"))]
#[derive(Clone, Debug)]
pub struct FocusTarget {
    pub pid: i32,
    pub role: String,
    pub window_title: String,
}

#[cfg(not(target_os = "macos"))]
pub fn permissions() -> Permissions {
    Permissions {
        microphone: "unavailable".into(),
        accessibility: false,
        platform: std::env::consts::OS.into(),
        text_injection: false,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_permission(_kind: &str) -> Result<(), String> {
    Err("Native permission requests are not implemented on this platform.".into())
}

#[cfg(not(target_os = "macos"))]
pub fn capture_target() -> Result<FocusTarget, String> {
    Err("Native text targeting is not implemented on this platform.".into())
}

#[cfg(not(target_os = "macos"))]
pub fn insert_text(_target: &FocusTarget, _text: &str) -> Result<(), String> {
    Err("Native text insertion is not implemented on this platform.".into())
}

#[cfg(not(target_os = "macos"))]
pub fn copy_recovery_text(_text: &str) -> Result<(), String> {
    Err("Clipboard recovery is not implemented on this platform.".into())
}

#[cfg(not(target_os = "macos"))]
pub fn active_display_bounds() -> Result<DisplayBounds, String> {
    Err("Native display positioning is not implemented on this platform.".into())
}

#[cfg(not(target_os = "macos"))]
pub fn play_sound() {}

#[cfg(target_os = "macos")]
pub use native::FocusTarget as NativeFocusTarget;
