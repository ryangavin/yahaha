//! Tests of `fx` moved here from yahaha-fx because they need a higher layer (sff, engine).

#[cfg(feature = "slow-tests")]
use crate::fx::xg::{StyleFx, style_inserts};

/// Every corpus style: its effects read, and the reverb always matches (they are all
/// halls, rooms and plates). Prints how often each type occurs.
#[cfg(feature = "slow-tests")]
#[test]
fn corpus_styles_effects() {
    let files = crate::library::corpus_loaded();
    if files.is_empty() {
        eprintln!("corpus missing; skipping");
        return;
    }
    let mut counts = std::collections::BTreeMap::<(usize, String, Option<u8>), usize>::new();
    let (mut with_params, mut with_ret) = (0, [0usize; 3]);
    let mut n = 0;
    for (f, s) in files {
        n += 1;
        let fx = StyleFx::parse(&s.sint().sysex);
        if let Some(r) = &fx.blocks[0] {
            assert!(r.kind.is_some(), "{f:?}: reverb {}", r.name);
        }
        with_params += fx.blocks[0].as_ref().is_some_and(|r| !r.params.is_empty()) as usize;
        for (b, c) in fx.blocks.iter().enumerate() {
            with_ret[b] += c.as_ref().is_some_and(|c| c.ret.is_some()) as usize;
            let key = c.as_ref().map_or((b, "(none)".to_string(), None), |c| (b, c.name.clone(), c.kind));
            *counts.entry(key).or_default() += 1;
        }
    }
    assert!(n > 100, "{n} styles");
    // #269: the insertion effects, as each style's first setup routes them.
    let mut ins = std::collections::BTreeMap::<(String, String), usize>::new();
    let (mut blocks, mut styles) = (0, 0);
    for (_, s) in files {
        let prep = crate::engine::Prepared::new(s);
        let found = style_inserts(prep.setups[0].init.iter());
        styles += !found.is_empty() as usize;
        blocks += found.len();
        for i in found {
            *ins.entry((i.kind.map_or("(dry)", |k| k.0.name()).to_string(), i.name)).or_default() += 1;
        }
    }
    eprintln!("inserts: {blocks} on the Style parts of {styles} styles");
    for ((kind, name), c) in &ins {
        eprintln!("insert {kind:<11} {name:<26} {c}");
    }
    assert!(blocks > 100, "{blocks} inserts");
    eprintln!("{n} styles; reverb parameters in {with_params}; returns applied (reverb, chorus, variation) {with_ret:?}");
    for ((b, name, kind), c) in counts {
        eprintln!("{} {name:<22} -> {kind:?}: {c}", ["reverb", "chorus", "variation"][b]);
    }
}
