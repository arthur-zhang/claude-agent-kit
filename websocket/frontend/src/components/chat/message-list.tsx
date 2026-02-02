import { useEffect, useRef, useState, useMemo } from 'react';
import { ScrollArea } from '../ui/scroll-area';
import { MessageBubble, AgentToolDisplay } from './message-bubble';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { cn } from '../../lib/utils';
import type { ChatMessage } from '../../types';
import { ChevronDown, Terminal, MessageSquare, Brain, Grip } from 'lucide-react';

interface MessageListProps {
  messages: ChatMessage[];
  isProcessing?: boolean;
  cwd?: string;
}

// Group messages by agent - SubAgent tool calls are nested under their parent Task
interface MessageGroup {
  message: ChatMessage;
  children: ChatMessage[];
  // For tool calls: the corresponding result message
  toolResultMessage?: ChatMessage;
}

// Turn - a round of conversation (user message -> assistant response with all tools)
interface Turn {
  id: string;
  userMessage?: ChatMessage;
  assistantGroups: MessageGroup[];
  isCompleted: boolean; // Whether this turn has finished (has turnStats)
  stats: {
    toolCalls: number;
    messages: number;
    subagents: number;
    thinkingCount: number;
  };
}

export function MessageList({ messages, isProcessing = false, cwd }: MessageListProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const [elapsedTime, setElapsedTime] = useState(0);
  const startTimeRef = useRef<number | null>(null);
  const [collapsedTurns, setCollapsedTurns] = useState<Set<string>>(new Set());

  // Group messages into turns and then by agent
  const turns = useMemo(() => {
    const result: Turn[] = [];
    let currentTurn: Turn | null = null;
    const taskToolIds = new Map<string, { turnIndex: number; groupIndex: number }>();
    // Map tool_id to the group index for pairing tool calls with results
    const toolCallMap = new Map<string, { turnIndex: number; groupIndex: number }>();

    for (const msg of messages) {
      const toolUse = msg.toolUse;
      const toolResult = msg.toolResult;

      // Start a new turn when we see a user message
      if (msg.role === 'user' && !toolUse && !toolResult) {
        if (currentTurn) {
          result.push(currentTurn);
        }
        currentTurn = {
          id: `turn-${result.length}`,
          userMessage: msg,
          assistantGroups: [],
          isCompleted: false,
          stats: { toolCalls: 0, messages: 0, subagents: 0, thinkingCount: 0 },
        };
        continue;
      }

      // If no current turn, create one (for system messages at start)
      if (!currentTurn) {
        currentTurn = {
          id: `turn-${result.length}`,
          assistantGroups: [],
          isCompleted: false,
          stats: { toolCalls: 0, messages: 0, subagents: 0, thinkingCount: 0 },
        };
      }

      // Handle turn completion (turnStats indicates end of turn)
      if (msg.turnStats) {
        currentTurn.isCompleted = true;
        currentTurn.assistantGroups.push({ message: msg, children: [] });
        result.push(currentTurn);
        currentTurn = null;
        continue;
      }

      // Count stats
      if (msg.role === 'assistant') {
        currentTurn.stats.messages++;
      }
      if (toolUse) {
        if (toolUse.tool_name === 'Task') {
          currentTurn.stats.subagents++;
        } else if (!toolUse.parent_tool_use_id) {
          currentTurn.stats.toolCalls++;
        }
      }
      if (msg.isThinking) {
        currentTurn.stats.thinkingCount++;
      }

      // Handle tool result - try to pair with existing tool call
      if (toolResult && !toolResult.parent_tool_use_id) {
        const toolCallInfo = toolCallMap.get(toolResult.tool_id);
        if (toolCallInfo && toolCallInfo.turnIndex === result.length) {
          // Pair with existing tool call
          currentTurn.assistantGroups[toolCallInfo.groupIndex].toolResultMessage = msg;
          continue;
        }
      }

      // Group by Task/SubAgent relationships
      if (toolUse?.tool_name === 'Task') {
        const groupIndex = currentTurn.assistantGroups.length;
        currentTurn.assistantGroups.push({ message: msg, children: [] });
        if (toolUse.tool_id) {
          taskToolIds.set(toolUse.tool_id, { turnIndex: result.length, groupIndex });
          toolCallMap.set(toolUse.tool_id, { turnIndex: result.length, groupIndex });
        }
      } else if (toolUse?.parent_tool_use_id) {
        const parent = taskToolIds.get(toolUse.parent_tool_use_id);
        if (parent && parent.turnIndex === result.length) {
          currentTurn.assistantGroups[parent.groupIndex]?.children.push(msg);
        } else {
          currentTurn.assistantGroups.push({ message: msg, children: [] });
        }
      } else if (toolResult?.parent_tool_use_id) {
        const parent = taskToolIds.get(toolResult.parent_tool_use_id);
        if (parent && parent.turnIndex === result.length) {
          currentTurn.assistantGroups[parent.groupIndex]?.children.push(msg);
        } else {
          currentTurn.assistantGroups.push({ message: msg, children: [] });
        }
      } else if (toolUse) {
        // Regular tool call - add to groups and track for pairing
        const groupIndex = currentTurn.assistantGroups.length;
        currentTurn.assistantGroups.push({ message: msg, children: [] });
        if (toolUse.tool_id) {
          toolCallMap.set(toolUse.tool_id, { turnIndex: result.length, groupIndex });
        }
      } else if (toolResult?.tool_id) {
        // Orphan tool result (shouldn't happen often)
        const parent = taskToolIds.get(toolResult.tool_id);
        if (parent && parent.turnIndex === result.length) {
          currentTurn.assistantGroups[parent.groupIndex]?.children.push(msg);
        } else {
          currentTurn.assistantGroups.push({ message: msg, children: [] });
        }
      } else {
        currentTurn.assistantGroups.push({ message: msg, children: [] });
      }
    }

    // Push the last turn if exists (has user message or assistant content)
    if (currentTurn && (currentTurn.userMessage || currentTurn.assistantGroups.length > 0)) {
      result.push(currentTurn);
    }

    return result;
  }, [messages]);

  // Toggle turn collapse
  const toggleTurn = (turnId: string) => {
    setCollapsedTurns(prev => {
      const next = new Set(prev);
      if (next.has(turnId)) {
        next.delete(turnId);
      } else {
        next.add(turnId);
      }
      return next;
    });
  };

  // Auto-collapse turns when they complete
  const prevCompletedTurnsRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    const completedTurnIds = new Set(
      turns.filter(t => t.isCompleted).map(t => t.id)
    );

    // Find newly completed turns
    const newlyCompleted: string[] = [];
    completedTurnIds.forEach(id => {
      if (!prevCompletedTurnsRef.current.has(id)) {
        newlyCompleted.push(id);
      }
    });

    // Auto-collapse newly completed turns
    if (newlyCompleted.length > 0) {
      setCollapsedTurns(prev => {
        const next = new Set(prev);
        newlyCompleted.forEach(id => next.add(id));
        return next;
      });
    }

    prevCompletedTurnsRef.current = completedTurnIds;
  }, [turns]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isProcessing]);

  // Timer for processing state
  useEffect(() => {
    if (isProcessing) {
      startTimeRef.current = Date.now();
      setElapsedTime(0);

      const interval = setInterval(() => {
        if (startTimeRef.current) {
          setElapsedTime((Date.now() - startTimeRef.current) / 1000);
        }
      }, 100);

      return () => clearInterval(interval);
    } else {
      startTimeRef.current = null;
      setElapsedTime(0);
    }
  }, [isProcessing]);

  return (
    <ScrollArea className="flex-1 px-4">
      <div className="py-4 space-y-4">
        {turns.length === 0 && !isProcessing ? (
          <div className="flex flex-col items-center justify-center h-[50vh] text-muted-foreground opacity-50">
            <div className="text-4xl mb-2">👋</div>
            <p className="text-sm font-medium">Start a conversation with Claude</p>
          </div>
        ) : (
          turns.map((turn) => {
            const isCollapsed = collapsedTurns.has(turn.id);
            const hasContent = turn.assistantGroups.length > 0;
            // Show collapsible for completed turns
            const showCollapsible = hasContent && turn.isCompleted;

            // Find the last assistant text message (not tool use, not turnStats)
            const lastAssistantMessage = turn.assistantGroups
              .slice()
              .reverse()
              .find(g =>
                g.message.role === 'assistant' &&
                g.message.content &&
                !g.message.toolUse &&
                !g.message.toolResult &&
                !g.message.turnStats
              );

            // Find turnStats message if exists
            const turnStatsGroup = turn.assistantGroups.find(g => g.message.turnStats);

            // Find status messages (like "INTERRUPTED BY USER") that should always be visible
            const statusMessages = turn.assistantGroups.filter(g => g.message.isStatusMessage);

            return (
              <div key={turn.id} className="space-y-2">
                {/* User message */}
                {turn.userMessage && (
                  <MessageBubble message={turn.userMessage} cwd={cwd} />
                )}

                {/* Assistant response with collapsible */}
                {hasContent && (
                  showCollapsible ? (
                    <div className="space-y-2">
                      <Collapsible open={!isCollapsed} onOpenChange={() => toggleTurn(turn.id)}>
                        {/* Collapse header */}
                        <CollapsibleTrigger className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer w-full py-1.5">
                          <ChevronDown className={cn(
                            "h-4 w-4 transition-transform duration-200 text-muted-foreground/70",
                            isCollapsed && "-rotate-90"
                          )} />
                          <TurnSummary stats={turn.stats} />
                        </CollapsibleTrigger>

                        <CollapsibleContent className="space-y-2">
                          {turn.assistantGroups.map((group) => (
                            <RenderMessageGroup key={group.message.id} group={group} cwd={cwd} />
                          ))}
                        </CollapsibleContent>
                      </Collapsible>

                      {/* Show last assistant message when collapsed */}
                      {isCollapsed && lastAssistantMessage && (
                        <MessageBubble message={lastAssistantMessage.message} cwd={cwd} />
                      )}

                      {/* Show status messages (like INTERRUPTED) when collapsed */}
                      {isCollapsed && statusMessages.map((group) => (
                        <MessageBubble key={group.message.id} message={group.message} cwd={cwd} />
                      ))}

                      {/* Show turn stats when collapsed */}
                      {isCollapsed && turnStatsGroup && (
                        <MessageBubble message={turnStatsGroup.message} cwd={cwd} />
                      )}
                    </div>
                  ) : (
                    // In-progress turn - show directly without collapse
                    <div className="space-y-2">
                      {turn.assistantGroups.map((group) => (
                        <RenderMessageGroup key={group.message.id} group={group} cwd={cwd} />
                      ))}
                    </div>
                  )
                )}
              </div>
            );
          })
        )}

        {/* Processing indicator */}
        {isProcessing && (
          <div className="flex items-center gap-3 animate-in fade-in duration-300">
            <ProcessingDots />
            <span className="text-sm font-mono text-muted-foreground">
              {elapsedTime.toFixed(1)}s
            </span>
          </div>
        )}

        <div ref={bottomRef} />
      </div>
    </ScrollArea>
  );
}

// Render a message group (Task with children or standalone message)
function RenderMessageGroup({ group, cwd }: { group: MessageGroup; cwd?: string }) {
  if (group.message.toolUse?.tool_name === 'Task' && group.children.length > 0) {
    return (
      <AgentToolDisplay
        agentMessage={group.message}
        childMessages={group.children}
        cwd={cwd}
      />
    );
  }
  return <MessageBubble message={group.message} toolResultMessage={group.toolResultMessage} cwd={cwd} />;
}

// Turn summary component showing stats
function TurnSummary({ stats }: { stats: Turn['stats'] }) {
  const parts: string[] = [];

  if (stats.toolCalls > 0) {
    parts.push(`${stats.toolCalls} tool call${stats.toolCalls > 1 ? 's' : ''}`);
  }
  if (stats.messages > 0) {
    parts.push(`${stats.messages} message${stats.messages > 1 ? 's' : ''}`);
  }
  if (stats.subagents > 0) {
    parts.push(`${stats.subagents} subagent${stats.subagents > 1 ? 's' : ''}`);
  }

  const summary = parts.length > 0 ? parts.join(', ') : 'Response';

  return (
    <div className="flex items-center gap-2">
      <span className="text-foreground/80">{summary}</span>
      <div className="flex items-center gap-1 text-muted-foreground/50">
        {stats.thinkingCount > 0 && <Brain className="h-3.5 w-3.5" />}
        {stats.toolCalls > 0 && <Terminal className="h-3.5 w-3.5" />}
        {stats.messages > 0 && <MessageSquare className="h-3.5 w-3.5" />}
        {stats.subagents > 0 && <Grip className="h-3.5 w-3.5" />}
      </div>
    </div>
  );
}

// Animated processing dots component
function ProcessingDots() {
  return (
    <div className="flex items-center gap-1">
      <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/60 animate-bounce [animation-delay:-0.3s]" />
      <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/60 animate-bounce [animation-delay:-0.15s]" />
      <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/60 animate-bounce" />
    </div>
  );
}
