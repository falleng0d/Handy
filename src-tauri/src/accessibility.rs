//! Accessibility helpers for reading the currently focused text field.
//!
//! Used to detect "continuation" (focused field already contains text) and to
//! inject the focused field's contents into post-processing prompts. macOS reads
//! via Accessibility (AX), Windows via UI Automation (UIA), and other platforms
//! are no-ops returning `None`/`false`.

#[cfg(target_os = "macos")]
mod imp {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXValueAttribute,
        AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide, AXUIElementSetMessagingTimeout,
    };
    use core_foundation::base::{CFGetTypeID, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ptr;

    /// Copy an attribute value of `element` as a raw CFTypeRef following the CF
    /// "create rule" (caller owns the returned ref). Returns null on failure.
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

    /// Read the text content of the currently focused UI element, if it is a
    /// text-bearing element. Returns `None` when there is no focused element,
    /// the element exposes no string value, or accessibility is not trusted.
    pub fn focused_field_text() -> Option<String> {
        unsafe {
            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return None;
            }
            // Guard against a hung target app stalling the transcription pipeline.
            AXUIElementSetMessagingTimeout(system, 0.5);

            let focused = copy_attribute(system as CFTypeRef, kAXFocusedUIElementAttribute);
            CFRelease(system as CFTypeRef);
            if focused.is_null() {
                return None;
            }

            let value = copy_attribute(focused, kAXValueAttribute);
            CFRelease(focused);
            if value.is_null() {
                return None;
            }

            // AXValue is only a CFString for text fields; bail out otherwise
            // (e.g. sliders return CFNumber, ranges return AXValue wrappers).
            if CFGetTypeID(value) == CFString::type_id() {
                let s = CFString::wrap_under_create_rule(value as CFStringRef);
                Some(s.to_string())
            } else {
                CFRelease(value);
                None
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::mpsc;
    use std::time::Duration;

    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationValuePattern,
        UIA_TextPatternId, UIA_ValuePatternId,
    };

    /// Read the focused element's text via UI Automation. Must run on a thread
    /// with COM already initialized.
    unsafe fn read_focused_text() -> Option<String> {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let element = automation.GetFocusedElement().ok()?;

        // ValuePattern: typical single-line / edit controls expose their text here.
        if let Ok(unknown) = element.GetCurrentPattern(UIA_ValuePatternId) {
            if let Ok(value) = unknown.cast::<IUIAutomationValuePattern>() {
                if let Ok(bstr) = value.CurrentValue() {
                    let s = bstr.to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }

        // TextPattern: documents / multiline rich-text editors.
        if let Ok(unknown) = element.GetCurrentPattern(UIA_TextPatternId) {
            if let Ok(text) = unknown.cast::<IUIAutomationTextPattern>() {
                if let Ok(range) = text.DocumentRange() {
                    if let Ok(bstr) = range.GetText(-1) {
                        return Some(bstr.to_string());
                    }
                }
            }
        }

        None
    }

    /// Text content of the currently focused text field, or `None`.
    ///
    /// UI Automation calls marshal across processes and can block on an
    /// unresponsive target app, and COM objects are apartment-bound. We run the
    /// read on a dedicated COM-initialized (MTA) thread and bound the wait so the
    /// transcription pipeline can't stall (mirrors the macOS AX messaging timeout).
    pub fn focused_field_text() -> Option<String> {
        let (tx, rx) = mpsc::channel::<Option<String>>();
        std::thread::spawn(move || {
            let result = unsafe {
                let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
                let text = read_focused_text();
                // Only balance CoUninitialize when we actually initialized COM
                // on this thread (S_OK/S_FALSE are success HRESULTs).
                if hr.is_ok() {
                    CoUninitialize();
                }
                text
            };
            let _ = tx.send(result);
        });

        rx.recv_timeout(Duration::from_millis(500)).unwrap_or(None)
    }
}

/// Text content of the currently focused text field, or `None`.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn focused_field_text() -> Option<String> {
    imp::focused_field_text()
}

/// Text content of the currently focused text field, or `None` (unsupported platform).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn focused_field_text() -> Option<String> {
    None
}

/// Whether the focused field already contains non-trivial text (not empty and
/// not only whitespace/newlines). Used to detect a transcription "continuation".
pub fn focused_field_has_text() -> bool {
    focused_field_text()
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false)
}
