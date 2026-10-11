export type Route = 'home' | 'dashboard' | 'history' | 'models' | 'settings'
export const formatShortcut = (value?: string) => (value ?? 'CommandOrControl+Shift+Space').replace('CommandOrControl', '⌘ / Ctrl').replace('Shift', '⇧').replace('Space', 'Spazio').replaceAll('+', ' ')
