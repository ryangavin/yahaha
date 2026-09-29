//! The one counting global allocator of this test crate. It counts per thread, and only
//! while that thread has counting on (`count_here`, `counted`): the tests run in parallel
//! threads of one process, and each checks the code it runs on its own thread (the engine,
//! input or audio thread it plays), so another test's allocations, or the harness's own
//! work on another thread, never land in its counts. Nothing in here allocates.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::thread::LocalKey;

struct Counting;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}

fn bump(n: &'static LocalKey<Cell<usize>>) {
    if ON.try_with(Cell::get).unwrap_or(false) {
        let _ = n.try_with(|n| n.set(n.get() + 1));
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        bump(&ALLOCS);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        bump(&FREES);
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// This thread's counted allocations and frees so far.
pub fn counts() -> (usize, usize) {
    (ALLOCS.with(Cell::get), FREES.with(Cell::get))
}

/// Counting on, for this thread, until the guard drops (then back to how it was).
#[must_use]
pub struct Counted(bool);

impl Drop for Counted {
    fn drop(&mut self) {
        ON.with(|c| c.set(self.0));
    }
}

pub fn count_here() -> Counted {
    Counted(ON.with(|c| c.replace(true)))
}

/// The allocations and frees `f` makes on this thread.
pub fn counted(f: impl FnOnce()) -> (usize, usize) {
    let before = counts();
    let _on = count_here();
    f();
    let after = counts();
    (after.0 - before.0, after.1 - before.1)
}
