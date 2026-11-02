import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { DictationSnapshot } from '../../types/contracts'
import { api } from '../../services/desktop'
import { Overlay } from './Overlay'

let levelHandler: ((value: Pick<DictationSnapshot, 'level' | 'speech' | 'elapsed_ms'>) => void) | undefined
vi.mock('../../services/desktop', () => ({
  isDesktop: true,
  api: { snapshot: vi.fn(), onSnapshot: vi.fn(), onAudioLevel: vi.fn(), cancel: vi.fn(), copyRecovery: vi.fn() },
}))

const value = (state: DictationSnapshot['state']): DictationSnapshot => ({ state, session_id: 's1', elapsed_ms: 0, level: .01, speech: state === 'recording', message: null, recovery_text: null, recovery_pending: false })
beforeEach(() => {
  levelHandler = undefined
  vi.mocked(api.snapshot).mockResolvedValue(value('idle'))
  vi.mocked(api.onSnapshot).mockResolvedValue(() => {})
  vi.mocked(api.onAudioLevel).mockImplementation(async handler => { levelHandler = handler; return () => {} })
  vi.mocked(api.cancel).mockResolvedValue()
  vi.mocked(api.copyRecovery).mockResolvedValue()
})
afterEach(() => { cleanup(); vi.clearAllMocks() })

describe('compact audio orb', () => {
  it('shows only the orb and reacts immediately to microphone level', async () => {
    vi.mocked(api.snapshot).mockResolvedValue(value('recording'))
    render(<Overlay />)
    const orb = await screen.findByRole('button', { name: 'Annulla dettatura' })
    expect(screen.queryByText(/\d{2}:\d{2}/)).not.toBeInTheDocument()
    const bar = document.querySelector('.overlay-bars i')!
    const before = bar.getAttribute('style')
    act(() => levelHandler?.({ level: .08, speech: true, elapsed_ms: 1000 }))
    expect(bar.getAttribute('style')).not.toBe(before)
    fireEvent.click(orb)
    expect(api.cancel).toHaveBeenCalledOnce()
  })

  it('preserves explicit recovery copy on the orb', async () => {
    vi.mocked(api.snapshot).mockResolvedValue({ ...value('error'), recovery_text: 'Ciao', recovery_pending: true })
    render(<Overlay />)
    fireEvent.click(await screen.findByRole('button', { name: 'Copia il testo recuperabile' }))
    expect(api.copyRecovery).toHaveBeenCalledOnce()
  })
})
