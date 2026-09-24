//! Hand-written AudioToolbox / CoreFoundation / Objective-C runtime declarations.
//!
//! Only what the prototype calls. Everything here is a plain C API that has been stable
//! since macOS 10.x, so the spike needs no new crates (see docs/plugin-hosting.md for why a
//! production host would switch to `objc2-audio-toolbox`).

#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use std::ffi::c_void;

pub type OSStatus = i32;
pub type OSType = u32;
pub type AudioComponent = *mut c_void;
pub type AudioUnit = *mut c_void;
pub type CFStringRef = *const c_void;
pub type CFDataRef = *const c_void;
pub type CFTypeRef = *const c_void;
pub type CFURLRef = *const c_void;
pub type CFAllocatorRef = *const c_void;
pub type CFIndex = isize;

/// Four-character code, e.g. `fourcc(b"aumu")`.
pub const fn fourcc(s: &[u8; 4]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | s[3] as u32
}

pub const kAudioUnitType_MusicDevice: OSType = fourcc(b"aumu");
pub const kAudioFormatLinearPCM: u32 = fourcc(b"lpcm");
pub const kAudioFormatFlagIsFloat: u32 = 1 << 0;
pub const kAudioFormatFlagIsPacked: u32 = 1 << 3;
pub const kAudioFormatFlagIsNonInterleaved: u32 = 1 << 5;

pub const kAudioComponentFlag_IsV3AudioUnit: u32 = 1 << 2;
pub const kAudioComponentFlag_RequiresAsyncInstantiation: u32 = 1 << 3;

pub const kAudioUnitScope_Global: u32 = 0;
pub const kAudioUnitScope_Output: u32 = 2;

pub const kAudioUnitProperty_ClassInfo: u32 = 0;
pub const kAudioUnitProperty_SampleRate: u32 = 2;
pub const kAudioUnitProperty_StreamFormat: u32 = 8;
pub const kAudioUnitProperty_Latency: u32 = 12;
pub const kAudioUnitProperty_MaximumFramesPerSlice: u32 = 14;
pub const kAudioUnitProperty_CocoaUI: u32 = 31;

pub const kAudioTimeStampSampleTimeValid: u32 = 1 << 0;

pub const kCFPropertyListBinaryFormat_v1_0: CFIndex = 200;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioComponentDescription {
    pub componentType: OSType,
    pub componentSubType: OSType,
    pub componentManufacturer: OSType,
    pub componentFlags: u32,
    pub componentFlagsMask: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioStreamBasicDescription {
    pub mSampleRate: f64,
    pub mFormatID: u32,
    pub mFormatFlags: u32,
    pub mBytesPerPacket: u32,
    pub mFramesPerPacket: u32,
    pub mBytesPerFrame: u32,
    pub mChannelsPerFrame: u32,
    pub mBitsPerChannel: u32,
    pub mReserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SMPTETime {
    pub mSubframes: i16,
    pub mSubframeDivisor: i16,
    pub mCounter: u32,
    pub mType: u32,
    pub mFlags: u32,
    pub mHours: i16,
    pub mMinutes: i16,
    pub mSeconds: i16,
    pub mFrames: i16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AudioTimeStamp {
    pub mSampleTime: f64,
    pub mHostTime: u64,
    pub mRateScalar: f64,
    pub mWordClockTime: u64,
    pub mSMPTETime: SMPTETime,
    pub mFlags: u32,
    pub mReserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AudioBuffer {
    pub mNumberChannels: u32,
    pub mDataByteSize: u32,
    pub mData: *mut c_void,
}

/// `AudioBufferList` with room for two (non-interleaved stereo) buffers. Same layout as
/// the C struct with its flexible `mBuffers[1]` array extended by one.
#[repr(C)]
pub struct AudioBufferList2 {
    pub mNumberBuffers: u32,
    pub mBuffers: [AudioBuffer; 2],
}

#[repr(C)]
pub struct AudioUnitCocoaViewInfo {
    pub mCocoaAUViewBundleLocation: CFURLRef,
    pub mCocoaAUViewClass: [CFStringRef; 1],
}

#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    pub fn AudioComponentFindNext(inComponent: AudioComponent, inDesc: *const AudioComponentDescription) -> AudioComponent;
    pub fn AudioComponentCount(inDesc: *const AudioComponentDescription) -> u32;
    pub fn AudioComponentCopyName(inComponent: AudioComponent, outName: *mut CFStringRef) -> OSStatus;
    pub fn AudioComponentGetDescription(inComponent: AudioComponent, outDesc: *mut AudioComponentDescription) -> OSStatus;
    pub fn AudioComponentGetVersion(inComponent: AudioComponent, outVersion: *mut u32) -> OSStatus;
    pub fn AudioComponentInstanceNew(inComponent: AudioComponent, outInstance: *mut AudioUnit) -> OSStatus;
    pub fn AudioComponentInstanceDispose(inInstance: AudioUnit) -> OSStatus;
    pub fn AudioUnitInitialize(inUnit: AudioUnit) -> OSStatus;
    pub fn AudioUnitUninitialize(inUnit: AudioUnit) -> OSStatus;
    pub fn AudioUnitReset(inUnit: AudioUnit, inScope: u32, inElement: u32) -> OSStatus;
    pub fn AudioUnitSetProperty(inUnit: AudioUnit, inID: u32, inScope: u32, inElement: u32, inData: *const c_void, inDataSize: u32) -> OSStatus;
    pub fn AudioUnitGetProperty(inUnit: AudioUnit, inID: u32, inScope: u32, inElement: u32, outData: *mut c_void, ioDataSize: *mut u32) -> OSStatus;
    pub fn AudioUnitGetPropertyInfo(inUnit: AudioUnit, inID: u32, inScope: u32, inElement: u32, outDataSize: *mut u32, outWritable: *mut u8) -> OSStatus;
    pub fn AudioUnitRender(inUnit: AudioUnit, ioActionFlags: *mut u32, inTimeStamp: *const AudioTimeStamp, inOutputBusNumber: u32, inNumberFrames: u32, ioData: *mut AudioBufferList2) -> OSStatus;
    pub fn MusicDeviceMIDIEvent(inUnit: AudioUnit, inStatus: u32, inData1: u32, inData2: u32, inOffsetSampleFrame: u32) -> OSStatus;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFPropertyListCreateData(allocator: CFAllocatorRef, propertyList: CFTypeRef, format: CFIndex, options: usize, error: *mut *const c_void) -> CFDataRef;
    pub fn CFPropertyListCreateWithData(allocator: CFAllocatorRef, data: CFDataRef, options: usize, format: *mut CFIndex, error: *mut *const c_void) -> CFTypeRef;
    pub fn CFDataCreate(allocator: CFAllocatorRef, bytes: *const u8, length: CFIndex) -> CFDataRef;
    pub fn CFDataGetLength(data: CFDataRef) -> CFIndex;
    pub fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
}

// Objective-C runtime, for the plugin window (gui.rs).
pub type Id = *mut c_void;
pub type Sel = *const c_void;

#[link(name = "objc")]
unsafe extern "C" {
    pub fn objc_getClass(name: *const std::ffi::c_char) -> Id;
    pub fn sel_registerName(name: *const std::ffi::c_char) -> Sel;
    pub fn objc_msgSend();
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "CoreAudioKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_the_c_headers() {
        assert_eq!(std::mem::size_of::<AudioComponentDescription>(), 20);
        assert_eq!(std::mem::size_of::<AudioStreamBasicDescription>(), 40);
        assert_eq!(std::mem::size_of::<SMPTETime>(), 24);
        assert_eq!(std::mem::size_of::<AudioTimeStamp>(), 64);
        assert_eq!(std::mem::size_of::<AudioBuffer>(), 16);
        assert_eq!(std::mem::size_of::<AudioBufferList2>(), 8 + 2 * 16);
        assert_eq!(fourcc(b"aumu"), 0x61756d75);
    }
}
