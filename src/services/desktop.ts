import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Dashboard, DashboardFilter, DictationSnapshot, InputDevice, ModelInfo, ModelProgress, PermissionKind, Permissions, Session, Settings } from '../types/contracts'

export const isDesktop = isTauri()
export class DesktopRequiredError extends Error { constructor() { super('Apri Dicta desktop per usare questa funzione.'); this.name = 'DesktopRequiredError' } }
const call = <T>(command: string, args?: Record<string, unknown>) => {
  if (!isDesktop) return Promise.reject(new DesktopRequiredError())
  return invoke<T>(command, args)
}
export const api = {
  snapshot: () => call<DictationSnapshot>('get_snapshot'), start: () => call<void>('start_dictation'), stop: () => call<void>('stop_dictation'), cancel: () => call<void>('cancel_dictation'), copyRecovery: () => call<void>('copy_recovery'),
  settings: () => call<Settings>('get_settings'), saveSettings: (settings: Settings) => call<void>('save_settings', { settings }), permissions: () => call<Permissions>('get_permissions'), requestPermission: (kind: PermissionKind) => call<void>('request_permission', { kind }),
  devices: () => call<InputDevice[]>('list_input_devices'), models: () => call<ModelInfo[]>('list_models'), downloadModel: (id: string) => call<void>('download_model', { id }), cancelModelDownload: () => call<void>('cancel_model_download'), deleteModel: (id: string) => call<void>('delete_model', { id }),
  dashboard: (filter: DashboardFilter) => call<Dashboard>('get_dashboard', { filter }),
  sessions: (filter: DashboardFilter, search: string, limit = 50, offset = 0) => call<Session[]>('list_sessions', { filter, search, limit, offset }), deleteTranscript: (id: string) => call<void>('delete_transcript', { id }), clearData: (kind: 'transcripts' | 'metrics' | 'all') => call<void>('clear_data', { kind }), exportData: (kind: 'sessions' | 'transcripts', format: 'json' | 'csv') => call<string>('export_data', { kind, format }), exportFile: (kind: 'sessions' | 'transcripts', format: 'json' | 'csv') => call<string | null>('export_file', { kind, format }), historyEnabled: () => call<boolean>('get_history_enabled'),
  onSnapshot: (handler: (value: DictationSnapshot) => void) => isDesktop ? listen<DictationSnapshot>('dictation-state', e => handler(e.payload)) : Promise.resolve<UnlistenFn>(() => {}),
  onProgress: (handler: (value: ModelProgress) => void) => isDesktop ? listen<ModelProgress>('model-progress', e => handler(e.payload)) : Promise.resolve<UnlistenFn>(() => {}),
  onSessionsChanged: (handler: () => void) => isDesktop ? listen('sessions-changed', handler) : Promise.resolve<UnlistenFn>(() => {}),
  onAudioLevel: (handler: (value: Pick<DictationSnapshot, 'level' | 'speech' | 'elapsed_ms'>) => void) => isDesktop ? listen('audio-level', e => handler(e.payload as Pick<DictationSnapshot, 'level' | 'speech' | 'elapsed_ms'>)) : Promise.resolve<UnlistenFn>(() => {}),
}

