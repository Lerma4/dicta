import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Dashboard as DashboardData } from '../../types/contracts'
import { api } from '../../services/desktop'
import { Dashboard } from './Dashboard'

vi.mock('../../services/desktop', () => ({
  isDesktop: true,
  api: { dashboard: vi.fn(), sessions: vi.fn(), models: vi.fn(), onSessionsChanged: vi.fn() },
}))
const empty: DashboardData = { started: 0, completed: 0, failed: 0, cancelled: 0, words: 0, chars: 0, recorded_ms: 0, inference_ms: 0, insertion_ms: 0, mean_inference_ms: 0, median_inference_ms: 0, p95_inference_ms: 0, wpm: 0, throughput: 0, active_days: 0, previous_words: 0, words_change_pct: null, top_language: null, top_model: null, timeline: [], heatmap: [], duration_histogram: [], models: [], errors: [] }

beforeEach(() => {
  vi.mocked(api.dashboard).mockResolvedValue(empty)
  vi.mocked(api.sessions).mockResolvedValue([])
  vi.mocked(api.models).mockResolvedValue([])
  vi.mocked(api.onSessionsChanged).mockResolvedValue(() => {})
})
afterEach(() => { cleanup(); vi.clearAllMocks() })

describe('dashboard empty state', () => {
  it('shows no invented metrics when the backend has no sessions', async () => {
    render(<Dashboard />)
    expect(await screen.findByText('Qui nasceranno le tue statistiche')).toBeInTheDocument()
    expect(screen.queryByText('1.200')).not.toBeInTheDocument()
  })
})
