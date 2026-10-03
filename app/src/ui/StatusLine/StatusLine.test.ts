// StatusLine across prop changes, which a story's play can't do: the button keeps its focus
// through a new seq or text (D10), clears to an empty line, and takes its width only when given.

import { cleanup, render, screen } from '@testing-library/svelte'
import { afterEach, describe, expect, it } from 'vitest'
import StatusLine from './StatusLine.svelte'

afterEach(cleanup)

const ERROR = "Can't delete Sunday drive: it's the loaded rack."
const NOTICE = 'Added SlowWalker.T552 to the library and loaded it.'

describe('StatusLine', () => {
  it('keeps the focused button across new messages and clears to an empty line', async () => {
    const { rerender } = render(StatusLine, { props: { text: ERROR, error: true, seq: 4 } })
    const button = screen.getByRole('button', { name: `Error: ${ERROR}` })
    button.focus()

    await rerender({ seq: 5 })
    expect(document.activeElement).toBe(button)
    expect(screen.getByRole('button', { name: `Error: ${ERROR}` })).toBe(button)

    await rerender({ text: NOTICE, error: false, seq: 6 })
    expect(document.activeElement).toBe(button)
    expect(screen.getByRole('button', { name: NOTICE })).toBe(button)
    expect(document.querySelector('[data-hue="warn"]')).toBeNull()

    await rerender({ text: null })
    expect(screen.queryByRole('button')).toBeNull()
    expect(screen.getByRole('status').textContent).toBe('')
  })

  it('sets an inline width only when given', () => {
    render(StatusLine, { props: { text: NOTICE } })
    expect(screen.getByRole('status').style.width).toBe('')
    cleanup()
    render(StatusLine, { props: { text: NOTICE, width: 600 } })
    expect(screen.getByRole('status').style.width).toBe('600px')
  })
})
