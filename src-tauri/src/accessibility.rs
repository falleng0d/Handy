//! macOS Accessibility (AX) helpers for reading the currently focused text field.
//!
//! Used to detect "continuation" (focused field already contains text) and to
//! inject the focused field's contents into post-processing prompts. On
//! non-macOS platforms these functions are no-ops returning `None`/`false`.

#[cfg(target_os = "macos")]
mod imp {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXValueAttribute,
        AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
        AXUIElementSetMessagingTimeout,
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

/// Text content of the currently focused text field, or `None`.
#[cfg(target_os = "macos")]
pub fn focused_field_text() -> Option<String> {
    imp::focused_field_text()
}

/// Text content of the currently focused text field, or `None` (non-macOS no-op).
#[cfg(not(target_os = "macos"))]
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
