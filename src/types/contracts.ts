export type DictationState = 'idle' | 'preparing' | 'recording' | 'processing' | 'inserting' | 'completed' | 'cancelled' | 'error'
export type Settings = {
  language: string; model_id: string; input_device: string | null; shortcut: string; shortcut_mode: 'toggle' | 'hold'
  history_enabled: boolean; retention_days: number; overlay_position: 'bottom' | 'top'; autostart: boolean; vad_threshold: number; sounds: boolean
}
export type DictationSnapshot = { state: DictationState; session_id: string | null; elapsed_ms: number; level: number; speech: boolean; message: string | null; recovery_text: string | null; recovery_pending: boolean }
export type Permissions = { microphone: 'granted' | 'denied' | 'undetermined' | 'unavailable'; accessibility: boolean; platform: string; text_injection: boolean }
export type PermissionKind = 'microphone' | 'accessibility'
export type ModelInfo = { id: string; name: string; size_bytes: number; checksum: string; installed: boolean; downloading: boolean; description: string }
export type ModelProgress = { id: string; downloaded: number; total: number; state: string; error: string | null }
export type InputDevice = { name: string; is_default: boolean }
export type DashboardFilter = { days: number; language: string | null; model_id: string | null; start_date: string | null; end_date: string | null; granularity: 'day' | 'week' | 'month' }
export type Session = { id: string; started_at_utc: string; ended_at_utc: string | null; status: string; recorded_ms: number; inference_ms: number; insertion_ms: number; words_count: number; chars_count: number; language_code: string; model_id: string; error_category: string | null; input_mode: string; text: string | null }
export type Dashboard = {
  started: number; completed: number; failed: number; cancelled: number; words: number; chars: number; recorded_ms: number; inference_ms: number; insertion_ms: number
  mean_inference_ms: number; median_inference_ms: number; p95_inference_ms: number; wpm: number; throughput: number; active_days: number; previous_words: number; words_change_pct: number | null
  top_language: string | null; top_model: string | null; timeline: { date: string; sessions: number; words: number; inference_ms: number }[]
  heatmap: { weekday: number; hour: number; sessions: number; words: number }[]; duration_histogram: { label: string; count: number }[]
  models: { model_id: string; sessions: number; mean_inference_ms: number }[]; errors: { category: string; count: number }[]
}
export const emptyFilter = (): DashboardFilter => ({ days: 7, language: null, model_id: null, start_date: null, end_date: null, granularity: 'day' })
