/**
 * Button's hold ends when it can't continue (SPEC D19): props that change mid-gesture, which a
 * story's play can't do.
 */
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import Button from './Button.svelte'

const INIT = { pointerId: 1, button: 0, clientX: 0, clientY: 0 }

function held() {
  const onhold = vi.fn()
  const props = { label: 'Tempo', symbol: 'plus' as const, size: 'band' as const, hold: true, onhold }
  const view = render(Button, props)
  const button = screen.getByRole('button')
  fireEvent.pointerDown(button, INIT)
  expect(onhold).toHaveBeenLastCalledWith(true)
  return { ...view, button, onhold, props }
}

afterEach(cleanup)

describe('Button hold', () => {
  it('1: ends when disabled turns true', async () => {
    const { button, onhold, props, rerender } = held()
    await rerender({ ...props, disabled: true })
    expect(onhold).toHaveBeenCalledTimes(2)
    expect(onhold).toHaveBeenLastCalledWith(false)
    await fireEvent.pointerUp(button, INIT)
    expect(onhold).toHaveBeenCalledTimes(2)
  })

  it('2: ends when hold turns false', async () => {
    const { button, onhold, props, rerender } = held()
    await rerender({ ...props, hold: false })
    expect(onhold).toHaveBeenCalledTimes(2)
    expect(onhold).toHaveBeenLastCalledWith(false)
    await fireEvent.pointerUp(button, INIT)
    expect(onhold).toHaveBeenCalledTimes(2)
  })

  it('3: ends on lostpointercapture', async () => {
    const { button, onhold } = held()
    await fireEvent(button, new PointerEvent('lostpointercapture', { pointerId: 1 }))
    expect(onhold).toHaveBeenCalledTimes(2)
    expect(onhold).toHaveBeenLastCalledWith(false)
    await fireEvent.pointerUp(button, INIT)
    expect(onhold).toHaveBeenCalledTimes(2)
  })

  it('4: ends on unmount', () => {
    const { onhold, unmount } = held()
    unmount()
    expect(onhold).toHaveBeenCalledTimes(2)
    expect(onhold).toHaveBeenLastCalledWith(false)
  })
})
