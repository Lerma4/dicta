import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { LoaderCircle } from 'lucide-react'
export function Button({ children, className = '', busy, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { busy?: boolean }) { return <button className={`button ${className}`} {...props}>{busy && <LoaderCircle size={15} className="spin" />}{children}</button> }
export function Card({ children, className = '' }: { children: ReactNode; className?: string }) { return <section className={`card ${className}`}>{children}</section> }
export function Eyebrow({ children }: { children: ReactNode }) { return <div className="eyebrow">{children}</div> }
export function PageTitle({ overline, title, detail }: { overline: string; title: string; detail: string }) { return <div className="page-title"><Eyebrow>{overline}</Eyebrow><h1>{title}</h1><p>{detail}</p></div> }
export function Empty({ title, detail }: { title: string; detail: string }) { return <div className="empty"><span className="empty-mark">∅</span><strong>{title}</strong><p>{detail}</p></div> }
export function Notice({ children, tone = 'info' }: { children: ReactNode; tone?: 'info' | 'error' }) { return <div role={tone === 'error' ? 'alert' : 'status'} className={`notice ${tone}`}>{children}</div> }
export function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) { return <label className="toggle-row"><span>{label}</span><button type="button" role="switch" aria-checked={checked} className={`toggle ${checked ? 'on' : ''}`} onClick={() => onChange(!checked)}><i /></button></label> }
