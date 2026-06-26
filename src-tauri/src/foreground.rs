//! Cross-platform helper to read the name of the foreground (active) application.
//!
//! Used to substitute the `${app}` placeholder in post-processing prompts with
//! the active application's name. Returns `None` when it cannot be determined
//! (e.g. on Linux, or when the platform API fails); callers should then treat
//! the value as an empty string.

#[cfg(target_os = "macos")]
mod imp {
    use accessibility_sys::{
        kAXErrorSuccess, AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
        AXUIElementSetMessagingTimeout,
    };
    use core_foundation::base::{CFGetTypeID, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ptr;

    // System-wide AX attribute names (plain CFString attribute identifiers).
    const AX_FOCUSED_APPLICATION: &str = "AXFocusedApplication";
    const AX_TITLE: &str = "AXTitle";

    unsafe fn copy_attribute(element: CFTypeRef, attribute: &str) -> CFTypeRef {
        let attr = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err =
            AXUIElementCopyAttributeValue(element as _, attr.as_concrete_TypeRef(), &mut value);
        if err != kAXErrorSuccess {
            return ptr::null();
        }
        value
    }

    /// Localized name of the frontmost application (e.g. "Safari"), via the
    /// system-wide focused-application element's title.
    pub fn foreground_app_name() -> Option<String> {
        unsafe {
            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return None;
            }
            // Guard against a hung target app stalling the pipeline.
            AXUIElementSetMessagingTimeout(system, 0.5);

            let app = copy_attribute(system as CFTypeRef, AX_FOCUSED_APPLICATION);
            CFRelease(system as CFTypeRef);
            if app.is_null() {
                return None;
            }

            let title = copy_attribute(app, AX_TITLE);
            CFRelease(app);
            if title.is_null() {
                return None;
            }

            if CFGetTypeID(title) == CFString::type_id() {
                let s = CFString::wrap_under_create_rule(title as CFStringRef);
                let name = s.to_string();
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                CFRelease(title);
                None
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, FALSE};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    /// Reduce an executable path to a friendly stem, e.g.
    /// `C:\\...\\Code.exe` -> `Code`. Falls back to the raw input.
    fn exe_stem(path: &str) -> String {
        std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string())
    }

    /// Process image-name stem of the foreground window's owning process.
    pub fn foreground_app_name() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
            if pid == 0 {
                return None;
            }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid).ok()?;

            let mut buf = [0u16; 260];
            let mut size = buf.len() as u32;
            let result = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(handle);
            result.ok()?;

            let path = String::from_utf16_lossy(&buf[..size as usize]);
            let name = exe_stem(&path);
            let trimmed = name.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    pub fn foreground_app_name() -> Option<String> {
        None
    }
}

/// Name of the foreground (active) application, or `None` if it cannot be
/// determined on this platform / at this time.
pub fn foreground_app_name() -> Option<String> {
    imp::foreground_app_name()
}
