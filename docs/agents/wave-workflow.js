export const meta = {
  name: 'wave-m5-m8',
  description: 'Swarm: M5 sections/transport, M6 wiring+controllers, M7 multipads+registration, M8 iReal player, engine follow-ups — implement, adversarial review, fix (no merging)',
  phases: [
    { title: 'Implement', detail: 'one agent per track, own worktree + PR' },
    { title: 'Review', detail: 'adversarial review + fix rounds, up to 3' },
  ],
}

const BRIEF = 'docs/agents/wave-brief.md'

const TRACKS = [
  { key: 'm5-timing', branch: 'm5/section-timing', title: 'Section timing, ritardando, fade, Synchro Stop window, section reset & retrigger', issues: [22, 23, 25], scope: `Tickets:
- Section Change Timing settings: Main uses Immediate or Next Bar; Intro/Ending use Next Bar or End of Section. Implement through Engine::change_point.
- Ending ritardando: press the Ending again during playback.
- Fade In/Out: 0–20 s, hold 0–5 s.
- Synchro Stop window.
- Style Section Reset: Tap during playback.
- Style Retrigger: lengths 1/2/4/8/16/32.
Expose each as a setting. The Settings drawer already has "coming soon" placeholders for section-change timing and the Synchro Stop window; make them real. Add Launchkey/TUI access for section reset, retrigger and fade.` },
  { key: 'm5-rules', branch: 'm5/fills-rules', title: 'Fill variants, style-change rules, Stop ACMP modes, OTS Link timing', issues: [24, 26, 27, 28], scope: `Tickets:
- Fill Up / Down / Self / Break and Half Bar Fill, as assignable functions.
- Style-change rules: Lock/Hold/Reset for tempo, part on/off and the default section. This is the Genos parameter lock behaviour.
- Stop Accompaniment modes: Off / Style / Fixed.
- OTS Link timing: At Main Section Change vs Immediate. On the Genos, recalling an OTS turns on ACMP and Sync Start.
Make the Settings drawer's OTS Link timing placeholder real.` },
  { key: 'm5-looper', branch: 'm5/looper-solo-metronome', title: 'Chord Looper; part solo, Style Track Mute, tempo 5–500, metronome', issues: [29, 30], scope: `Tickets:
- Chord Looper: 8 memories. Recording, looping and memory changes are quantised to the next measure, per the manual. Build it as an engine chord source: while looping it feeds chords as if they were played.
- Part solo for style parts and keyboard parts. The Mixer drawer shows Solo disabled today; enable it.
- Style Track Mute.
- Tempo range 5–500.
- A metronome with its own volume, on the built-in synth only, as a click voice; it must not go out on the MIDI port.
Add a Chord Looper panel or drawer in the app, with tooltips.` },
  { key: 'm6-harmony-arp', branch: 'm6/harmony-arp-wiring', title: 'Wire Keyboard Harmony and Arpeggio into the right-hand processor slot', issues: [32, 33], scope: `Wire the merged pure modules src/harmony.rs (#76) and src/arp/ (#75) into the right-hand input pipeline's processor slot, src/live/pipeline.rs (see docs/architecture.md). Harmony and Arpeggio are mutually exclusive, as on the Genos: one HARMONY/ARPEGGIO switch plus a type choice.
Handle the time domains:
- EchoGen (Echo, Tremolo, Trill) runs on the engine thread in engine ns.
- The arpeggio runs in style ticks, with its own clock while the style is stopped.
Other requirements:
- Chord source rules: which chord Harmony uses for each ACMP/LEFT combination.
- The review follow-ups on #75/#76. Read those PR comments. In particular: count held notes per part+key, set_ppq on style change, and use all_off on stop.
- Settings: Harmony type, volume, assign, chord-note-only and touch limit; Arp pattern, quantize, hold, velocity mode and Keep Key On.
- A Harmony/Arp panel in the app, a Launchkey mapping and TUI keys.
- Sim tests: no stuck notes under randomized play.` },
  { key: 'm6-controllers', branch: 'm6/controllers', title: 'Controllers: sustain/pedals, joystick, assignable functions', issues: [34], scope: `Ticket #34:
- Sustain pedal on the keyboard parts, respecting part on/off. Decide how sustain affects Left, following the Genos manual.
- Pitch bend and modulation from the keyboard (the Launchkey has wheels), per part, with the bend range.
- Assignable functions per the manual's live-play list: footswitch functions such as Start/Stop, fill, Break and Registration +/−. Keep it data-driven, with a table of assignable targets.
- Settings UI and tooltips.
Stuck-note and stuck-pedal safety across style changes, panic and device removal (see #86's source-removal handling).` },
  { key: 'm7-multipad', branch: 'm7/multipad-wiring', title: 'Wire Multi Pads into the engine and app', issues: [37], scope: `Wire the merged src/multipad/ core (#78) into the engine: one player per loaded bank, built off the real-time thread and swapped in.
- Use a fixed-PPQ clock, or rescale on style change; see the review note on #78.
- Chord Match follows the style chord, or the left-hand chord when ACMP is off.
- Start at the next measure while the style plays.
- Synchro Start.
- STOP / STOP+pad / SELECT+pad.
- Multi Pad Synchro Stop settings.
- Output on MIDI channels 5–8, plus built-in synth voices for those channels.
- Commands: load a bank (a .pad path); library/browser support for .pad files.
- A Multi Pad panel in the app. Launchkey access is TBD; propose one. Add tooltips.
- Test with synthetic banks. Real .pad files may not exist in the corpus; if they do, use them.` },
  { key: 'm7-registration', branch: 'm7/registration', title: 'Registration Memory (banks, Freeze, Sequence) and Playlist', issues: [36, 38], scope: `Tickets #36 and #38:
- Registration Memory: 10 buttons per bank, banks saved as files in a yahaha format (JSON is fine; don't use Yamaha's .rgt).
- Each button captures the panel state: style, tempo, the four parts (voice, volume, octave, on/off), mixer, fingering, split, transpose, OTS Link, harmony/arp settings if present, and so on. Do this via a per-feature "registrable" section, so later features add their own.
- Freeze groups, per the manual.
- Registration Sequence, with its end actions.
- Playlist of registrations/styles.
- Parameter Lock interplay: coordinate with the m5-rules agent via the board.
- App UI: a Registration bar and a Playlist panel. Launchkey access: propose one (e.g. Shift+pad page 1). Add tooltips.
- Plugin voices come later (phase 2), so leave the voice-reference type extensible.` },
  { key: 'm8-ireal', branch: 'm8/ireal-player', title: 'iReal Pro chart player mode + lead-sheet band', issues: [73], scope: `Build on the merged iReal core src/ireal/ (#80). Add a chart-player mode in which the engine takes chords from an expanded chart in time instead of from the left hand.
- Chart sections A/B/C/D map to Main A–D.
- Auto fills lead into section changes.
- An Intro before and an Ending after.
- Choruses and a loop section.
- Left-hand override: play a chord to reharmonize, and the chart resumes at the next bar.
- Keyboard transpose applies to the chart.
- The iReal style name suggests a library style; the user can override it.
- Import: open an .html playlist or paste an irealb:// link.
- App: a song/playlist browser for charts, and the lead-sheet band above the Launchkey mirror (the #85 slot) showing the chart with the current bar highlighted.
- Tests use synthetic charts only; commit no real songs.
- Open a new GitHub issue for the player (M8 milestone) and reference it.` },
  { key: 'engine-followups', branch: 'engine/followups', title: 'Engine follow-ups: #47 zero-length notes, #64 SInt routing, #65 rolled chords, #74 MIDI hotplug', issues: [47, 64, 65, 74], scope: `Four issues:
- #47: a chord change retriggers notes that the pattern cuts on the same tick, making zero-length notes. Also cover the "chord + another input in the same wake" case from the #63 review.
- #64: route the SInt by each section's own channel rules on a section change.
- #65: rolled chords. Notes attacked on a passing chord get cut 3 ms later. Design a short chord-settle window, justify it against latency, and make it a setting.
- #74: MIDI device hotplug. Move the CoreMIDI client to a session-owned run-loop thread. Handle Launchkey replug by putting it back into DAW mode and restoring LEDs, and update io.inputs and pads.connected.
Each needs sim tests. The corpus-wide chaos test from #63 must stay green.` },
]

const IMPL = { type: 'object', properties: { pr_url: { type: 'string' }, pr_number: { type: 'number' }, branch: { type: 'string' }, summary: { type: 'string' }, decisions: { type: 'array', items: { type: 'string' } } }, required: ['pr_url', 'pr_number', 'branch', 'summary'] }
const REVIEW = { type: 'object', properties: { approve: { type: 'boolean' }, blocking: { type: 'array', items: { type: 'string' } }, nonblocking: { type: 'array', items: { type: 'string' } }, summary: { type: 'string' } }, required: ['approve', 'blocking', 'nonblocking', 'summary'] }
const FIX = { type: 'object', properties: { addressed: { type: 'array', items: { type: 'string' } }, declined: { type: 'array', items: { type: 'string' } } }, required: ['addressed', 'declined'] }

const iss = t => t.issues.map(i => '#' + i).join(', ')

phase('Implement')
const results = await pipeline(
  TRACKS,
  t => agent(`Read "${BRIEF}" first and follow it exactly.

YOUR TRACK: ${t.title}.
Issues: ${iss(t)}. Read each one with gh issue view.
Branch: ${t.branch}.

Scope:
${t.scope}

Return the PR URL and number, the branch, a summary, and your decisions.`, { label: `impl ${t.key}`, phase: 'Implement', schema: IMPL, isolation: 'worktree' }),
  async (impl, t) => {
    if (!impl) return null
    let review = null
    const history = []
    for (let round = 1; round <= 3; round++) {
      review = await agent(`Read "${BRIEF}" first for project rules and context.

You are the ADVERSARIAL reviewer for PR #${impl.pr_number} (${impl.pr_url}, branch ${impl.branch}), round ${round}.
It covers ${t.title} (${iss(t)}).
Scope the implementer was given:
${t.scope}

Setup:
- git fetch origin
- git checkout --detach origin/${impl.branch}
- Symlink the corpus.
- Run ALL the gates yourself, including npm run verify.

Attack these areas:
- Genos fidelity: verify the claims against the manual text, and check that Decisions are backed by evidence.
- Stuck notes, pedals and bends.
- State that isn't reset on stop, style change or section change.
- Real-time safety: allocation, locks, panics.
- The mixer principle.
- Wire-format compatibility.
- Architecture fit: did it use the hooks and modules, or hack the hotspots?
- Conflicts with sibling tracks listed on the board.
- UI: correctness, tooltip accuracy, fit with the design system.
- Tests: real, or fake?

${round > 1 ? 'Verify the fixes claimed in earlier rounds. Do not re-raise items that were declined with a sound reason.' : ''}
Prove every finding with a throwaway test.
Post your review with gh pr comment ${impl.pr_number}, marking each item blocking or nonblocking.
Do not push commits and do not merge.
Return approve=true only if there is nothing blocking.`, { label: `review ${t.key} r${round}`, phase: 'Review', schema: REVIEW, isolation: 'worktree' })
      if (!review) break
      const h = { round, approve: review.approve, blocking: review.blocking.length }
      history.push(h)
      if (review.approve && review.nonblocking.length === 0) break
      h.fix = await agent(`Read "${BRIEF}" first and follow it.

You are the implementer of ${t.title} (PR #${impl.pr_number}, branch ${impl.branch}), continuing the work. Earlier summary:
${impl.summary}

Review round ${round}.
BLOCKING:
${review.blocking.map(b => '- ' + b).join('\n') || '(none)'}
NONBLOCKING:
${review.nonblocking.map(b => '- ' + b).join('\n') || '(none)'}

Steps:
1. git fetch origin && git checkout -B fix-${t.key} origin/${impl.branch}
2. Symlink the corpus.
3. If integration/m3-ui has moved, merge it in with a merge commit (not a rebase) and resolve any conflicts.
4. Fix every blocking item and prove each fix with a test. If you think a blocking item is wrong, prove that with a test or manual citation instead.
5. Implement the nonblocking items that are sound and in scope; decline the rest with a reason.
6. Run all gates.
7. Push after each step: git push origin HEAD:${impl.branch}
8. Comment on the PR with what you addressed and what you declined.`, { label: `fix ${t.key} r${round}`, phase: 'Review', schema: FIX, isolation: 'worktree' })
      if (review.approve) break
    }
    log(`${t.key} PR #${impl.pr_number}: ${history.map(h => `r${h.round} ${h.approve ? 'ok' : h.blocking + 'B'}`).join(' ')}`)
    return { key: t.key, pr: impl.pr_number, branch: impl.branch, approved: !!(review && review.approve), summary: impl.summary, decisions: impl.decisions, history }
  },
)
return results
