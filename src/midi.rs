//! Thin CoreMIDI wrapper (raw coremidi-sys) so endpoint refs stay plain `u32`s that can
//! move between threads freely.

use anyhow::{bail, Result};
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use coremidi_sys::*;
use std::ffi::c_void;

pub type Endpoint = MIDIEndpointRef;

pub struct Client {
    pub client: MIDIClientRef,
}

fn check(st: OSStatus, what: &str) -> Result<()> {
    if st != 0 {
        bail!("{what} failed (OSStatus {st})");
    }
    Ok(())
}

pub fn name_of(obj: MIDIObjectRef) -> String {
    unsafe {
        let mut s: CFStringRef = std::ptr::null();
        if MIDIObjectGetStringProperty(obj, kMIDIPropertyDisplayName, &mut s) != 0 || s.is_null() {
            return String::from("?");
        }
        CFString::wrap_under_create_rule(s).to_string()
    }
}

pub fn sources() -> Vec<(Endpoint, String)> {
    unsafe { (0..MIDIGetNumberOfSources()).map(|i| MIDIGetSource(i)).map(|e| (e, name_of(e))).collect() }
}

pub fn destinations() -> Vec<(Endpoint, String)> {
    unsafe { (0..MIDIGetNumberOfDestinations()).map(|i| MIDIGetDestination(i)).map(|e| (e, name_of(e))).collect() }
}

impl Client {
    pub fn new(name: &str) -> Result<Client> {
        let mut client = 0;
        let n = CFString::new(name);
        check(
            unsafe { MIDIClientCreate(n.as_concrete_TypeRef(), None, std::ptr::null_mut(), &mut client) },
            "MIDIClientCreate",
        )?;
        Ok(Client { client })
    }

    /// Close the client: its ports and virtual endpoints go away, and its input
    /// handlers are not called again.
    pub fn dispose(&self) {
        unsafe {
            MIDIClientDispose(self.client);
        }
    }

    pub fn virtual_source(&self, name: &str) -> Result<Endpoint> {
        let mut ep = 0;
        let n = CFString::new(name);
        check(unsafe { MIDISourceCreate(self.client, n.as_concrete_TypeRef(), &mut ep) }, "MIDISourceCreate")?;
        Ok(ep)
    }

    pub fn output_port(&self, name: &str) -> Result<MIDIPortRef> {
        let mut p = 0;
        let n = CFString::new(name);
        check(unsafe { MIDIOutputPortCreate(self.client, n.as_concrete_TypeRef(), &mut p) }, "MIDIOutputPortCreate")?;
        Ok(p)
    }

    /// Create an input port whose callback runs on CoreMIDI's receive thread.
    /// `handler` is leaked for the life of the process (ports live that long here).
    pub fn input_port<H: InputHandler + 'static>(&self, name: &str, handler: H) -> Result<InputPort> {
        let boxed: Box<Box<dyn InputHandler>> = Box::new(Box::new(handler));
        let ctx = Box::into_raw(boxed) as *mut c_void;
        let mut p = 0;
        let n = CFString::new(name);
        #[allow(deprecated)]
        check(
            unsafe { MIDIInputPortCreate(self.client, n.as_concrete_TypeRef(), Some(read_proc), ctx, &mut p) },
            "MIDIInputPortCreate",
        )?;
        Ok(InputPort { port: p })
    }
}

/// An input port: a plain reference, valid until the client is disposed.
#[derive(Clone, Copy, Debug)]
pub struct InputPort {
    port: MIDIPortRef,
}

impl InputPort {
    /// Connect a source; `tag` is handed to the handler with every packet from it.
    pub fn connect(&self, src: Endpoint, tag: usize) -> Result<()> {
        check(unsafe { MIDIPortConnectSource(self.port, src, tag as *mut c_void) }, "MIDIPortConnectSource")
    }

    /// Stop listening to a source.
    pub fn disconnect(&self, src: Endpoint) -> Result<()> {
        check(unsafe { MIDIPortDisconnectSource(self.port, src) }, "MIDIPortDisconnectSource")
    }
}

pub trait InputHandler: Send {
    /// Called on CoreMIDI's receive thread. `host_time` is the packet's host timestamp.
    fn packet(&mut self, tag: usize, host_time: u64, data: &[u8]);
    /// Called once after all packets in a list.
    fn end_of_list(&mut self) {}
}

unsafe extern "C" fn read_proc(list: *const MIDIPacketList, ctx: *mut c_void, src: *mut c_void) {
    // SAFETY: `ctx` is the leaked Box<Box<dyn InputHandler>> from `input_port`; CoreMIDI
    // calls this serially on its receive thread, and `list` is valid for the call.
    unsafe {
        let handler = &mut **(ctx as *mut Box<dyn InputHandler>);
        let n = (*list).numPackets;
        let mut p = std::ptr::addr_of!((*list).packet) as *const MIDIPacket;
        for _ in 0..n {
            let len = std::ptr::addr_of!((*p).length).read_unaligned() as usize;
            let ts = std::ptr::addr_of!((*p).timeStamp).read_unaligned();
            let data = std::slice::from_raw_parts(std::ptr::addr_of!((*p).data) as *const u8, len);
            handler.packet(src as usize, ts, data);
            p = MIDIPacketNext(p);
        }
        handler.end_of_list();
    }
}

/// Split a MIDI byte stream into messages (handles running status, skips sysex/realtime).
pub fn for_each_message(data: &[u8], running: &mut u8, mut f: impl FnMut(&[u8])) {
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        if b >= 0xF8 {
            i += 1; // realtime
            continue;
        }
        if b == 0xF0 {
            while i < data.len() && data[i] != 0xF7 {
                i += 1;
            }
            i += 1;
            *running = 0;
            continue;
        }
        let (status, start) = if b & 0x80 != 0 {
            *running = if b < 0xF0 { b } else { 0 };
            (b, i + 1)
        } else if *running != 0 {
            (*running, i)
        } else {
            i += 1;
            continue;
        };
        let n = match status & 0xF0 {
            0xC0 | 0xD0 => 1,
            0xF0 => 0,
            _ => 2,
        };
        let mut msg = [status, 0, 0];
        let (mut got, mut j) = (0, start);
        while got < n && j < data.len() {
            let d = data[j];
            if d >= 0xF8 {
                j += 1; // realtime may sit inside a message
                continue;
            }
            if d & 0x80 != 0 {
                break;
            }
            msg[1 + got] = d;
            got += 1;
            j += 1;
        }
        if got < n {
            if j >= data.len() {
                return;
            }
            // A status byte where a data byte belongs cuts the message short: drop it and
            // read on from that byte, so no consumer ever sees a data byte of 0x80 or more.
            i = j;
            continue;
        }
        f(&msg[..1 + n]);
        i = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(data: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut rs = 0;
        for_each_message(data, &mut rs, |m| out.push(m.to_vec()));
        out
    }

    /// A status byte where a data byte belongs (a malformed or cut-off message) never
    /// reaches a consumer as data: the cut message is dropped and the new one is read.
    #[test]
    fn data_bytes_are_always_below_0x80() {
        assert_eq!(split(&[0x90, 60, 0x90, 64, 100]), vec![vec![0x90, 64, 100]]);
        assert_eq!(split(&[0x90, 0x80, 60, 0]), vec![vec![0x80, 60, 0]]);
        assert_eq!(split(&[0xC0, 0xB0, 7, 100]), vec![vec![0xB0, 7, 100]]);
        // Running status carries on after the interrupting message.
        assert_eq!(split(&[0x90, 60, 100, 62, 0x80, 60, 0, 64, 0]), vec![
            vec![0x90, 60, 100],
            vec![0x80, 60, 0],
            vec![0x80, 64, 0]
        ]);
        // Realtime bytes may sit inside a message without breaking it.
        assert_eq!(split(&[0x90, 60, 0xF8, 100]), vec![vec![0x90, 60, 100]]);
        // Sysex in the middle of a message ends it too.
        assert_eq!(split(&[0x90, 60, 0xF0, 1, 2, 0xF7, 0x90, 61, 1]), vec![vec![0x90, 61, 1]]);
        // Every byte value in every position: no panic, and no data byte of 0x80 or more.
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                for m in split(&[0x90, a, b, 0xB0, b, a]) {
                    assert!(m[1..].iter().all(|&d| d < 0x80), "{m:?}");
                }
            }
        }
    }
}
