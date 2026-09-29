//! Tests of `fingering` moved here from yahaha-core because they need a higher layer (engine, sff, sim).

use crate::fingering::Fingering;

/// Full Keyboard types switch Sync Stop off and keep it off.
#[test]
fn full_keyboard_disables_sync_stop() {
    use crate::engine::{Button, Engine, Prepared};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
    if !path.exists() {
        return;
    }
    let mut e = Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&path).unwrap())));
    let mut rec = crate::sim::Recorder::default();
    e.button(Button::SyncStop, 0, &mut rec);
    assert!(e.snapshot(0).sync_stop);
    e.allow_sync_stop(Fingering::AiFullKeyboard.allows_sync_stop());
    assert!(!e.snapshot(0).sync_stop);
    e.button(Button::SyncStop, 0, &mut rec);
    assert!(!e.snapshot(0).sync_stop);
    e.allow_sync_stop(Fingering::AiFingered.allows_sync_stop());
    e.button(Button::SyncStop, 0, &mut rec);
    assert!(e.snapshot(0).sync_stop);
}
