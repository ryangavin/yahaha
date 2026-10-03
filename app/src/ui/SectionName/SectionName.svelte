<!--
  SectionName: the playing section's name as solid text in its hue ("Main B"): 18px light on the
  count row, 44px light with the hue's soft glow on the display. Its "next" partner is the
  WaitingChip. A readout, not a control.
-->
<script lang="ts">
  type Props = {
    /** The section's shown name ("Main B", "Intro II", "Fill"). Empty: "—". */
    label: string
    /** The section hue. */
    hue?: 'intro' | 'main' | 'ending' | 'brk' | 'fill'
    /** `count` 18px (count row), `display` 44px with its glow (now playing). */
    size?: 'count' | 'display'
  }

  let { label, hue = 'main', size = 'count' }: Props = $props()
</script>

<span
  class="name {size}"
  style:--hue="var(--{hue})"
  style:--glow="var(--section-glow-{hue})"
  data-hue={hue}
  data-size={size}>{label.trim() === '' ? '—' : label}</span
>

<style>
  .name {
    flex: none;
    font-family: var(--font-sans);
    font-weight: var(--weight-light);
    font-variant-numeric: tabular-nums;
    color: var(--hue);
    white-space: nowrap;
  }
  .count {
    font-size: var(--text-18);
    line-height: var(--leading-24);
  }
  .display {
    font-size: var(--text-44);
    line-height: var(--leading-52);
    letter-spacing: var(--tracking-44);
    text-shadow: var(--glow);
  }
</style>
