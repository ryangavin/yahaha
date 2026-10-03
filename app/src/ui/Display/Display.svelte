<!--
  Display: the Stage's 300px display, no frame, on the ground. The art at its right edge; over it
  at the left, the style line, now playing and the sound row. Data comes in as each part's
  props; every press comes back through the callbacks here.
-->
<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import type { Action } from 'svelte/action'
  import DisplayArt from '../DisplayArt/DisplayArt.svelte'
  import NowPlaying from '../NowPlaying/NowPlaying.svelte'
  import SoundRow from '../SoundRow/SoundRow.svelte'
  import StyleLine from '../StyleLine/StyleLine.svelte'

  type StyleLineProps = ComponentProps<typeof StyleLine>
  type SoundRowProps = ComponentProps<typeof SoundRow>
  type Callbacks = 'tipAction' | `on${string}`

  type Props = {
    /** The style line: style, category, time signature, queued style, One Touch, sends. */
    styleLine: Omit<StyleLineProps, Callbacks>
    /** Now playing: the chord, the sections, the tempo and Running. */
    nowPlaying: ComponentProps<typeof NowPlaying>
    /** The sound row: the rack and the four parts. */
    soundRow: Omit<SoundRowProps, Callbacks>
    /** The app's tooltip action (`use:tip`), applied to every control. */
    tipAction?: Action<HTMLElement, string>
    /** ◀: the previous style. */
    onprev?: StyleLineProps['onprev']
    /** ▶: the next style. */
    onnext?: StyleLineProps['onnext']
    /** The style's name: opens the Browser. */
    onbrowse?: StyleLineProps['onbrowse']
    /** One Touch 1-4: applies it. */
    ononetouch?: StyleLineProps['ononetouch']
    /** The sends: opens Effects. */
    onsends?: StyleLineProps['onsends']
    /** The rack: opens the Rack page. */
    onrack?: SoundRowProps['onrack']
    /** A part's short name: opens Channel. */
    onpart?: SoundRowProps['onpart']
    /** A part's sound: opens the quick sound list. */
    onsound?: SoundRowProps['onsound']
  }

  let {
    styleLine,
    nowPlaying,
    soundRow,
    tipAction,
    onprev,
    onnext,
    onbrowse,
    ononetouch,
    onsends,
    onrack,
    onpart,
    onsound,
  }: Props = $props()
</script>

<section class="display" aria-label="Stage display">
  <div class="art"><DisplayArt /></div>
  <div class="content">
    <StyleLine {...styleLine} {tipAction} {onprev} {onnext} {onbrowse} {ononetouch} {onsends} />
    <div class="now"><NowPlaying {...nowPlaying} /></div>
    <div class="sounds"><SoundRow {...soundRow} {tipAction} {onrack} {onpart} {onsound} /></div>
  </div>
</section>

<style>
  .display {
    position: relative;
    box-sizing: border-box;
    width: var(--stage-row-width);
    height: var(--display-height);
    border: var(--line-width) solid transparent;
    background: var(--g);
    overflow: hidden;
  }
  .art {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
  }
  .content {
    position: absolute;
    top: var(--display-pad-top);
    left: var(--display-pad-left);
    display: flex;
    flex-direction: column;
    width: var(--display-content-width);
  }
  .now {
    margin-top: var(--space-8);
  }
  .sounds {
    margin-top: var(--sound-row-gap);
  }
</style>
