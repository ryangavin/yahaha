# Reference captures

Digests of what real hardware (a Genos or PSR-SX) played for the capture script `docs/capture-kit/capture.script`. Owners record them with the capture kit (`docs/capture-kit/README.md`), and `yahaha capture-import` turns them into these files. They are the ground truth for the M2 chord-to-note work. The golden snapshots in `tests/golden` only pin down what yahaha does now.

| File | Contents |
| --- | --- |
| `<style file>.digest` | The recording in the golden digest format (`tests/golden/README.md`): our script's bar headers, and for each part in each bar its channel, name and a 64-bit FNV-1a hash of its note line. No notes. |
| `<style file>.known` | The `bar N chX` keys where yahaha differed from the recording when it was imported, one per line. `#` starts a comment. |

`capture::tests::reference_captures` plays the capture script on each style in `corpus/` that has a digest here. It fails on any bar and part that differs from the recording unless that key is listed in the `.known` file. It also reports listed keys that now match, so they can be removed from the list. The test is skipped when the style is not in the corpus.

## Adding a recording

```sh
yahaha capture-import OrganCruise.recording.mid corpus/MOX_v2/OrganCruise.S930.STY            # read the report
yahaha capture-import OrganCruise.recording.mid corpus/MOX_v2/OrganCruise.S930.STY --golden tests/reference
```

`--golden` writes the digest and the `.known` list, but only if the recording is verified:

- the parts that play as written (drums) match ours for at least 98% of their notes;
- the instrument played at the style's tempo (within 0.2%);
- the recording runs to the end of the script.

The import report also lists every chord the instrument read differently from the script (from its Chord SysEx). Those bars compare the hardware on one chord with yahaha on another, so look at them first.

Commit the digest and the `.known` list. Keep the recording itself, and anything written with `--listing`, off the repo: both transcribe the style. Put the instrument, firmware and owner in the commit message.

Timing: a recorded note that starts within the tolerance (25 ms by default) of one of ours on the same key takes our start. When its end also agrees, it takes our length too. MIDI jitter therefore never changes a digest, and a real difference in timing or length still does.
