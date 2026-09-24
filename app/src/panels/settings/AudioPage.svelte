<!--
  Audio: the built-in SoundFont synth (yahaha-only; the Genos has its own tone
  generator). On/off, the output pair (multi-output interfaces list every pair), the
  SoundFont (`setSoundFont`, from the .sf2 files in the synth's folder), the buffer size
  (`setAudioBuffer`, for heavy plugins) and master volume.
-->
<script lang="ts">
  import { settings } from '../../lib/api/settings.svelte'
  import { app } from '../../lib/store.svelte'
  import Toggle from '../../lib/ui/Toggle.svelte'
  import Choice from './Choice.svelte'
  import Field from './Field.svelte'
  import HSlider from './HSlider.svelte'

  const synth = $derived(app.state.io.synth)
  /** `setAudioBuffer`'s sizes, in frames. */
  const BUFFERS = [64, 128, 256] as const
  const master = $derived(app.state.mixer.master)
  const view = $derived(settings.view(app.state))
  // A setting the engine lacks (an older engine) is badged and inert: it never pretends to work.
  const inert = $derived(view.mocked.soundFont)

  const pairs = $derived.by(() => {
    const n = synth ? Math.max(1, Math.floor(synth.channels / 2)) : 1
    return Array.from({ length: n }, (_, i) => ({ id: i * 2, label: `${i * 2 + 1}/${i * 2 + 2}`, tip: 'audio.output' as const }))
  })
  const fonts = $derived(view.soundFonts.map((f) => ({ id: f, label: f.replace(/\.sf2$/i, ''), tip: 'audio.soundfont' as const })))
  const deviceLine = $derived(
    synth
      ? `${synth.device} · ${synth.channels} outputs · ${(synth.sampleRate / 1000).toFixed(1).replace(/\.0$/, '')} kHz${synth.bufferFrames ? ` · ${synth.bufferFrames}-frame buffer` : ''}`
      : null,
  )
</script>

<Field
  name="Built-in synth"
  inline
  note={synth
    ? synth.muted
      ? 'Silent. The yahaha MIDI port still plays, for Ableton or your own sounds.'
      : 'Plays the band and your parts through the SoundFont below.'
    : 'Not running: yahaha started with --no-synth, or found no SoundFont in soundfonts/.'}
>
  <span class="gate" class:off={!synth}>
    <Toggle on={!!synth && !synth.muted} tip="audio.synth_on" onclick={() => synth && app.send({ type: 'setSynthMuted', on: !synth.muted })}>
      {synth ? (synth.muted ? 'Off' : 'On') : 'Not running'}
    </Toggle>
  </span>
</Field>

<Field
  name="Output pair"
  note={deviceLine ? (pairs.length > 1 ? deviceLine : `${deviceLine}. A multi-output interface such as a TASCAM Model 16 lists all its pairs here.`) : null}
>
  <Choice
    label="Output pair"
    disabled={!synth}
    value={synth ? synth.outputPair[0] - 1 : null}
    columns={Math.min(4, pairs.length)}
    options={pairs}
    onselect={(first) => app.send({ type: 'setAudioOutput', first })}
  />
</Field>

<Field
  name="Buffer size"
  note={synth
    ? `${synth.bufferFrames ? `${((synth.bufferFrames / synth.sampleRate) * 1000).toFixed(1)} ms per buffer. ` : ''}Raise it if a heavy plugin crackles or shows overruns.`
    : null}
>
  <Choice
    label="Buffer size"
    disabled={!synth}
    value={synth?.bufferFrames ?? null}
    columns={3}
    options={BUFFERS.map((n) => ({ id: n, label: `${n}`, tip: 'audio.buffer' as const }))}
    onselect={(frames) => app.send({ type: 'setAudioBuffer', frames: frames as (typeof BUFFERS)[number] })}
  />
</Field>

<Field
  name="SoundFont"
  mock={inert}
  note={inert
    ? 'The SoundFont the synth is playing. Switching needs an engine update; for now pass --sf2 file at launch.'
    : 'The .sf2 files in soundfonts/. General MIDI SoundFonts sound closest to the styles.'}
>
  <Choice
    label="SoundFont"
    disabled={!synth || inert}
    value={view.soundFontFile}
    columns={1}
    options={fonts}
    onselect={(file) => settings.send({ type: 'setSoundFont', file })}
  />
</Field>

<Field name="Master volume" genos="MASTER VOLUME" note={master === null ? 'Only with the built-in synth.' : '100 is unity. A soft clipper guards the output above it.'}>
  <HSlider
    label="Master volume"
    tip="mixer.master"
    value={master ?? 0}
    unity={100}
    disabled={master === null}
    onchange={(volume) => app.send({ type: 'setMasterVolume', volume })}
  />
</Field>

<style>
  .gate.off {
    opacity: 0.5;
  }
</style>
