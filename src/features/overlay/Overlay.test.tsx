import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { DictationSnapshot } from '../../types/contracts'
import { api } from '../../services/desktop'
import { Overlay } from './Overlay'

let levelHandler: ((value: Pick<DictationSnapshot, 'level' | 'speech' | 'elapsed_ms'>) => void) | undefined
vi.mock('../../services/desktop', () => ({
  isDesktop: true,
  api: {
    snapshot: vi.fn(),
    onSnapshot: vi.fn(),
    onAudioLevel: vi.fn(),
    cancel: vi.fn(),
    copyRecovery: vi.fn(),
  },
}))

const value = (state: DictationSnapshot['state']): DictationSnapshot => ({ state, session_id: 's1', elapsed_ms: 12_000, level: .4, speech: state === 'recording', message: null, recovery_text: null, recovery_pending: false })
const headings: [DictationSnapshot['state'], string][] = [
  ['idle', 'Dicta è pronto'], ['preparing', 'Mi preparo…'], ['recording', 'Ti ascolto'], ['processing', 'Metto in ordine le parole'],
  ['inserting', 'Quasi fatto'], ['completed', 'Dettatura completata'], ['cancelled', 'Dettatura annullata'], ['error', 'Non è andata'],
]

beforeEach(() => {
  levelHandler = undefined
  vi.mocked(api.snapshot).mockResolvedValue(value('idle'))
  vi.mocked(api.onSnapshot).mockResolvedValue(() => {})
  vi.mocked(api.onAudioLevel).mockImplementation(async handler => { levelHandler = handler; return () => {} })
  vi.mocked(api.cancel).mockResolvedValue()
  vi.mocked(api.copyRecovery).mockResolvedValue()
})
afterEach(() => { cleanup(); vi.clearAllMocks() })

describe('overlay state feedback', () => {
  it.each(headings)('shows the %s state with clear status', async (state, title) => {
    vi.mocked(api.snapshot).mockResolvedValue(value(state))
    render(<Overlay />)
    expect(within(await screen.findByRole('main')).getByText(title, { selector: 'b' })).toBeInTheDocument()
  })

  it('uses real audio-level events and offers cancel while recording', async () => {
    vi.mocked(api.snapshot).mockResolvedValue(value('recording'))
    render(<Overlay />)
    await screen.findByText('Ti ascolto')
    const bar = document.querySelector('.overlay-bars i')!
    const before = bar.getAttribute('style')
    act(() => levelHandler?.({ level: .9, speech: true, elapsed_ms: 18_000 }))
    expect(bar.getAttribute('style')).not.toBe(before)
    fireEvent.click(screen.getByRole('button', { name: 'Annulla dettatura' }))
    expect(api.cancel).toHaveBeenCalledOnce()
  })

  it('shows recovery only when insertion failed and copies via the native command', async () => {
    const failed = { ...value('completed'), message: 'Testo non inserito: nessun campo esterno attivo. Copialo da Dicta.', recovery_text: 'Ciao Dicta', recovery_pending: true }
    vi.mocked(api.snapshot).mockResolvedValue(failed)
    render(<Overlay />)
    expect(await screen.findByText('Testo pronto da copiare')).toBeInTheDocument()
    expect(await screen.findByText(failed.message!)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Copia il testo recuperabile' }))
    expect(api.copyRecovery).toHaveBeenCalledOnce()
  })
})
