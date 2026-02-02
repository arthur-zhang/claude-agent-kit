import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { Badge } from '../ui/badge';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/tooltip';
import type { ChatMessage, TurnStats } from '../../types';
import { cn } from '../../lib/utils';
import { Terminal, CheckCircle2, XCircle, Copy, CornerUpRight, Users, FileText, Grip, Search, Globe, ListTodo, Zap, MessageCircleQuestion, Code, FolderSearch, Pencil, Play, Square, FileCode, Brain, Plus, Minus } from 'lucide-react';

interface MessageBubbleProps {
  message: ChatMessage;
  toolResultMessage?: ChatMessage;
  cwd?: string;
}

// Helper function to convert absolute path to relative path
function toRelativePath(absolutePath: string, cwd?: string): string {
  if (!cwd || !absolutePath) return absolutePath;
  // Normalize paths (remove trailing slashes)
  const normalizedCwd = cwd.endsWith('/') ? cwd.slice(0, -1) : cwd;
  if (absolutePath.startsWith(normalizedCwd + '/')) {
    return absolutePath.slice(normalizedCwd.length + 1);
  }
  return absolutePath;
}

// Helper to safely get string from unknown tool input
function getString(input: unknown, key: string): string {
  if (input && typeof input === 'object' && key in input) {
    const value = (input as Record<string, unknown>)[key];
    return typeof value === 'string' ? value : '';
  }
  return '';
}

// Helper to safely get number from unknown tool input
function getNumber(input: unknown, key: string): number | undefined {
  if (input && typeof input === 'object' && key in input) {
    const value = (input as Record<string, unknown>)[key];
    return typeof value === 'number' ? value : undefined;
  }
  return undefined;
}

// Helper to safely get boolean from unknown tool input
function getBoolean(input: unknown, key: string): boolean {
  if (input && typeof input === 'object' && key in input) {
    const value = (input as Record<string, unknown>)[key];
    return typeof value === 'boolean' ? value : false;
  }
  return false;
}

// Helper to safely get array length from unknown tool input
function getArrayLength(input: unknown, key: string): number {
  if (input && typeof input === 'object' && key in input) {
    const value = (input as Record<string, unknown>)[key];
    return Array.isArray(value) ? value.length : 0;
  }
  return 0;
}

// Helper to safely get nested string from array in tool input
function getNestedString(input: unknown, arrayKey: string, index: number, stringKey: string): string {
  if (input && typeof input === 'object' && arrayKey in input) {
    const arr = (input as Record<string, unknown>)[arrayKey];
    if (Array.isArray(arr) && arr[index] && typeof arr[index] === 'object') {
      const value = (arr[index] as Record<string, unknown>)[stringKey];
      return typeof value === 'string' ? value : '';
    }
  }
  return '';
}

// Hover-switchable icon component: shows icon normally, +/- on hover
function HoverIcon({ icon: Icon, iconClassName, isExpanded }: {
  icon: React.ComponentType<{ className?: string }>;
  iconClassName?: string;
  isExpanded: boolean;
}) {
  return (
    <span className="relative h-3.5 w-3.5">
      <Icon className={cn("h-3.5 w-3.5 transition-opacity duration-150 group-hover:opacity-0", iconClassName)} />
      {isExpanded ? (
        <Minus className="h-3.5 w-3.5 absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-150 text-muted-foreground" />
      ) : (
        <Plus className="h-3.5 w-3.5 absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-150 text-muted-foreground" />
      )}
    </span>
  );
}

export function MessageBubble({ message, toolResultMessage, cwd }: MessageBubbleProps) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';
  const [isToolExpanded, setIsToolExpanded] = useState(false);

  // System messages with turn stats - show duration with tooltip
  if (isSystem && message.turnStats) {
    return (
      <div className="flex justify-start mb-4 animate-in fade-in duration-200">
        <TurnStatsDisplay stats={message.turnStats} duration={message.content} />
      </div>
    );
  }

  // System messages with thinking/reasoning - collapsible with brain icon
  if (isSystem && message.isThinking) {
    const [isThinkingExpanded, setIsThinkingExpanded] = useState(false);
    const contentPreview = message.content.slice(0, 60);
    const hasMore = message.content.length > 60;

    return (
      <div className="flex justify-start mb-4 animate-in fade-in duration-200">
        <Collapsible open={isThinkingExpanded} onOpenChange={setIsThinkingExpanded} className="w-full max-w-[85%] lg:max-w-[75%]">
          <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
            <HoverIcon icon={Brain} iconClassName="text-pink-500" isExpanded={isThinkingExpanded} />
            <span className="font-medium text-pink-600 dark:text-pink-400">Thinking</span>
            {!isThinkingExpanded && (
              <span className="text-muted-foreground/70 truncate font-mono text-[11px] italic">
                {contentPreview}{hasMore ? '...' : ''}
              </span>
            )}
          </CollapsibleTrigger>
          <CollapsibleContent className="mt-2">
            <div className="rounded-lg p-3 text-xs overflow-x-auto border border-pink-200 dark:border-pink-900/50 bg-pink-50/50 dark:bg-pink-950/20 max-h-[300px] overflow-y-auto custom-scrollbar">
              <pre className="whitespace-pre-wrap break-all text-muted-foreground italic">{message.content}</pre>
            </div>
          </CollapsibleContent>
        </Collapsible>
      </div>
    );
  }

  // System messages with tool use - combined tool call + result display
  if (isSystem && message.toolUse) {
    const toolName = message.toolUse.tool_name;
    const toolInput = message.toolUse.tool_input;
    const parentToolUseId = message.toolUse.parent_tool_use_id;

    // Get paired tool result if available
    const pairedResult = toolResultMessage?.toolResult;
    const hasResult = !!pairedResult;
    const isError = pairedResult?.is_error || false;

    // Check if this is a SubAgent tool call (has parent_tool_use_id)
    const isSubAgent = !!parentToolUseId;

    // Special handling for Task tool
    const isTaskTool = toolName === 'Task';
    const subagentType = isTaskTool ? getString(toolInput, 'subagent_type') : '';
    const taskDescription = isTaskTool ? getString(toolInput, 'description') : '';
    const taskPrompt = isTaskTool ? getString(toolInput, 'prompt') : '';

    // Generate summary info based on tool type
    let summaryText = '';
    let fileNameBadge = '';

    if (toolName === 'Read') {
      const filePath = getString(toolInput, 'file_path');
      fileNameBadge = toRelativePath(filePath, cwd);
      // Count lines from result if available
      if (pairedResult?.content) {
        const lineCount = pairedResult.content.split('\n').length;
        summaryText = `Read ${lineCount} lines`;
      } else {
        summaryText = 'Read';
      }
    } else if (toolName === 'Write') {
      const filePath = getString(toolInput, 'file_path');
      fileNameBadge = toRelativePath(filePath, cwd);
      summaryText = 'Write';
    } else if (toolName === 'Edit') {
      const filePath = getString(toolInput, 'file_path');
      fileNameBadge = toRelativePath(filePath, cwd);
      summaryText = getBoolean(toolInput, 'replace_all') ? 'Replace all' : 'Edit';
    } else if (toolName === 'Bash') {
      const cmd = getString(toolInput, 'command');
      // Extract first command/program name
      const firstWord = cmd.split(/\s+/)[0] || 'Bash';
      summaryText = firstWord;
      fileNameBadge = cmd.length > 50 ? cmd.slice(0, 50) + '...' : cmd;
    } else if (toolName === 'Glob') {
      summaryText = 'Glob';
      const pattern = getString(toolInput, 'pattern');
      const path = getString(toolInput, 'path');
      fileNameBadge = path ? `${pattern} in ${toRelativePath(path, cwd)}` : pattern;
    } else if (toolName === 'Grep') {
      summaryText = 'Grep';
      const pattern = getString(toolInput, 'pattern');
      const path = getString(toolInput, 'path');
      fileNameBadge = path ? `"${pattern}" in ${toRelativePath(path, cwd)}` : `"${pattern}"`;
    } else if (toolName === 'WebFetch') {
      summaryText = 'Fetch';
      // Extract domain from URL
      const urlStr = getString(toolInput, 'url');
      try {
        const url = new URL(urlStr || 'http://unknown');
        fileNameBadge = url.hostname;
      } catch {
        fileNameBadge = urlStr.slice(0, 40);
      }
    } else if (toolName === 'WebSearch') {
      summaryText = 'Search';
      fileNameBadge = getString(toolInput, 'query').slice(0, 50);
    } else if (toolName === 'TodoWrite') {
      const todoCount = getArrayLength(toolInput, 'todos');
      summaryText = `${todoCount} todo${todoCount !== 1 ? 's' : ''}`;
    } else if (toolName === 'AskUserQuestion') {
      const questionCount = getArrayLength(toolInput, 'questions');
      summaryText = `${questionCount} question${questionCount !== 1 ? 's' : ''}`;
      fileNameBadge = getNestedString(toolInput, 'questions', 0, 'question').slice(0, 40);
    } else if (toolName === 'Skill') {
      summaryText = 'Skill';
      fileNameBadge = getString(toolInput, 'skill');
    } else if (toolName === 'EnterPlanMode') {
      summaryText = 'Enter Plan Mode';
    } else if (toolName === 'ExitPlanMode') {
      summaryText = 'Exit Plan Mode';
    } else if (toolName === 'LSP') {
      summaryText = getString(toolInput, 'operation') || 'LSP';
      const filePath = getString(toolInput, 'filePath');
      const line = getNumber(toolInput, 'line');
      fileNameBadge = filePath ? `${toRelativePath(filePath, cwd)}${line ? `:${line}` : ''}` : '';
    } else if (toolName === 'NotebookEdit') {
      const editMode = getString(toolInput, 'edit_mode');
      summaryText = editMode === 'insert' ? 'Insert cell' :
                    editMode === 'delete' ? 'Delete cell' : 'Edit cell';
      const notebookPath = getString(toolInput, 'notebook_path');
      fileNameBadge = toRelativePath(notebookPath, cwd);
    } else if (toolName === 'TaskOutput') {
      summaryText = 'Task Output';
      fileNameBadge = getString(toolInput, 'task_id');
    } else if (toolName === 'KillShell') {
      summaryText = 'Kill Shell';
      fileNameBadge = getString(toolInput, 'shell_id');
    } else if (isTaskTool) {
      summaryText = subagentType ? `Agent (${subagentType})` : 'Agent';
      fileNameBadge = taskDescription;
    } else {
      summaryText = toolName;
      const filePath = getString(toolInput, 'file_path');
      const pattern = getString(toolInput, 'pattern');
      const query = getString(toolInput, 'query');
      if (filePath) {
        fileNameBadge = toRelativePath(filePath, cwd);
      } else if (pattern) {
        fileNameBadge = pattern;
      } else if (query) {
        fileNameBadge = query.slice(0, 40);
      }
    }

    // Icon based on tool type and result status
    const getToolIconInfo = (): { icon: React.ComponentType<{ className?: string }>; className: string } => {
      if (hasResult) {
        return isError
          ? { icon: XCircle, className: "text-red-500" }
          : { icon: CheckCircle2, className: "text-green-500" };
      }
      if (isSubAgent) return { icon: Users, className: "text-purple-500" };
      switch (toolName) {
        case 'Read': return { icon: FileText, className: "text-blue-500" };
        case 'Write': return { icon: Pencil, className: "text-yellow-500" };
        case 'Edit': return { icon: Pencil, className: "text-orange-500" };
        case 'Bash': return { icon: Terminal, className: "text-green-500" };
        case 'Glob': return { icon: FolderSearch, className: "text-cyan-500" };
        case 'Grep': return { icon: Search, className: "text-cyan-500" };
        case 'WebFetch': return { icon: Globe, className: "text-blue-500" };
        case 'WebSearch': return { icon: Search, className: "text-blue-500" };
        case 'TodoWrite': return { icon: ListTodo, className: "text-purple-500" };
        case 'AskUserQuestion': return { icon: MessageCircleQuestion, className: "text-amber-500" };
        case 'Skill': return { icon: Zap, className: "text-yellow-500" };
        case 'EnterPlanMode':
        case 'ExitPlanMode': return { icon: FileText, className: "text-indigo-500" };
        case 'LSP': return { icon: Code, className: "text-violet-500" };
        case 'NotebookEdit': return { icon: FileCode, className: "text-orange-500" };
        case 'Task': return { icon: Users, className: "text-purple-500" };
        case 'TaskOutput': return { icon: Play, className: "text-green-500" };
        case 'KillShell': return { icon: Square, className: "text-red-500" };
        default: return { icon: Terminal, className: "text-muted-foreground" };
      }
    };

    const toolIconInfo = getToolIconInfo();

    return (
      <div className="flex justify-start mb-4 animate-in fade-in duration-200">
        <Collapsible open={isToolExpanded} onOpenChange={setIsToolExpanded} className={cn(
          "w-full max-w-[85%] lg:max-w-[75%]",
          isSubAgent && "pl-4 border-l-2 border-purple-300 dark:border-purple-700"
        )}>
          <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
            <HoverIcon icon={toolIconInfo.icon} iconClassName={toolIconInfo.className} isExpanded={isToolExpanded} />
            {isSubAgent && (
              <span className="text-[10px] px-1 py-0.5 rounded bg-purple-100 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 font-medium">
                SubAgent
              </span>
            )}
            <span className={cn(
              "font-medium",
              hasResult && isError ? "text-red-500" : "text-foreground"
            )}>{summaryText}</span>
            {fileNameBadge && (
              <span className="px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-mono text-[11px] truncate max-w-[200px]">
                {fileNameBadge}
              </span>
            )}
          </CollapsibleTrigger>
          <CollapsibleContent className="mt-2 space-y-2">
            {/* For Task tool, always show the prompt info */}
            {isTaskTool && taskPrompt ? (
              <div className="bg-muted/40 rounded-lg p-3 text-xs overflow-x-auto border border-border/50 space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground font-medium">Subagent:</span>
                  <span className="font-mono text-foreground">{subagentType || 'N/A'}</span>
                </div>
                {taskDescription && (
                  <div className="flex items-center gap-2">
                    <span className="text-muted-foreground font-medium">Description:</span>
                    <span className="text-foreground">{taskDescription}</span>
                  </div>
                )}
                <div>
                  <span className="text-muted-foreground font-medium block mb-1">Prompt:</span>
                  <pre className="whitespace-pre-wrap break-all text-foreground bg-background/50 rounded p-2 border border-border/30">
                    {taskPrompt}
                  </pre>
                </div>
              </div>
            ) : pairedResult ? (
              /* If we have result, only show result (no input) */
              <div className={cn(
                "rounded-lg p-3 text-xs font-mono overflow-x-auto border max-h-[300px] overflow-y-auto custom-scrollbar",
                isError
                  ? "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-900/50 text-red-700 dark:text-red-300"
                  : "bg-muted/40 border-border/50 text-muted-foreground"
              )}>
                <pre className="whitespace-pre-wrap break-all">{pairedResult.content}</pre>
              </div>
            ) : (
              /* No result yet, show input */
              <div className="bg-muted/40 rounded-lg p-3 text-xs font-mono overflow-x-auto border border-border/50">
                <pre className="whitespace-pre-wrap break-all">{JSON.stringify(toolInput, null, 2)}</pre>
              </div>
            )}
          </CollapsibleContent>
        </Collapsible>
      </div>
    );
  }

  // System messages with orphan tool results (no paired tool call)
  if (isSystem && message.toolResult) {
    const isError = message.toolResult.is_error;
    const contentLines = message.toolResult.content.split('\n');
    const previewLine = contentLines[0]?.slice(0, 60) || '';
    const hasMore = contentLines.length > 1 || previewLine.length < (contentLines[0]?.length || 0);

    const resultIcon = isError ? XCircle : CheckCircle2;
    const resultIconClass = isError ? "text-red-500" : "text-green-500";

    return (
      <div className="flex justify-start mb-4 animate-in fade-in duration-200">
        <Collapsible open={isToolExpanded} onOpenChange={setIsToolExpanded} className="w-full max-w-[85%] lg:max-w-[75%]">
          <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
            <HoverIcon icon={resultIcon} iconClassName={resultIconClass} isExpanded={isToolExpanded} />
            <span className={cn(
              "font-medium",
              isError ? "text-red-500" : "text-green-600 dark:text-green-400"
            )}>
              {isError ? 'Error' : 'Result'}
            </span>
            {!isToolExpanded && (
              <span className="text-muted-foreground/70 truncate font-mono text-[11px]">
                {previewLine}{hasMore ? '...' : ''}
              </span>
            )}
          </CollapsibleTrigger>
          <CollapsibleContent className="mt-2">
            <div className={cn(
              "rounded-lg p-3 text-xs font-mono overflow-x-auto border max-h-[300px] overflow-y-auto custom-scrollbar",
              isError
                ? "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-900/50 text-red-700 dark:text-red-300"
                : "bg-muted/40 border-border/50 text-muted-foreground"
            )}>
              <pre className="whitespace-pre-wrap break-all">{message.toolResult.content}</pre>
            </div>
          </CollapsibleContent>
        </Collapsible>
      </div>
    );
  }

  // Regular system messages - status messages left-aligned, others centered
  if (isSystem) {
    return (
      <div className={cn(
        "flex mb-4 animate-in fade-in zoom-in duration-300",
        message.isStatusMessage ? "justify-start" : "justify-center"
      )}>
        <Badge variant="secondary" className="px-3 py-1 text-xs font-normal opacity-80">
          {message.content}
        </Badge>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex gap-3 mb-6 animate-in fade-in slide-in-from-bottom-2 duration-300 group",
        isUser ? "flex-row-reverse" : "flex-row"
      )}
    >
      <div className={cn(
        "flex flex-col max-w-[85%] lg:max-w-[75%]",
        isUser ? "items-end ml-auto" : "items-start"
      )}>
        <div className="flex items-center gap-2 mb-1 px-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <span className="text-xs font-semibold text-foreground/70">
            {isUser ? 'You' : 'Claude'}
          </span>
          <span className="text-[10px] text-muted-foreground">
            {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </span>
        </div>

        <div
          className={cn(
            "relative p-3.5 rounded-2xl text-sm overflow-hidden",
            isUser
              ? "bg-primary text-primary-foreground rounded-tr-sm shadow-sm"
              : "rounded-tl-sm"
          )}
        >
          {/* Markdown Content */}
          <div className={cn(
            "prose prose-sm max-w-none break-words leading-relaxed",
            isUser
              ? "prose-invert dark:prose-invert prose-p:leading-relaxed"
              : "dark:prose-invert"
          )}>
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                p({ children }) {
                  return <p className="mb-2 last:mb-0">{children}</p>
                },
                code({ node, inline, className, children, ...props }: any) {
                  const match = /language-(\w+)/.exec(className || '');
                  return !inline && match ? (
                    <div className="relative my-3 rounded-md overflow-hidden bg-zinc-950 border border-zinc-800">
                      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-800 text-zinc-400 text-xs font-mono">
                        <span>{match[1]}</span>
                      </div>
                      <SyntaxHighlighter
                        {...props}
                        style={vscDarkPlus}
                        language={match[1]}
                        PreTag="div"
                        customStyle={{ margin: 0, borderRadius: 0, padding: '1rem', fontSize: '13px' }}
                      >
                        {String(children).replace(/\n$/, '')}
                      </SyntaxHighlighter>
                    </div>
                  ) : (
                    <code {...props} className={cn(
                      "px-1.5 py-0.5 rounded font-mono text-[0.9em]",
                      isUser
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted text-foreground border border-border"
                    )}>
                      {children}
                    </code>
                  );
                },
                a({ children, href }) {
                  return <a href={href} target="_blank" rel="noreferrer" className="underline underline-offset-4 decoration-current/50 hover:decoration-current">{children}</a>
                }
              }}
            >
              {message.content}
            </ReactMarkdown>
          </div>

          {message.isStreaming && (
             <span className="inline-block w-1.5 h-4 ml-1 bg-current animate-pulse align-middle" />
          )}
        </div>

        {/* Tool Use Section - for assistant messages */}
        {message.toolUse && (
          <ToolCallCollapsible
            toolName={message.toolUse.tool_name}
            toolInput={message.toolUse.tool_input}
            parentToolUseId={message.toolUse.parent_tool_use_id}
          />
        )}

        {/* Tool Result Section - for assistant messages */}
        {message.toolResult && (
          <ToolResultCollapsible
            content={message.toolResult.content}
            isError={message.toolResult.is_error}
          />
        )}

      </div>
    </div>
  );
}

// Separate component for tool call display
function ToolCallCollapsible({ 
  toolName, 
  toolInput, 
  parentToolUseId 
}: { 
  toolName: string; 
  toolInput: any; 
  parentToolUseId?: string | null;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Check if this is a SubAgent tool call (has parent_tool_use_id)
  const isSubAgent = !!parentToolUseId;

  // Special handling for Task tool - show subagent_type and description
  const isTaskTool = toolName === 'Task';
  const subagentType = isTaskTool ? toolInput?.subagent_type : null;
  const taskDescription = isTaskTool ? toolInput?.description : null;
  const taskPrompt = isTaskTool ? toolInput?.prompt : null;

  let description = '';
  if (isTaskTool) {
    // For Task tool, show subagent_type and description
    if (taskDescription) {
      description = taskDescription;
    }
  } else if (toolInput?.command) {
    description = toolInput.command.slice(0, 50);
  } else if (toolInput?.pattern) {
    description = `pattern: ${toolInput.pattern}`;
  } else if (toolInput?.file_path) {
    description = toolInput.file_path.split('/').pop() || toolInput.file_path;
  } else if (toolInput?.query) {
    description = toolInput.query.slice(0, 40);
  }

  // Display name for Task tool includes subagent type
  const displayName = isTaskTool && subagentType 
    ? `Agent (${subagentType})` 
    : toolName;

  // Icon based on whether this is a SubAgent or regular tool
  const ToolIcon = isSubAgent ? Users : Terminal;
  const iconColor = isSubAgent ? "text-purple-500" : "text-blue-500";

  return (
    <div className={cn("mt-2 w-full", isSubAgent && "pl-4 border-l-2 border-purple-300 dark:border-purple-700")}>
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
          <HoverIcon icon={ToolIcon} iconClassName={iconColor} isExpanded={isExpanded} />
          {isSubAgent && (
            <span className="text-[10px] px-1 py-0.5 rounded bg-purple-100 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 font-medium">
              SubAgent
            </span>
          )}
          <span className="font-medium text-foreground">{displayName}</span>
          {!isExpanded && description && (
            <span className="text-muted-foreground/70 truncate font-mono text-[11px]">
              {description}
            </span>
          )}
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-2">
          {isTaskTool && taskPrompt ? (
            <div className="bg-muted/40 rounded-lg p-3 text-xs overflow-x-auto border border-border/50 space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground font-medium">Subagent:</span>
                <span className="font-mono text-foreground">{subagentType || 'N/A'}</span>
              </div>
              {taskDescription && (
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground font-medium">Description:</span>
                  <span className="text-foreground">{taskDescription}</span>
                </div>
              )}
              <div>
                <span className="text-muted-foreground font-medium block mb-1">Prompt:</span>
                <pre className="whitespace-pre-wrap break-all text-foreground bg-background/50 rounded p-2 border border-border/30">
                  {taskPrompt}
                </pre>
              </div>
            </div>
          ) : (
            <div className="bg-muted/40 rounded-lg p-3 text-xs font-mono overflow-x-auto border border-border/50">
              <pre className="whitespace-pre-wrap break-all">{JSON.stringify(toolInput, null, 2)}</pre>
            </div>
          )}
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}

// Separate component for tool result display
function ToolResultCollapsible({ content, isError }: { content: string; isError: boolean }) {
  const [isExpanded, setIsExpanded] = useState(false);

  const contentLines = content.split('\n');
  const previewLine = contentLines[0]?.slice(0, 60) || '';
  const hasMore = contentLines.length > 1 || previewLine.length < (contentLines[0]?.length || 0);

  const resultIcon = isError ? XCircle : CheckCircle2;
  const resultIconClass = isError ? "text-red-500" : "text-green-500";

  return (
    <div className="mt-2 w-full">
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
          <HoverIcon icon={resultIcon} iconClassName={resultIconClass} isExpanded={isExpanded} />
          <span className={cn(
            "font-medium",
            isError ? "text-red-500" : "text-green-600 dark:text-green-400"
          )}>
            {isError ? 'Error' : 'Result'}
          </span>
          {!isExpanded && (
            <span className="text-muted-foreground/70 truncate font-mono text-[11px]">
              {previewLine}{hasMore ? '...' : ''}
            </span>
          )}
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-2">
          <div className={cn(
            "rounded-lg p-3 text-xs font-mono overflow-x-auto border max-h-[300px] overflow-y-auto custom-scrollbar",
            isError
              ? "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-900/50 text-red-700 dark:text-red-300"
              : "bg-muted/40 border-border/50 text-muted-foreground"
          )}>
            <pre className="whitespace-pre-wrap break-all">{content}</pre>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}

// Turn stats display with tooltip
function TurnStatsDisplay({ stats, duration }: { stats: TurnStats; duration: string }) {
  const formatDate = (date: Date) => {
    return date.toLocaleString([], {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const formatNumber = (num: number) => {
    return num.toLocaleString();
  };

  return (
    <TooltipProvider>
      <Tooltip delayDuration={200}>
        <TooltipTrigger asChild>
          <div className="flex items-center gap-2 text-xs text-muted-foreground cursor-default hover:text-foreground transition-colors">
            <span className="font-mono">{duration}</span>
            <span className="text-muted-foreground/50">·</span>
            <Copy className="h-3 w-3 opacity-50 hover:opacity-100 cursor-pointer" />
            <CornerUpRight className="h-3 w-3 opacity-50 hover:opacity-100 cursor-pointer" />
          </div>
        </TooltipTrigger>
        <TooltipContent side="top" align="start" className="p-0 w-72">
          <div className="p-3 space-y-2">
            {/* Model and time range */}
            <div className="space-y-1">
              <div className="font-medium text-sm">
                {stats.model || 'Claude'} via Claude Code
              </div>
              <div className="text-[11px] text-muted-foreground flex items-center gap-1">
                {formatDate(stats.start_time)}
                <span className="mx-1">→</span>
                {formatDate(stats.end_time)}
              </div>
            </div>

            {/* Separator */}
            <div className="border-t border-border" />

            {/* Token stats */}
            <div className="space-y-1.5 text-xs">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Input</span>
                <span className="font-mono">{formatNumber(stats.input_tokens)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Output</span>
                <span className="font-mono">{formatNumber(stats.output_tokens)}</span>
              </div>
              {stats.cached_tokens > 0 && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Cache read +</span>
                  <span className="font-mono">{formatNumber(stats.cached_tokens)}</span>
                </div>
              )}
            </div>

            {/* Cost */}
            {stats.total_cost_usd !== undefined && (
              <>
                <div className="border-t border-border" />
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Cost</span>
                  <span className="font-mono">${stats.total_cost_usd.toFixed(4)}</span>
                </div>
              </>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

// Agent Tool Display - shows Agent with nested child tools (like in the reference image)
interface AgentToolDisplayProps {
  agentMessage: ChatMessage;
  childMessages: ChatMessage[];
  cwd?: string;
}

export function AgentToolDisplay({ agentMessage, childMessages, cwd }: AgentToolDisplayProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [isPromptExpanded, setIsPromptExpanded] = useState(false);

  const toolInput = agentMessage.toolUse?.tool_input;
  const description = getString(toolInput, 'description');
  const prompt = getString(toolInput, 'prompt');

  // Pair tool calls with their results
  const pairedChildren: { toolCall: ChatMessage; toolResult?: ChatMessage }[] = [];
  const toolCallMap = new Map<string, number>(); // tool_id -> index in pairedChildren
  const processedResultIds = new Set<string>();

  for (const child of childMessages) {
    if (child.toolUse) {
      const index = pairedChildren.length;
      pairedChildren.push({ toolCall: child });
      if (child.toolUse.tool_id) {
        toolCallMap.set(child.toolUse.tool_id, index);
      }
    } else if (child.toolResult) {
      const toolId = child.toolResult.tool_id;
      const index = toolCallMap.get(toolId);
      if (index !== undefined) {
        pairedChildren[index].toolResult = child;
        processedResultIds.add(child.id);
      }
    }
  }

  // Filter out paired results, keep orphan results
  const orphanResults = childMessages.filter(
    child => child.toolResult && !processedResultIds.has(child.id)
  );

  return (
    <div className="flex justify-start mb-4 animate-in fade-in duration-200">
      <div className="w-full max-w-[85%] lg:max-w-[75%]">
        {/* Agent Header */}
        <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
          <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
            <HoverIcon icon={Grip} iconClassName="text-muted-foreground" isExpanded={isExpanded} />
            <span className="font-semibold text-foreground">Agent</span>
            <span className="text-muted-foreground/80">{description}</span>
          </CollapsibleTrigger>

          <CollapsibleContent className="mt-1">
            {/* Vertical line for nesting */}
            <div className="border-l border-muted-foreground/30 ml-1.5 pl-4 space-y-1">
              {/* Prompt section */}
              <Collapsible open={isPromptExpanded} onOpenChange={setIsPromptExpanded}>
                <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer group">
                  <HoverIcon icon={FileText} iconClassName="text-muted-foreground" isExpanded={isPromptExpanded} />
                  <span className="font-medium">Prompt</span>
                </CollapsibleTrigger>
                <CollapsibleContent className="mt-1 ml-5">
                  <div className="bg-muted/40 rounded-lg p-2 text-xs border border-border/50 max-h-[200px] overflow-y-auto">
                    <pre className="whitespace-pre-wrap break-all text-muted-foreground">{prompt}</pre>
                  </div>
                </CollapsibleContent>
              </Collapsible>

              {/* Paired tool calls with results */}
              {pairedChildren.map((pair) => (
                <NestedToolCall
                  key={pair.toolCall.id}
                  message={pair.toolCall}
                  pairedResult={pair.toolResult}
                  cwd={cwd}
                />
              ))}

              {/* Orphan results (shouldn't happen often) */}
              {orphanResults.map((child) => (
                <NestedToolCall key={child.id} message={child} cwd={cwd} />
              ))}
            </div>
          </CollapsibleContent>
        </Collapsible>
      </div>
    </div>
  );
}

// Nested tool call or result display for SubAgent children
function NestedToolCall({ message, pairedResult, cwd }: { message: ChatMessage; pairedResult?: ChatMessage; cwd?: string }) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toolUse = message.toolUse;
  const toolResult = message.toolResult;

  // Get paired result content if available
  const resultContent = pairedResult?.toolResult;
  const hasResult = !!resultContent;
  const isResultError = resultContent?.is_error || false;

  // Handle tool result display
  if (toolResult) {
    const isError = toolResult.is_error;
    const contentLines = toolResult.content.split('\n');
    const previewLine = contentLines[0]?.slice(0, 50) || '';
    const hasMore = contentLines.length > 1 || previewLine.length < (contentLines[0]?.length || 0);

    const resultIcon = isError ? XCircle : CheckCircle2;
    const resultIconClass = isError ? "text-red-500" : "text-green-500";

    return (
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
          <HoverIcon icon={resultIcon} iconClassName={resultIconClass} isExpanded={isExpanded} />
          <span className={cn(
            "font-medium",
            isError ? "text-red-500" : "text-green-600 dark:text-green-400"
          )}>
            {isError ? 'Error' : 'Result'}
          </span>
          {!isExpanded && (
            <span className="text-muted-foreground/70 truncate font-mono text-[11px] max-w-[300px]">
              {previewLine}{hasMore ? '...' : ''}
            </span>
          )}
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-1 ml-5">
          <div className={cn(
            "rounded-lg p-2 text-xs font-mono border max-h-[200px] overflow-y-auto",
            isError
              ? "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-900/50 text-red-700 dark:text-red-300"
              : "bg-muted/40 border-border/50 text-muted-foreground"
          )}>
            <pre className="whitespace-pre-wrap break-all">{toolResult.content}</pre>
          </div>
        </CollapsibleContent>
      </Collapsible>
    );
  }
  
  // Handle tool use display
  if (!toolUse) return null;

  const toolName = toolUse.tool_name;
  const toolInput = toolUse.tool_input;

  // Generate preview text based on tool type
  let preview = '';
  let previewBadge = '';

  if (toolName === 'Read') {
    const path = getString(toolInput, 'file_path') || getString(toolInput, 'path');
    preview = toRelativePath(path, cwd);
    // Try to determine line count from result if available
    if (resultContent?.content) {
      const lineCount = resultContent.content.split('\n').length;
      previewBadge = `Read ${lineCount} lines`;
    } else {
      // Check for read_range array
      if (toolInput && typeof toolInput === 'object' && 'read_range' in toolInput) {
        const range = (toolInput as Record<string, unknown>)['read_range'];
        if (Array.isArray(range) && range.length === 2 && typeof range[0] === 'number' && typeof range[1] === 'number') {
          const lines = range[1] - range[0] + 1;
          previewBadge = `Read ${lines} lines`;
        } else {
          previewBadge = 'Read';
        }
      } else {
        previewBadge = 'Read';
      }
    }
  } else if (toolName === 'Bash') {
    const cmd = getString(toolInput, 'command');
    const firstWord = cmd.split(/\s+/)[0] || 'Bash';
    previewBadge = firstWord;
    preview = cmd.length > 50 ? cmd.slice(0, 50) + '...' : cmd;
  } else if (toolName === 'Write') {
    const path = getString(toolInput, 'file_path') || getString(toolInput, 'path');
    preview = toRelativePath(path, cwd);
    previewBadge = 'Write';
  } else if (toolName === 'Edit') {
    const path = getString(toolInput, 'file_path') || getString(toolInput, 'path');
    preview = toRelativePath(path, cwd);
    previewBadge = getBoolean(toolInput, 'replace_all') ? 'Replace all' : 'Edit';
  } else if (toolName === 'Grep') {
    const pattern = getString(toolInput, 'pattern');
    const path = getString(toolInput, 'path');
    previewBadge = 'Grep';
    preview = path ? `"${pattern}" in ${toRelativePath(path, cwd)}` : `"${pattern}"`;
  } else if (toolName === 'Glob') {
    const pattern = getString(toolInput, 'pattern') || getString(toolInput, 'filePattern');
    const path = getString(toolInput, 'path');
    previewBadge = 'Glob';
    preview = path ? `${pattern} in ${toRelativePath(path, cwd)}` : pattern;
  } else if (toolName === 'WebFetch') {
    previewBadge = 'Fetch';
    const urlStr = getString(toolInput, 'url');
    try {
      const url = new URL(urlStr || 'http://unknown');
      preview = url.hostname;
    } catch {
      preview = urlStr.slice(0, 40);
    }
  } else if (toolName === 'WebSearch') {
    previewBadge = 'Search';
    preview = getString(toolInput, 'query').slice(0, 50);
  } else if (toolName === 'LSP') {
    previewBadge = getString(toolInput, 'operation') || 'LSP';
    const filePath = getString(toolInput, 'filePath');
    const line = getNumber(toolInput, 'line');
    preview = filePath ? `${toRelativePath(filePath, cwd)}${line ? `:${line}` : ''}` : '';
  } else {
    previewBadge = toolName;
    preview = '';
  }

  // Get icon based on tool name and result status
  const getToolIconInfo = (): { icon: React.ComponentType<{ className?: string }>; className: string } => {
    if (hasResult) {
      return isResultError
        ? { icon: XCircle, className: "text-red-500" }
        : { icon: CheckCircle2, className: "text-green-500" };
    }
    switch (toolName) {
      case 'Read': return { icon: FileText, className: "text-blue-500" };
      case 'Write': return { icon: Pencil, className: "text-yellow-500" };
      case 'Edit': return { icon: Pencil, className: "text-orange-500" };
      case 'Bash': return { icon: Terminal, className: "text-green-500" };
      case 'Glob': return { icon: FolderSearch, className: "text-cyan-500" };
      case 'Grep': return { icon: Search, className: "text-cyan-500" };
      case 'WebFetch': return { icon: Globe, className: "text-blue-500" };
      case 'WebSearch': return { icon: Search, className: "text-blue-500" };
      case 'LSP': return { icon: Code, className: "text-violet-500" };
      default: return { icon: Terminal, className: "text-muted-foreground" };
    }
  };

  const toolIconInfo = getToolIconInfo();

  return (
    <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
      <CollapsibleTrigger className="flex items-center gap-2 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full group">
        <HoverIcon icon={toolIconInfo.icon} iconClassName={toolIconInfo.className} isExpanded={isExpanded} />
        {previewBadge && (
          <span className={cn(
            "font-medium",
            hasResult && isResultError ? "text-red-500" : "text-foreground"
          )}>{previewBadge}</span>
        )}
        {preview && (
          <span className="px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-mono text-[11px] truncate max-w-[300px]">
            {preview}
          </span>
        )}
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-1 ml-5">
        {resultContent ? (
          /* If we have result, only show result */
          <div className={cn(
            "rounded-lg p-2 text-xs font-mono border max-h-[200px] overflow-y-auto",
            isResultError
              ? "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-900/50 text-red-700 dark:text-red-300"
              : "bg-muted/40 border-border/50 text-muted-foreground"
          )}>
            <pre className="whitespace-pre-wrap break-all">{resultContent.content}</pre>
          </div>
        ) : (
          /* No result yet, show input */
          <div className="bg-muted/40 rounded-lg p-2 text-xs font-mono border border-border/50 max-h-[200px] overflow-y-auto">
            <pre className="whitespace-pre-wrap break-all">{JSON.stringify(toolInput, null, 2)}</pre>
          </div>
        )}
      </CollapsibleContent>
    </Collapsible>
  );
}
