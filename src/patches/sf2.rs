//! The presets a SoundFont offers, read from its preset headers (the `phdr` chunk of
//! `pdta`) without loading its samples: quick enough to browse a 100 MB file.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// One preset: its SoundFont bank (128 = drum kits) and program, and its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub bank: u16,
    pub program: u8,
    pub name: String,
}

fn chunk_header(r: &mut impl Read) -> std::io::Result<([u8; 4], u32)> {
    let mut h = [0u8; 8];
    r.read_exact(&mut h)?;
    Ok(([h[0], h[1], h[2], h[3]], u32::from_le_bytes([h[4], h[5], h[6], h[7]])))
}

/// The presets of the SoundFont at `path`, sorted by bank and program.
pub fn presets(path: &Path) -> Result<Vec<Preset>> {
    let mut f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let (id, _) = chunk_header(&mut f).context("reading the RIFF header")?;
    let mut form = [0u8; 4];
    f.read_exact(&mut form)?;
    if &id != b"RIFF" || &form != b"sfbk" {
        bail!("not a SoundFont");
    }
    loop {
        let (id, size) = chunk_header(&mut f).context("no preset data")?;
        let padded = size as u64 + (size as u64 & 1);
        if &id == b"LIST" {
            let mut ty = [0u8; 4];
            f.read_exact(&mut ty)?;
            if &ty == b"pdta" {
                if size > 64 << 20 {
                    bail!("preset data too big");
                }
                let mut data = vec![0u8; size as usize - 4];
                f.read_exact(&mut data)?;
                return parse_pdta(&data);
            }
            f.seek(SeekFrom::Current(padded as i64 - 4))?;
        } else {
            f.seek(SeekFrom::Current(padded as i64))?;
        }
    }
}

fn parse_pdta(data: &[u8]) -> Result<Vec<Preset>> {
    let mut i = 0;
    while i + 8 <= data.len() {
        let id = &data[i..i + 4];
        let size = u32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]) as usize;
        let body = data.get(i + 8..i + 8 + size).context("truncated preset data")?;
        if id == b"phdr" {
            let mut out: Vec<Preset> = body
                .chunks_exact(38)
                .map(|r| {
                    let name = r[..20].split(|&b| b == 0).next().unwrap_or_default();
                    Preset {
                        name: String::from_utf8_lossy(name).trim().to_string(),
                        program: u16::from_le_bytes([r[20], r[21]]).min(127) as u8,
                        bank: u16::from_le_bytes([r[22], r[23]]),
                    }
                })
                .collect();
            // The last record is the terminal "EOP".
            out.pop();
            out.sort_by_key(|p| (p.bank, p.program));
            out.dedup_by_key(|p| (p.bank, p.program));
            return Ok(out);
        }
        i += 8 + size + (size & 1);
    }
    bail!("no preset headers")
}

/// A tiny, valid SoundFont built in code (no samples from anywhere: one looped square
/// wave), with a preset for each `(bank, program, name)`. For tests, so they never
/// depend on which `.sf2` files a checkout happens to have.
#[doc(hidden)]
pub fn tiny_sound_font(presets: &[(u16, u8, &str)]) -> Vec<u8> {
    fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut c = id.to_vec();
        c.extend((body.len() as u32).to_le_bytes());
        c.extend(body);
        if body.len() & 1 == 1 {
            c.push(0);
        }
        c
    }
    fn list(ty: &[u8], chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut b = ty.to_vec();
        for c in chunks {
            b.extend(c);
        }
        chunk(b"LIST", &b)
    }
    fn name(n: &str) -> [u8; 20] {
        let mut b = [0u8; 20];
        let n = n.as_bytes();
        b[..n.len().min(19)].copy_from_slice(&n[..n.len().min(19)]);
        b
    }
    let u16s = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    // One square wave cycle of 100 samples (480 Hz at 48 kHz), looped, then the 46 zeros
    // the format asks for.
    let wave: Vec<i16> = (0..100).map(|i| if i < 50 { 8000 } else { -8000 }).chain(std::iter::repeat_n(0, 46)).collect();
    let smpl: Vec<u8> = wave.iter().flat_map(|s| s.to_le_bytes()).collect();
    let n = presets.len() as u16;
    let mut phdr = Vec::new();
    for (i, &(bank, program, nm)) in presets.iter().enumerate() {
        phdr.extend(name(nm));
        phdr.extend(u16s(&[program as u16, bank, i as u16]));
        phdr.extend([0u8; 12]);
    }
    phdr.extend(name("EOP"));
    phdr.extend(u16s(&[0, 0, n]));
    phdr.extend([0u8; 12]);
    // A bag per preset with one generator: instrument 0.
    let pbag = u16s(&(0..=n).flat_map(|i| [i, 0]).collect::<Vec<u16>>());
    let pmod = vec![0u8; 10];
    let mut pgen = Vec::new();
    for _ in 0..n {
        pgen.extend(u16s(&[41, 0]));
    }
    pgen.extend(u16s(&[0, 0]));
    let mut inst = name("Square").to_vec();
    inst.extend(u16s(&[0]));
    inst.extend(name("EOI"));
    inst.extend(u16s(&[1]));
    let ibag = u16s(&[0, 0, 2, 0]);
    let imod = vec![0u8; 10];
    // Loop continuously (sampleModes 1), then the sample (sampleID must come last).
    let igen = u16s(&[54, 1, 53, 0, 0, 0]);
    let mut shdr = name("Square").to_vec();
    for v in [0u32, 100, 0, 100, 48_000] {
        shdr.extend(v.to_le_bytes());
    }
    shdr.extend([69u8, 0]); // original pitch A4 (close enough), no correction
    shdr.extend(u16s(&[0, 1]));
    shdr.extend(name("EOS"));
    shdr.extend([0u8; 26]);
    let info = list(b"INFO", &[chunk(b"ifil", &u16s(&[2, 1])), chunk(b"isng", b"EMU8000\0"), chunk(b"INAM", b"yahaha test\0")]);
    let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);
    let pdta = list(
        b"pdta",
        &[
            chunk(b"phdr", &phdr),
            chunk(b"pbag", &pbag),
            chunk(b"pmod", &pmod),
            chunk(b"pgen", &pgen),
            chunk(b"inst", &inst),
            chunk(b"ibag", &ibag),
            chunk(b"imod", &imod),
            chunk(b"igen", &igen),
            chunk(b"shdr", &shdr),
        ],
    );
    let mut body = b"sfbk".to_vec();
    body.extend(info);
    body.extend(sdta);
    body.extend(pdta);
    chunk(b"RIFF", &body)
}

/// The tiny test SoundFont with a GM-like set: bank 0 programs 0-127 and a drum kit on
/// bank 128.
#[doc(hidden)]
pub fn tiny_gm_sound_font() -> Vec<u8> {
    let names: Vec<String> = (0..128).map(|p| format!("Tone {}", p + 1)).collect();
    let mut presets: Vec<(u16, u8, &str)> = names.iter().enumerate().map(|(p, n)| (0, p as u8, n.as_str())).collect();
    presets.push((128, 0, "Kit"));
    tiny_sound_font(&presets)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tiny_sound_font_parses_and_sounds() {
        let bytes = super::tiny_gm_sound_font();
        let font = std::sync::Arc::new(rustysynth::SoundFont::new(&mut &bytes[..]).expect("parses"));
        assert_eq!(font.get_presets().len(), 129);
        let mut s = rustysynth::Synthesizer::new(&font, &rustysynth::SynthesizerSettings::new(48_000)).unwrap();
        s.note_on(0, 60, 100);
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        s.render(&mut l, &mut r);
        assert!(l.iter().any(|x| x.abs() > 1e-3), "it makes a sound");
    }

    use super::*;

    /// A minimal SoundFont: an INFO list, an empty sdta, and a pdta with two presets and
    /// the terminal record.
    fn tiny() -> Vec<u8> {
        let rec = |name: &str, prog: u16, bank: u16| {
            let mut r = vec![0u8; 38];
            r[..name.len()].copy_from_slice(name.as_bytes());
            r[20..22].copy_from_slice(&prog.to_le_bytes());
            r[22..24].copy_from_slice(&bank.to_le_bytes());
            r
        };
        let mut phdr = Vec::new();
        for r in [rec("Standard", 0, 128), rec("Finger Bass", 33, 0), rec("EOP", 0, 0)] {
            phdr.extend(r);
        }
        let chunk = |id: &[u8], body: &[u8]| {
            let mut c = id.to_vec();
            c.extend((body.len() as u32).to_le_bytes());
            c.extend(body);
            if body.len() & 1 == 1 {
                c.push(0);
            }
            c
        };
        let list = |ty: &[u8], body: Vec<u8>| {
            let mut b = ty.to_vec();
            b.extend(body);
            chunk(b"LIST", &b)
        };
        let mut body = b"sfbk".to_vec();
        body.extend(list(b"INFO", chunk(b"ifil", &[2, 0, 1, 0])));
        body.extend(list(b"sdta", chunk(b"smpl", &[0u8; 11])));
        body.extend(list(b"pdta", chunk(b"phdr", &phdr)));
        chunk(b"RIFF", &body)
    }

    #[test]
    fn reads_the_preset_headers() {
        let dir = std::env::temp_dir().join(format!("yahaha-sf2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("tiny.sf2");
        std::fs::write(&p, tiny()).unwrap();
        let got = presets(&p).unwrap();
        assert_eq!(
            got,
            [Preset { bank: 0, program: 33, name: "Finger Bass".into() }, Preset { bank: 128, program: 0, name: "Standard".into() }]
        );
        std::fs::write(&p, b"RIFF\0\0\0\0WAVE").unwrap();
        assert!(presets(&p).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every SoundFont in the checkout's soundfonts/ lists its presets, and a GM one has
    /// the 128 melodic programs on bank 0.
    #[test]
    fn real_soundfonts_list_presets() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
        for f in crate::library::sound_font_files(&dir) {
            let got = presets(&dir.join(&f)).unwrap();
            assert!(!got.is_empty(), "{f}");
            if f.contains("GeneralUser") {
                assert_eq!(got.iter().filter(|p| p.bank == 0).count(), 128, "{f}");
                assert!(got.iter().any(|p| p.bank == 128), "{f} has drum kits");
            }
        }
    }
}
