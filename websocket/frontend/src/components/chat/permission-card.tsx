import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Card } from '../ui/card';
import { AlertTriangle, Shield, CheckCircle, XCircle } from 'lucide-react';
import type { PermissionRequest } from '../../types';

interface PermissionCardProps {
  request: PermissionRequest;
  onAllow: () => void;
  onDeny: () => void;
  onAllowAlways: () => void;
}

export function PermissionCard({
  request,
  onAllow,
  onDeny,
  onAllowAlways,
}: PermissionCardProps) {
  const getRiskVariant = (risk: string): 'default' | 'secondary' | 'destructive' => {
    switch (risk) {
      case 'high':
        return 'destructive';
      case 'medium':
        return 'default';
      case 'low':
        return 'secondary';
      default:
        return 'secondary';
    }
  };

  return (
    <Card className="my-4 p-4 border-2 border-primary/20 bg-card shadow-lg">
      {/* Header */}
      <div className="flex items-start gap-3 mb-4">
        <div className="p-2 rounded-lg bg-primary/10">
          <Shield className="h-5 w-5 text-primary" />
        </div>
        <div className="flex-1">
          <h3 className="text-base font-semibold flex items-center gap-2">
            Permission Required
            <Badge variant={getRiskVariant(request.context.risk_level)} className="text-xs">
              {request.context.risk_level.toUpperCase()} RISK
            </Badge>
          </h3>
          <p className="text-sm text-muted-foreground mt-1">
            Claude needs your permission to execute this tool
          </p>
        </div>
      </div>

      {/* Tool Information */}
      <div className="space-y-3 mb-4">
        {/* Tool name */}
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-muted-foreground min-w-[80px]">
            Tool:
          </span>
          <code className="text-sm font-mono bg-muted px-2 py-1 rounded">
            {request.toolName}
          </code>
        </div>

        {/* Description */}
        <div className="flex items-start gap-2">
          <span className="text-sm font-medium text-muted-foreground min-w-[80px]">
            Action:
          </span>
          <p className="text-sm flex-1">{request.context.description}</p>
        </div>

        {/* Tool input - collapsible */}
        <details className="group">
          <summary className="text-sm font-medium text-muted-foreground cursor-pointer hover:text-foreground transition-colors">
            View Tool Input ▼
          </summary>
          <Card className="mt-2 p-3 bg-muted">
            <pre className="text-xs overflow-x-auto whitespace-pre-wrap max-h-[200px] overflow-y-auto">
              {JSON.stringify(request.input, null, 2)}
            </pre>
          </Card>
        </details>
      </div>

      {/* Warning */}
      {request.context.risk_level !== 'low' && (
        <Card className="p-3 mb-4 border-yellow-500/50 bg-yellow-50 dark:bg-yellow-950/30">
          <div className="flex gap-2">
            <AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-400 shrink-0 mt-0.5" />
            <p className="text-xs text-yellow-800 dark:text-yellow-200">
              Please review the tool input carefully before allowing execution.
            </p>
          </div>
        </Card>
      )}

      {/* Action Buttons */}
      <div className="flex gap-2 flex-wrap">
        <Button
          variant="outline"
          size="sm"
          onClick={onDeny}
          className="flex-1 min-w-[100px]"
        >
          <XCircle className="h-4 w-4 mr-1" />
          Deny
        </Button>
        <Button
          variant="secondary"
          size="sm"
          onClick={onAllow}
          className="flex-1 min-w-[100px]"
        >
          <CheckCircle className="h-4 w-4 mr-1" />
          Allow Once
        </Button>
        <Button
          size="sm"
          onClick={onAllowAlways}
          className="flex-1 min-w-[100px]"
        >
          <CheckCircle className="h-4 w-4 mr-1" />
          Allow Always
        </Button>
      </div>
    </Card>
  );
}
