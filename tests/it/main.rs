//! The integration tests, as one test crate: each `tests/*.rs` file is a crate of its own
//! that links the whole library, so they live here as modules and link it once.
//! `alloc_count` is the crate's counting global allocator (per thread), shared by the
//! no-allocation checks.

mod alloc_count;

mod arp_no_alloc;
mod chart_no_alloc;
mod devices_live;
mod engine_no_alloc;
mod input_no_alloc;
mod multipad_no_alloc;
mod perform_no_alloc;
mod plugin_rack_no_alloc;
mod sound_library_no_alloc;
mod style_change_stress;
mod synth_no_alloc;
mod synth_velocity_tone;
