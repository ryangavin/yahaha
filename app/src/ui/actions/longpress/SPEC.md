# longpress

## Identity (all stations)

- **Kind:** primitive (a Svelte action, not a component)
- **Built from:** —
- **Purpose:** Lets a player hold a button for a moment (or right-click it) to reach its second function, such as a part's swap mode, Loop rec, or Sound held down.
- **File:** `app/src/ui/actions/longpress/index.ts`, imported as `import { longpress } from '../actions/longpress'` (D1). Tests: `app/src/ui/actions/longpress/longpress.test.ts`.
- **Boards:** no drawing. The behaviour comes from the lamp row's notes: `Stage-Dark.dc.html:277-284` (part On/Off "Long press: swap mode", Looper "Long press: Loop rec", Sound "hold"); light `Stage-Light.dc.html:253-260`.
- **Used by:** `LampButton` (`onlongpress`, `onlongrelease`), `Button` (`onlongpress`, `onlongrelease`). No other component runs its own long-press timer.
- **Not this action's job:** no store, no API, no Tauri, no drawing (it adds no class, attribute or style). It doesn't know what the second function is: it calls the callbacks it is given. Not Shift-click (`ui.shift` is the parent's). Not repeat-while-held (Tempo ± use `Button`'s `onhold` and `app/src/lib/tempoHold.ts`).

## API (Component station)

```ts
import type { ActionReturn } from 'svelte/action'

export type LongPressParams = {
  onlongpress?: () => void
  onlongrelease?: () => void
  disabled?: boolean
}
export const LONG_PRESS_FALLBACK_MS = 350
export const MOVE_TOLERANCE_PX = 4
export function parseDuration(value: string): number | null
export function longpress(node: HTMLElement, params?: LongPressParams): ActionReturn<LongPressParams>
```

Use: `<button use:longpress={{ onlongpress, onlongrelease, disabled }}>`. Every JSDoc above becomes the module's docs.

### Parameters

| Param | Type | Default | Meaning |
|---|---|---|---|
| `onlongpress` | `(() => void) \| undefined` | — | Called once when a press has been held for the long-press time without moving more than 4px, while the pointer is still down; or on a right-click (`contextmenu`). Undefined: the action behaves exactly as `disabled` (D10). |
| `onlongrelease` | `(() => void) \| undefined` | — | Called once when a press that fired `onlongpress` ends: its `pointerup` or `pointercancel`, a right-click's release, or `destroy` while it is still held. Never called for a press that didn't fire. |
| `disabled` | `boolean` | `false` | No long press: a `pointerdown` starts nothing (no timer, no `window` listeners, no swallow) and a `contextmenu` calls nothing (its native menu is still prevented). Clicks pass through untouched. |

`update(params)` replaces all three at once (a param left out is undefined). The callbacks called are always the latest ones given. "Off" below means `disabled` is true or `onlongpress` is undefined.

### New tokens

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--long-press` | `350ms` | `350ms` | the hold time before `onlongpress` (in `scale.css`, theme-independent) |

It is not in `app/src/ui/tokens/*` today; it lands in the orchestrator's tokens contract PR before this action is built. The action reads it by name and never hard-codes 350 except as `LONG_PRESS_FALLBACK_MS` (D2). No contrast rows (the action draws nothing).

### Exports besides the action

| Export | Value / signature | Meaning |
|---|---|---|
| `LONG_PRESS_FALLBACK_MS` | `350` | Used when the `--long-press` token can't be read (jsdom, a missing token). |
| `MOVE_TOLERANCE_PX` | `4` | A press that moves further than this from where it went down is not a long press. |
| `parseDuration(value)` | `string → number \| null` | Parses a CSS time: trims, then `<n>ms` → n, `<n>s` → n × 1000 (n a non-negative decimal); anything else (empty, `abc`, `-5ms`, a bare number) → `null`. `'350ms'` → 350, `'0.5s'` → 500, `' 200ms '` → 200. |

### Behaviour

- **The time.** At each `pointerdown` that starts a press, the action calls `window.getComputedStyle(node).getPropertyValue('--long-press')` (always through `window`, so a test can spy on it) and parses the result with `parseDuration`; `null` → `LONG_PRESS_FALLBACK_MS` (D2). The time is fixed for that press.
- **A press in progress** is either a primary press (from its `pointerdown` until its `pointerup` or `pointercancel`, fired or not) or a held right-click (from a `contextmenu` with the right button down until that button's release, below). At most one is in progress at a time.
- **Start.** A `pointerdown` on the node with `button === 0` (the primary button, a pen or a touch), while not off and while no press is in progress, starts a press: it records `pointerId`, `clientX`, `clientY`, starts one `setTimeout` for the time, and adds `pointermove`, `pointerup` and `pointercancel` listeners on `window`. Any other button, a `pointerdown` while off, or a second pointer while a press is in progress starts nothing.
- **Which `pointerdown` clears the swallow (D14).** Every `pointerdown` on the node while no press is in progress clears a pending swallow, whatever its button and whether or not it starts a press (a right or middle button, or one while off, included). A `pointerdown` while a press is in progress (a second finger, a second pointer) changes nothing, so it can't undo the swallow of the press that is still held.
- **No pointer capture (D3).** The action never calls `setPointerCapture`; it follows the press with the `window` listeners, filtered by `pointerId`, so the browser's own click rules (a release outside the button is no click) are unchanged.
- **Cancel before it fires.** Before the time is up, a `pointermove` of the same pointer whose distance from the start, `Math.hypot(dx, dy)`, is greater than `MOVE_TOLERANCE_PX` (4 exactly is still a press), or its `pointerup` or `pointercancel`, clears the timeout and removes the `window` listeners. Nothing is called; the click that follows a `pointerup` goes through as an ordinary click.
- **Fire.** When the timeout runs out, the action calls `onlongpress()` and marks the next click swallowed. From then on moves don't matter. The press's `pointerup` or `pointercancel` calls `onlongrelease()` and removes the `window` listeners.
- **Swallow (D4).** The action adds one `click` listener and one `keydown` listener on the node, both with `{ capture: true }`, when it is created. While a swallow is pending, the first `click` that reaches it calls `event.stopImmediatePropagation()` and `event.preventDefault()` and clears the swallow, so the component's own click handler (Svelte's delegated `onclick` included) never sees it: a long press never also toggles or presses. The swallow is also cleared by a `pointerdown` as above and by any `keydown` on the node (the keydown itself is untouched), so a release outside the button (no click) can't eat a later real click or a Space / Enter press.
- **Right-click (D5).** A `contextmenu` event on the node always gets `preventDefault()` (no browser menu on these controls), even while off. If not off and no press is in progress, the action calls `onlongpress()`, then:
  - if `event.buttons & 2` (the right button is still down: macOS, where the menu opens on press), the right-click is held: it is a press in progress, and `onlongrelease()` comes on the first `window` `pointerup` whose `button === 2`, or the first `window` `pointercancel` of any pointer (D11). A `pointerup` of another button doesn't end it;
  - otherwise (Windows, where it comes after the release, or the keyboard's Menu key / Shift+F10), `onlongrelease()` straight after `onlongpress()`, and nothing is in progress.
  A right-click marks nothing swallowed (browsers send no `click` for it). A `contextmenu` while a press is in progress (a touch long-press menu during a primary press, or a second `contextmenu` during a held right-click) only gets `preventDefault()`; a primary `pointerdown` during a held right-click starts nothing (D12).
- **Keyboard (D6).** No hold-to-long-press on Space or Enter (key repeat makes it unreliable, and Space / Enter must stay the click). The keyboard route is the `contextmenu` event the platform sends for the Menu key or Shift+F10 on a focused control, handled as a right-click with the button up.
- **Turned off while pressing.** An `update` that turns the action off (`disabled: true`, or `onlongpress` undefined) during a primary press that hasn't fired cancels it as above (timeout cleared, `window` listeners removed, nothing called). A primary press that already fired, and a held right-click, still get their `onlongrelease()` on release (the latest `onlongrelease` given, if any), so a momentary function (Sound held) always ends (D13).
- **Destroy.** Clears the timeout, removes the node's and `window`'s listeners, and drops any swallow. If a press (or a right-click with the button down) has fired and not yet been released, it calls `onlongrelease()` once first. After `destroy`, no event calls anything.
- **Real-time and allocation.** Runs on the UI thread only; adding listeners per press is fine. The timeout measures a gesture, not motion, so it is allowed (kit › Interaction conventions; axiom 10 is about drawing).

### Accessibility

- The action adds no role, name or attribute. Components that use it say in their accessible name or tooltip what the long press does (e.g. "Right 1 on. Long press: swap mode"), as the parent's `name` or tooltip key.
- Keyboard: the `contextmenu` route above. Every long-press function also has another way in the app (kit › Lamp row: Shift-click, the Channel page, the Looper page), so nothing is pointer-only.

## Tests (instead of Stories)

An action has no stories. The checks are vitest unit tests in `app/src/ui/actions/longpress/longpress.test.ts`, run with `npx vitest run src/ui/actions/longpress`. Each test: `vi.useFakeTimers()`; a fresh `<button>` appended to `document.body`; `const handle = longpress(button, { onlongpress, onlongrelease })` with `vi.fn()` callbacks; a plain bubble-phase `click` listener `clicked = vi.fn()` added to the button after the action (it stands for the component's handler); events built with `new PointerEvent(type, { bubbles: true, cancelable: true, button: 0, buttons: 1, pointerId: 1, clientX, clientY })` (jsdom 30, which vitest uses, has `PointerEvent` with `pointerId`, `button` and `clientX`; L4), `new MouseEvent('click' | 'contextmenu', { bubbles: true, cancelable: true, buttons })` and `new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key })`, dispatched on the button with `dispatchEvent` (they bubble to `window`). A case's values not listed (`button`, `buttons`, `pointerId`, `clientX`, `clientY`) are those defaults, with `clientX` and `clientY` 10. `afterEach`: `handle.destroy()`, remove the button, `vi.useRealTimers()`, `vi.restoreAllMocks()`. jsdom has no `--long-press` value, so the time is the 350 ms fallback unless a test stubs it.

| # | Case | Steps | Expect |
|---|---|---|---|
| 1 | Fires at the time, not on release | `pointerdown` at (10, 10); advance 349 ms | `onlongpress` not called; advance 1 ms → called once; `onlongrelease` not called |
| 2 | Release after firing | case 1, then `pointerup` | `onlongrelease` called once, after `onlongpress` (`invocationCallOrder`) |
| 3 | Short press is a click | `pointerdown`; advance 200; `pointerup`; `click`; advance 1000 | `onlongpress` and `onlongrelease` never called; `clicked` called once; the click's `defaultPrevented` is `false`; `vi.getTimerCount()` is 0 |
| 4 | Moving 5px cancels | `pointerdown` at (10, 10); advance 100; `pointermove` at (13, 14); advance 1000 | `onlongpress` not called |
| 5 | Moving 4px doesn't | `pointerdown` at (10, 10); `pointermove` at (14, 10); advance 350 | `onlongpress` called once |
| 6 | Move after firing is ignored | case 1, then `pointermove` at (60, 60), then `pointerup` | `onlongrelease` called once |
| 7 | The click after a fire is swallowed exactly once | case 2, then `click`, then `click` | first click: `clicked` not called, `defaultPrevented` `true`; second: `clicked` called once |
| 8 | Swallow cleared by a new press | case 2 (no click); `pointerdown`; advance 100; `pointerup`; `click` | `clicked` called once; `onlongpress` called once in all |
| 9 | Swallow cleared by a key | case 2; `new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: ' ' })` on the button; `click` | `clicked` called once; the keydown's `defaultPrevented` is `false` |
| 10 | `pointercancel` before firing | `pointerdown`; advance 100; `pointercancel`; advance 1000 | neither callback called |
| 11 | `pointercancel` after firing | case 1; `pointercancel` | `onlongrelease` called once |
| 12 | Other pointers and buttons | `pointerdown` with `button: 2`; advance 1000. Then a press with `pointerId: 1`, and at 100 ms a `pointerdown` with `pointerId: 2`; at 200 a `pointerup` with `pointerId: 2`; advance 150 | first: nothing called. Second: `onlongpress` called once at 350 (pointer 2 neither started nor ended the press) |
| 13 | Right-click, button up | `contextmenu` with `buttons: 0` | `defaultPrevented` `true`; `onlongpress` then `onlongrelease`, each once, in that order; no timer pending; a following `click` reaches `clicked` |
| 14 | Right-click, button held | `contextmenu` with `buttons: 2` | `onlongpress` once, `onlongrelease` not yet; `pointerup` (`button: 2`, `buttons: 0`) → `onlongrelease` once |
| 15 | Disabled | `longpress(button, { onlongpress, onlongrelease, disabled: true })`; `pointerdown`; advance 1000; `pointerup`; `click`; `contextmenu` | no callback called; `clicked` called once; the `contextmenu`'s `defaultPrevented` is `true` |
| 16 | Update swaps callbacks | `pointerdown`; advance 100; `handle.update({ onlongpress: other, onlongrelease })`; advance 250 | `other` called once, the first `onlongpress` never |
| 17 | Update to disabled mid-press | `pointerdown`; advance 100; `update({ onlongpress, onlongrelease, disabled: true })`; advance 1000 | nothing called. Same after firing (case 1, then update to disabled, then `pointerup`): `onlongrelease` called once |
| 18 | Destroy while pending | `pointerdown`; advance 100; `handle.destroy()`; advance 1000; `pointerup`; `click` | nothing called; `vi.getTimerCount()` 0; `clicked` called once |
| 19 | Destroy while held after firing | case 1; `destroy()`; `pointerup` | `onlongrelease` called exactly once (by `destroy`) |
| 20 | The token sets the time (the action calls `window.getComputedStyle`) | `vi.spyOn(window, 'getComputedStyle').mockReturnValue({ getPropertyValue: () => '500ms' } as unknown as CSSStyleDeclaration)`; `pointerdown`; advance 499 | not called; advance 1 → called once |
| 21 | `parseDuration` | pure | `'350ms'` → 350, `'0.5s'` → 500, `' 200ms '` → 200, `'0ms'` → 0, `''` → null, `'abc'` → null, `'-5ms'` → null, `'350'` → null |
| 22 | Ignores events elsewhere | `pointerdown` on another element in `document.body`; advance 1000 | nothing called |
| 23 | An ignored `pointerdown` clears the swallow | case 2 (no click); `pointerdown` with `button: 2, buttons: 2`; `pointerup` with `button: 2, buttons: 0`; `click` | `clicked` called once; `onlongpress` called once in all |
| 24 | A second pointer during a fired press keeps the swallow | case 1; `pointerdown` with `pointerId: 2`; `pointerup` with `pointerId: 1`; `click` | `onlongrelease` called once; `clicked` not called |
| 25 | Held right-click ends only on the right button | `contextmenu` with `buttons: 2`; `pointerup` with `button: 0, buttons: 2`; then `pointerup` with `button: 2, buttons: 0` | after the first `pointerup`, `onlongrelease` not called; after the second, called once. Again with `pointercancel` (`pointerId: 7`) in place of the second `pointerup`: called once |
| 26 | Held right-click is a press in progress | `contextmenu` with `buttons: 2`; second `contextmenu` with `buttons: 2`; `pointerdown` (`button: 0`); advance 1000; `pointerup` with `button: 2, buttons: 0` | second `contextmenu` `defaultPrevented` `true`; `onlongpress` called once in all; `onlongrelease` called once |
| 27 | No `onlongpress` is off | `longpress(button, { onlongrelease })`; `pointerdown`; advance 1000; `pointerup`; `click`; `contextmenu` | `onlongrelease` never called; `clicked` called once; `vi.getTimerCount()` 0 after the `pointerdown`; the `contextmenu`'s `defaultPrevented` is `true` |
| 28 | Turned off during a held right-click | `contextmenu` with `buttons: 2`; `update({ onlongpress, onlongrelease, disabled: true })`; `pointerup` with `button: 2, buttons: 0` | `onlongrelease` called once |

The components' stories cover the action inside a real component (LampButton › `LongPress`, `RightClick`; Button › `LongPress`).

## Done when (Inspect station)

- Every case in the table exists and passes (`npx vitest run src/ui/actions/longpress`).
- `index.ts` uses no timer other than the one press timeout, adds nothing to the DOM, and reads the time only from `--long-press` (with the 350 fallback). The token itself comes from the tokens contract PR (D9); until it lands every press uses the fallback, which is the same 350 ms.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · File.** The action lives at `app/src/ui/actions/longpress/index.ts` (this folder holds its spec and test too), so the import path is the kit's `actions/longpress`; kit.md's `actions/longpress.ts` is read as that path.
- **D2 · Reading the time.** The action reads `--long-press` with `getComputedStyle` at each press (axiom 2: the duration is a token) and falls back to 350 ms when it can't be read, which is always the case in jsdom; there is no `ms` parameter.
- **D3 · No pointer capture.** The press is followed by `window` listeners for that `pointerId`, not pointer capture, so dragging off a button and letting go still cancels its click as it does natively, and the release is still seen outside the button.
- **D4 · Swallowing the click.** A capture-phase `click` listener on the node stops the one click after a fired long press (`stopImmediatePropagation` and `preventDefault`); the swallow ends at the next `pointerdown` or `keydown` instead of on a timer.
- **D5 · Right-click release.** A right-click calls `onlongpress` and then `onlongrelease` on the right button's release when it is still down (macOS), else at once (Windows, keyboard), so a momentary function such as Sound lasts as long as the right button is held where the platform allows it.
- **D6 · Keyboard.** There is no keyboard hold; the Menu key or Shift+F10 (the platform's `contextmenu`) is the keyboard long press, and every long-press function has another app route.
- **D7 · Primary button only.** Only `button === 0` starts the timer; the other buttons reach the long press through `contextmenu` alone, so a right-click never fires twice.
- **D8 · Destroy ends a held press.** Unmounting a component during a fired press calls `onlongrelease`, so a momentary state never sticks.
- **D9 · L1, the token.** `--long-press` (350ms, both themes) is listed under New tokens and lands in the orchestrator's tokens contract PR; this spec never edits `tokens/*`.
- **D10 · No `onlongpress` is off.** With `onlongpress` undefined the action behaves as `disabled` (no timer, no swallow, `contextmenu` still prevented); an `onlongrelease` alone is never called.
- **D11 · Right-click release by button.** A held right-click ends on the first `pointerup` with `button === 2` or any `pointercancel`, not by `pointerId` (a `contextmenu` carries no reliable `pointerId` in every browser).
- **D12 · A held right-click is a press in progress.** While it is held, a second `contextmenu` and a primary `pointerdown` start nothing, so `onlongpress` never fires twice before one `onlongrelease`.
- **D13 · Off mid-press.** Turning the action off cancels a primary press that hasn't fired; a fired press or a held right-click still gets `onlongrelease` on release.
- **D14 · Swallow and ignored presses.** Any `pointerdown` while nothing is in progress clears the swallow, whatever its button; one during a press in progress (a second pointer) does not, so the held press's click is still swallowed.
- **D15 · L4, the test environment.** The unit tests use jsdom 30's `PointerEvent` with fake timers; the action calls no pointer-capture API, so nothing needs guarding; the components' long-press plays use real time with `waitFor` (timeout 1000).
