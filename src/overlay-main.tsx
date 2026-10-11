import React from 'react'
import ReactDOM from 'react-dom/client'
import { Overlay } from './features/overlay/Overlay'
import './styles/global.css'
import './styles/overlay.css'

ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><Overlay/></React.StrictMode>)
