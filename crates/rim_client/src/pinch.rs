//! Pinch to zoom on a Mac trackpad.
//!
//! miniquad's macOS view never registers `magnifyWithEvent:`, so a pinch
//! never reaches the game. Its view is an Objective-C class registered by
//! name ("RenderViewClass"); `install` adds the method to it at runtime,
//! and the handler sums each gesture step's magnification until the frame
//! takes it. System frameworks only: no new dependency. Elsewhere this is
//! a no-op, and Cmd/Ctrl with a scroll is the zoom.

use std::sync::atomic::{AtomicU32, Ordering};

/// Magnification since the frame last took it, as f32 bits.
static PENDING: AtomicU32 = AtomicU32::new(0);

/// Add one gesture step's magnification (the fraction the fingers spread:
/// 0.05 is 5% apart, negative pinches in).
pub fn add(m: f32) {
    let mut cur = PENDING.load(Ordering::Relaxed);
    loop {
        let next = (f32::from_bits(cur) + m).to_bits();
        match PENDING.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(now) => cur = now,
        }
    }
}

/// This frame's pinch, and start the next from nothing.
pub fn take() -> f32 {
    f32::from_bits(PENDING.swap(0, Ordering::Relaxed))
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
        fn class_addMethod(cls: Id, sel: Sel, imp: *const c_void, types: *const c_char) -> bool;
        fn objc_msgSend();
    }

    /// `-[NSView magnifyWithEvent:]`: read `[event magnification]` and keep it.
    extern "C" fn magnify(_this: Id, _cmd: Sel, event: Id) {
        // SAFETY: `event` is the NSEvent AppKit passes a magnify handler, and
        // `magnification` returns a CGFloat (f64 on every Mac we build for);
        // objc_msgSend returns a double in the float register on arm64 and
        // x86_64 alike.
        let m = unsafe {
            let send: extern "C" fn(Id, Sel) -> f64 = std::mem::transmute(objc_msgSend as *const c_void);
            send(event, sel_registerName(c"magnification".as_ptr()))
        };
        super::add(m as f32);
    }

    pub fn install() -> bool {
        // SAFETY: plain Objective-C runtime calls on a class miniquad has
        // registered by the time the window is up; the type encoding "v@:@"
        // matches `magnify` (void return, self, _cmd, one object).
        unsafe {
            let cls = objc_getClass(c"RenderViewClass".as_ptr());
            if cls.is_null() {
                return false;
            }
            let sel = sel_registerName(c"magnifyWithEvent:".as_ptr());
            class_addMethod(cls, sel, magnify as *const c_void, c"v@:@".as_ptr())
        }
    }
}

/// Teach the game's view to hear a pinch. Call once the window is up.
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

#[cfg(test)]
mod tests {
    #[test]
    fn a_frame_takes_the_sum_of_its_steps_once() {
        super::take();
        super::add(0.05);
        super::add(0.03);
        super::add(-0.01);
        assert!((super::take() - 0.07).abs() < 1e-6);
        assert_eq!(super::take(), 0.0, "and the next frame starts from nothing");
    }
}
