import { useEffect, useState } from 'react'
import { Clipboard, Download, Search, Trash2 } from 'lucide-react'
import { Button, Card, Empty, Eyebrow, PageTitle } from '../../components/ui'
import { api, isDesktop } from '../../services/desktop'
import type { DashboardFilter, Session, Settings } from '../../types/contracts'
import { emptyFilter } from '../../types/contracts'

export function HistoryPage({ settings }: { settings: Settings | null }) {
  const [enabled, setEnabled] = useState(false); const [rows, setRows] = useState<Session[]>([]); const [search, setSearch] = useState(''); const [days, setDays] = useState(30); const [offset, setOffset] = useState(0); const [loading, setLoading] = useState(false); const [error, setError] = useState(''); const [exported, setExported] = useState('')
  useEffect(() => { if (isDesktop) void api.historyEnabled().then(setEnabled).catch(e => setError(String(e))) }, [])
  useEffect(() => {
    if (!isDesktop || !settings?.history_enabled) return
    let alive = true; setLoading(true)
    const filter: DashboardFilter = { ...emptyFilter(), days }
    const timer = window.setTimeout(() => { void api.sessions(filter, search, 50, offset).then(data => { if (alive) { setRows(data); setError('') } }).catch(e => alive && setError(e instanceof Error ? e.message : String(e))).finally(() => alive && setLoading(false)) }, 180)
    return () => { alive = false; clearTimeout(timer) }
  }, [settings?.history_enabled, search, days, offset])
  const exportRows = async (format: 'json' | 'csv') => { try { const path = await api.exportFile('transcripts', format); if (path) setExported(path); setError('') } catch (e) { setError(e instanceof Error ? e.message : String(e)) } }
  if (!isDesktop) return <><PageTitle overline="LE TUE PAROLE" title="Cronologia" detail="Un archivio facoltativo, sempre sotto il tuo controllo."/><Card><Empty title="Apri Dicta desktop" detail="La cronologia contiene soltanto testi salvati sul dispositivo." /></Card></>
  return <>
    <PageTitle overline="LE TUE PAROLE" title="Cronologia" detail="Ritrova, copia o elimina le tue dettature salvate." />
    {!settings?.history_enabled && !enabled ? <Card className="history-optin"><span className="optin-icon"><Clipboard size={19}/></span><Eyebrow>MEMORIA FACOLTATIVA</Eyebrow><h3>Le tue parole restano tue.</h3><p>La cronologia è disattivata. Quando la abiliti, le trascrizioni vengono salvate localmente e puoi cercarle, copiarle o cancellarle in qualsiasi momento.</p><p className="history-tip">Per attivarla, vai in <b>Impostazioni → Privacy e sistema</b>.</p></Card> : <>
      <div className="history-toolbar"><label className="search-field"><Search size={16}/><input aria-label="Cerca nella cronologia" placeholder="Cerca nelle tue dettature…" value={search} onChange={e => { setSearch(e.target.value); setOffset(0) }}/></label><select aria-label="Intervallo cronologia" value={days} onChange={e => { setDays(Number(e.target.value)); setOffset(0) }}><option value={7}>Ultimi 7 giorni</option><option value={30}>Ultimi 30 giorni</option><option value={90}>Ultimi 90 giorni</option></select><Button onClick={() => void exportRows('csv')}><Download size={14}/> CSV</Button><Button onClick={() => void exportRows('json')}>JSON</Button></div>
      {exported && <p role="status" className="export-confirmation">Salvato in {exported}</p>}
      {error && <p className="inline-error">{error}</p>}
      <div className="history-list">{rows.map(row => <Card className="history-row" key={row.id}><div className="history-date"><b>{new Date(row.started_at_utc).toLocaleDateString('it-IT', { day: '2-digit', month: 'short' })}</b><small>{new Date(row.started_at_utc).toLocaleTimeString('it-IT', { hour: '2-digit', minute: '2-digit' })}</small></div><div className="history-content">{row.text ? <p>{row.text}</p> : <p className="redacted">Testo non conservato</p>}<div className="history-meta"><span>{row.words_count} parole</span><span>{Math.round(row.recorded_ms / 1000)} sec</span><span>{row.language_code}</span><span className={`status-chip ${row.status}`}>{row.status === 'completed' ? 'Completata' : row.status}</span></div></div><div className="history-actions"><button className="icon-button" title="Copia testo" disabled={!row.text} onClick={() => row.text && void navigator.clipboard.writeText(row.text)}><Clipboard size={15}/></button>{row.text && <button className="icon-button danger" title="Elimina trascrizione" onClick={async () => { await api.deleteTranscript(row.id); setRows(rows.filter(s => s.id !== row.id)) }}><Trash2 size={15}/></button>}</div></Card>)}
      {!rows.length && !loading && <Card><Empty title={search ? 'Nessun risultato' : 'Ancora nessuna dettatura'} detail={search ? 'Prova con altre parole.' : 'Le dettature salvate appariranno qui.'}/></Card>}{loading && <div className="loading">Cerco nella cronologia…</div>}
      <div className="pagination"><Button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - 50))}>Indietro</Button><span>{rows.length ? `${offset + 1}–${offset + rows.length}` : '0 risultati'}</span><Button disabled={rows.length < 50} onClick={() => setOffset(offset + 50)}>Avanti</Button></div>
      </div>
    </>}
  </>
}
