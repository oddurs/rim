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
        mac::install()
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

    pub fn install() -> bool {
        // SAFETY: plain Objective-C runtime calls on an AppKit class, which
        // is loaded before main (miniquad links AppKit); "c@:Q" matches
        // `decline` (BOOL return, self, _cmd, an NSUInteger).
        unsafe {
            let cls = objc_getClass(c"NSRunningApplication".as_ptr());
            if cls.is_null() {
                return false;
            }
            let sel = sel_registerName(c"activateWithOptions:".as_ptr());
            class_replaceMethod(cls, sel, decline as *const c_void, c"c@:Q".as_ptr());
            true
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Nothing can watch the focus from a test, but the app's own
        /// activation is only ever this method: after `install` it is ours.
        #[test]
        fn activating_the_app_is_declined() {
            assert!(install(), "AppKit's NSRunningApplication is there to patch");
            // SAFETY: runtime lookups of a class and a selector that exist.
            let imp = unsafe {
                class_getMethodImplementation(
                    objc_getClass(c"NSRunningApplication".as_ptr()),
                    sel_registerName(c"activateWithOptions:".as_ptr()),
                )
            };
            assert_eq!(imp, decline as *const c_void);
        }
    }
}
