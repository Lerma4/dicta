import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '../../services/desktop'
import type { Settings as SettingsData } from '../../types/contracts'
import { SettingsPage } from './Settings'

vi.mock('../../services/desktop', () => ({ isDesktop: true, api: { devices: vi.fn(), saveSettings: vi.fn() } }))
const initial: SettingsData = { language: 'it', model_id: 'tiny', input_device: null, shortcut: 'CommandOrControl+Shift+Space', shortcut_mode: 'toggle', history_enabled: false, retention_days: 30, overlay_position: 'bottom', autostart: false, vad_threshold: .01, sounds: false }

beforeEach(() => { vi.mocked(api.devices).mockResolvedValue([]); vi.mocked(api.saveSettings).mockRejectedValue(new Error('Scorciatoia già occupata.')) })
afterEach(() => { cleanup(); vi.clearAllMocks() })

describe('settings save feedback', () => {
  it('does not report a failed settings save as successful', async () => {
    const act = async (job: () => Promise<unknown>) => { try { await job(); return true } catch { return false } }
    render(<SettingsPage settings={initial} setSettings={() => {}} permissions={null} setPermissions={() => {}} models={[]} act={act} />)
    fireEvent.click(screen.getByRole('button', { name: 'Salva impostazioni' }))
    await waitFor(() => expect(api.saveSettings).toHaveBeenCalledOnce())
    expect(screen.queryByText('Modifiche salvate')).not.toBeInTheDocument()
  })
})
