<!--
  WaitingChip: names what comes next (the next section, a style waiting for the bar line) in an
  outline of its hue, so it reads as "coming" rather than "playing". A readout, not a control: no
  click, focus or tooltip. An empty or whitespace-only label renders nothing.
-->
<script lang="ts">
  type Props = {
    /** The text in the outline (a section's shown name or a style's name), drawn as given. Blank: nothing renders. */
    label: string
    /** The hue role of the border and text: a section hue, `a` for a queued style, `t` for neutral. */
    hue?: 'intro' | 'main' | 'ending' | 'brk' | 'fill' | 'a' | 't'
    /** `count` 26 tall, 18px light (count row); `line` 26 tall, 14px, at most 200 wide (style line); `display` 48 tall, 36px light (display). */
    size?: 'count' | 'line' | 'display'
  }

  let { label, hue = 't', size = 'count' }: Props = $props()
</script>

{#if label.trim() !== ''}
  <span class="chip {size}" style:--hue="var(--{hue})" data-face="waiting" data-hue={hue} data-size={size}
    >{label}</span
  >
{/if}

<style>
  .chip {
    display: inline-block;
    box-sizing: border-box;
    flex: none;
    height: var(--chip-height);
    padding: 0 var(--space-8);
    border: var(--line-width) solid var(--hue);
    border-radius: var(--radius);
    background: transparent;
    color: var(--hue);
    font-family: var(--font-sans);
    font-size: var(--text-18);
    font-weight: var(--weight-light);
    font-variant-numeric: tabular-nums;
    line-height: calc(var(--chip-height) - 2 * var(--line-width));
    white-space: nowrap;
    vertical-align: middle;
  }
  .line {
    flex: 0 1 auto;
    min-width: 0;
    max-width: var(--chip-max);
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: var(--text-14);
    font-weight: var(--weight-regular);
  }
  .display {
    height: var(--chip-height-display);
    padding: 0 var(--space-10);
    font-size: var(--text-36);
    letter-spacing: var(--tracking-36);
    line-height: calc(var(--chip-height-display) - 2 * var(--line-width));
  }
</style>
