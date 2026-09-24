// The settings adapter: the Settings drawer's view of the SoundFont, MIDI input, palette
// LED and style-folder settings, and the commands that change them. The engine has all of
// them (docs/app-api.md):
//
//   {"type":"setSoundFont","file":"FluidR3_GM.sf2"}   io.soundFonts, io.soundFontFile, io.soundFontLoading
//   {"type":"setMidiInputs","all":true,"names":[]}    io.sources, io.allInputs
//   {"type":"setPaletteLeds","on":true}               pads.paletteLeds
//   {"type":"rescanLibrary"}                          library.roots, library.scanning
//
// `view()` reads those fields and `send()` sends the command. `mocked` says, per setting,
// whether the engine lacks it: only an engine older than these fields does. The drawer
// then badges the setting and makes it inert, and the view shows only what that engine
// reports (the SoundFont playing, the inputs it has open), never made-up data.

import { app } from '../store.svelte'
import type { AppCmd, AppState } from './types'

export type SettingsCmd = Extract<AppCmd, { type: 'setSoundFont' | 'setMidiInputs' | 'setPaletteLeds' | 'rescanLibrary' }>

export interface MidiSource {
  name: string
  /** The Launchkey's DAW port (pads and buttons). */
  pads: boolean
  /** yahaha plays from it. */
  listening: boolean
}

export interface SettingsView {
  soundFonts: string[]
  soundFontFile: string | null
  /** A SoundFont is loading. */
  soundFontLoading: boolean
  sources: MidiSource[]
  /** null: the engine doesn't say. */
  allInputs: boolean | null
  paletteLeds: boolean | null
  roots: string[]
  scanning: boolean
  /** Per setting: the engine lacks it (an engine older than these settings). */
  mocked: { soundFont: boolean; inputs: boolean; paletteLeds: boolean; library: boolean }
}

/** The fields as an older engine may lack them. */
type Maybe = {
  io: Partial<Pick<AppState['io'], 'soundFonts' | 'soundFontFile' | 'soundFontLoading' | 'sources' | 'allInputs'>>
  pads: Partial<Pick<AppState['pads'], 'paletteLeds'>>
  library: Partial<Pick<AppState['library'], 'roots' | 'scanning'>>
}

function isPadsPort(name: string): boolean {
  return name.endsWith(' (pads)')
}

/** Which settings the engine behind `s` has. */
function mockedIn(s: AppState): SettingsView['mocked'] {
  const p = s as unknown as Maybe
  const inputs = p.io.sources === undefined
  return {
    soundFont: p.io.soundFonts === undefined,
    inputs,
    // pads.paletteLeds came before setPaletteLeds (#77), so the command's age goes by the
    // fields that came with it.
    paletteLeds: inputs || p.pads.paletteLeds === undefined,
    library: p.library.roots === undefined,
  }
}

const KIND: Record<SettingsCmd['type'], keyof SettingsView['mocked']> = {
  setSoundFont: 'soundFont',
  setMidiInputs: 'inputs',
  setPaletteLeds: 'paletteLeds',
  rescanLibrary: 'library',
}

class SettingsAdapter {
  /** Everything the drawer shows, from the engine's state. */
  view(s: AppState): SettingsView {
    const p = s as unknown as Maybe
    const mocked = mockedIn(s)
    const synthFile = s.io.synth ? `${s.io.synth.soundFont}.sf2` : null
    const sources = mocked.inputs
      ? // The inputs it has open, and all of them play.
        s.io.inputs.map((name) => ({ name, pads: isPadsPort(name), listening: true }))
      : p.io.sources!.map((x) => ({ name: x.name, pads: x.pads ?? isPadsPort(x.name), listening: x.listening }))
    return {
      // Without the list: only the one the synth plays.
      soundFonts: mocked.soundFont ? (synthFile ? [synthFile] : []) : p.io.soundFonts!,
      soundFontFile: mocked.soundFont ? synthFile : (p.io.soundFontFile ?? null),
      soundFontLoading: p.io.soundFontLoading ?? false,
      sources,
      allInputs: p.io.allInputs ?? null,
      paletteLeds: p.pads.paletteLeds ?? null,
      roots: p.library.roots ?? [],
      scanning: p.library.scanning ?? s.library.pending > 0,
      mocked,
    }
  }

  /** Send a settings command, if the engine has it. */
  send(cmd: SettingsCmd) {
    if (!mockedIn(app.state)[KIND[cmd.type]]) app.send(cmd)
  }
}

export const settings = new SettingsAdapter()
