import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { App } from './App'

afterEach(cleanup)

describe('desktop-only app views', () => {
  it('requires the desktop app and does not imply browser data is real', async () => {
    render(<App />)
    expect(screen.getByText(/Anteprima non collegata/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /Statistiche/ }))
    expect(await screen.findByText('Statistiche non disponibili')).toBeInTheDocument()
  })

  it('keeps primary views reachable by keyboard buttons', () => {
    render(<App />)
    const history = within(screen.getByRole('navigation', { name: 'Navigazione principale' })).getByRole('button', { name: /Cronologia/ })
    history.focus()
    expect(history).toHaveFocus()
    fireEvent.click(history)
    expect(screen.getByRole('heading', { name: 'Cronologia' })).toBeInTheDocument()
    expect(screen.getByText('Apri Dicta desktop')).toBeInTheDocument()
  })
})
