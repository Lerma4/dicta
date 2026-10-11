import { lazy, Suspense, useState } from 'react'
import { AudioLines, ChartNoAxesColumnIncreasing, CircleHelp, Command, History, House, Moon, Settings2, Sun } from 'lucide-react'
import dictaMark from '../../src-tauri/icons/icon.svg'
import { useDicta } from '../hooks/useDicta'
import { isDesktop } from '../services/desktop'
import { formatShortcut } from './routes'
import { Home } from '../features/home/Home'
import { HistoryPage } from '../features/history/History'
import { ModelsPage } from '../features/models/Models'
import { SettingsPage } from '../features/settings/Settings'
import { Notice } from '../components/ui'
import '../styles/app.css'

const Dashboard = lazy(() => import('../features/dashboard/Dashboard').then(m => ({ default: m.Dashboard })))
type Route = 'home' | 'dashboard' | 'history' | 'models' | 'settings'
const nav: { id: Route; label: string; icon: typeof House }[] = [
  { id: 'home', label: 'Panoramica', icon: House }, { id: 'dashboard', label: 'Statistiche', icon: ChartNoAxesColumnIncreasing },
  { id: 'history', label: 'Cronologia', icon: History }, { id: 'models', label: 'Modelli', icon: AudioLines }, { id: 'settings', label: 'Impostazioni', icon: Settings2 },
]
export function App() {
  const [route, setRoute] = useState<Route>('home'); const [dark, setDark] = useState(false)
  const data = useDicta()
  const active = nav.find(n => n.id === route)!
  const ready = Boolean(data.settings && data.permissions?.microphone === 'granted' && data.models.some(m => m.id === data.settings?.model_id && m.installed))
  const page = route === 'home' ? <Home {...data} go={setRoute} /> : route === 'dashboard' ? <Suspense fallback={<div className="loading">Carico le statistiche…</div>}><Dashboard /></Suspense> : route === 'history' ? <HistoryPage settings={data.settings} /> : route === 'models' ? <ModelsPage {...data} /> : <SettingsPage {...data} />
  return <div className={`app ${dark ? 'dark' : ''}`}>
    <aside className="sidebar">
      <div className="brand"><div className="brand-icon"><img src={dictaMark} alt="" /></div><span>dicta<small>voce, in locale</small></span></div>
      <div className="nav-caption">SPAZIO DI LAVORO</div>
      <nav aria-label="Navigazione principale">{nav.map(({ id, label, icon: Icon }) => <button key={id} className={`nav-link ${route === id ? 'selected' : ''}`} onClick={() => setRoute(id)}><Icon size={17} strokeWidth={1.8} /><span>{label}</span>{route === id && <i />}</button>)}</nav>
      <div className="sidebar-bottom">
        <div className="privacy-badge"><span className="privacy-dot" /><div><b>Solo sul tuo Mac</b><small>La voce non lascia il dispositivo</small></div></div>
        <button className="help-link" onClick={() => setRoute('settings')}><CircleHelp size={15} /> Privacy e supporto</button>
        <div className="account"><div className="avatar">D</div><span><b>Dicta</b><small>{data.settings?.model_id ?? 'Modello non selezionato'}</small></span><button title="Cambia tema" aria-label="Cambia tema" onClick={() => setDark(v => !v)}>{dark ? <Sun size={16} /> : <Moon size={16} />}</button></div>
      </div>
    </aside>
    <main className="main-area">
      <header className="topbar"><div className="breadcrumb"><span>Dicta</span><span className="slash">/</span><strong>{active.label}</strong></div><div className="top-actions"><span className="ready-indicator"><i className={data.snapshot?.state === 'recording' ? 'live' : ''} />{data.snapshot?.state === 'recording' ? 'In ascolto' : ready ? 'Pronto' : 'Da configurare'}</span><span className="shortcut-hint"><Command size={12} /> {formatShortcut(data.settings?.shortcut)}</span></div></header>
      <div className="content-wrap">{!isDesktop && <Notice>Anteprima non collegata. Apri Dicta desktop per accedere a permessi, microfono e dati reali.</Notice>}{data.error && <Notice tone="error">{data.error}</Notice>}{page}</div>
    </main>
  </div>
}
