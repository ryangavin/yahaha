//! The plugin's editor window. Main thread only (AppKit), enforced by `MainThreadMarker`.
//!
//! [`open_editor`] picks the best view the unit offers:
//!
//! 1. **Cocoa view** (`kAudioUnitProperty_CocoaUI`): an AUv2 unit loaded in process names a
//!    view-factory class in its bundle; `uiViewForAudioUnit:withSize:` makes the NSView.
//! 2. **View controller** (`kAudioUnitProperty_RequestViewController`): AUv3 units, and any
//!    unit loaded out of process, hand over an `NSViewController` (a remote view for an
//!    out-of-process unit). The request completes asynchronously; `open_editor` spins the
//!    main run loop for up to two seconds waiting for it.
//! 3. **Generic view**: CoreAudioKit's `AUGenericView`, a slider per parameter.
//!
//! The window is a plain titled `NSWindow`. The desktop app (Tauri) owns the
//! `NSApplication` and its run loop, so it only calls [`open_editor`] / [`Editor::close`] from
//! its main thread (e.g. `app.run_on_main_thread`). A CLI tool without an app calls
//! [`prepare_app`] once and [`pump_events`] in its loop.
//!
//! An [`Editor`] holds its own reference to the Audio Unit (through the [`EditorTarget`]),
//! so a part swapped out while its window is open stays alive until the window closes.

use anyhow::{Result, anyhow};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEventMask, NSView,
    NSViewController, NSWindow, NSWindowStyleMask,
};
use objc2_audio_toolbox::{kAudioUnitProperty_CocoaUI, kAudioUnitProperty_RequestViewController};
use objc2_core_audio_kit::AUGenericView;
use objc2_foundation::{NSBundle, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSRunLoop, NSSize, NSString, NSURL};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::instance::EditorTarget;
use super::sys::AudioUnit;

/// Which kind of view the editor shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorKind {
    /// The plugin's own AUv2 Cocoa view.
    Cocoa,
    /// The plugin's view controller (AUv3, or out of process).
    ViewController,
    /// CoreAudioKit's generic parameter view (the plugin has no editor of its own).
    Generic,
}

/// An open editor window. Not `Send`: it lives on the main thread. Dropping it closes the
/// window.
pub struct Editor {
    window: Retained<NSWindow>,
    _controller: Option<Retained<NSViewController>>,
    _target: EditorTarget,
    kind: EditorKind,
    size: (f64, f64),
}

impl Editor {
    pub fn kind(&self) -> EditorKind {
        self.kind
    }
    /// The view's size in points.
    pub fn size(&self) -> (f64, f64) {
        self.size
    }
    /// False once the user has closed the window.
    pub fn is_open(&self) -> bool {
        self.window.isVisible()
    }
    /// Bring the window to the front.
    pub fn focus(&self) {
        self.window.makeKeyAndOrderFront(None);
    }
    /// Close the window (same as dropping the editor).
    pub fn close(self) {}
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.window.close();
    }
}

/// Open `target`'s editor in a new window. Main thread only.
pub fn open_editor(mtm: MainThreadMarker, target: &EditorTarget) -> Result<Editor> {
    super::sys::guard("opening the editor", || open_editor_inner(mtm, target))
}

fn open_editor_inner(mtm: MainThreadMarker, target: &EditorTarget) -> Result<Editor> {
    let unit = target.unit.raw();
    let (view, controller, kind) = if !target.out_of_process
        && let Some(v) = cocoa_view(mtm, unit)
    {
        (v, None, EditorKind::Cocoa)
    } else if let Some(vc) = request_view_controller(mtm, target, Duration::from_secs(2)) {
        (vc.view(), Some(vc), EditorKind::ViewController)
    } else {
        // SAFETY: `unit` is alive (the target holds it) and initialised.
        let g = unsafe { AUGenericView::initWithAudioUnit(AUGenericView::alloc(mtm), unit) };
        (Retained::into_super(g), None, EditorKind::Generic)
    };

    let frame = view.frame();
    let size = NSSize::new(frame.size.width.max(320.0), frame.size.height.max(120.0));
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable | NSWindowStyleMask::Resizable;
    // SAFETY: plain AppKit window creation on the main thread.
    let window = unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm), rect, style, NSBackingStoreType::Buffered, false) };
    // SAFETY: we keep our own strong reference, so AppKit must not release it on close.
    unsafe { window.setReleasedWhenClosed(false) };
    let title = if kind == EditorKind::Generic { format!("{} (generic view)", target.title) } else { target.title.clone() };
    window.setTitle(&NSString::from_str(&title));
    match &controller {
        Some(vc) => window.setContentViewController(Some(vc)),
        None => window.setContentView(Some(&view)),
    }
    window.center();
    window.makeKeyAndOrderFront(None);
    Ok(Editor { window, _controller: controller, _target: target.clone(), kind, size: (size.width, size.height) })
}

/// Close an editor (for symmetry with [`open_editor`]; dropping it does the same).
pub fn close_editor(editor: Editor) {
    editor.close();
}

/// The plugin's own AUv2 Cocoa view, if it publishes one.
fn cocoa_view(mtm: MainThreadMarker, unit: AudioUnit) -> Option<Retained<NSView>> {
    let _ = mtm;
    #[repr(C)]
    struct CocoaViewInfo {
        bundle: *mut NSURL,
        class: [*mut NSString; 1],
    }
    let target = super::sys::UnitRef(unit);
    let size = target.property_size(kAudioUnitProperty_CocoaUI)?;
    if (size as usize) < size_of::<CocoaViewInfo>() {
        return None;
    }
    let mut info = CocoaViewInfo { bundle: std::ptr::null_mut(), class: [std::ptr::null_mut()] };
    // SAFETY: the property is declared as AudioUnitCocoaViewInfo (a CFURL and CFStrings,
    // toll-free bridged to NSURL / NSString); we read only the first class name.
    if unsafe { target.get_raw(kAudioUnitProperty_CocoaUI, &mut info as *mut _ as *mut c_void, size_of::<CocoaViewInfo>() as u32) } != 0 {
        return None;
    }
    // The Copy rule: both references are +1.
    let url = unsafe { Retained::from_raw(info.bundle) }?;
    let class_name = unsafe { Retained::from_raw(info.class[0]) }?;
    let bundle = NSBundle::bundleWithURL(&url)?;
    let class = bundle.classNamed(&class_name)?;
    // SAFETY: `+new` on the plugin's factory class (it conforms to AUCocoaUIBase).
    let factory: Retained<AnyObject> = unsafe { objc2::msg_send![class, new] };
    // `uiViewForAudioUnit:withSize:` takes an AudioUnit, which objc2's encoding check would
    // reject under its SDK name, so it is sent through a typed objc_msgSend.
    type UiView = unsafe extern "C-unwind" fn(*const AnyObject, Sel, AudioUnit, NSSize) -> *mut NSView;
    // SAFETY: objc_msgSend called with the method's exact C signature (arm64 and x86_64
    // both pass these arguments as a normal C call).
    let send: UiView = unsafe { std::mem::transmute(objc2::ffi::objc_msgSend as unsafe extern "C-unwind" fn()) };
    let view = unsafe { send(&*factory, sel!(uiViewForAudioUnit:withSize:), unit, NSSize::new(640.0, 400.0)) };
    // The factory returns an autoreleased view: retain it for ourselves.
    unsafe { Retained::retain(view) }
}

/// Ask the unit for its view controller and wait (spinning the main run loop) until it
/// arrives or `timeout` passes.
fn request_view_controller(mtm: MainThreadMarker, target: &EditorTarget, timeout: Duration) -> Option<Retained<NSViewController>> {
    let _ = mtm;
    let unit = super::sys::UnitRef(target.unit.raw());
    unit.property_size(kAudioUnitProperty_RequestViewController)?;
    // The block may run on any thread; it parks the (retained) pointer for the main thread.
    let slot: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
    let got = Arc::new(Mutex::new(false));
    let (s2, g2) = (slot.clone(), got.clone());
    let block = block2::RcBlock::new(move |vc: *mut NSViewController| {
        if !vc.is_null() {
            // SAFETY: a valid object pointer; take our own +1 so it survives the block.
            let raw = unsafe { objc2::ffi::objc_retain(vc.cast()) };
            *s2.lock().unwrap() = Some(raw as usize);
        }
        *g2.lock().unwrap() = true;
    });
    let ptr: *const block2::DynBlock<dyn Fn(*mut NSViewController)> = &*block;
    // SAFETY: the property's value is the block pointer; AudioToolbox copies the block.
    let st = unsafe { unit.set_raw(kAudioUnitProperty_RequestViewController, &ptr as *const _ as *const c_void, size_of::<*const c_void>() as u32) };
    if st != 0 {
        return None;
    }
    let deadline = Instant::now() + timeout;
    let run_loop = NSRunLoop::currentRunLoop();
    while !*got.lock().unwrap() && Instant::now() < deadline {
        let until = NSDate::dateWithTimeIntervalSinceNow(0.01);
        // SAFETY: running the current (main) run loop once in the default mode.
        unsafe { run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &until) };
    }
    let raw = slot.lock().unwrap().take()?;
    // SAFETY: the +1 reference taken in the block.
    unsafe { Retained::from_raw(raw as *mut NSViewController) }
}

/// For command-line tools with no `NSApplication` of their own: make this process a regular
/// app so its windows can come to the front. The desktop app must not call this.
pub fn prepare_app(mtm: MainThreadMarker) {
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.finishLaunching();
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
}

/// Handle pending window events for up to `max` (command-line tools only; the desktop app
/// has its own event loop).
pub fn pump_events(mtm: MainThreadMarker, max: Duration) {
    let app = NSApplication::sharedApplication(mtm);
    let end = Instant::now() + max;
    loop {
        let until = NSDate::dateWithTimeIntervalSinceNow(end.saturating_duration_since(Instant::now()).as_secs_f64().min(0.02));
        // SAFETY: standard event dequeue on the main thread.
        let ev = unsafe { app.nextEventMatchingMask_untilDate_inMode_dequeue(NSEventMask::Any, Some(&until), NSDefaultRunLoopMode, true) };
        match ev {
            Some(ev) => app.sendEvent(&ev),
            None => {
                if Instant::now() >= end {
                    break;
                }
            }
        }
        app.updateWindows();
        if Instant::now() >= end {
            break;
        }
    }
}

/// Run the main thread's run loop for up to `max` (just sleeps off the main thread).
///
/// Some plugins finish setting themselves up with work queued on the main thread (Kontakt
/// does, even when created on a load thread) and block later calls, such as reading their
/// state, until it has run. The desktop app's main thread always runs its loop; a
/// command-line tool must call this while it waits for a load.
pub fn run_main_loop(max: Duration) {
    if MainThreadMarker::new().is_none() {
        std::thread::sleep(max);
        return;
    }
    let until = NSDate::dateWithTimeIntervalSinceNow(max.as_secs_f64());
    // SAFETY: running the current (main) thread's run loop once in the default mode.
    let ran = unsafe { NSRunLoop::currentRunLoop().runMode_beforeDate(NSDefaultRunLoopMode, &until) };
    if !ran {
        // No sources attached yet: the call returns at once. Don't spin.
        std::thread::sleep(max);
    }
}

/// The main-thread marker, or an error off the main thread.
pub fn main_thread() -> Result<MainThreadMarker> {
    MainThreadMarker::new().ok_or_else(|| anyhow!("plugin editors must be opened on the main thread"))
}
