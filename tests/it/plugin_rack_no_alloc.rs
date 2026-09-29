//! The plugin rack's audio-thread calls (`begin_block`, `midi`, `render_add`, `set_strip`
//! and `render_add_sends` into six send buses), including a swap, a crossfade and a clear
//! with a strip compressor and insert 2 on, must not allocate on the host side (a fault only sets a
//! flag and pushes an event; `plugin::tests` covers its behaviour). The crate's counting
//! allocator (`alloc_count`, on the test's own thread, which plays the audio thread) checks
//! it. What the plugin itself does in its render is its own business and does not go
//! through Rust's allocator.
#![cfg(feature = "plugins")]

use crate::alloc_count::counted;
use yahaha::fx::master::CompPreset;
use yahaha::fx::{InsertSettings, InsertSlot, InsertType, PartComp, SENDS};
use yahaha::plugin::{LoadConfig, PluginHost, PluginId, PluginInstance, Swap, rack};

fn dls() -> PluginInstance {
    PluginHost::new(None).load(&PluginId::DLS, LoadConfig { max_frames: 256, ..Default::default() }).unwrap()
}

#[test]
fn rack_audio_path_does_not_allocate() {
    let (mut rack, mut ctl) = rack(256, 48_000.0);
    let (a, b) = (dls(), dls());
    let (mut l, mut r) = (vec![0f32; 256], vec![0f32; 256]);
    // The effect bus's six send buses (the three buses and sends 4-6), each side 256 frames.
    let mut sends = vec![0f32; 2 * SENDS * 256];
    // Each channel's gain into each (send 4 up on several).
    let mut gains = [[0f32; SENDS]; 16];
    for (ch, g) in gains.iter_mut().enumerate().take(4) {
        *g = [0.3, 0.1, 0.0, 0.8 - 0.1 * ch as f32, 0.5, 0.2];
    }
    // `sends`: through `render_add_sends` into the send buses, else `render_add`.
    let mut run = |rack: &mut yahaha::plugin::PluginRack, msgs: &[[u8; 3]], with_sends: bool| -> usize {
        counted(|| {
            rack.begin_block();
            for m in msgs {
                rack.midi(*m, 17);
            }
            l.fill(0.0);
            r.fill(0.0);
            if with_sends {
                sends.fill(0.0);
                rack.render_add_sends(&mut l, &mut r, Some((&mut sends[..], &gains)));
            } else {
                rack.render_add(&mut l, &mut r);
            }
        })
        .0
    };
    let strip = |on: bool| {
        if on {
            (PartComp::of(true, CompPreset::Loud), InsertSlot::of(InsertType::Distortion).settings(120.0, false))
        } else {
            (PartComp::default(), InsertSettings::NONE)
        }
    };
    ctl.assign(0, a, Swap::default()).ok().unwrap();
    assert_eq!(run(&mut rack, &[[0xB0, 7, 110], [0xB0, 1, 40], [0xE0, 0, 70], [0x90, 60, 100], [0x90, 64, 100]], false), 0, "assign + notes");
    // RPN 0-2 (bend range, tuning) and an NRPN select: tracked, then replayed on the swap.
    let rpn = [[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 12], [0xB0, 38, 0], [0xB0, 100, 1], [0xB0, 6, 64], [0xB0, 99, 1], [0xB0, 98, 8]];
    assert_eq!(run(&mut rack, &rpn, false), 0, "RPN / NRPN tracking");
    for _ in 0..20 {
        assert_eq!(run(&mut rack, &[], false), 0, "steady");
    }
    // The mixer strip on the plugin's output (a compressor and insert 2), set from the
    // audio thread, and its sends into all six send buses (send 4 up).
    let (comp, second) = strip(true);
    assert_eq!(counted(|| rack.set_strip(0, comp, second)).0, 0, "set_strip");
    for _ in 0..10 {
        assert_eq!(run(&mut rack, &[[0x90, 62, 100], [0x80, 62, 0]], true), 0, "strip and sends 4-6");
    }
    ctl.assign(0, b, Swap::default()).ok().unwrap();
    assert_eq!(run(&mut rack, &[[0x90, 67, 100]], true), 0, "swap with controller replay");
    for _ in 0..4 {
        assert_eq!(run(&mut rack, &[[0xB0, 7, 90]], true), 0, "crossfade and fader moves");
    }
    let (comp, second) = strip(false);
    assert_eq!(counted(|| rack.set_strip(0, comp, second)).0, 0, "set_strip off");
    assert_eq!(run(&mut rack, &[], true), 0, "strip off");
    ctl.clear(0, 240);
    for _ in 0..4 {
        assert_eq!(run(&mut rack, &[[0x80, 60, 0]], false), 0, "clear and fade out");
    }
    // Instances come back to be dropped here, off the audio path.
    assert_eq!(ctl.take_retired().len(), 2);
}
