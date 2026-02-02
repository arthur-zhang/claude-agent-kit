import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Card } from '../ui/card';
import { AlertTriangle } from 'lucide-react';
import type { PermissionRequest } from '../../types';

interface PermissionDialogProps {
  request: PermissionRequest;
  onAllow: () => void;
  onDeny: () => void;
  onAllowAlways: () => void;
}

export function PermissionDialog({
  request,
  onAllow,
  onDeny,
  onAllowAlways,
}: PermissionDialogProps) {
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
    <Dialog open={true}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            🔐 Permission Required
          </DialogTitle>
          <DialogDescription>
            Claude needs your permission to execute this tool
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Tool name */}
          <div className="space-y-2">
            <label className="text-sm font-semibold text-muted-foreground">
              Tool
            </label>
            <Card className="p-3 bg-muted">
              <code className="text-sm font-mono">{request.toolName}</code>
            </Card>
          </div>

          {/* Description */}
          <div className="space-y-2">
            <label className="text-sm font-semibold text-muted-foreground">
              Description
            </label>
            <Card className="p-3 bg-primary/5">
              <p className="text-sm">{request.context.description}</p>
            </Card>
          </div>

          {/* Risk level */}
          <div className="space-y-2">
            <label className="text-sm font-semibold text-muted-foreground">
              Risk Level
            </label>
            <div>
              <Badge variant={getRiskVariant(request.context.risk_level)}>
                {request.context.risk_level.toUpperCase()}
              </Badge>
            </div>
          </div>

          {/* Tool input */}
          <div className="space-y-2">
            <label className="text-sm font-semibold text-muted-foreground">
              Tool Input
            </label>
            <Card className="p-3 bg-muted">
              <pre className="text-xs overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(request.input, null, 2)}
              </pre>
            </Card>
          </div>

          {/* Warning */}
          <Card className="p-4 border-yellow-500 bg-yellow-50 dark:bg-yellow-950">
            <div className="flex gap-3">
              <AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400 shrink-0" />
              <p className="text-sm text-yellow-800 dark:text-yellow-200">
                Please review the tool input carefully before allowing execution.
              </p>
            </div>
          </Card>
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onDeny}>
            ❌ Deny
          </Button>
          <Button variant="secondary" onClick={onAllow}>
            ✅ Allow Once
          </Button>
          <Button onClick={onAllowAlways}>
            ✅ Allow Always
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
