//! Audio Unit instruments: scan, load, feed MIDI, render, save and restore state.
//!
//! This goes through the AUv2 C API (`AudioComponent*`, `AudioUnitRender`,
//! `MusicDeviceMIDIEvent`). AUv3 instruments show up in the same scan and load through the
//! same calls: AudioToolbox bridges them to the v2 API (out of process unless the extension
//! allows in-process loading). Components flagged "requires async instantiation" cannot be
//! loaded synchronously and are reported as such.

use anyhow::{anyhow, bail, Result};
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef as CfStr};
use std::ffi::c_void;
use std::ptr;

use super::ffi::*;

/// One installed instrument (`aumu`) component.
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    /// "Manufacturer: Name", as AudioToolbox reports it.
    pub name: String,
    pub desc: AudioComponentDescription,
    pub version: u32,
    /// An AUv3 app extension (bridged to the v2 API when loaded here).
    pub v3: bool,
    /// Cannot be created with the synchronous `AudioComponentInstanceNew`.
    pub async_only: bool,
    component: usize,
}

impl ComponentInfo {
    /// "aumu dls  appl", the same triple `auval -a` prints.
    pub fn code(&self) -> String {
        let s = |c: u32| String::from_utf8_lossy(&c.to_be_bytes()).into_owned();
        format!("{} {} {}", s(self.desc.componentType), s(self.desc.componentSubType), s(self.desc.componentManufacturer))
    }
}

fn check(status: OSStatus, what: &str) -> Result<()> {
    if status == 0 { Ok(()) } else { Err(anyhow!("{what} failed: OSStatus {status}")) }
}

/// Every installed instrument Audio Unit (AUv2 and AUv3), sorted by name.
pub fn scan() -> Vec<ComponentInfo> {
    let want = AudioComponentDescription { componentType: kAudioUnitType_MusicDevice, ..Default::default() };
    let mut out = Vec::new();
    let mut comp: AudioComponent = ptr::null_mut();
    loop {
        comp = unsafe { AudioComponentFindNext(comp, &want) };
        if comp.is_null() {
            break;
        }
        let mut desc = AudioComponentDescription::default();
        let mut version = 0u32;
        let mut name_ref: CFStringRef = ptr::null();
        unsafe {
            AudioComponentGetDescription(comp, &mut desc);
            AudioComponentGetVersion(comp, &mut version);
            AudioComponentCopyName(comp, &mut name_ref);
        }
        let name = if name_ref.is_null() {
            String::from("(unnamed)")
        } else {
            unsafe { CFString::wrap_under_create_rule(name_ref as CfStr) }.to_string()
        };
        out.push(ComponentInfo {
            name,
            desc,
            version,
            v3: desc.componentFlags & kAudioComponentFlag_IsV3AudioUnit != 0,
            async_only: desc.componentFlags & kAudioComponentFlag_RequiresAsyncInstantiation != 0,
            component: comp as usize,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// The first instrument whose name contains `query` (case-insensitive), or whose
/// "type subtype manufacturer" code equals it.
pub fn find(query: &str) -> Option<ComponentInfo> {
    let q = query.to_lowercase();
    let all = scan();
    all.iter().find(|c| c.code().to_lowercase() == q)
        .or_else(|| all.iter().find(|c| c.name.to_lowercase().contains(&q)))
        .cloned()
}

/// A loaded, initialized instrument rendering non-interleaved stereo f32.
///
/// Real-time contract: [`Instrument::midi`] and [`Instrument::render`] do not allocate or
/// lock on the host side, so they may be called from the audio callback. What the plugin
/// does inside its own render is the plugin's business (well-behaved ones are RT-safe).
pub struct Instrument {
    unit: AudioUnit,
    pub name: String,
    pub sample_rate: f64,
    pub max_frames: u32,
    sample_time: f64,
}

// The AudioUnit handle may be used from any one thread at a time; AU also allows its view
// (main thread) and render (audio thread) to run concurrently, as every DAW does.
unsafe impl Send for Instrument {}

impl Instrument {
    /// Create, configure (stereo f32 at `sample_rate`, up to `max_frames` per render) and
    /// initialize the component.
    pub fn load(info: &ComponentInfo, sample_rate: f64, max_frames: u32) -> Result<Instrument> {
        if info.async_only {
            bail!("{} requires asynchronous instantiation (AUv3 out-of-process only); not supported by the prototype", info.name);
        }
        let mut unit: AudioUnit = ptr::null_mut();
        check(unsafe { AudioComponentInstanceNew(info.component as AudioComponent, &mut unit) }, "AudioComponentInstanceNew")?;
        let inst = Instrument { unit, name: info.name.clone(), sample_rate, max_frames, sample_time: 0.0 };
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
            check(inst.set(kAudioUnitProperty_StreamFormat, kAudioUnitScope_Output, &fmt), "set StreamFormat")?;
            // Not every instrument exposes a writable global sample rate; the output
            // stream format above is the one that matters.
            let _ = inst.set(kAudioUnitProperty_SampleRate, kAudioUnitScope_Output, &sample_rate);
            check(inst.set(kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitScope_Global, &max_frames), "set MaximumFramesPerSlice")?;
            check(AudioUnitInitialize(unit), "AudioUnitInitialize")?;
        }
        Ok(inst)
    }

    unsafe fn set<T>(&self, id: u32, scope: u32, v: &T) -> OSStatus {
        unsafe { AudioUnitSetProperty(self.unit, id, scope, 0, v as *const T as *const c_void, std::mem::size_of::<T>() as u32) }
    }

    /// The raw handle, for the plugin window.
    pub fn raw(&self) -> AudioUnit {
        self.unit
    }

    /// Queue one short MIDI message, `offset` frames into the next render. RT-safe.
    #[inline]
    pub fn midi(&mut self, m: [u8; 3], offset: u32) {
        unsafe { MusicDeviceMIDIEvent(self.unit, m[0] as u32, m[1] as u32, m[2] as u32, offset) };
    }

    /// Render `left.len()` frames (in `max_frames` slices) into the two buffers. RT-safe on
    /// the host side: the buffer list lives on the stack and points at the caller's slices.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), OSStatus> {
        let frames = left.len().min(right.len());
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(self.max_frames as usize);
            let bytes = (n * 4) as u32;
            let mut abl = AudioBufferList2 {
                mNumberBuffers: 2,
                mBuffers: [
                    AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: left[done..].as_mut_ptr() as *mut c_void },
                    AudioBuffer { mNumberChannels: 1, mDataByteSize: bytes, mData: right[done..].as_mut_ptr() as *mut c_void },
                ],
            };
            let ts = AudioTimeStamp { mSampleTime: self.sample_time, mFlags: kAudioTimeStampSampleTimeValid, ..Default::default() };
            let mut flags = 0u32;
            let st = unsafe { AudioUnitRender(self.unit, &mut flags, &ts, 0, n as u32, &mut abl) };
            if st != 0 {
                left[done..frames].fill(0.0);
                right[done..frames].fill(0.0);
                return Err(st);
            }
            // A plugin may legally redirect mData to its own buffers; copy back if so.
            unsafe {
                let l = abl.mBuffers[0].mData as *const f32;
                if l != left[done..].as_ptr() {
                    ptr::copy_nonoverlapping(l, left[done..].as_mut_ptr(), n);
                }
                let r = abl.mBuffers[1].mData as *const f32;
                if r != right[done..].as_ptr() {
                    ptr::copy_nonoverlapping(r, right[done..].as_mut_ptr(), n);
                }
            }
            self.sample_time += n as f64;
            done += n;
        }
        Ok(())
    }

    /// The plugin's reported processing latency (lookahead), in seconds.
    pub fn latency_seconds(&self) -> f64 {
        let mut v = 0f64;
        let mut size = std::mem::size_of::<f64>() as u32;
        let st = unsafe {
            AudioUnitGetProperty(self.unit, kAudioUnitProperty_Latency, kAudioUnitScope_Global, 0, &mut v as *mut f64 as *mut c_void, &mut size)
        };
        if st == 0 { v } else { 0.0 }
    }

    /// The full plugin state (`kAudioUnitProperty_ClassInfo`) as a binary plist: what a
    /// Registration Memory slot would store for this part.
    pub fn save_state(&self) -> Result<Vec<u8>> {
        let mut plist: CFTypeRef = ptr::null();
        let mut size = std::mem::size_of::<CFTypeRef>() as u32;
        check(
            unsafe { AudioUnitGetProperty(self.unit, kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global, 0, &mut plist as *mut CFTypeRef as *mut c_void, &mut size) },
            "get ClassInfo",
        )?;
        if plist.is_null() {
            bail!("plugin returned no state");
        }
        unsafe {
            let data = CFPropertyListCreateData(ptr::null(), plist, kCFPropertyListBinaryFormat_v1_0, 0, ptr::null_mut());
            CFRelease(plist);
            if data.is_null() {
                bail!("could not serialize plugin state");
            }
            let bytes = std::slice::from_raw_parts(CFDataGetBytePtr(data), CFDataGetLength(data) as usize).to_vec();
            CFRelease(data);
            Ok(bytes)
        }
    }

    /// Restore a state saved by [`Instrument::save_state`]. Not RT-safe: call it off the
    /// audio thread (with the part muted), as a Registration recall would.
    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        unsafe {
            let data = CFDataCreate(ptr::null(), bytes.as_ptr(), bytes.len() as CFIndex);
            let plist = CFPropertyListCreateWithData(ptr::null(), data, 0, ptr::null_mut(), ptr::null_mut());
            CFRelease(data);
            if plist.is_null() {
                bail!("not a property list");
            }
            let st = self.set(kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global, &plist);
            CFRelease(plist);
            check(st, "set ClassInfo")
        }
    }
}

impl Drop for Instrument {
    fn drop(&mut self) {
        unsafe {
            AudioUnitUninitialize(self.unit);
            AudioComponentInstanceDispose(self.unit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dls() -> ComponentInfo {
        find("aumu dls  appl").expect("Apple DLSMusicDevice is always installed")
    }

    #[test]
    fn scan_finds_apples_dls_synth() {
        let all = scan();
        assert!(all.iter().any(|c| c.code() == "aumu dls  appl"), "{:?}", all.iter().map(|c| c.code()).collect::<Vec<_>>());
        assert!(find("dlsmusicdevice").is_some());
    }

    fn energy(inst: &mut Instrument, frames: usize) -> f64 {
        let (mut l, mut r) = (vec![0f32; frames], vec![0f32; frames]);
        inst.render(&mut l, &mut r).unwrap();
        l.iter().chain(&r).map(|x| (*x as f64).powi(2)).sum()
    }

    #[test]
    fn dls_renders_a_note_offline() {
        let mut inst = Instrument::load(&dls(), 48_000.0, 512).unwrap();
        assert!(energy(&mut inst, 4800) < 1e-9, "silent before any note");
        inst.midi([0x90, 60, 110], 0);
        // 3000 frames is not a multiple of max_frames: exercises the slicing.
        assert!(energy(&mut inst, 3000) > 1e-3, "note-on makes sound");
        inst.midi([0x80, 60, 0], 0);
    }

    #[test]
    fn state_round_trips() {
        let mut inst = Instrument::load(&dls(), 48_000.0, 512).unwrap();
        let state = inst.save_state().unwrap();
        assert!(state.starts_with(b"bplist00"));
        inst.restore_state(&state).unwrap();
    }
}
