<!--
  StatusDot: the small round light that says connected, running, waiting or changed, in the hue
  of what it belongs to. Solid with a soft glow (dark only), solid without, or a hollow 1px ring
  for "armed, not yet" and "not there". Decorative unless named; hidden keeps its box so the text
  after it doesn't move. The parent picks every prop from state.
-->
<script lang="ts">
  type Props = {
    /** The colour role: `ok` connected or running, `d` absent, `t` modified or waiting, or a section hue. */
    hue?: 'ok' | 'd' | 't' | 'intro' | 'main' | 'ending' | 'brk' | 'fill'
    /** A 1px ring in the hue, empty inside and never glowing: "armed, not yet" or "not there". */
    hollow?: boolean
    /** The 6px glow around a solid dot (dark theme only). Ignored when `hollow`. */
    glow?: boolean
    /** `md` 6 × 6 (every status dot), `sm` 5 × 5 (the rack readout's modified dot). */
    size?: 'md' | 'sm'
    /** `false` hides the dot but keeps its box, so the text after it doesn't move. */
    visible?: boolean
    /** The accessible name for a dot with no word beside it ("Running"). Without it the dot is decorative. */
    name?: string
  }

  let { hue = 'ok', hollow = false, glow = true, size = 'md', visible = true, name }: Props = $props()

  let glows = $derived(glow && !hollow)
  let named = $derived(visible && name !== undefined)
</script>

<span
  class="dot {size}"
  class:hollow
  class:glows
  data-hue={hue}
  data-face={hollow ? 'hollow' : 'solid'}
  data-glow={glows ? 'true' : 'false'}
  data-size={size}
  style:visibility={visible ? undefined : 'hidden'}
  role={named ? 'img' : undefined}
  aria-label={named ? name : undefined}
  aria-hidden={named ? undefined : 'true'}
></span>

<style>
  /* Each hue sets the colour and its glow token (the glow's colour-mix lives in the tokens). */
  .dot {
    --hue: var(--ok);
    --glow: var(--dot-glow-ok);
    display: inline-block;
    flex: none;
    box-sizing: border-box;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    vertical-align: middle;
    background: var(--hue);
  }
  .sm {
    width: 5px;
    height: 5px;
  }
  .glows {
    box-shadow: var(--glow);
  }
  .hollow {
    background: transparent;
    border: var(--line-width) solid var(--hue);
  }
  [data-hue='d'] {
    --hue: var(--d);
    --glow: var(--dot-glow-d);
  }
  [data-hue='t'] {
    --hue: var(--t);
    --glow: var(--dot-glow-t);
  }
  [data-hue='intro'] {
    --hue: var(--intro);
    --glow: var(--dot-glow-intro);
  }
  [data-hue='main'] {
    --hue: var(--main);
    --glow: var(--dot-glow-main);
  }
  [data-hue='ending'] {
    --hue: var(--ending);
    --glow: var(--dot-glow-ending);
  }
  [data-hue='brk'] {
    --hue: var(--brk);
    --glow: var(--dot-glow-brk);
  }
  [data-hue='fill'] {
    --hue: var(--fill);
    --glow: var(--dot-glow-fill);
  }
</style>
