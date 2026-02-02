import { Badge } from '../ui/badge'
import { ThemeToggle } from '../theme-toggle'

interface HeaderProps {
  isConnected: boolean
  connectionStatus?: 'connecting' | 'connected' | 'disconnected'
}

export function Header({ isConnected, connectionStatus = 'disconnected' }: HeaderProps) {
  const statusVariant = isConnected ? 'default' : 'secondary'
  const statusLabel = connectionStatus === 'connecting' ? 'Connecting...' :
                     connectionStatus === 'connected' ? 'Connected' :
                     'Disconnected'

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 max-w-screen-2xl items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-lg font-semibold">Claude Agent</h1>
        </div>

        <div className="flex items-center gap-4">
          <Badge variant={statusVariant}>{statusLabel}</Badge>
          <ThemeToggle />
        </div>
      </div>
    </header>
  )
}
