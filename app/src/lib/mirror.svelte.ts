// The link between a drawer and the Launchkey mirror: hovering or focusing something in a
// drawer that lives on the hardware highlights where it is on the mirror. UI-only state;
// the engine doesn't know about it.
//
// - `panelFader`: 0–3 = Right 1, Right 2, Right 3, Left: fader 1–4 and the button under
//   it on the Panel fader page. The mirror highlights that strip while the faders are on
//   the Panel page, and the fader page button (under the master fader) while they're on
//   the Style page, since that's what you'd press to get there.

class MirrorLink {
  panelFader = $state<number | null>(null)
}

export const mirror = new MirrorLink()
