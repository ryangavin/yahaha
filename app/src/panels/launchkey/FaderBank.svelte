<!--
  The 8 faders + master, with the button under each, as the active fader page maps them:
  Panel = Right 1–3 and Left (5–8 unused), Style = the 8 band parts. Button lights follow
  src/launchkey.rs: blue on Panel, green on Style, lit while the part plays; the master
  button shows the page's colour and switches the page. With Shift, the Panel page's
  buttons 1–4 select the part to edit.
-->
<script lang="ts">
  import type { TipKey } from '../../help/tooltips'
  import type { AppCmd, Pad, Rgb } from '../../lib/api/types'
  import { app, ui } from '../../lib/store.svelte'
  import Fader from '../../lib/ui/Fader.svelte'
  import HwButton from '../../lib/ui/HwButton.svelte'

  const BLUE: Rgb = [10, 50, 127]
  const GREEN: Rgb = [0, 127, 16]
  const PANEL_TIPS: TipKey[] = ['mixer.panel.right1', 'mixer.panel.right2', 'mixer.panel.right3', 'mixer.panel.left']
  const ON_TIPS: TipKey[] = ['part.right1.on', 'part.right2.on', 'part.right3.on', 'part.left.on']
  const SELECT_TIPS: TipKey[] = ['part.right1.select', 'part.right2.select', 'part.right3.select', 'part.left.select']
  const SHORT = ['R1', 'R2', 'R3', 'Left']

  interface Strip {
    label: string
    value: number
    waiting: boolean
    lit: boolean
    faderTip: TipKey
    set: ((v: number) => void) | null
    button: { tip: TipKey; cmd: AppCmd | null; led: Pick<Pad, 'rgb' | 'level' | 'anim'>; text: string }
  }

  const s = $derived(app.state)
  const page = $derived(s.mixer.faderPage)
  const look = (rgb: Rgb, on: boolean, available = true): Pick<Pad, 'rgb' | 'level' | 'anim'> => ({
    rgb,
    level: !available ? 'off' : on ? 'bright' : 'dim',
    anim: 'solid',
  })

  const strips = $derived.by((): Strip[] => {
    if (page === 'style') {
      return s.mixer.styleParts.map((p, i) => ({
        label: p.name,
        value: p.volume,
        waiting: p.waiting,
        lit: p.on,
        faderTip: 'mixer.style.volume',
        set: (v) => app.send({ type: 'setStylePartVolume', part: i, volume: v }),
        button: { tip: 'mixer.style.mute', cmd: { type: 'toggleStylePart', part: i }, led: look(GREEN, p.on), text: p.mutedByManualBass ? 'MB' : p.on ? '' : 'Mute' },
      }))
    }
    return Array.from({ length: 8 }, (_, i): Strip => {
      const p = s.keyboardParts[i]
      if (!p) {
        return {
          label: '—', value: 0, waiting: false, lit: false, faderTip: 'launchkey.fader_unused', set: null,
          button: { tip: 'launchkey.fader_unused', cmd: null, led: look(BLUE, false, false), text: '' },
        }
      }
      return {
        label: SHORT[i],
        value: p.volume,
        waiting: p.waiting,
        lit: p.sounding,
        faderTip: PANEL_TIPS[i],
        set: (v) => app.send({ type: 'setPartVolume', part: i, volume: v }),
        button: ui.shift
          ? { tip: SELECT_TIPS[i], cmd: { type: 'selectPart', part: i }, led: look(BLUE, p.selected), text: 'Edit' }
          : { tip: ON_TIPS[i], cmd: { type: 'togglePart', part: i }, led: look(BLUE, p.sounding), text: '' },
      }
    })
  })
  const pageRgb = $derived(page === 'panel' ? BLUE : GREEN)
</script>

<div class="bank" role="group" aria-label="Faders, {page === 'panel' ? 'Panel' : 'Style'} page">
  {#each strips as st, i (i)}
    <div class="strip">
      <Fader value={st.value} tip={st.faderTip} label={st.label} pickup={st.waiting} lit={st.lit} disabled={!st.set} onchange={(v) => st.set?.(v)} />
      <HwButton tip={st.button.tip} led={st.button.led} label="{st.label} button" onclick={() => st.button.cmd && app.send(st.button.cmd)}>
        <span class="btxt">{st.button.text}</span>
      </HwButton>
    </div>
  {/each}
  <div class="strip master">
    <Fader value={s.mixer.master ?? 0} tip="mixer.master" label="Master" pickup={s.mixer.masterWaiting} disabled={s.mixer.master === null} onchange={(v) => app.send({ type: 'setMasterVolume', volume: v })} />
    <HwButton tip="mixer.page" led={look(pageRgb, true)} label="Fader page" onclick={() => app.send({ type: 'toggleFaderPage' })}>
      <span class="btxt">{page === 'panel' ? 'Panel' : 'Style'}</span>
    </HwButton>
  </div>
</div>

<style>
  .bank {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr)) minmax(0, 1.15fr);
    gap: 0.4em;
    height: 100%;
  }
  .strip {
    display: grid;
    grid-template-rows: 1fr auto;
    gap: 0.55em;
    min-width: 0;
  }
  .master {
    padding-left: 0.4em;
    border-left: 1px solid var(--seam);
  }
  .btxt {
    font-size: 0.72em;
    min-width: 1em;
  }
</style>
