import { useEffect } from 'react'
import { Check, CircleAlert, Download, HardDrive, Trash2 } from 'lucide-react'
import type { ModelInfo, ModelProgress, Settings } from '../../types/contracts'
import { api } from '../../services/desktop'
import { Button, Card, Empty, Eyebrow, PageTitle } from '../../components/ui'

type Props = { models: ModelInfo[]; setModels: (m: ModelInfo[]) => void; settings: Settings | null; setSettings: (s: Settings) => void; progress: ModelProgress | null; act: (job: () => Promise<unknown>) => Promise<boolean> }
const prettySize = (n: number) => `${(n / 1_000_000_000).toFixed(1)} GB`
export function ModelsPage({ models, setModels, settings, setSettings, progress, act }: Props) {
  useEffect(() => { if (progress?.state === 'completed') void api.models().then(setModels) }, [progress, setModels])
  const refresh = async () => setModels(await api.models())
  return <>
    <PageTitle overline="VOCE E MODELLI" title="I tuoi modelli" detail="Scegli quale voce trasformare in testo. I modelli restano sul tuo Mac." />
    <Card className="models-summary"><span className="summary-icon"><HardDrive size={18}/></span><div><b>Elaborazione sul dispositivo</b><small>Il download è manuale. Una volta installato, il modello funziona offline.</small></div><span className="local-pill"><i/> ARCHIVIO LOCALE</span></Card>
    <div className="models-head"><div><Eyebrow>CATALOGO</Eyebrow><h3>Disponibili per Dicta</h3></div><span>{models.length} {models.length === 1 ? 'modello' : 'modelli'}</span></div>
    {!models.length ? <Card><Empty title="Catalogo non disponibile" detail="Apri Dicta desktop quando il servizio modelli è pronto." /></Card> : <div className="model-list">{models.map(model => {
      const current = settings?.model_id === model.id; const active = progress?.id === model.id && progress.state === 'downloading'; const pct = active ? Math.min(100, progress.downloaded / Math.max(1, progress.total) * 100) : 0
      return <Card key={model.id} className={`model-card ${current ? 'current' : ''}`}><div className="model-mark"><span className={model.installed ? 'installed' : ''}>{model.installed ? <Check size={18}/> : <Download size={17}/>}</span></div><div className="model-info"><div className="model-name-row"><h3>{model.name}</h3>{current && <span className="current-tag">IN USO</span>}{model.installed && <span className="installed-tag">INSTALLATO</span>}</div><p>{model.description}</p><div className="model-meta"><span>{prettySize(model.size_bytes)}</span><span className="meta-dot"/><span>Italiano e multilingua</span>{model.checksum && <><span className="meta-dot"/><span>Checksum verificato</span></>}</div>{active && <div className="model-download"><div><span>Download</span><b>{pct.toFixed(0)}%</b></div><div className="progress-track"><i style={{ width: `${pct}%` }}/></div></div>}{progress?.id === model.id && progress.error && <div className="model-error"><CircleAlert size={14}/>{progress.error}</div>}</div><div className="model-actions">{model.installed ? <><Button disabled={current} onClick={() => { if (settings) { const next = { ...settings, model_id: model.id }; void act(async () => { await api.saveSettings(next); setSettings(next) }) } }}>{current ? 'Selezionato' : 'Usa modello'}</Button><button className="icon-button danger" aria-label={`Elimina ${model.name}`} disabled={current} title={current ? 'Seleziona un altro modello prima di eliminarlo' : `Elimina ${model.name}`} onClick={() => void act(async () => { await api.deleteModel(model.id); await refresh() })}><Trash2 size={16}/></button></> : active ? <Button onClick={() => void act(api.cancelModelDownload)}>Annulla</Button> : <Button className="primary" onClick={() => void act(async () => { await api.downloadModel(model.id); await refresh() })}><Download size={15}/> Scarica</Button>}</div></Card>
    })}</div>}
    <p className="fineprint">Le dimensioni e le caratteristiche dipendono dal catalogo fornito da Dicta. Nessun modello viene scaricato automaticamente.</p>
  </>
}
