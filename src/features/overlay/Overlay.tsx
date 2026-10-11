import { useEffect, useState } from 'react'
import { Check, Clipboard, Mic, MicOff, X } from 'lucide-react'
import { api, isDesktop } from '../../services/desktop'
import type { DictationSnapshot } from '../../types/contracts'

export function Overlay() {
  const [snapshot, setSnapshot] = useState<DictationSnapshot | null>(null)
  useEffect(() => {
    if (!isDesktop) return
    void api.snapshot().then(setSnapshot).catch(() => {})
    let active = true; const unlisten: (() => void)[] = []
    const subscribe = (listener: Promise<() => void>) => void listener.then(stop => { if (active) unlisten.push(stop); else stop() })
    subscribe(api.onSnapshot(setSnapshot))
    subscribe(api.onAudioLevel(value => setSnapshot(s => s ? { ...s, ...value } : s)))
    return () => { active = false; unlisten.forEach(stop => stop()) }
  }, [])
  if (!isDesktop) return <div className="overlay-unavailable">Apri Dicta desktop</div>
  const state = snapshot?.state ?? 'idle'
  const content: Record<string, { title: string; detail: string }> = {
    idle: { title: 'Dicta è pronto', detail: 'Premi la scorciatoia per iniziare' }, preparing: { title: 'Mi preparo…', detail: 'Attivo il microfono' },
    recording: { title: snapshot?.speech ? 'Ti ascolto' : 'In ascolto', detail: snapshot?.speech ? 'Sto raccogliendo le tue parole' : 'Parla quando vuoi' }, processing: { title: 'Metto in ordine le parole', detail: 'Elaborazione sul dispositivo' },
    inserting: { title: 'Quasi fatto', detail: 'Inserisco il testo nella finestra attiva' }, completed: { title: 'Dettatura completata', detail: snapshot?.message ?? 'Dettatura completata' },
    cancelled: { title: 'Dettatura annullata', detail: 'La registrazione è stata interrotta' }, error: { title: 'Non è andata', detail: snapshot?.message ?? 'Puoi riprovare o copiare il testo' },
  }
  const current = content[state] ?? content.idle
  const active = state === 'recording' || state === 'preparing'
  const recoverable = Boolean(snapshot?.recovery_pending && snapshot.recovery_text)
  const showCancel = active || state === 'processing'
  return <main className={`overlay-shell ${active ? 'is-live' : ''} ${snapshot?.speech ? 'is-speaking' : ''} state-${state}`} aria-live="polite">
    <div className="overlay-orb" aria-hidden="true"><div className="overlay-bars">{[.42,.7,1,.56,.82,.38,.63].map((v,i) => <i key={i} style={{ height: `${Math.max(4, 22 * v * (active ? .2 + (snapshot?.level ?? 0) * 2 : .58))}px` }}/>)}</div></div>
    <div className="overlay-copy"><b>{current.title}</b><small>{current.detail}</small></div>
    <div className="overlay-timer">{format(snapshot?.elapsed_ms ?? 0)}</div>
    {recoverable && <button aria-label="Copia il testo recuperabile" className="overlay-action" onClick={() => void api.copyRecovery()}><Clipboard size={15}/></button>}
    {showCancel && <button aria-label="Annulla dettatura" className="overlay-action cancel" onClick={() => void api.cancel()}><X size={15}/></button>}
    {state === 'completed' && <span className="overlay-complete"><Check size={14}/></span>}
    {state === 'cancelled' && <span className="overlay-complete muted"><MicOff size={14}/></span>}
    {state === 'idle' && <span className="overlay-complete muted"><Mic size={14}/></span>}
  </main>
}
function format(ms: number) { const sec = Math.floor(ms / 1000); return `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}` }
