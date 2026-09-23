# Forum post draft (PSR Tutorial)

Draft for Ryan to post in the PSR Tutorial forum, for example under Genos or General Discussion. Before posting:

- Attach the kit, made with `yahaha capture-kit <dir>` and zipped. It contains only our chord script, no style data.
- Fill in the bracketed parts.

---

**Subject:** Genos / Genos2 owners: 30 minutes of MIDI recording to help an open-source arranger follow chords like the real thing

Hi all,

I'm building **yahaha**, a free, open-source program (https://github.com/ryangavin/yahaha) that plays Yamaha style files live from any MIDI keyboard. The goal is for it to follow chords *exactly* the way a Genos does: which notes each part plays for every chord type, slash chords, what happens to held notes when the chord changes, fills and endings. It reads the style's own conversion settings (NTR, NTT, High Key, Note Limit, Retrigger Rule), but the manuals only describe those in words. The only real reference is the instrument itself.

**What I'm asking:** if you have a Genos or Genos2, could you record your instrument playing a style along to a MIDI file I provide? It takes about half an hour per style, and even one style helps.

**What you need:** your instrument, a USB cable, and a computer with any DAW or MIDI recorder (Cubase, Logic, Reaper, Cakewalk, GarageBand, MIDI-OX …).

**How it works:**

1. Download the attached kit. For each style it has a MIDI file with left-hand chords on channel 1, plus the section changes as Section Control messages. There are about 100 bars, covering every chord type in the Genos chord table, slash chords, fast changes, Break, fills and an Ending.
2. Load the style. Most are from Paul Drongowski's free *MOX performance styles V2* pack (sandsoftwaresound.net), so your instrument plays exactly the same file I have. A few are factory presets.
3. Set ACMP on, Fingered On Bass, and a couple of MIDI settings (listed step by step in the kit's README). Arm SYNC START.
4. Play the MIDI file into the instrument from the DAW, and record what comes back on MIDI channels 9–16.
5. Send me the recording(s): reply here, message me, or attach them to a GitHub issue.

**What happens with your recording:** my importer lines it up with what yahaha plays for the same chords and shows every bar and part where they differ. Those differences are exactly the bugs to fix. The recording contains only what your instrument played for my chord script. Nothing readable from it is published: the project keeps only a hash of each bar, so the style's content can't be reconstructed from the repository.

A bonus: if you turn on "Chord System Exclusive Message Transmit", the recording also shows how your instrument *reads* each chord. That includes a few ambiguous shapes, like Am7 over G or Dm11 over G. The manuals don't say how the instrument reads those.

Thank you! Any questions about the setup, just ask here.

[Ryan / contact]

[attachment: yahaha-capture-kit.zip]
