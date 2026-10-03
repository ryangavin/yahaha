<!--
  Separator: the 1px vertical hairline between groups in a row (the app bar's two tab runs, its
  right area). Decorative; the parent spaces it with its own gap or margins.
-->
<script lang="ts">
  type Props = {
    /** `short` 16px tall, centred in its row; `full` the row's whole height. */
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
  .short {
    height: var(--separator-length);
  }
  .full {
    align-self: stretch;
  }
</style>
