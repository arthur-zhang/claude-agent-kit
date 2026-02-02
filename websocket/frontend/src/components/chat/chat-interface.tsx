import { useState } from 'react';
import { useWebSocket } from '../../hooks/useWebSocket';
import { Header } from '../layout/header';
import { MessageList } from './message-list';
import { MessageInput } from './message-input';
import { PermissionCard } from './permission-card';
import { UserQuestionDialog } from '../dialogs/user-question-dialog';
import { Card } from '../ui/card';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { Label } from '../ui/label';
import { Badge } from '../ui/badge';
import { Separator } from '../ui/separator';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Checkbox } from '../ui/checkbox';
import type { PermissionMode } from '../../types';

const WS_URL = 'ws://localhost:3000/ws';

// Get resume session ID from URL parameter
function getResumeSessionId(): string {
  if (typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    return params.get('resume') || '';
  }
  return '';
}

export function ChatInterface() {
  const [cwd, setCwd] = useState('/tmp');
  const [model, setModel] = useState('opus');
  const [disallowedTools, setDisallowedTools] = useState('');
  const [enableThinking, setEnableThinking] = useState(true);
  const [permissionMode, setPermissionMode] = useState<PermissionMode>('default');
  const [dangerouslySkipPermissions, setDangerouslySkipPermissions] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [resumeSessionId, setResumeSessionId] = useState(getResumeSessionId);

  const {
    isConnected,
    messages,
    pendingPermission,
    pendingUserQuestion,
    sessionInfo,
    tokenUsage,
    sessionId: actualSessionId,
    isProcessing,
    connect,
    disconnect,
    sendMessage,
    respondToPermission,
    respondToUserQuestion,
    cancelUserQuestion,
    interrupt,
    clearMessages,
  } = useWebSocket({
    url: WS_URL,
    cwd,
    model,
    disallowedTools,
    enableThinking,
    permissionMode,
    dangerouslySkipPermissions: dangerouslySkipPermissions || undefined,
    resumeSessionId: resumeSessionId || undefined,
  });

  const handleSend = (message: string) => {
    if (isConnected) {
      sendMessage(message);
    }
  };

  const connectionStatus = isConnected ? 'connected' : 'disconnected';

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Header */}
      <Header isConnected={isConnected} connectionStatus={connectionStatus} />

      {/* Settings Panel (collapsible) */}
      {!isConnected && (
        <Card className="m-4 p-4">
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">Connection Settings</h2>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowSettings(!showSettings)}
              >
                {showSettings ? 'Hide' : 'Show'}
              </Button>
            </div>

            {showSettings && (
              <>
                <Separator />
                <div className="grid gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="cwd">Working Directory</Label>
                    <Input
                      id="cwd"
                      value={cwd}
                      onChange={(e) => setCwd(e.target.value)}
                      placeholder="/tmp"
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="model">Model</Label>
                    <Select value={model} onValueChange={setModel}>
                      <SelectTrigger id="model">
                        <SelectValue placeholder="Select model" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="opus">Claude Opus 4</SelectItem>
                        <SelectItem value="sonnet">Claude Sonnet 4</SelectItem>
                        <SelectItem value="haiku">Claude Haiku 3.5</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="disallowedTools">
                      Disallowed Tools (comma-separated)
                    </Label>
                    <Input
                      id="disallowedTools"
                      value={disallowedTools}
                      onChange={(e) => setDisallowedTools(e.target.value)}
                      placeholder="e.g., Bash,Edit,Write"
                    />
                  </div>

                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="enableThinking"
                      checked={enableThinking}
                      onCheckedChange={(checked) => setEnableThinking(checked === true)}
                    />
                    <Label htmlFor="enableThinking" className="cursor-pointer">
                      Enable Extended Thinking
                    </Label>
                  </div>

                  <Separator />

                  <div className="space-y-2">
                    <Label htmlFor="permissionMode">Permission Mode</Label>
                    <Select value={permissionMode} onValueChange={(v) => setPermissionMode(v as PermissionMode)}>
                      <SelectTrigger id="permissionMode">
                        <SelectValue placeholder="Select permission mode" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="default">Default (prompt for dangerous ops)</SelectItem>
                        <SelectItem value="acceptEdits">Accept Edits (auto-approve file edits)</SelectItem>
                        <SelectItem value="plan">Plan (planning only, no execution)</SelectItem>
                        <SelectItem value="bypassPermissions">Bypass Permissions (auto-approve all)</SelectItem>
                        <SelectItem value="delegate">Delegate (delegate to parent)</SelectItem>
                        <SelectItem value="dontAsk">Don't Ask (deny all without prompting)</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  {permissionMode === 'bypassPermissions' && (
                    <div className="flex items-center space-x-2 p-2 bg-destructive/10 rounded-md">
                      <Checkbox
                        id="dangerouslySkipPermissions"
                        checked={dangerouslySkipPermissions}
                        onCheckedChange={(checked) => setDangerouslySkipPermissions(checked === true)}
                      />
                      <Label htmlFor="dangerouslySkipPermissions" className="cursor-pointer text-destructive">
                        ⚠️ I understand the risks - allow bypassing permissions
                      </Label>
                    </div>
                  )}

                  <Separator />

                  <div className="space-y-2">
                    <Label htmlFor="resumeSessionId">
                      Resume Session ID (optional)
                    </Label>
                    <Input
                      id="resumeSessionId"
                      value={resumeSessionId}
                      onChange={(e) => setResumeSessionId(e.target.value)}
                      placeholder="e.g., 550e8400-e29b-41d4-a716-446655440000"
                    />
                    <p className="text-xs text-muted-foreground">
                      Enter a previous session ID to resume the conversation
                    </p>
                  </div>
                </div>
              </>
            )}

            <div className="flex gap-2">
              <Button onClick={connect} className="flex-1">
                Connect to Agent
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* System Info */}
      {sessionInfo && isConnected && (
        <Card className="m-4 p-4">
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold">Session Info</h3>
              <Badge variant="outline">{actualSessionId}</Badge>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div>
                <span className="text-muted-foreground">Model:</span>{' '}
                <span className="font-medium">{sessionInfo.model || 'N/A'}</span>
              </div>
              <div>
                <span className="text-muted-foreground">CWD:</span>{' '}
                <span className="font-medium">{sessionInfo.cwd || 'N/A'}</span>
              </div>
              {tokenUsage && (
                <>
                  <div>
                    <span className="text-muted-foreground">Total Tokens:</span>{' '}
                    <span className="font-medium">
                      {tokenUsage.total_tokens.toLocaleString()}
                    </span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Input/Output:</span>{' '}
                    <span className="font-medium">
                      {tokenUsage.input_tokens}/{tokenUsage.output_tokens}
                    </span>
                  </div>
                </>
              )}
            </div>
          </div>
        </Card>
      )}

      {/* Control Buttons */}
      {isConnected && (
        <div className="px-4 pb-2 flex gap-2">
          <Button variant="outline" size="sm" onClick={interrupt}>
            ⏸️ Interrupt
          </Button>
          <Button variant="outline" size="sm" onClick={disconnect}>
            Disconnect
          </Button>
          <Button variant="outline" size="sm" onClick={clearMessages}>
            🗑️ Clear
          </Button>
        </div>
      )}

      {/* Messages area */}
      <MessageList messages={messages} isProcessing={isProcessing} cwd={sessionInfo?.cwd || cwd} />

      {/* Permission card - inline in chat */}
      {pendingPermission && (
        <div className="px-4">
          <PermissionCard
            request={pendingPermission}
            onAllow={() => respondToPermission('allow')}
            onDeny={() => respondToPermission('deny')}
            onAllowAlways={() => respondToPermission('allow_always')}
          />
        </div>
      )}

      {/* Input area */}
      <MessageInput
        onSend={handleSend}
        onInterrupt={interrupt}
        disabled={!isConnected}
        isProcessing={isProcessing}
        placeholder={
          isConnected
            ? 'Ask to make changes, @mention files, run /commands'
            : 'Connect to start chatting'
        }
      />

      {/* User question dialog */}
      {pendingUserQuestion && (
        <UserQuestionDialog
          request={pendingUserQuestion}
          onSubmit={(answers) =>
            respondToUserQuestion(pendingUserQuestion.request_id, answers)
          }
          onCancel={cancelUserQuestion}
        />
      )}
    </div>
  );
}
