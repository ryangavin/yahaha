//! Real-time plumbing: host time, Mach time-constraint scheduling, a wait-free wakeup
//! semaphore, and an allocation-free CoreMIDI packet sink.

use coremidi_sys::{
    MIDIEndpointRef, MIDIPacketList, MIDIPacketListAdd, MIDIPacketListInit, MIDIPortRef, MIDIReceived,
    MIDISend,
};
use mach2::kern_return::KERN_SUCCESS;
use mach2::mach_time::{mach_absolute_time, mach_timebase_info};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

fn timebase() -> (u64, u64) {
    static TB: OnceLock<(u64, u64)> = OnceLock::new();
    *TB.get_or_init(|| {
        let mut info = mach_timebase_info { numer: 0, denom: 0 };
        unsafe { mach_timebase_info(&mut info) };
        (info.numer as u64, info.denom as u64)
    })
}

/// Monotonic host time in nanoseconds (same clock CoreMIDI timestamps use).
#[inline]
pub fn now_ns() -> u64 {
    let (n, d) = timebase();
    let t = unsafe { mach_absolute_time() };
    if n == d {
        t
    } else {
        (t as u128 * n as u128 / d as u128) as u64
    }
}

#[inline]
pub fn host_to_ns(t: u64) -> u64 {
    let (n, d) = timebase();
    (t as u128 * n as u128 / d as u128) as u64
}

pub fn ns_to_host(ns: u64) -> u64 {
    let (n, d) = timebase();
    (ns as u128 * d as u128 / n as u128) as u64
}

// ---------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------

/// Put the calling thread in the Mach time-constraint class (the class CoreAudio's IO
/// threads use). `period`/`computation`/`constraint` are in nanoseconds.
pub fn make_realtime(period_ns: u64, computation_ns: u64, constraint_ns: u64) -> bool {
    use mach2::mach_init::mach_thread_self;
    use mach2::thread_policy::{
        thread_policy_set, thread_time_constraint_policy_data_t, THREAD_TIME_CONSTRAINT_POLICY,
        THREAD_TIME_CONSTRAINT_POLICY_COUNT,
    };
    let mut policy = thread_time_constraint_policy_data_t {
        period: ns_to_host(period_ns) as u32,
        computation: ns_to_host(computation_ns) as u32,
        constraint: ns_to_host(constraint_ns) as u32,
        preemptible: 1,
    };
    let kr = unsafe {
        thread_policy_set(
            mach_thread_self(),
            THREAD_TIME_CONSTRAINT_POLICY,
            &mut policy as *mut _ as *mut i32,
            THREAD_TIME_CONSTRAINT_POLICY_COUNT,
        )
    };
    kr == KERN_SUCCESS
}

// ---------------------------------------------------------------------------
// Wakeup semaphore
// ---------------------------------------------------------------------------

/// Mach semaphore. `signal` never blocks and is safe from any thread, including
/// CoreMIDI's receive thread; only the engine thread waits on it.
#[derive(Clone, Copy)]
pub struct Wakeup(mach2::mach_types::semaphore_t);

impl Default for Wakeup {
    fn default() -> Wakeup {
        Wakeup::new()
    }
}

unsafe impl Send for Wakeup {}
unsafe impl Sync for Wakeup {}

impl Wakeup {
    pub fn new() -> Wakeup {
        let mut s = 0;
        unsafe {
            mach2::semaphore::semaphore_create(
                mach2::traps::mach_task_self(),
                &mut s,
                mach2::sync_policy::SYNC_POLICY_FIFO,
                0,
            )
        };
        Wakeup(s)
    }

    #[inline]
    pub fn signal(&self) {
        unsafe { mach2::semaphore::semaphore_signal(self.0) };
    }

    /// Wait until signalled or `timeout_ns` elapses. Returns true if signalled.
    #[inline]
    pub fn wait(&self, timeout_ns: u64) -> bool {
        let ts = mach2::clock_types::mach_timespec_t {
            tv_sec: (timeout_ns / 1_000_000_000) as u32,
            tv_nsec: (timeout_ns % 1_000_000_000) as i32,
        };
        let kr = unsafe { mach2::semaphore::semaphore_timedwait(self.0, ts) };
        kr != mach2::kern_return::KERN_OPERATION_TIMED_OUT
    }
}

// ---------------------------------------------------------------------------
// Packet sink
// ---------------------------------------------------------------------------

const BUF: usize = 4096;

#[repr(C, align(8))]
struct Aligned([u8; BUF]);

pub enum Target {
    /// Distribute from one of our virtual sources.
    Virtual(MIDIEndpointRef),
    /// Send through an output port to a destination.
    Port(MIDIPortRef, MIDIEndpointRef),
    /// Nowhere: flushing drops the messages (offline sessions, tests).
    Null,
}

/// Collects MIDI messages into a stack-style CoreMIDI packet list and flushes them in
/// one call. No heap allocation.
pub struct PacketSink {
    buf: Box<Aligned>,
    cur: *mut coremidi_sys::MIDIPacket,
    target: Target,
    pub sent: u64,
}

unsafe impl Send for PacketSink {}

impl PacketSink {
    pub fn new(target: Target) -> PacketSink {
        let mut s = PacketSink { buf: Box::new(Aligned([0; BUF])), cur: std::ptr::null_mut(), target, sent: 0 };
        s.reset();
        s
    }

    fn list(&mut self) -> *mut MIDIPacketList {
        self.buf.0.as_mut_ptr() as *mut MIDIPacketList
    }

    fn reset(&mut self) {
        self.cur = unsafe { MIDIPacketListInit(self.list()) };
    }

    pub fn is_empty(&self) -> bool {
        unsafe { (*(self.buf.0.as_ptr() as *const MIDIPacketList)).numPackets == 0 }
    }

    pub fn push(&mut self, msg: &[u8]) {
        let list = self.list();
        let mut p = unsafe { MIDIPacketListAdd(list, BUF as u64, self.cur, 0, msg.len() as u64, msg.as_ptr()) };
        if p.is_null() {
            self.flush();
            let list = self.list();
            p = unsafe { MIDIPacketListAdd(list, BUF as u64, self.cur, 0, msg.len() as u64, msg.as_ptr()) };
        }
        if !p.is_null() {
            self.cur = p;
        }
    }

    pub fn flush(&mut self) {
        if self.is_empty() {
            return;
        }
        let list = self.list();
        unsafe {
            match self.target {
                Target::Virtual(src) => MIDIReceived(src, list),
                Target::Port(port, dest) => MIDISend(port, dest, list),
                Target::Null => 0,
            };
            self.sent += (*list).numPackets as u64;
        }
        self.reset();
    }
}

impl crate::engine::Sink for PacketSink {
    #[inline]
    fn send(&mut self, msg: &[u8]) {
        self.push(msg);
    }
}

// ---------------------------------------------------------------------------
// Lateness histogram (engine wake accuracy), written by the RT thread, read by the UI.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Buckets in microseconds: 0..BUCKETS-1, last bucket = overflow.
pub const BUCKETS: usize = 1001;

pub struct Histogram {
    pub buckets: [AtomicU64; BUCKETS],
    pub max_ns: AtomicU64,
}

impl Default for Histogram {
    fn default() -> Histogram {
        Histogram::new()
    }
}

impl Histogram {
    pub const fn new() -> Histogram {
        Histogram { buckets: [const { AtomicU64::new(0) }; BUCKETS], max_ns: AtomicU64::new(0) }
    }

    #[inline]
    pub fn record(&self, ns: u64) {
        let us = ((ns / 1000) as usize).min(BUCKETS - 1);
        self.buckets[us].fetch_add(1, Relaxed);
        self.max_ns.fetch_max(ns, Relaxed);
    }

    pub fn count(&self) -> u64 {
        self.buckets.iter().map(|b| b.load(Relaxed)).sum()
    }

    /// Percentile upper bound in microseconds.
    pub fn percentile_us(&self, p: f64) -> usize {
        let total = self.count();
        if total == 0 {
            return 0;
        }
        let want = (total as f64 * p).ceil() as u64;
        let mut acc = 0;
        for (i, b) in self.buckets.iter().enumerate() {
            acc += b.load(Relaxed);
            if acc >= want {
                return i + 1;
            }
        }
        BUCKETS
    }
}
