import { useState } from 'react'
import { Activity, ArrowRight, Check, Clipboard, Download, Mic, ShieldCheck, Sparkles, Volume2 } from 'lucide-react'
import { api } from '../../services/desktop'
import { formatShortcut } from '../../app/routes'
import type { DictationSnapshot, ModelInfo, ModelProgress, Permissions, Settings } from '../../types/contracts'
import { Button, Card, Eyebrow, Notice, PageTitle } from '../../components/ui'
import type { Route } from '../../app/routes'

type Props = { settings: Settings | null; snapshot: DictationSnapshot | null; permissions: Permissions | null; setPermissions: (v: Permissions) => void; models: ModelInfo[]; setModels: (v: ModelInfo[]) => void; progress: ModelProgress | null; act: (job: () => Promise<unknown>) => Promise<boolean>; go: (page: Route) => void }
const size = (n: number) => n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : `${Math.round(n / 1e6)} MB`
export function Home({ settings, snapshot, permissions, setPermissions, models, setModels, progress, act, go }: Props) {
  const [busy, setBusy] = useState(false)
  const installed = models.find(m => m.id === settings?.model_id && m.installed)
  const modelReady = Boolean(installed)
  const ready = modelReady && permissions?.microphone === 'granted'
  const recording = snapshot?.state === 'recording' || snapshot?.state === 'preparing'
  const transitioning = snapshot?.state === 'processing' || snapshot?.state === 'inserting'
  const invokeAction = async (job: () => Promise<unknown>) => { setBusy(true); await act(job); setBusy(false) }
  const request = (kind: 'microphone' | 'accessibility') => act(async () => { await api.requestPermission(kind); setPermissions(await api.permissions()) })
  const download = async () => {
    if (!settings) return
    setBusy(true)
    await act(async () => { await api.downloadModel(settings.model_id); setModels(await api.models()) })
    setBusy(false)
  }
  return <>
    <PageTitle overline="IL TUO SPAZIO" title="Buongiorno, che cosa vuoi dire?" detail="La tua voce diventa testo, direttamente su questo Mac." />
    <section className="hero-card">
      <div className="hero-copy"><Eyebrow><span className="mini-pulse" /> DETTATURA LOCALE</Eyebrow><h2>Le parole arrivano<br />quando sei pronto.</h2><p>{ready ? 'Tutto pronto per una nuova idea. Dicta ascolta solo mentre detti.' : !modelReady ? 'Una volta scaricato il modello, puoi dettare anche senza connessione.' : 'Concedi l’accesso al microfono per iniziare.'}</p>
        <Button className="primary start-button" disabled={(!ready && !recording) || busy || transitioning} busy={busy} onClick={() => invokeAction(recording ? api.stop : api.start)}><Mic size={16} />{recording ? 'Termina dettatura' : transitioning ? 'Elaboro…' : 'Inizia a dettare'}<span className="keycap">{formatShortcut(settings?.shortcut)}</span></Button>
        {!ready && <small className="inline-hint">Installa un modello per iniziare</small>}
      </div>
      <div className={`hero-orb ${recording ? 'listening' : ''}`} aria-hidden="true"><div className="orb-ring r1"/><div className="orb-ring r2"/><div className="orb-core"><AudioGlyph level={snapshot?.level ?? 0} active={recording} /></div><span className="orb-caption">{recording ? 'TI ASCOLTO' : 'SOLO TU'}</span></div>
      <div className="hero-foot"><span><ShieldCheck size={14} /> Nessuna voce viene caricata</span><span>100% offline</span></div>
    </section>
    <div className="home-grid">
      <Card className="setup-card"><div className="section-head"><div><Eyebrow>PRIMI PASSI</Eyebrow><h3>Prepariamo Dicta</h3></div><span className="step-count">{(permissions?.microphone === 'granted' ? 1 : 0) + (permissions?.accessibility ? 1 : 0) + (modelReady ? 1 : 0)}/3</span></div>
        <div className="setup-step"><span className={`step-icon ${permissions?.microphone === 'granted' ? 'done' : ''}`}>{permissions?.microphone === 'granted' ? <Check size={16}/> : <Mic size={16}/>}</span><div className="step-body"><b>Consenti l’uso del microfono</b><small>{permissions?.microphone === 'granted' ? 'Autorizzazione attiva' : 'Richiesto soltanto durante la dettatura'}</small></div>{permissions?.microphone === 'granted' ? <span className="done-tag">Pronto</span> : <Button onClick={() => void request('microphone')} disabled={!permissions}>Consenti</Button>}</div>
        <div className="setup-step"><span className={`step-icon ${permissions?.accessibility ? 'done' : ''}`}>{permissions?.accessibility ? <Check size={16}/> : <Activity size={16}/>}</span><div className="step-body"><b>Abilita l’inserimento del testo</b><small>{permissions?.accessibility ? 'Accessibilità attiva' : 'Permette di scrivere nella finestra che stai usando'}</small></div>{permissions?.accessibility ? <span className="done-tag">Pronto</span> : <Button onClick={() => void request('accessibility')} disabled={!permissions}>Configura</Button>}</div>
        <div className="setup-step"><span className={`step-icon ${modelReady ? 'done' : ''}`}>{modelReady ? <Check size={16}/> : <Download size={16}/>}</span><div className="step-body"><b>Scarica il modello italiano</b><small>{modelReady ? installed?.name : `Modello ${settings?.model_id ?? 'tiny'} · ${models.find(m => m.id === settings?.model_id) ? size(models.find(m => m.id === settings?.model_id)!.size_bytes) : 'dimensione in verifica'}`}</small></div>{modelReady ? <span className="done-tag">Installato</span> : <Button className="primary" disabled={!settings || busy || Boolean(progress && progress.state === 'downloading')} busy={busy} onClick={() => void download()}>{progress?.state === 'downloading' ? `${Math.floor(progress.downloaded / Math.max(progress.total, 1) * 100)}%` : 'Scarica'}</Button>}</div>
        {progress?.state === 'downloading' && <div className="download-progress"><div><span>Download in corso</span><button onClick={() => void invokeAction(api.cancelModelDownload)}>Annulla</button></div><div className="progress-track"><i style={{ width: `${Math.min(100, progress.downloaded / Math.max(progress.total, 1) * 100)}%` }}/></div></div>}
      </Card>
      <Card className="quick-card"><Eyebrow>ACCESSO RAPIDO</Eyebrow><h3>Uno spazio fatto<br />per le tue parole.</h3><button className="quick-row" onClick={() => go('dashboard')}><span className="quick-icon"><Activity size={17}/></span><span><b>Le tue statistiche</b><small>Ritrova il tuo ritmo</small></span><ArrowRight size={15}/></button><button className="quick-row" onClick={() => go('settings')}><span className="quick-icon"><Sparkles size={17}/></span><span><b>Personalizza Dicta</b><small>Scorciatoia, lingua e privacy</small></span><ArrowRight size={15}/></button></Card>
    </div>
    {snapshot?.recovery_pending && snapshot.recovery_text && <Card className="recovery-card"><span className="recovery-icon"><Clipboard size={17}/></span><div><b>Il testo è pronto da recuperare</b><small>Non è stato possibile inserirlo nella finestra attiva. È ancora disponibile in memoria.</small></div><Button onClick={() => invokeAction(api.copyRecovery)}><Clipboard size={14}/> Copia testo</Button></Card>}
    {snapshot?.state === 'error' && snapshot.message && <Notice tone="error">{snapshot.message} <button className="text-button" onClick={() => invokeAction(api.cancel)}>Chiudi</button></Notice>}
    <div className="home-note"><Volume2 size={14}/> L’audio viene elaborato sul tuo dispositivo. Le trascrizioni non vengono salvate senza consenso.</div>
  </>
}
function AudioGlyph({ level, active }: { level: number; active: boolean }) { return <div className={`audio-glyph ${active ? 'active' : ''}`}>{[10, 18, 28, 19, 13].map((h, i) => <i key={i} style={{ height: active ? `${Math.max(4, h * Math.max(.2, Math.min(1.7, level * 2)))}px` : `${h}px` }}/>)}</div> }
