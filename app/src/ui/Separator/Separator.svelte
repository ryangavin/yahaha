<!--
  Separator: the 1px vertical hairline between groups in a row (the app bar's two tab runs, its
  right area). Decorative; the parent spaces it with its own gap or margins.
-->
<script lang="ts">
  type Props = {
    /** `short` 16px tall, centred on a header row's text (it sits `--separator-lift` above the row's end); `full` the row's whole height. */
    length?: 'short' | 'full'
  }

  let { length = 'short' }: Props = $props()
</script>

<span class="separator {length}" data-length={length} aria-hidden="true"></span>

<style>
  .separator {
    display: block;
    flex: none;
    align-self: center;
    width: var(--line-width);
    background: var(--line);
  }
  /* In a header row (35px of content, text on --header-baseline) the short line is centred on the
     text rather than on the row: it stands on the row's end, lifted. */
  .short {
    align-self: flex-end;
    height: var(--separator-length);
    margin-bottom: var(--separator-lift);
  }
  .full {
    align-self: stretch;
  }
</style>
