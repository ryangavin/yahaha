import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest'
import { longpress, parseDuration, type LongPressParams } from '.'

type Init = { button?: number; buttons?: number; pointerId?: number; clientX?: number; clientY?: number }

let button: HTMLButtonElement
let onlongpress: Mock<() => void>
let onlongrelease: Mock<() => void>
let clicked: Mock<(event: MouseEvent) => void>
let handle: { update?: (params: LongPressParams) => void; destroy?: () => void }

function pointer(type: string, init: Init = {}, target: Element = button) {
  const event = new PointerEvent(type, {
    bubbles: true,
    cancelable: true,
    button: 0,
    buttons: 1,
    pointerId: 1,
    clientX: 10,
    clientY: 10,
    ...init,
  })
  target.dispatchEvent(event)
  return event
}

function click() {
  const event = new MouseEvent('click', { bubbles: true, cancelable: true })
  button.dispatchEvent(event)
  return event
}

function contextmenu(buttons = 0) {
  const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, buttons })
  button.dispatchEvent(event)
  return event
}

/** Mounts the action, then the stand-in for the component's own click handler. */
function mount(params: LongPressParams = { onlongpress, onlongrelease }) {
  handle = longpress(button, params)
  button.addEventListener('click', clicked)
}

/** Case 1: a press held to the time. */
function fire() {
  pointer('pointerdown')
  vi.advanceTimersByTime(350)
}

beforeEach(() => {
  vi.useFakeTimers()
  button = document.createElement('button')
  document.body.append(button)
  onlongpress = vi.fn()
  onlongrelease = vi.fn()
  clicked = vi.fn()
})

afterEach(() => {
  handle.destroy?.()
  button.remove()
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe('longpress', () => {
  it('1: fires at the time, not on release', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(349)
    expect(onlongpress).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(onlongpress).toHaveBeenCalledTimes(1)
    expect(onlongrelease).not.toHaveBeenCalled()
  })

  it('2: release after firing', () => {
    mount()
    fire()
    pointer('pointerup', { buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
    expect(onlongrelease.mock.invocationCallOrder[0]).toBeGreaterThan(onlongpress.mock.invocationCallOrder[0])
  })

  it('3: a short press is a click', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(200)
    pointer('pointerup', { buttons: 0 })
    const event = click()
    vi.advanceTimersByTime(1000)
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(event.defaultPrevented).toBe(false)
    expect(vi.getTimerCount()).toBe(0)
  })

  it('4: moving 5px cancels', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    pointer('pointermove', { clientX: 13, clientY: 14 })
    vi.advanceTimersByTime(1000)
    expect(onlongpress).not.toHaveBeenCalled()
  })

  it("5: moving 4px doesn't", () => {
    mount()
    pointer('pointerdown')
    pointer('pointermove', { clientX: 14, clientY: 10 })
    vi.advanceTimersByTime(350)
    expect(onlongpress).toHaveBeenCalledTimes(1)
  })

  it('6: a move after firing is ignored', () => {
    mount()
    fire()
    pointer('pointermove', { clientX: 60, clientY: 60 })
    pointer('pointerup', { buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('7: the click after a fire is swallowed exactly once', () => {
    mount()
    fire()
    pointer('pointerup', { buttons: 0 })
    const first = click()
    expect(clicked).not.toHaveBeenCalled()
    expect(first.defaultPrevented).toBe(true)
    click()
    expect(clicked).toHaveBeenCalledTimes(1)
  })

  it('8: the swallow is cleared by a new press', () => {
    mount()
    fire()
    pointer('pointerup', { buttons: 0 })
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    pointer('pointerup', { buttons: 0 })
    click()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(onlongpress).toHaveBeenCalledTimes(1)
  })

  it('9: the swallow is cleared by a key', () => {
    mount()
    fire()
    pointer('pointerup', { buttons: 0 })
    const key = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: ' ' })
    button.dispatchEvent(key)
    click()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(key.defaultPrevented).toBe(false)
  })

  it('10: pointercancel before firing', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    pointer('pointercancel', { buttons: 0 })
    vi.advanceTimersByTime(1000)
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()
  })

  it('11: pointercancel after firing', () => {
    mount()
    fire()
    pointer('pointercancel', { buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('12: other pointers and buttons', () => {
    mount()
    pointer('pointerdown', { button: 2, buttons: 2 })
    vi.advanceTimersByTime(1000)
    expect(onlongpress).not.toHaveBeenCalled()
    pointer('pointerup', { button: 2, buttons: 0 })

    pointer('pointerdown', { pointerId: 1 })
    vi.advanceTimersByTime(100)
    pointer('pointerdown', { pointerId: 2 })
    vi.advanceTimersByTime(100)
    pointer('pointerup', { pointerId: 2, buttons: 0 })
    vi.advanceTimersByTime(149)
    expect(onlongpress).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(onlongpress).toHaveBeenCalledTimes(1)
  })

  it('13: right-click with the button up', () => {
    mount()
    const event = contextmenu(0)
    expect(event.defaultPrevented).toBe(true)
    expect(onlongpress).toHaveBeenCalledTimes(1)
    expect(onlongrelease).toHaveBeenCalledTimes(1)
    expect(onlongrelease.mock.invocationCallOrder[0]).toBeGreaterThan(onlongpress.mock.invocationCallOrder[0])
    expect(vi.getTimerCount()).toBe(0)
    click()
    expect(clicked).toHaveBeenCalledTimes(1)
  })

  it('14: right-click with the button held', () => {
    mount()
    contextmenu(2)
    expect(onlongpress).toHaveBeenCalledTimes(1)
    expect(onlongrelease).not.toHaveBeenCalled()
    pointer('pointerup', { button: 2, buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('15: disabled', () => {
    mount({ onlongpress, onlongrelease, disabled: true })
    pointer('pointerdown')
    vi.advanceTimersByTime(1000)
    pointer('pointerup', { buttons: 0 })
    click()
    const menu = contextmenu(0)
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(menu.defaultPrevented).toBe(true)
  })

  it('16: update swaps the callbacks', () => {
    mount()
    const other = vi.fn()
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    handle.update?.({ onlongpress: other, onlongrelease })
    vi.advanceTimersByTime(250)
    expect(other).toHaveBeenCalledTimes(1)
    expect(onlongpress).not.toHaveBeenCalled()
  })

  it('17: update to disabled mid-press', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    handle.update?.({ onlongpress, onlongrelease, disabled: true })
    vi.advanceTimersByTime(1000)
    pointer('pointerup', { buttons: 0 })
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()

    handle.update?.({ onlongpress, onlongrelease })
    fire()
    handle.update?.({ onlongpress, onlongrelease, disabled: true })
    pointer('pointerup', { buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('18: destroy while pending', () => {
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(100)
    handle.destroy?.()
    vi.advanceTimersByTime(1000)
    pointer('pointerup', { buttons: 0 })
    click()
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()
    expect(vi.getTimerCount()).toBe(0)
    expect(clicked).toHaveBeenCalledTimes(1)
  })

  it('19: destroy while held after firing', () => {
    mount()
    fire()
    handle.destroy?.()
    pointer('pointerup', { buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('20: the token sets the time', () => {
    vi.spyOn(window, 'getComputedStyle').mockReturnValue({
      getPropertyValue: () => '500ms',
    } as unknown as CSSStyleDeclaration)
    mount()
    pointer('pointerdown')
    vi.advanceTimersByTime(499)
    expect(onlongpress).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(onlongpress).toHaveBeenCalledTimes(1)
  })

  it('21: parseDuration', () => {
    mount()
    expect(parseDuration('350ms')).toBe(350)
    expect(parseDuration('0.5s')).toBe(500)
    expect(parseDuration(' 200ms ')).toBe(200)
    expect(parseDuration('0ms')).toBe(0)
    expect(parseDuration('')).toBeNull()
    expect(parseDuration('abc')).toBeNull()
    expect(parseDuration('-5ms')).toBeNull()
    expect(parseDuration('350')).toBeNull()
  })

  it('22: ignores events elsewhere', () => {
    mount()
    const other = document.createElement('div')
    document.body.append(other)
    pointer('pointerdown', {}, other)
    vi.advanceTimersByTime(1000)
    other.remove()
    expect(onlongpress).not.toHaveBeenCalled()
    expect(onlongrelease).not.toHaveBeenCalled()
  })

  it('23: an ignored pointerdown clears the swallow', () => {
    mount()
    fire()
    pointer('pointerup', { buttons: 0 })
    pointer('pointerdown', { button: 2, buttons: 2 })
    pointer('pointerup', { button: 2, buttons: 0 })
    click()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(onlongpress).toHaveBeenCalledTimes(1)
  })

  it('24: a second pointer during a fired press keeps the swallow', () => {
    mount()
    fire()
    pointer('pointerdown', { pointerId: 2 })
    pointer('pointerup', { pointerId: 1, buttons: 0 })
    click()
    expect(onlongrelease).toHaveBeenCalledTimes(1)
    expect(clicked).not.toHaveBeenCalled()
  })

  it('25: a held right-click ends only on the right button', () => {
    mount()
    contextmenu(2)
    pointer('pointerup', { button: 0, buttons: 2 })
    expect(onlongrelease).not.toHaveBeenCalled()
    pointer('pointerup', { button: 2, buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)

    contextmenu(2)
    pointer('pointerup', { button: 0, buttons: 2 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
    pointer('pointercancel', { pointerId: 7, buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(2)
  })

  it('26: a held right-click is a press in progress', () => {
    mount()
    contextmenu(2)
    const second = contextmenu(2)
    pointer('pointerdown')
    vi.advanceTimersByTime(1000)
    pointer('pointerup', { button: 2, buttons: 0 })
    expect(second.defaultPrevented).toBe(true)
    expect(onlongpress).toHaveBeenCalledTimes(1)
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })

  it('27: no onlongpress is off', () => {
    mount({ onlongrelease })
    pointer('pointerdown')
    expect(vi.getTimerCount()).toBe(0)
    vi.advanceTimersByTime(1000)
    pointer('pointerup', { buttons: 0 })
    click()
    const menu = contextmenu(0)
    expect(onlongrelease).not.toHaveBeenCalled()
    expect(clicked).toHaveBeenCalledTimes(1)
    expect(menu.defaultPrevented).toBe(true)
  })

  it('28: turned off during a held right-click', () => {
    mount()
    contextmenu(2)
    handle.update?.({ onlongpress, onlongrelease, disabled: true })
    pointer('pointerup', { button: 2, buttons: 0 })
    expect(onlongrelease).toHaveBeenCalledTimes(1)
  })
})
