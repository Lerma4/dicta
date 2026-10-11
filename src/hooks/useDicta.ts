import { useCallback, useEffect, useState } from 'react'
import type { DictationSnapshot, ModelInfo, ModelProgress, Permissions, Settings } from '../types/contracts'
import { api, isDesktop } from '../services/desktop'

export function useDicta() {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [snapshot, setSnapshot] = useState<DictationSnapshot | null>(null)
  const [permissions, setPermissions] = useState<Permissions | null>(null)
  const [models, setModels] = useState<ModelInfo[]>([])
  const [progress, setProgress] = useState<ModelProgress | null>(null)
  const [error, setError] = useState('')
  const refresh = useCallback(async () => {
    if (!isDesktop) return
    try {
      const [s, d, p, m] = await Promise.all([api.settings(), api.snapshot(), api.permissions(), api.models()])
      setSettings(s); setSnapshot(d); setPermissions(p); setModels(m); setError('')
    } catch (e) { setError(message(e)) }
  }, [])
  useEffect(() => {
    void refresh()
    let active = true; const unlisten: (() => void)[] = []
    const subscribe = (listener: Promise<() => void>) => void listener.then(stop => { if (active) unlisten.push(stop); else stop() })
    subscribe(api.onSnapshot(setSnapshot))
    subscribe(api.onProgress(value => { setProgress(value); if (value.state !== 'downloading') void api.models().then(setModels).catch(e => setError(message(e))) }))
    subscribe(api.onAudioLevel(value => setSnapshot(s => s ? { ...s, ...value } : s)))
    return () => { active = false; unlisten.forEach(stop => stop()) }
  }, [refresh])
  const act = useCallback(async (job: () => Promise<unknown>) => { try { await job(); setError(''); await refresh(); return true } catch (e) { setError(message(e)); return false } }, [refresh])
  return { settings, setSettings, snapshot, setSnapshot, permissions, setPermissions, models, setModels, progress, error, setError, refresh, act }
}
export function message(error: unknown) { return error instanceof Error ? error.message : String(error) }
