import { useEffect, useState } from 'react'
import { api, isDesktop } from '../../services/desktop'
import type { DictationSnapshot } from '../../types/contracts'

const bars = [.42, .7, 1, .56, .82, .38, .63]

export function Overlay() {
  const [snapshot, setSnapshot] = useState<DictationSnapshot | null>(null)
  useEffect(() => {
    if (!isDesktop) return
    void api.snapshot().then(setSnapshot).catch(() => {})
    let active = true
    const unlisten: (() => void)[] = []
    const subscribe = (listener: Promise<() => void>) => void listener.then(stop => { if (active) unlisten.push(stop); else stop() })
    subscribe(api.onSnapshot(setSnapshot))
    subscribe(api.onAudioLevel(value => setSnapshot(s => s ? { ...s, ...value } : s)))
    return () => { active = false; unlisten.forEach(stop => stop()) }
  }, [])
  if (!isDesktop) return <div className="overlay-unavailable">Apri Dicta desktop</div>

  const state = snapshot?.state ?? 'idle'
  const recording = state === 'recording'
  const recoverable = Boolean(snapshot?.recovery_pending && snapshot.recovery_text)
  const level = Math.min(1, Math.sqrt(Math.max(0, snapshot?.level ?? 0)) * 3.2)
  const label = recording ? 'Annulla dettatura' : recoverable ? 'Copia il testo recuperabile' : `Dicta: ${state}`
  const activate = () => {
    if (recording) void api.cancel()
    else if (recoverable) void api.copyRecovery()
  }

  return <main className="overlay-page" aria-live="off">
    <button className="overlay-orb" aria-label={label} title={label} onClick={activate}>
      <span className="overlay-bars" aria-hidden="true">{bars.map((bar, i) => <i key={i} style={{ height: `${3 + 19 * bar * level}px` }} />)}</span>
    </button>
  </main>
}
