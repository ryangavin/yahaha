<!--
  LampButton: the canvas's on/off control (Accomp, Metronome, part On, Sound, Looper).
  On is the lime lamp face with an ink label; off is the plain button face with a grey label.
  Takes props and calls ontoggle; it holds its own pressed state between clicks, and follows `on`
  whenever the parent changes it.
-->
<script lang="ts">
  type Props = {
    /** The word on the face. */
    label: string
    /** Lit (lamp face) or off. */
    on?: boolean
    /** Small code after the label, e.g. `ACMP`. */
    code?: string
    /** Shown, not pressable. */
    disabled?: boolean
    /** Record lamp: lit is the solid record-red face instead of the lamp face. */
    rec?: boolean
    /** `md` 32px tall (section row), `sm` 28px (settings rows), `cell` 32px filling its container (the band's lamp row). */
    size?: 'md' | 'sm' | 'cell'
    /** A fixed width in px, label centred (e.g. 64 for the settings rows' On/Off). */
    width?: number
    /** The accessible name when the label alone isn't enough ("Right 1 on"). Default: label and code. */
    name?: string
    /** Called with the new state after a click, Space or Enter. */
    ontoggle?: (on: boolean) => void
  }

  let {
    label,
    on = false,
    code,
    disabled = false,
    rec = false,
    size = 'md',
    width,
    name,
    ontoggle,
  }: Props = $props()

  // Follows `on`, and a click overrides it until `on` changes again.
  let pressed = $derived(on)

  function toggle() {
    if (disabled) return
    pressed = !pressed
    ontoggle?.(pressed)
  }
</script>

<button
  type="button"
  class="lamp {size}"
  class:rec
  class:fixed={width !== undefined}
  style:width={width === undefined ? undefined : `${width}px`}
  aria-pressed={pressed}
  aria-disabled={disabled}
  aria-label={name ?? (code ? `${label} ${code}` : label)}
  onclick={toggle}
>
  {label}{#if code}<span class="sub">{code}</span>{/if}
</button>

<style>
  .lamp {
    box-sizing: border-box;
    height: var(--control-height);
    margin: 0;
    padding: 0 var(--space-16);
    border: 0;
    border-radius: var(--radius);
    background: var(--btn);
    color: var(--m);
    font-family: var(--font-sans);
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
    font-variant-numeric: tabular-nums;
    line-height: normal;
    white-space: nowrap;
    cursor: pointer;
  }
  .sm {
    height: var(--control-height-compact);
    padding: 0 var(--space-14);
    font-size: var(--text-13);
  }
  .cell {
    width: 100%;
    min-width: 0;
    padding: 0;
    font-size: var(--text-13);
  }
  .fixed {
    padding: 0;
  }
  .sub {
    margin-left: var(--space-6);
    color: var(--m);
    font-size: var(--text-12);
    font-weight: var(--weight-regular);
  }
  .lamp[aria-pressed='true'] {
    background: var(--lamp);
    color: var(--lamp-ink);
    font-weight: var(--weight-medium);
  }
  .lamp[aria-pressed='true'] .sub {
    color: var(--lamp-ink);
    opacity: var(--code-opacity);
  }
  .rec[aria-pressed='true'] {
    background: var(--rec);
    color: var(--solid-ink);
  }
  .rec[aria-pressed='true'] .sub {
    color: var(--solid-ink);
  }
  .lamp[aria-disabled='true'] {
    color: var(--d);
    cursor: default;
  }
  .lamp:focus-visible {
    outline: var(--line-width) solid var(--focus);
    outline-offset: var(--focus-offset);
  }
</style>
