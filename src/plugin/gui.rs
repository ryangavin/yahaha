//! The plugin's editor in its own window, through the raw Objective-C runtime.
//!
//! AUv2 instruments publish a Cocoa view factory (`kAudioUnitProperty_CocoaUI`); those that
//! don't get CoreAudioKit's `AUGenericView`, a slider
//! per parameter. Must run on the main thread. A production host would use `objc2` /
//! `objc2-app-kit` instead of hand-typed `objc_msgSend` casts (docs/plugin-hosting.md).

use anyhow::{bail, Result};
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use std::ffi::{c_void, CStr};
use std::mem::transmute;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use super::ffi::*;

#[repr(C)]
#[derive(Clone, Copy)]
struct NSSize {
    w: f64,
    h: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NSRect {
    x: f64,
    y: f64,
    size: NSSize,
}

unsafe extern "C" {
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

fn cls(name: &CStr) -> Id {
    unsafe { objc_getClass(name.as_ptr()) }
}

fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

// Typed views of objc_msgSend. On arm64 (and for these argument/return types on x86_64)
// objc_msgSend follows the normal C calling convention of the method it dispatches to.
unsafe fn send0(obj: Id, s: &CStr) -> Id {
    let f: unsafe extern "C" fn(Id, Sel) -> Id = unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(s)) }
}

unsafe fn send1(obj: Id, s: &CStr, a: *const c_void) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *const c_void) -> Id = unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(s), a) }
}

unsafe fn send_int(obj: Id, s: &CStr, a: isize) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, isize) -> Id = unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(s), a) }
}

unsafe fn send_bool_ret(obj: Id, s: &CStr) -> bool {
    let f: unsafe extern "C" fn(Id, Sel) -> i8 = unsafe { transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(obj, sel(s)) != 0 }
}

/// The instrument's own Cocoa view, if it publishes one.
unsafe fn cocoa_view(unit: AudioUnit) -> Id {
    let mut size = 0u32;
    let mut writable = 0u8;
    if unsafe { AudioUnitGetPropertyInfo(unit, kAudioUnitProperty_CocoaUI, kAudioUnitScope_Global, 0, &mut size, &mut writable) } != 0
        || (size as usize) < std::mem::size_of::<AudioUnitCocoaViewInfo>()
    {
        return ptr::null_mut();
    }
    let mut info = AudioUnitCocoaViewInfo { mCocoaAUViewBundleLocation: ptr::null(), mCocoaAUViewClass: [ptr::null()] };
    let mut size = std::mem::size_of::<AudioUnitCocoaViewInfo>() as u32;
    if unsafe { AudioUnitGetProperty(unit, kAudioUnitProperty_CocoaUI, kAudioUnitScope_Global, 0, &mut info as *mut _ as *mut c_void, &mut size) } != 0 {
        return ptr::null_mut();
    }
    unsafe {
        // CFURL / CFString are toll-free bridged to NSURL / NSString.
        let bundle = send1(cls(c"NSBundle"), c"bundleWithURL:", info.mCocoaAUViewBundleLocation);
        let factory_class = if bundle.is_null() { ptr::null_mut() } else { send1(bundle, c"classNamed:", info.mCocoaAUViewClass[0]) };
        let view = if factory_class.is_null() {
            ptr::null_mut()
        } else {
            let factory = send0(send0(factory_class, c"alloc"), c"init");
            let f: unsafe extern "C" fn(Id, Sel, AudioUnit, NSSize) -> Id = transmute(objc_msgSend as unsafe extern "C" fn());
            f(factory, sel(c"uiViewForAudioUnit:withSize:"), unit, NSSize { w: 640.0, h: 400.0 })
        };
        if !info.mCocoaAUViewBundleLocation.is_null() {
            CFRelease(info.mCocoaAUViewBundleLocation);
        }
        if !info.mCocoaAUViewClass[0].is_null() {
            CFRelease(info.mCocoaAUViewClass[0]);
        }
        view
    }
}

/// Open the instrument's editor and pump the main-thread event loop until the window is
/// closed or `running` goes false. Blocks the calling (main) thread.
///
/// # Safety
/// `unit` must be a live, initialized Audio Unit (e.g. [`super::Instrument::raw`]) that
/// outlives this call, and this must be called on the main thread.
pub unsafe fn show(unit: AudioUnit, title: &str, running: &AtomicBool) -> Result<()> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let app = send0(cls(c"NSApplication"), c"sharedApplication");
        send_int(app, c"setActivationPolicy:", 0); // NSApplicationActivationPolicyRegular
        send0(app, c"finishLaunching");

        let mut view = cocoa_view(unit);
        let generic = view.is_null();
        if generic {
            let g = cls(c"AUGenericView");
            if g.is_null() {
                objc_autoreleasePoolPop(pool);
                bail!("CoreAudioKit's AUGenericView is not available");
            }
            view = send1(send0(g, c"alloc"), c"initWithAudioUnit:", unit);
        }
        if view.is_null() {
            objc_autoreleasePoolPop(pool);
            bail!("the plugin did not provide a view");
        }

        #[cfg(target_arch = "aarch64")]
        let frame: NSRect = {
            let f: unsafe extern "C" fn(Id, Sel) -> NSRect = transmute(objc_msgSend as unsafe extern "C" fn());
            f(view, sel(c"frame"))
        };
        #[cfg(not(target_arch = "aarch64"))]
        let frame = NSRect { x: 0.0, y: 0.0, size: NSSize { w: 640.0, h: 400.0 } };
        let rect = NSRect { x: 0.0, y: 0.0, size: NSSize { w: frame.size.w.max(320.0), h: frame.size.h.max(120.0) } };

        println!("gui:     {} view, {:.0}x{:.0}", if generic { "generic (AUGenericView)" } else { "plugin's own Cocoa" }, rect.size.w, rect.size.h);
        let window = send0(cls(c"NSWindow"), c"alloc");
        let init: unsafe extern "C" fn(Id, Sel, NSRect, usize, usize, i8) -> Id = transmute(objc_msgSend as unsafe extern "C" fn());
        // Titled | Closable | Miniaturizable | Resizable, buffered backing.
        let window = init(window, sel(c"initWithContentRect:styleMask:backing:defer:"), rect, 1 | 2 | 4 | 8, 2, 0);
        let set_bool: unsafe extern "C" fn(Id, Sel, i8) = transmute(objc_msgSend as unsafe extern "C" fn());
        set_bool(window, sel(c"setReleasedWhenClosed:"), 0);
        let title = if generic { format!("{title} (generic view)") } else { title.to_string() };
        let name = CFString::new(&title);
        send1(window, c"setTitle:", name.as_concrete_TypeRef() as *const c_void);
        send1(window, c"setContentView:", view);
        send0(window, c"center");
        send1(window, c"makeKeyAndOrderFront:", ptr::null());
        set_bool(app, sel(c"activateIgnoringOtherApps:"), 1);

        let mode = CFString::new("kCFRunLoopDefaultMode");
        let next: unsafe extern "C" fn(Id, Sel, u64, Id, *const c_void, i8) -> Id = transmute(objc_msgSend as unsafe extern "C" fn());
        let date_in: unsafe extern "C" fn(Id, Sel, f64) -> Id = transmute(objc_msgSend as unsafe extern "C" fn());
        while running.load(Relaxed) && send_bool_ret(window, c"isVisible") {
            let inner = objc_autoreleasePoolPush();
            let until = date_in(cls(c"NSDate"), sel(c"dateWithTimeIntervalSinceNow:"), 0.02);
            let ev = next(app, sel(c"nextEventMatchingMask:untilDate:inMode:dequeue:"), u64::MAX, until, mode.as_concrete_TypeRef() as *const c_void, 1);
            if !ev.is_null() {
                send1(app, c"sendEvent:", ev);
            }
            send0(app, c"updateWindows");
            objc_autoreleasePoolPop(inner);
        }
        send0(window, c"close");
        objc_autoreleasePoolPop(pool);
    }
    Ok(())
}
