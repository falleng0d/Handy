//! Accessibility helpers for reading the currently focused text field.
//!
//! Used to detect "continuation" (text before the caret exists) and to
//! inject text up to the caret into post-processing prompts. macOS reads
//! via Accessibility (AX), Windows via UI Automation (UIA), and other platforms
//! are no-ops returning `None`/`false`.

#[cfg(target_os = "macos")]
mod imp {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXSelectedTextRangeAttribute,
        kAXValueAttribute, kAXValueTypeCFRange, AXUIElementCopyAttributeValue,
        AXUIElementCreateSystemWide, AXUIElementSetMessagingTimeout, AXValueGetValue, AXValueRef,
    };
    use core_foundation::base::{CFGetTypeID, CFRange, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
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

    /// Text content of the focused UI element up to the caret (i.e. the text
    /// before the insertion point). Falls back to the full value when the caret
    /// position can't be determined. Returns `None` when there is no focused
    /// element, it exposes no string value, or accessibility is not trusted.
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
            if value.is_null() {
                CFRelease(focused);
                return None;
            }
            // AXValue is only a CFString for text fields; bail out otherwise
            // (e.g. sliders return CFNumber).
            if CFGetTypeID(value) != CFString::type_id() {
                CFRelease(value);
                CFRelease(focused);
                return None;
            }
            let full = CFString::wrap_under_create_rule(value as CFStringRef).to_string();

            let caret = caret_utf16_index(focused);
            CFRelease(focused);

            Some(match caret {
                Some(loc) => truncate_utf16(&full, loc),
                None => full,
            })
        }
    }

    /// Caret position as a UTF-16 offset, from the element's selected text range
    /// (the start of the current selection / insertion point). `None` when the
    /// attribute is missing or not a `CFRange` AXValue.
    unsafe fn caret_utf16_index(element: CFTypeRef) -> Option<usize> {
        let range_ref = copy_attribute(element, kAXSelectedTextRangeAttribute);
        if range_ref.is_null() {
            return None;
        }
        let mut range = CFRange {
            location: 0,
            length: 0,
        };
        // AXValueGetValue is defensive: it returns false if `range_ref` is not a
        // CFRange-typed AXValue, so this is safe to call on any CFTypeRef.
        let ok = AXValueGetValue(
            range_ref as AXValueRef,
            kAXValueTypeCFRange,
            &mut range as *mut _ as *mut c_void,
        );
        CFRelease(range_ref);
        if ok && range.location >= 0 {
            Some(range.location as usize)
        } else {
            None
        }
    }

    /// Keep the first `utf16_len` UTF-16 code units of `s`.
    fn truncate_utf16(s: &str, utf16_len: usize) -> String {
        let units: Vec<u16> = s.encode_utf16().collect();
        let end = utf16_len.min(units.len());
        String::from_utf16_lossy(&units[..end])
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
        TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start, UIA_TextPatternId,
        UIA_ValuePatternId,
    };

    /// Read the focused element's text via UI Automation, up to the caret when
    /// possible. Must run on a thread with COM already initialized.
    unsafe fn read_focused_text() -> Option<String> {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let element = automation.GetFocusedElement().ok()?;

        // Prefer TextPattern: it exposes the caret, so we can take the text
        // before the insertion point.
        if let Ok(unknown) = element.GetCurrentPattern(UIA_TextPatternId) {
            if let Ok(text) = unknown.cast::<IUIAutomationTextPattern>() {
                if let Some(s) = text_before_caret(&text) {
                    return Some(s);
                }
            }
        }

        // Fallback: ValuePattern exposes the full value but no caret position.
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

        None
    }

    /// Text from the start of the document to the caret (start of the current
    /// selection). Falls back to the full document text when the selection /
    /// caret can't be resolved.
    unsafe fn text_before_caret(text: &IUIAutomationTextPattern) -> Option<String> {
        let doc = text.DocumentRange().ok()?;
        if let Ok(selection) = text.GetSelection() {
            if let Ok(caret) = selection.GetElement(0) {
                if doc
                    .MoveEndpointByRange(
                        TextPatternRangeEndpoint_End,
                        &caret,
                        TextPatternRangeEndpoint_Start,
                    )
                    .is_ok()
                {
                    if let Ok(bstr) = doc.GetText(-1) {
                        return Some(bstr.to_string());
                    }
                }
            }
        }
        // Fall back to the full document text (fetch a fresh, unmodified range).
        text.DocumentRange()
            .ok()?
            .GetText(-1)
            .ok()
            .map(|b| b.to_string())
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
