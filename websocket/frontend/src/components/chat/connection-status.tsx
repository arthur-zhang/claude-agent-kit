import { Badge } from '../ui/badge'

type ConnectionStatusType = 'connecting' | 'connected' | 'disconnected' | 'error'

interface ConnectionStatusProps {
  status: ConnectionStatusType
}

export function ConnectionStatus({ status }: ConnectionStatusProps) {
  const getVariant = (s: ConnectionStatusType): 'default' | 'secondary' | 'destructive' => {
    switch (s) {
      case 'connected':
        return 'default'
      case 'connecting':
        return 'secondary'
      case 'error':
        return 'destructive'
      case 'disconnected':
      default:
        return 'secondary'
    }
  }

  const getLabel = (s: ConnectionStatusType): string => {
    switch (s) {
      case 'connected':
        return 'Connected'
      case 'connecting':
        return 'Connecting...'
      case 'error':
        return 'Connection Error'
      case 'disconnected':
      default:
        return 'Disconnected'
    }
  }

  return (
    <Badge variant={getVariant(status)} className="animate-pulse">
      {getLabel(status)}
    </Badge>
  )
}
