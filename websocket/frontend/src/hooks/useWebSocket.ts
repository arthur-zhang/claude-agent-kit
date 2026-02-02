import { useEffect, useRef, useState, useCallback } from 'react';
import type {
  SDKMessage,
  SDKAssistantMessage,
  SDKResultMessage,
  SidecarMessage,
  SidecarError,
  WorkspaceInitRequest,
  WorkspaceInitResponse,
  PermissionRequest,
  PermissionResponse,
  AskUserQuestionRequest,
  AskUserQuestionResponse,
  QuestionAnswer,
  ChatMessage,
  TokenUsage,
  TurnStats,
  SessionInfo,
  PermissionMode,
} from '../types';

interface UseWebSocketOptions {
  url: string;
  cwd?: string;
  model?: string;
  disallowedTools?: string;
  enableThinking?: boolean;
  /** Permission mode for the session */
  permissionMode?: PermissionMode;
  /** Allow bypassing permission checks (required for bypassPermissions mode) */
  dangerouslySkipPermissions?: boolean;
  /** Resume a previous session by its session ID */
  resumeSessionId?: string;
  onMessage?: (message: SDKMessage) => void;
  onError?: (error: Event) => void;
  onClose?: () => void;
}

export function useWebSocket({
  url,
  cwd = '/tmp',
  model = 'opus',
  disallowedTools = '',
  enableThinking = true,
  permissionMode,
  dangerouslySkipPermissions,
  resumeSessionId,
  onMessage,
  onError,
  onClose,
}: UseWebSocketOptions) {
  const [isConnected, setIsConnected] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [pendingPermission, setPendingPermission] = useState<PermissionRequest | null>(null);
  const [pendingUserQuestion, setPendingUserQuestion] = useState<AskUserQuestionRequest | null>(null);
  const [sessionInfo, setSessionInfo] = useState<SessionInfo | null>(null);
  const [tokenUsage, setTokenUsage] = useState<TokenUsage | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const currentMessageRef = useRef<ChatMessage | null>(null);
  const turnStartTimeRef = useRef<Date | null>(null);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    const ws = new WebSocket(url);

    ws.onopen = () => {
      console.log('WebSocket connected');
      setIsConnected(true);

      // Clear previous state when connecting
      setMessages([]);
      setTokenUsage(null);
      setPendingPermission(null);
      setPendingUserQuestion(null);
      setSessionInfo(null);
      setIsProcessing(false);
      currentMessageRef.current = null;
      turnStartTimeRef.current = null;

      // Send WorkspaceInitRequest
      const disallowedToolsArray = disallowedTools
        .split(',')
        .map(tool => tool.trim())
        .filter(tool => tool.length > 0);

      const initRequest: WorkspaceInitRequest = {
        type: 'workspace_init',
        id: '',  // session_id will be assigned by server
        agentType: 'claude',
        options: {
          cwd,
          model,
          permissionMode,
          disallowedTools: disallowedToolsArray.length > 0 ? disallowedToolsArray : undefined,
          maxThinkingTokens: enableThinking ? 10000 : undefined,
          dangerouslySkipPermissions,
          resume: resumeSessionId,
        },
      };

      ws.send(JSON.stringify(initRequest));
      console.log('Sent WorkspaceInitRequest:', initRequest);
    };

    ws.onmessage = (event) => {
      try {
        const rawMessage = JSON.parse(event.data);
        console.log('Received message:', rawMessage);

        // Handle different message types
        if (rawMessage.type === 'workspace_init_output') {
          handleWorkspaceInitResponse(rawMessage as WorkspaceInitResponse);
        } else if (rawMessage.type === 'message' && rawMessage.agentType === 'claude') {
          handleSidecarMessage(rawMessage as SidecarMessage);
        } else if (rawMessage.type === 'error' && rawMessage.agentType === 'claude') {
          handleSidecarError(rawMessage as SidecarError);
        } else if (rawMessage.type === 'permission_request') {
          handlePermissionRequest(rawMessage as PermissionRequest);
        } else if (rawMessage.type === 'ask_user_question') {
          handleAskUserQuestion(rawMessage as AskUserQuestionRequest);
        } else {
          console.log('Unknown message type:', rawMessage);
        }
      } catch (error) {
        console.error('Failed to parse message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      onError?.(error);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
      onClose?.();
    };

    wsRef.current = ws;
  }, [url, cwd, model, disallowedTools, enableThinking, permissionMode, dangerouslySkipPermissions, resumeSessionId, onError, onClose]);

  // Handle WorkspaceInitResponse
  function handleWorkspaceInitResponse(response: WorkspaceInitResponse) {
    if (response.error) {
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'system',
          content: `❌ Initialization failed: ${response.error}`,
          timestamp: new Date(),
          isStatusMessage: true,
        },
      ]);
      setIsConnected(false);
      return;
    }

    // Save session info - id is now the session_id
    setSessionInfo({
      sessionId: response.id,
      cwd: response.cwd || cwd,
      model: response.model,
      tools: response.tools,
      agents: response.agents,
      skills: response.skills,
      slashCommands: response.slashCommands,
      mcpServers: response.mcpServers,
      plugins: response.plugins,
      claudeCodeVersion: response.claudeCodeVersion,
    });

    // Update browser URL with session_id
    if (response.id) {
      const newUrl = new URL(window.location.href);
      newUrl.searchParams.set('session_id', response.id);
      window.history.replaceState({}, '', newUrl.toString());
    }

    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: '✅ Session initialized',
        timestamp: new Date(),
        isStatusMessage: true,
      },
    ]);
  }

  // Handle SidecarMessage (SDK messages)
  function handleSidecarMessage(sidecarMsg: SidecarMessage) {
    const sdkMsg = sidecarMsg.data;
    onMessage?.(sdkMsg);

    processSdkMessage(sdkMsg);
  }

  // Process SDK message and update UI state
  function processSdkMessage(sdkMsg: SDKMessage) {
    if (sdkMsg.type === 'assistant') {
      processAssistantMessage(sdkMsg as SDKAssistantMessage);
    } else if (sdkMsg.type === 'result') {
      processResultMessage(sdkMsg as SDKResultMessage);
    } else if (sdkMsg.type === 'system') {
      // System messages (hooks, etc.) - log but don't show in UI
      console.log('System message:', sdkMsg);
    } else if (sdkMsg.type === 'user') {
      // User message echo - ignore
      console.log('User message echo:', sdkMsg);
    } else if (sdkMsg.type === 'tool_progress') {
      // Tool progress - could show in UI
      console.log('Tool progress:', sdkMsg);
    } else {
      console.log('Other SDK message:', sdkMsg);
    }
  }

  // Process assistant message
  function processAssistantMessage(msg: SDKAssistantMessage) {
    const parentToolUseId = msg.parent_tool_use_id;

    for (const block of msg.message.content || []) {
      if (block.type === 'text' && 'text' in block) {
        // Text content
        if (currentMessageRef.current) {
          currentMessageRef.current.content = block.text;
          currentMessageRef.current.isStreaming = false;
          setMessages((prev) => [...prev]);
          currentMessageRef.current = null;
        } else {
          setMessages((prev) => [
            ...prev,
            {
              id: crypto.randomUUID(),
              role: 'assistant',
              content: block.text,
              timestamp: new Date(),
              isStreaming: false,
            },
          ]);
        }
      } else if (block.type === 'thinking' && 'thinking' in block) {
        // Thinking/reasoning
        setMessages((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            role: 'system',
            content: block.thinking,
            timestamp: new Date(),
            isThinking: true,
          },
        ]);
      } else if (block.type === 'tool_use') {
        // Tool use started
        setMessages((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            role: 'system',
            content: `🔧 Using tool: ${block.name}`,
            timestamp: new Date(),
            toolUse: {
              tool_name: block.name,
              tool_id: block.id,
              tool_input: block.input,
              parent_tool_use_id: parentToolUseId,
            },
          },
        ]);
      } else if (block.type === 'tool_result') {
        // Tool result
        const blockContent = block.content;
        const content = typeof blockContent === 'string'
          ? blockContent
          : Array.isArray(blockContent)
            ? blockContent.map(c => typeof c === 'string' ? c : JSON.stringify(c)).join('\n')
            : JSON.stringify(blockContent);
        const isError = block.is_error || false;
        const toolStatus = isError ? '❌' : '✅';

        setMessages((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            role: 'system',
            content: `${toolStatus} ${content}`,
            timestamp: new Date(),
            toolResult: {
              tool_id: block.tool_use_id,
              content,
              is_error: isError,
              parent_tool_use_id: parentToolUseId,
            },
          },
        ]);
      }
    }
  }

  // Process result message (turn completed)
  function processResultMessage(msg: SDKResultMessage) {
    setIsProcessing(false);

    // Check if error
    if (msg.subtype !== 'success') {
      const errorMsg = msg as { errors?: string[] };
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'system',
          content: `❌ Error: ${errorMsg.errors?.join('; ') || msg.subtype}`,
          timestamp: new Date(),
        },
      ]);
    }

    // Update token usage
    const usage = msg.usage || { input_tokens: 0, output_tokens: 0, cache_read_input_tokens: 0 };
    const newTokenUsage: TokenUsage = {
      input_tokens: usage.input_tokens || 0,
      output_tokens: usage.output_tokens || 0,
      cached_tokens: usage.cache_read_input_tokens || 0,
      total_tokens: (usage.input_tokens || 0) + (usage.output_tokens || 0),
    };
    setTokenUsage(newTokenUsage);

    // Calculate duration
    const endTime = new Date();
    const startTime = turnStartTimeRef.current || endTime;
    const durationSeconds = msg.duration_ms
      ? Math.round(msg.duration_ms / 1000)
      : Math.round((endTime.getTime() - startTime.getTime()) / 1000);

    const turnStats: TurnStats = {
      model: sessionInfo?.model,
      duration_ms: msg.duration_ms,
      duration_api_ms: msg.duration_api_ms,
      input_tokens: newTokenUsage.input_tokens,
      output_tokens: newTokenUsage.output_tokens,
      cached_tokens: newTokenUsage.cached_tokens,
      total_tokens: newTokenUsage.total_tokens,
      total_cost_usd: msg.total_cost_usd,
      num_turns: msg.num_turns,
      start_time: startTime,
      end_time: endTime,
    };

    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: `${durationSeconds}s`,
        timestamp: endTime,
        turnStats,
      },
    ]);

    turnStartTimeRef.current = null;

    // Mark current message as complete if streaming
    if (currentMessageRef.current) {
      currentMessageRef.current.isStreaming = false;
      setMessages((prev) => [...prev]);
      currentMessageRef.current = null;
    }
  }

  // Handle SidecarError
  function handleSidecarError(error: SidecarError) {
    console.error('Sidecar error:', error.error);
    setIsProcessing(false);
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: `❌ Error: ${error.error}`,
        timestamp: new Date(),
      },
    ]);
  }

  // Handle permission request
  function handlePermissionRequest(request: PermissionRequest) {
    setPendingPermission(request);
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: `🔐 Permission requested for: ${request.toolName} (${request.context?.risk_level || 'medium'} risk)`,
        timestamp: new Date(),
      },
    ]);
  }

  // Handle ask user question
  function handleAskUserQuestion(request: AskUserQuestionRequest) {
    setPendingUserQuestion(request);
    const questionCount = request.questions.length;
    const questionPreview = request.questions[0]?.question.slice(0, 50) || '';
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: `❓ Claude has ${questionCount} question${questionCount > 1 ? 's' : ''} for you: ${questionPreview}${questionPreview.length >= 50 ? '...' : ''}`,
        timestamp: new Date(),
      },
    ]);
  }

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  const sendMessage = useCallback(
    (content: string) => {
      if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
        console.error('WebSocket is not connected');
        return;
      }

      if (!sessionInfo?.sessionId) {
        console.error('Session not initialized');
        return;
      }

      // Send query message (conductor-bundle protocol format)
      const message = {
        type: 'query',
        id: sessionInfo.sessionId,
        agentType: 'claude',
        prompt: content,
        options: {
          cwd: sessionInfo.cwd || '/tmp',
        },
      };

      wsRef.current.send(JSON.stringify(message));

      // Set processing state
      setIsProcessing(true);
      turnStartTimeRef.current = new Date();

      // Add user message to chat
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'user',
          content,
          timestamp: new Date(),
        },
      ]);
    },
    [sessionInfo]
  );

  const respondToPermission = useCallback(
    (decision: 'allow' | 'deny' | 'allow_always') => {
      if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
        console.error('WebSocket is not connected');
        return;
      }

      const response: PermissionResponse = {
        type: 'permission_response',
        id: sessionInfo?.sessionId || '',
        agentType: 'claude',
        decision,
      };

      wsRef.current.send(JSON.stringify(response));
      setPendingPermission(null);
    },
    [sessionInfo]
  );

  const interrupt = useCallback(() => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      console.error('WebSocket is not connected');
      return;
    }

    // Send cancel message (conductor-bundle protocol format)
    const message = {
      type: 'cancel',
      id: sessionInfo?.sessionId || '',
      agentType: 'claude',
    };

    wsRef.current.send(JSON.stringify(message));

    // Show interrupted message and clear processing state
    setIsProcessing(false);
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: 'INTERRUPTED BY USER',
        timestamp: new Date(),
        isStatusMessage: true,
      },
    ]);

    // Mark current streaming message as complete if any
    if (currentMessageRef.current) {
      currentMessageRef.current.isStreaming = false;
      currentMessageRef.current = null;
    }
  }, [sessionInfo]);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const respondToUserQuestion = useCallback(
    (requestId: string, answers: QuestionAnswer[]) => {
      if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
        console.error('WebSocket is not connected');
        return;
      }

      const response: AskUserQuestionResponse = {
        type: 'user_question_response',
        session_id: sessionInfo?.sessionId || '',
        request_id: requestId,
        answers,
      };

      wsRef.current.send(JSON.stringify(response));
      setPendingUserQuestion(null);

      // Add acknowledgment message
      setMessages((prev) => [
        ...prev,
        {
          id: crypto.randomUUID(),
          role: 'system',
          content: `✅ Response submitted with ${answers.length} answer${answers.length > 1 ? 's' : ''}`,
          timestamp: new Date(),
        },
      ]);
    },
    [sessionInfo]
  );

  const cancelUserQuestion = useCallback(() => {
    setPendingUserQuestion(null);
    setMessages((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        role: 'system',
        content: '⚠️ User question cancelled',
        timestamp: new Date(),
      },
    ]);
  }, []);

  useEffect(() => {
    return () => {
      disconnect();
    };
  }, [disconnect]);

  return {
    isConnected,
    messages,
    pendingPermission,
    pendingUserQuestion,
    sessionInfo,
    tokenUsage,
    sessionId: sessionInfo?.sessionId || null,
    isProcessing,
    connect,
    disconnect,
    sendMessage,
    respondToPermission,
    respondToUserQuestion,
    cancelUserQuestion,
    interrupt,
    clearMessages,
  };
}
