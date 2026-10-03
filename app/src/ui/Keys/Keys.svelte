<!--
  Keys: the 56px OLED key strip. Dark keys over a key range; with a split, the left zone's keys are
  tinted, a teal line with its glow runs along its top and a 2px white line marks the split. Held
  keys fill with their part's hue and glow. Every C carries its name in mono. One picture with an
  accessible label that says the split and the held notes.
-->
<script lang="ts">
  import { keysLabel, layout, type KeyPart, type KeyRange } from './keys'

  type Props = {
    /** The lowest and highest MIDI note shown, both included (C3 = 60). Default: C1–C6, 61 keys. */
    range?: KeyRange
    /** The split note, the highest note of the left zone; null = no split (no left zone). */
    split?: number | null
    /** The held notes the left part plays: they fill in the L hue. */
    heldLeft?: number[]
    /** The held notes the right hand plays: they fill in `rightPart`'s hue. */
    heldRight?: number[]
    /** The right-hand part whose hue the right's held keys take. */
    rightPart?: Exclude<KeyPart, 'l'>
    /** The strip's width in px, its 1px border included. */
    width?: number
  }

  let {
    range = { low: 36, high: 96 },
    split = 54,
    heldLeft = [],
    heldRight = [],
    rightPart = 'r1',
    width = 1392,
  }: Props = $props()

  // The strip's 1px border on each side.
  let keys = $derived(layout(range, Math.max(0, width - 2), split, heldLeft, heldRight, rightPart))
  let label = $derived(keysLabel(range, split, heldLeft, heldRight))
</script>

<div class="strip" role="img" aria-label={label} style:width="{width}px">
  {#each keys.whites as key (key.note)}
    <div
      class="white"
      class:left={key.left}
      data-held={key.held ?? undefined}
      style:left="{key.x}px"
      style:width="{keys.whiteWidth}px"
    >
      {key.label}
    </div>
  {/each}
  {#each keys.blacks as key (key.note)}
    <div class="black" class:left={key.left} data-held={key.held ?? undefined} style:left="{key.x}px"></div>
  {/each}
  {#if keys.splitX !== null}
    <div class="zone" style:width="{keys.splitX}px"></div>
    <div class="split" style:left="{keys.splitX}px"></div>
  {/if}
</div>

<style>
  .strip {
    position: relative;
    flex: none;
    box-sizing: border-box;
    height: var(--keys-height);
    border: var(--line-width) solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--g);
  }
  .white {
    position: absolute;
    top: 0;
    box-sizing: border-box;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: var(--keys-label-bottom);
    border-right: var(--line-width) solid var(--keyline);
    background: var(--key-white);
    color: var(--d);
    font-family: var(--font-mono);
    font-size: var(--text-11);
    line-height: var(--keys-label-line);
  }
  .white.left {
    background: var(--key-white-left);
  }
  .black {
    position: absolute;
    top: 0;
    width: var(--keys-black-width);
    height: var(--keys-black-height);
    margin-left: calc(var(--keys-black-width) / -2);
    border-radius: 0 0 var(--keys-black-radius) var(--keys-black-radius);
    background: var(--key-black);
    box-shadow: inset 0 0 0 var(--line-width) var(--key-black-ring);
  }
  .black.left {
    box-shadow: inset 0 0 0 var(--line-width) var(--key-black-ring-left);
  }
  /* `.strip` first so a held key beats the left zone's tint. */
  .strip [data-held] {
    color: var(--solid-ink);
  }
  .strip [data-held='r1'] {
    background: var(--r1);
    box-shadow: var(--key-glow-r1);
  }
  .strip [data-held='r2'] {
    background: var(--r2);
    box-shadow: var(--key-glow-r2);
  }
  .strip [data-held='r3'] {
    background: var(--r3);
    box-shadow: var(--key-glow-r3);
  }
  .strip [data-held='l'] {
    background: var(--l);
    box-shadow: var(--key-glow-l);
  }
  .zone {
    position: absolute;
    top: 0;
    left: 0;
    height: var(--keys-zone-line);
    background: var(--l);
    box-shadow: var(--bl);
  }
  .split {
    position: absolute;
    top: 0;
    bottom: 0;
    width: var(--keys-split-width);
    background: var(--t);
  }
</style>
