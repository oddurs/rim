//! Open the window without taking focus: `--background`, and always for the
//! autotest and the render bench.
//!
//! Once it's running, miniquad activates the app
//! (`-[NSRunningApplication activateWithOptions:]`, ignoring other apps).
//! macOS then moves the person to the Space the new window opened in, out
//! of whatever they were doing: the terminal that started it, often a full
//! screen Space a new window can't join. `install` turns that one call into
//! a no-op for this process, before miniquad starts. The window still opens,
//! with a Dock icon, for a click or Cmd-Tab to bring forward. System
//! frameworks only: no new dependency. Elsewhere this does nothing.

/// Keep the app from activating itself. Call before the window opens.
/// Returns whether it took (always false off macOS).
pub fn install() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::install().is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::{c_char, c_void};

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn class_replaceMethod(cls: Id, sel: Sel, imp: *const c_void, types: *const c_char) -> *const c_void;
        #[cfg(test)]
        fn class_getMethodImplementation(cls: Id, sel: Sel) -> *const c_void;
    }

    /// `-[NSRunningApplication activateWithOptions:]` that declines: NO.
    /// BOOL is a signed char on x86_64 and a bool on arm64; returning 0 in
    /// an i8 is the same bits for both.
    extern "C" fn decline(_this: Id, _cmd: Sel, _options: usize) -> i8 {
        0
    }

    /// The implementation now answering, if it took. Rust promises a
    /// function no single address across codegen units, so this is the one
    /// pointer to compare the runtime's answer with.
    pub fn install() -> Option<*const c_void> {
        // SAFETY: plain Objective-C runtime calls on an AppKit class, which
        // is loaded before main (miniquad links AppKit); "c@:Q" matches
        // `decline` (BOOL return, self, _cmd, an NSUInteger).
        unsafe {
            let cls = objc_getClass(c"NSRunningApplication".as_ptr());
            if cls.is_null() {
                return None;
            }
            let sel = sel_registerName(c"activateWithOptions:".as_ptr());
            let imp = decline as *const c_void;
            class_replaceMethod(cls, sel, imp, c"c@:Q".as_ptr());
            Some(imp)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Nothing can watch the focus from a test, but the app's own
        /// activation is only ever this method: after `install` it is the
        /// one `install` registered, and it declines.
        #[test]
        fn activating_the_app_is_declined() {
            let ours = install().expect("AppKit's NSRunningApplication is there to patch");
            let sel = unsafe { sel_registerName(c"activateWithOptions:".as_ptr()) };
            // SAFETY: a runtime lookup of a class and selector that exist.
            let imp = unsafe { class_getMethodImplementation(objc_getClass(c"NSRunningApplication".as_ptr()), sel) };
            assert_eq!(imp, ours, "the runtime answers with what install registered");
            // SAFETY: `imp` is `decline` (just checked), which reads none of
            // its arguments; the signature is the one it was registered with.
            let call: extern "C" fn(Id, Sel, usize) -> i8 = unsafe { std::mem::transmute(imp) };
            assert_eq!(call(std::ptr::null_mut(), sel, 2), 0, "and it declines");
        }
    }
}
