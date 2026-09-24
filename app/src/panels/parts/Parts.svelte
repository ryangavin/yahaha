<!--
  PANEL SLOT: Keyboard parts + One Touch Settings. Not built yet; see app/CONTRIBUTING.md.
  State: app.state.keyboardParts (on, sounding, selected, volume, waiting, voiceName,
  octave …) and app.state.ots. Pad page 3 is the hardware twin.
  Tooltips ready in the catalog: part.*.on, part.*.select, part.voice_*, part.octave_*,
  ots.1–4, ots.link, mixer.panel.* (volume).
-->
<script lang="ts">
  import Panel from '../../lib/ui/Panel.svelte'
  import PanelSlot from '../../lib/ui/PanelSlot.svelte'
</script>

<Panel id="parts" title="Keyboard parts">
  <PanelSlot
    items={[
      'Right 1, Right 2, Right 3, Left: voice, on/off, volume, octave',
      'Which part Voice −/+ edits (Edit R1 … Edit L)',
      'OTS 1–4 (the last recalled lit, missing ones dark) and OTS Link',
    ]}
  />
</Panel>
