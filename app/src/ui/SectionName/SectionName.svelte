<!--
  SectionName: the playing section's name on the display, 44px light text in its hue with the
  hue's soft glow ("Main B"). A readout, not a control.
-->
<script lang="ts">
  type Props = {
    /** The section's shown name ("Main B", "Intro II", "Fill"). Empty: "—". */
    label: string
    /** The section hue. */
    hue?: 'intro' | 'main' | 'ending' | 'brk' | 'fill'
  }

  let { label, hue = 'main' }: Props = $props()
</script>

<span class="name" style:--hue="var(--{hue})" style:--glow="var(--section-glow-{hue})" data-hue={hue}
  >{label.trim() === '' ? '—' : label}</span
>

<style>
  .name {
    flex: none;
    font-family: var(--font-sans);
    font-size: var(--text-44);
    font-weight: var(--weight-light);
    font-variant-numeric: tabular-nums;
    line-height: var(--leading-44);
    letter-spacing: var(--tracking-44);
    color: var(--hue);
    text-shadow: var(--glow);
    white-space: nowrap;
  }
</style>
