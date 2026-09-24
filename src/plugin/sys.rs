//! The thin unsafe core: every call into AudioToolbox / CoreFoundation goes through here.
//!
//! The declarations come from the objc2 project's generated bindings
//! (`objc2-audio-toolbox`, `objc2-core-foundation`, `objc2-core-audio-types`). What this file
//! adds is ownership and the few invariants the C API leaves to the caller:
//!
//! - [`Unit`] owns one `AudioComponentInstance` and disposes of it exactly once, on drop.
//!   It is shared as `Arc<Unit>` between the playing [`super::PluginInstance`] and an open
//!   editor window, so neither can outlive the other's handle.
//! - [`Unit::render`] builds its `AudioBufferList` on the stack over the caller's slices:
//!   no allocation, no lock, no Objective-C messaging. [`Unit::midi`] is one C call.
//! - State goes through `kAudioUnitProperty_ClassInfo` as a binary property list, so a
//!   preset is plain bytes.
//! - Instantiation comes in two flavours: [`instantiate_sync`] (`AudioComponentInstanceNew`,
//!   AUv2 in-process) and [`instantiate_async`] (`AudioComponentInstantiate`, required for
//!   AUv3 and the only way to load a unit out of process).
//!
//! Nothing outside this file writes `unsafe` against the Apple APIs except the editor
//! (`editor.rs`), which is AppKit and main-thread only.

#![allow(non_upper_case_globals)]

use anyhow::{Result, anyhow, bail};
use block2::RcBlock;
use objc2_audio_toolbox::{
    AudioComponent, AudioComponentCopyName, AudioComponentDescription, AudioComponentFindNext,
    AudioComponentFlags, AudioComponentGetDescription, AudioComponentGetVersion,
    AudioComponentInstance, AudioComponentInstanceDispose, AudioComponentInstanceNew,
    AudioComponentInstantiate, AudioComponentInstantiationOptions, AudioUnitGetProperty,
    AudioUnitGetPropertyInfo, AudioUnitInitialize, AudioUnitRender, AudioUnitRenderActionFlags,
    AudioUnitSetProperty, AudioUnitUninitialize, AURenderCallbackStruct, MusicDeviceMIDIEvent,
    kAudioUnitProperty_ClassInfo, kAudioUnitProperty_ElementCount, kAudioUnitProperty_Latency,
    kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitProperty_SampleRate,
    kAudioUnitProperty_SetRenderCallback, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Global,
    kAudioUnitScope_Input, kAudioUnitScope_Output, kAudioUnitType_MusicDevice,
};
use objc2_core_audio_types::{
    AudioBuffer, AudioBufferList, AudioStreamBasicDescription, AudioTimeStamp, AudioTimeStampFlags,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsNonInterleaved, kAudioFormatFlagIsPacked,
    kAudioFormatLinearPCM,
};
use objc2_core_foundation::{
    CFData, CFPropertyList, CFPropertyListCreateData, CFPropertyListCreateWithData,
    CFPropertyListFormat, CFRetained, CFString,
};
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::mpsc;

pub use objc2_audio_toolbox::AudioUnit;
pub type OSStatus = i32;

/// `AudioBufferList` with room for two (non-interleaved stereo) buffers: the C struct with
/// its flexible `mBuffers[1]` array extended by one. The generated binding declares the
/// one-buffer form, so the render call casts this to it.
#[repr(C)]
struct AudioBufferList2 {
    number_buffers: u32,
    buffers: [AudioBuffer; 2],
}

/// Four-character code as a string, e.g. `0x61756d75` -> "aumu".
pub fn fourcc_str(c: u32) -> String {
    c.to_be_bytes().iter().map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' }).collect()
}

/// Parse a four-character code; shorter strings are padded with spaces ("dls" -> "dls ").
pub fn fourcc_parse(s: &str) -> Option<u32> {
    let b = s.as_bytes();
    if b.is_empty() || b.len() > 4 {
        return None;
    }
    let mut v = [b' '; 4];
    v[..b.len()].copy_from_slice(b);
    Some(u32::from_be_bytes(v))
}

fn check(status: OSStatus, what: &str) -> Result<()> {
    if status == 0 { Ok(()) } else { Err(anyhow!("{what} failed: OSStatus {status} ({})", status_name(status))) }
}

/// A readable name for the OSStatus values plugins actually return.
pub fn status_name(st: OSStatus) -> &'static str {
    match st {
        0 => "noErr",
        -50 => "paramErr",
        -10879 => "InvalidProperty",
        -10878 => "InvalidParameter",
        -10877 => "InvalidElement",
        -10876 => "NoConnection",
        -10875 => "FailedInitialization",
        -10874 => "TooManyFramesToProcess",
        -10868 => "FormatNotSupported",
        -10867 => "Uninitialized",
        -10866 => "InvalidScope",
        -10865 => "PropertyNotWritable",
        -10863 => "CannotDoInCurrentContext",
        -10851 => "InvalidPropertyValue",
        -10849 => "Initialized",
        -10847 => "Unauthorized",
        -66745 => "RenderTimeout",
        -66744 => "ExtensionNotFound",
        -66746 => "InvalidFormat",
        -66747 => "InitializationTimedOut",
        -66748 => "NotPermitted",
        -66749 => "InstanceInvalidated (the plugin process died)",
        -66750 => "TooManyInstances",
        -66751 => "UnsupportedType",
        -66754 => "InstanceTimedOut",
        _ => "",
    }
}

/// Run a non-real-time plugin call (load, state, editor) inside an Objective-C `@try`, so a
/// plugin that throws an `NSException` fails that call instead of aborting yahaha. C++
/// exceptions and crashes are not caught (out-of-process loading is the answer to those).
/// Not used on the render path.
pub fn guard<R>(what: &str, f: impl FnOnce() -> Result<R>) -> Result<R> {
    match objc2::exception::catch(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(e) => Err(anyhow!("{what}: the plugin threw an Objective-C exception{}", e.map(|e| format!(": {e:?}")).unwrap_or_default())),
    }
}

/// One installed component, as the registrar reports it. Cheap: no instance is created.
#[derive(Clone, Copy, Debug)]
pub struct Component {
    raw: usize,
    pub desc: [u32; 3],
    pub flags: u32,
    pub version: u32,
}

// The registrar's component handles are process-global and immutable.
unsafe impl Send for Component {}
unsafe impl Sync for Component {}

pub const FLAG_SANDBOX_SAFE: u32 = AudioComponentFlags::SandboxSafe.0;
pub const FLAG_IS_V3: u32 = AudioComponentFlags::IsV3AudioUnit.0;
pub const FLAG_REQUIRES_ASYNC: u32 = AudioComponentFlags::RequiresAsyncInstantiation.0;
pub const FLAG_CAN_LOAD_IN_PROCESS: u32 = AudioComponentFlags::CanLoadInProcess.0;

impl Component {
    fn raw(&self) -> AudioComponent {
        self.raw as AudioComponent
    }

    /// "Manufacturer: Name", as AudioToolbox reports it.
    pub fn name(&self) -> String {
        let mut name: *const CFString = ptr::null();
        let st = unsafe { AudioComponentCopyName(self.raw(), NonNull::from(&mut name)) };
        match NonNull::new(name as *mut CFString) {
            Some(n) if st == 0 => unsafe { CFRetained::from_raw(n) }.to_string(),
            _ => String::from("(unnamed)"),
        }
    }
}

/// Every installed instrument (`aumu`) component, AUv2 and AUv3, in registrar order.
pub fn instruments() -> Vec<Component> {
    let want = AudioComponentDescription {
        componentType: kAudioUnitType_MusicDevice,
        componentSubType: 0,
        componentManufacturer: 0,
        componentFlags: 0,
        componentFlagsMask: 0,
    };
    let mut out = Vec::new();
    let mut comp: AudioComponent = ptr::null_mut();
    loop {
        comp = unsafe { AudioComponentFindNext(comp, NonNull::from(&want)) };
        if comp.is_null() {
            break;
        }
        let mut d = want;
        let mut version = 0u32;
        unsafe {
            AudioComponentGetDescription(comp, NonNull::from(&mut d));
            AudioComponentGetVersion(comp, NonNull::from(&mut version));
        }
        out.push(Component {
            raw: comp as usize,
            desc: [d.componentType, d.componentSubType, d.componentManufacturer],
            flags: d.componentFlags,
            version,
        });
    }
    out
}

/// One Audio Unit instance. Disposed (uninitialized first, if needed) on drop, on whatever
/// thread drops the last `Arc<Unit>`: never the audio thread (the rack hands retired
/// instances back to the control side for that).
pub struct Unit {
    raw: AudioUnit,
    initialized: bool,
    lifecycle: Arc<Mutex<()>>,
}

/// One lock per component (type, subtype, manufacturer), held around
/// `AudioUnitInitialize`, `AudioUnitUninitialize` and `AudioComponentInstanceDispose`.
///
/// Instances of the same component can share process-wide state behind those calls:
/// DLSMusicDevice asserts inside CoreAudio (`CAAssertRtn` under `AudioUnitInitialize`) or
/// crashes in `AudioUnitUninitialize` when two of its instances go through them on two
/// threads at once, which `load_async` (a thread per load) and a retired instance dropping
/// on the control side make possible. Per component rather than global, so one plugin that
/// hangs in `AudioUnitInitialize` (the load deadline's case) holds up only its own kind.
type LifecycleLocks = Mutex<HashMap<[u32; 3], Arc<Mutex<()>>>>;

fn lifecycle_lock(desc: [u32; 3]) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<LifecycleLocks> = OnceLock::new();
    let mut m = LOCKS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    m.entry(desc).or_default().clone()
}

impl Unit {
    fn lifecycle(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lifecycle.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// AudioToolbox allows one thread at a time per call family; the host serialises: render and
// MIDI on the audio thread, properties off it while the unit is not playing (state) or with
// the unit's own locking (views), as every DAW does.
unsafe impl Send for Unit {}
unsafe impl Sync for Unit {}

impl Drop for Unit {
    fn drop(&mut self) {
        let lifecycle = self.lifecycle.clone();
        let _g = lifecycle.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            if self.initialized {
                AudioUnitUninitialize(self.raw);
            }
            AudioComponentInstanceDispose(self.raw);
        }
    }
}

/// Create an instance synchronously (`AudioComponentInstanceNew`). AUv2 only, in process.
pub fn instantiate_sync(c: &Component) -> Result<Unit> {
    let mut raw: AudioComponentInstance = ptr::null_mut();
    check(unsafe { AudioComponentInstanceNew(c.raw(), NonNull::from(&mut raw)) }, "AudioComponentInstanceNew")?;
    if raw.is_null() {
        bail!("AudioComponentInstanceNew returned no instance");
    }
    Ok(Unit { raw, initialized: false, lifecycle: lifecycle_lock(c.desc) })
}

/// Create an instance asynchronously (`AudioComponentInstantiate`). `out_of_process` asks for
/// the AUHostingService (AUv3 extensions, and AUv2 units on macOS 11+); otherwise in-process.
///
/// Returns a receiver that gets the result on an arbitrary thread. If the receiver is gone
/// by then (the load timed out), the completion disposes of the instance itself.
pub fn instantiate_async(c: &Component, out_of_process: bool) -> mpsc::Receiver<Result<Unit>> {
    let (tx, rx) = mpsc::sync_channel::<Result<Unit>>(1);
    // The block is `Fn`; AudioToolbox calls it once. The Mutex<Option<_>> makes that a take.
    let tx = Mutex::new(Some(tx));
    let lifecycle = lifecycle_lock(c.desc);
    let block = RcBlock::new(move |inst: AudioComponentInstance, st: OSStatus| {
        let r = if st == 0 && !inst.is_null() {
            Ok(Unit { raw: inst, initialized: false, lifecycle: lifecycle.clone() })
        } else {
            if !inst.is_null() {
                let _g = lifecycle.lock().unwrap_or_else(|e| e.into_inner());
                unsafe { AudioComponentInstanceDispose(inst) };
            }
            Err(anyhow!("AudioComponentInstantiate failed: OSStatus {st} ({})", status_name(st)))
        };
        if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
            // A send error hands the Unit back inside the error, where it drops: disposed.
            let _ = tx.try_send(r);
        }
    });
    let opts = if out_of_process {
        AudioComponentInstantiationOptions::LoadOutOfProcess
    } else {
        AudioComponentInstantiationOptions::LoadInProcess
    };
    unsafe { AudioComponentInstantiate(c.raw(), opts, &block) };
    rx
}

/// A plain-data property value (numbers, structs of numbers).
///
/// # Safety
/// `T` must be the C type the property `id` is declared with.
unsafe fn set_prop<T>(raw: AudioUnit, id: u32, scope: u32, v: &T) -> OSStatus {
    unsafe { set_prop_el(raw, id, scope, 0, v) }
}

/// # Safety
/// `T` must be the C type the property `id` is declared with.
unsafe fn set_prop_el<T>(raw: AudioUnit, id: u32, scope: u32, element: u32, v: &T) -> OSStatus {
    unsafe { AudioUnitSetProperty(raw, id, scope, element, v as *const T as *const c_void, size_of::<T>() as u32) }
}

/// The input callback for instruments that also have audio inputs (FM8, vocoders, anything
/// with a sidechain): silence. Without one, a unit hosted out of process fails every render
/// with `NoConnection`. RT-safe: it only zeroes the buffers it is given.
///
/// The buffer list is a C struct with a flexible array (`mBuffers[1]` declared, `n`
/// present), so the buffers are reached by raw pointer arithmetic from the list pointer
/// itself: a Rust slice or reference made from the one-element field would not cover the
/// others.
unsafe extern "C-unwind" fn silent_input(
    _ref: *mut c_void,
    flags: *mut AudioUnitRenderActionFlags,
    _ts: *const AudioTimeStamp,
    _bus: u32,
    _frames: u32,
    data: *mut AudioBufferList,
) -> OSStatus {
    unsafe {
        if !data.is_null() {
            let n = ptr::addr_of!((*data).mNumberBuffers).read() as usize;
            let first = ptr::addr_of_mut!((*data).mBuffers) as *mut AudioBuffer;
            for i in 0..n {
                let b = first.add(i);
                let d = ptr::addr_of!((*b).mData).read();
                if !d.is_null() {
                    ptr::write_bytes(d as *mut u8, 0, ptr::addr_of!((*b).mDataByteSize).read() as usize);
                }
            }
        }
        if !flags.is_null() {
            (*flags).0 |= 1 << 4; // kAudioUnitRenderAction_OutputIsSilence
        }
    }
    0
}

/// `silent_input` as the binding's callback type, whose pointer arguments are `NonNull`.
/// The callback takes raw pointers instead, so a host that passes null (for the refcon, or
/// anything else) is not undefined behaviour; the ABI is the same.
fn silent_input_proc() -> objc2_audio_toolbox::AURenderCallback {
    type Raw = unsafe extern "C-unwind" fn(*mut c_void, *mut AudioUnitRenderActionFlags, *const AudioTimeStamp, u32, u32, *mut AudioBufferList) -> OSStatus;
    let f: Raw = silent_input;
    // SAFETY: same ABI; `NonNull<T>` is guaranteed to have the layout of `*mut T`.
    Some(unsafe { std::mem::transmute::<Raw, _>(f) })
}

/// # Safety
/// `T` must be the C type the property `id` is declared with.
unsafe fn get_prop<T: Copy>(raw: AudioUnit, id: u32, scope: u32, v: &mut T) -> OSStatus {
    let mut size = size_of::<T>() as u32;
    unsafe { AudioUnitGetProperty(raw, id, scope, 0, NonNull::from(v).cast(), NonNull::from(&mut size)) }
}

impl Unit {
    pub fn raw(&self) -> AudioUnit {
        self.raw
    }

    /// Stereo non-interleaved f32 at `sample_rate`, up to `max_frames` per render call, then
    /// `AudioUnitInitialize`. Not RT-safe. Call before the unit plays (or after
    /// [`Unit::uninitialize`] to change the rate).
    pub fn configure_and_initialize(&mut self, sample_rate: f64, max_frames: u32) -> Result<()> {
        let fmt = AudioStreamBasicDescription {
            mSampleRate: sample_rate,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked | kAudioFormatFlagIsNonInterleaved,
            mBytesPerPacket: 4,
            mFramesPerPacket: 1,
            mBytesPerFrame: 4,
            mChannelsPerFrame: 2,
            mBitsPerChannel: 32,
            mReserved: 0,
        };
        unsafe {
            check(set_prop(self.raw, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Output, &fmt), "set StreamFormat")?;
            // Audio inputs, if the instrument has any, get silence (see `silent_input`).
            let mut inputs = 0u32;
            if get_prop(self.raw, kAudioUnitProperty_ElementCount, kAudioUnitScope_Input, &mut inputs) == 0 {
                let cb = AURenderCallbackStruct { inputProc: silent_input_proc(), inputProcRefCon: ptr::null_mut() };
                for bus in 0..inputs.min(16) {
                    let _ = set_prop_el(self.raw, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Input, bus, &fmt);
                    let _ = set_prop_el(self.raw, kAudioUnitProperty_SetRenderCallback, kAudioUnitScope_Input, bus, &cb);
                }
            }
            // Not every instrument exposes a writable sample rate; the stream format is what counts.
            let _ = set_prop(self.raw, kAudioUnitProperty_SampleRate, kAudioUnitScope_Output, &sample_rate);
            check(set_prop(self.raw, kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitScope_Global, &max_frames), "set MaximumFramesPerSlice")?;
            let st = {
                let _g = self.lifecycle();
                AudioUnitInitialize(self.raw)
            };
            check(st, "AudioUnitInitialize")?;
        }
        self.initialized = true;
        Ok(())
    }

    pub fn uninitialize(&mut self) {
        if self.initialized {
            {
                let _g = self.lifecycle();
                unsafe { AudioUnitUninitialize(self.raw) };
            }
            self.initialized = false;
        }
    }

    /// One short MIDI message, `offset` frames into the next render. RT-safe (one C call).
    #[inline]
    pub fn midi(&self, m: [u8; 3], offset: u32) -> OSStatus {
        unsafe { MusicDeviceMIDIEvent(self.raw, m[0] as u32, m[1] as u32, m[2] as u32, offset) }
    }

    /// Render `n` frames at `sample_time` into `left` / `right` (each at least `n` long, `n`
    /// no more than the configured maximum). RT-safe on the host side: the buffer list is on
    /// the stack over the caller's memory. On error the output is left zeroed.
    #[inline]
    pub fn render(&self, sample_time: f64, left: &mut [f32], right: &mut [f32], n: usize) -> OSStatus {
        debug_assert!(left.len() >= n && right.len() >= n);
        let bytes = (n * 4) as u32;
        let mut abl = AudioBufferList2 {
            number_buffers: 2,
            buffers: [
                AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: left.as_mut_ptr() as *mut c_void },
                AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: right.as_mut_ptr() as *mut c_void },
            ],
        };
        // SAFETY: AudioTimeStamp is plain old data; all-zero is a valid "nothing valid" stamp.
        let mut ts: AudioTimeStamp = unsafe { std::mem::zeroed() };
        ts.mSampleTime = sample_time;
        ts.mFlags = AudioTimeStampFlags::SampleTimeValid;
        let mut flags = AudioUnitRenderActionFlags(0);
        let st = unsafe {
            AudioUnitRender(self.raw, &mut flags, NonNull::from(&ts), 0, n as u32, NonNull::from(&mut abl).cast())
        };
        if st != 0 {
            left[..n].fill(0.0);
            right[..n].fill(0.0);
            return st;
        }
        // A unit may legally point mData at its own buffers; copy back if it did.
        unsafe {
            let l = abl.buffers[0].mData as *const f32;
            if !l.is_null() && l != left.as_ptr() {
                ptr::copy_nonoverlapping(l, left.as_mut_ptr(), n);
            }
            let r = abl.buffers[1].mData as *const f32;
            if !r.is_null() && r != right.as_ptr() {
                ptr::copy_nonoverlapping(r, right.as_mut_ptr(), n);
            }
        }
        0
    }

    /// The unit's reported processing latency (lookahead), in seconds.
    pub fn latency_seconds(&self) -> f64 {
        let mut v = 0f64;
        if unsafe { get_prop(self.raw, kAudioUnitProperty_Latency, kAudioUnitScope_Global, &mut v) } == 0 { v } else { 0.0 }
    }

    /// The complete plugin state (`kAudioUnitProperty_ClassInfo`) as a binary plist. Not
    /// RT-safe.
    pub fn class_info(&self) -> Result<Vec<u8>> {
        let mut plist: *const CFPropertyList = ptr::null();
        check(unsafe { get_prop(self.raw, kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global, &mut plist) }, "get ClassInfo")?;
        let plist = NonNull::new(plist as *mut CFPropertyList).ok_or_else(|| anyhow!("the plugin returned no state"))?;
        // The Copy rule: the property hands us a +1 reference.
        let plist = unsafe { CFRetained::from_raw(plist) };
        let data = unsafe { CFPropertyListCreateData(None, Some(&plist), CFPropertyListFormat::BinaryFormat_v1_0, 0, ptr::null_mut()) }
            .ok_or_else(|| anyhow!("could not serialise the plugin state"))?;
        Ok(data.to_vec())
    }

    /// Restore a state from [`Unit::class_info`]. Not RT-safe.
    pub fn set_class_info(&self, bytes: &[u8]) -> Result<()> {
        let data = CFData::from_bytes(bytes);
        let plist = unsafe { CFPropertyListCreateWithData(None, Some(&data), 0, ptr::null_mut(), ptr::null_mut()) }
            .ok_or_else(|| anyhow!("the state is not a property list"))?;
        let p: *const CFPropertyList = CFRetained::as_ptr(&plist).as_ptr();
        check(unsafe { set_prop(self.raw, kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global, &p) }, "set ClassInfo")
    }
}

/// A borrowed unit handle for the editor's property calls (the editor keeps the owning
/// `Arc<Unit>` alive for as long as it uses this).
#[derive(Clone, Copy)]
pub struct UnitRef(pub AudioUnit);

impl UnitRef {
    /// The size of a global-scope property, if the unit has it.
    pub fn property_size(&self, id: u32) -> Option<u32> {
        let mut size = 0u32;
        let mut writable = 0u8;
        let st = unsafe { AudioUnitGetPropertyInfo(self.0, id, kAudioUnitScope_Global, 0, &mut size, &mut writable) };
        (st == 0).then_some(size)
    }

    /// # Safety
    /// `data` must point to `size` writable bytes of the property's C type.
    pub unsafe fn get_raw(&self, id: u32, data: *mut c_void, size: u32) -> OSStatus {
        let mut size = size;
        match NonNull::new(data) {
            Some(d) => unsafe { AudioUnitGetProperty(self.0, id, kAudioUnitScope_Global, 0, d, NonNull::from(&mut size)) },
            None => -50,
        }
    }

    /// # Safety
    /// `data` must point to `size` bytes of the property's C type.
    pub unsafe fn set_raw(&self, id: u32, data: *const c_void, size: u32) -> OSStatus {
        unsafe { AudioUnitSetProperty(self.0, id, kAudioUnitScope_Global, 0, data, size) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_and_codes() {
        assert_eq!(size_of::<AudioBufferList2>(), 8 + 2 * 16);
        assert_eq!(size_of::<AudioTimeStamp>(), 64);
        assert_eq!(fourcc_parse("aumu"), Some(kAudioUnitType_MusicDevice));
        assert_eq!(fourcc_str(kAudioUnitType_MusicDevice), "aumu");
        assert_eq!(fourcc_parse("dls"), fourcc_parse("dls "));
        assert_eq!(fourcc_parse("toolong"), None);
    }
}
