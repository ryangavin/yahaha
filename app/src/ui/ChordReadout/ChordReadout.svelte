<!--
  ChordReadout: the display's chord, the brightest thing on the screen. A "Chord" label, the
  128px chord in the accent with its extension in thin weight, the row of notes each with its
  interval beneath, and the fingering mode's word. Held (detection unsure, the band keeps the
  last chord): the chord dims to muted grey without its glow, with a small "held" beside it.
  A readout, not a control.
-->
<script lang="ts">
  type Note = {
    /** The note's name ("A", "C", "F♯"). */
    note: string
    /** Its interval from the root ("R", "m3", "5", "m7"). */
    interval: string
  }

  type Props = {
    /** The chord before its extension ("Am", "C", "F♯m"). Empty: "—". */
    chord: string
    /** The extension, drawn thin after the chord ("7", "maj7", "sus4"). */
    extension?: string
    /** The chord's notes, root first. */
    notes?: Note[]
    /** The fingering mode's word ("Fingered", "Single Finger", "AI Full Keyboard"). */
    fingering?: string
    /** Detection is unsure and the last chord is held: muted, no glow, "held" beside it. */
    held?: boolean
    /** The label over the chord. */
    label?: string
  }

  let { chord, extension = '', notes = [], fingering = '', held = false, label = 'Chord' }: Props = $props()

  const shown = $derived(chord.trim() === '' ? '—' : chord)
  const spoken = $derived(
    `${label}: ${chord.trim() === '' ? 'none' : chord + extension}${held ? ', held' : ''}` +
      (notes.length ? `. Notes ${notes.map((n) => n.note).join(' ')}` : '') +
      (fingering ? `. ${fingering}` : ''),
  )
</script>

<div class="readout" class:held role="group" aria-label={spoken} data-held={held ? 'true' : 'false'}>
  <span class="label" aria-hidden="true">{label}</span>
  <div class="chord" aria-hidden="true">
    {shown}<span class="ext">{extension}</span>{#if held}<span class="held-word">held</span>{/if}
  </div>
  <div class="notes" aria-hidden="true">
    {#each notes as n, i (i)}
      <span class="note"><span class="name">{n.note}</span><span class="interval">{n.interval}</span></span>
    {/each}
    {#if fingering}<span class="fingering">{fingering}</span>{/if}
  </div>
</div>

<style>
  .readout {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-family: var(--font-sans);
    font-variant-numeric: tabular-nums;
  }
  .label {
    height: var(--label-height);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    line-height: var(--leading-16);
    color: var(--m);
  }
  .chord {
    margin-top: var(--space-4);
    height: var(--leading-104);
    font-size: var(--text-128);
    font-weight: var(--weight-light);
    line-height: var(--leading-104);
    letter-spacing: var(--tracking-128);
    color: var(--chord);
    text-shadow: var(--chord-glow);
    white-space: nowrap;
  }
  .ext {
    font-weight: var(--weight-thin);
    letter-spacing: 0;
  }
  .held .chord {
    color: var(--chord-held);
    text-shadow: none;
  }
  .held-word {
    margin-left: var(--space-8);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    letter-spacing: 0;
    color: var(--m);
    text-shadow: none;
  }
  .notes {
    margin-top: var(--space-6);
    height: var(--note-row-height);
    display: flex;
    align-items: flex-end;
    gap: var(--space-16);
  }
  .note {
    width: var(--note-width);
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .name {
    font-size: var(--text-20);
    font-weight: var(--weight-light);
    line-height: var(--leading-18);
    color: var(--t);
  }
  .interval {
    font-size: var(--text-12);
    line-height: var(--leading-14);
    color: var(--m);
  }
  .fingering {
    margin-left: var(--space-4);
    font-size: var(--text-14);
    line-height: var(--leading-16);
    color: var(--m);
    white-space: nowrap;
  }
</style>
