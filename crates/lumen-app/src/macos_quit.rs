//! Minimal AppKit termination veto installed on winit's existing delegate.
//!
//! Keep winit's delegate instance and lifecycle methods intact: add only the
//! optional `applicationShouldTerminate:` method to its runtime class. Install
//! and reply on the AppKit main thread, before the native UI is exposed.

use objc2::{ffi, rc::Retained, runtime::Sel, sel};
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use objc2_foundation::MainThreadMarker;
use std::sync::OnceLock;

static ROOT_CONTEXT: OnceLock<egui::Context> = OnceLock::new();

pub fn install(root_context: egui::Context) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        "AppKit termination guard must be installed on the main thread".to_owned()
    })?;
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: AppKit owns the application delegate; the main-thread marker
    // guarantees this access occurs on its required thread.
    let delegate = unsafe { app.delegate() }
        .ok_or_else(|| "winit has not installed its NSApplication delegate".to_owned())?;
    let selector = sel!(applicationShouldTerminate:);
    let delegate_ptr = Retained::as_ptr(&delegate).cast::<ffi::objc_object>();
    // SAFETY: delegate_ptr is a live Objective-C object retained above.
    let class = unsafe { ffi::object_getClass(delegate_ptr) };
    if class.is_null() {
        return Err("winit application delegate has no Objective-C class".to_owned());
    }
    // Refuse to overwrite any existing implementation. The inspected winit
    // 0.30.13 delegate does not implement this optional protocol method.
    // SAFETY: class and selector are valid runtime values.
    if !unsafe { ffi::class_getInstanceMethod(class, selector.as_ptr()) }.is_null() {
        return Err(
            "winit application delegate already implements applicationShouldTerminate:".to_owned(),
        );
    }
    ROOT_CONTEXT
        .set(root_context)
        .map_err(|_| "AppKit termination guard was installed more than once".to_owned())?;

    // `applicationShouldTerminate:` returns NSUInteger on macOS (Q on both
    // supported 64-bit ABIs). Its three arguments are self, _cmd, and sender.
    type NativeTerminate = unsafe extern "C" fn(
        *mut ffi::objc_object,
        Sel,
        *mut ffi::objc_object,
    ) -> NSApplicationTerminateReply;
    let method: NativeTerminate = native_should_terminate;
    // SAFETY: The callback's Objective-C ABI matches `Q@:@` above. The method
    // is added only to winit's existing delegate class, preserving all of its
    // other AppKit lifecycle behavior.
    let imp: ffi::IMP =
        Some(unsafe { std::mem::transmute::<NativeTerminate, unsafe extern "C" fn()>(method) });
    // SAFETY: class is the existing delegate's live class; selector, IMP, and
    // encoding match the optional NSApplicationDelegate method exactly.
    let installed =
        unsafe { ffi::class_addMethod(class.cast_mut(), selector.as_ptr(), imp, c"Q@:@".as_ptr()) };
    if !installed {
        return Err("failed to install AppKit termination guard".to_owned());
    }
    Ok(())
}

pub fn cancel_deferred_termination() -> Result<(), String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "AppKit termination reply must run on the main thread".to_owned())?;
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: This replies to the NSTerminateLater request received by the
    // installed delegate. It does not replace or mutate the winit delegate.
    unsafe { app.replyToApplicationShouldTerminate(false) };
    Ok(())
}

unsafe extern "C" fn native_should_terminate(
    _this: *mut ffi::objc_object,
    _selector: Sel,
    _sender: *mut ffi::objc_object,
) -> NSApplicationTerminateReply {
    crate::quit::record_request();
    if let Some(ctx) = ROOT_CONTEXT.get() {
        ctx.request_repaint();
    }
    // AppKit keeps running while Lumen presents its Save / Discard / Cancel
    // decision. The root cancels this deferred native request and, after
    // approval, closes normally through the event loop.
    NSApplicationTerminateReply::NSTerminateLater
}
