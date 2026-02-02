// WebSocket Protocol Types
// Based on conductor-bundle/shared/sidecarProtocol.ts

// Re-export SDK message types from @anthropic-ai/claude-agent-sdk
export type {
  SDKMessage,
  SDKAssistantMessage,
  SDKUserMessage,
  SDKUserMessageReplay,
  SDKResultMessage,
  SDKSystemMessage,
  SDKPartialAssistantMessage,
  SDKCompactBoundaryMessage,
  SDKStatusMessage,
  SDKHookResponseMessage,
  SDKToolProgressMessage,
  SDKAuthStatusMessage,
  PermissionMode,
  SlashCommand,
  McpServerStatus,
  AccountInfo,
} from '@anthropic-ai/claude-agent-sdk';

import type { SDKMessage, PermissionMode, SlashCommand, McpServerStatus } from '@anthropic-ai/claude-agent-sdk';

// ============================================================================
// Frontend -> Sidecar requests (conductor-bundle format)
// ============================================================================

export interface QueryRequest {
  type: 'query';
  id: string;  // session_id
  agentType: 'claude';
  prompt: string;
  options: {
    cwd: string;
    model?: string;
    permissionMode?: PermissionMode;
    maxTurns?: number;
    resume?: string;
    resumeSessionAt?: string;
  };
}

export interface CancelRequest {
  type: 'cancel';
  id: string;  // session_id
  agentType: 'claude';
}

export interface WorkspaceInitRequest {
  type: 'workspace_init';
  id: string;  // session_id (empty for new session)
  agentType: 'claude';
  options: {
    cwd: string;
    model?: string;
    permissionMode?: PermissionMode;
    disallowedTools?: string[];
    maxThinkingTokens?: number;
  };
}

export interface UpdatePermissionModeRequest {
  type: 'update_permission_mode';
  id: string;
  agentType: 'claude';
  permissionMode: PermissionMode;
}

export type ClientRequest =
  | QueryRequest
  | CancelRequest
  | WorkspaceInitRequest
  | UpdatePermissionModeRequest;

// ============================================================================
// Sidecar -> Frontend notifications/responses
// ============================================================================

export interface SidecarMessage {
  id: string;
  type: 'message';
  agentType: 'claude';
  data: SDKMessage;
}

export interface SidecarError {
  id: string;
  type: 'error';
  agentType: 'claude';
  error: string;
  data?: unknown;
}

export interface WorkspaceInitResponse {
  id: string;  // session_id
  type: 'workspace_init_output';
  agentType: 'claude';
  slashCommands?: SlashCommand[];
  mcpServers?: McpServerStatus[];
  tools?: string[];
  agents?: string[];
  skills?: string[];
  plugins?: PluginInfo[];
  model?: string;
  cwd?: string;
  claudeCodeVersion?: string;
  error?: string;
}

export interface PluginInfo {
  name: string;
  path: string;
}

export type SidecarResponse =
  | SidecarMessage
  | SidecarError
  | WorkspaceInitResponse;

// ============================================================================
// Sidecar -> Frontend RPC (permission requests, user questions)
// ============================================================================

export interface PermissionContext {
  description: string;
  risk_level: 'low' | 'medium' | 'high';
}

export interface PermissionRequest {
  type: 'permission_request';
  id: string;  // session_id
  agentType: 'claude';
  toolName: string;
  toolUseId?: string;
  input: unknown;
  context: PermissionContext;
}

export interface PermissionResponse {
  type: 'permission_response';
  id: string;  // session_id
  agentType: 'claude';
  decision: 'allow' | 'deny' | 'allow_always';
}

export interface AskUserQuestionRequest {
  type: 'ask_user_question';
  session_id: string;
  request_id: string;
  questions: UserQuestion[];
}

export interface UserQuestion {
  header: string;
  question: string;
  options: QuestionOption[];
  multiSelect: boolean;
}

export interface QuestionOption {
  label: string;
  description: string;
}

export interface AskUserQuestionResponse {
  type: 'user_question_response';
  session_id: string;
  request_id: string;
  answers: QuestionAnswer[];
}

export interface QuestionAnswer {
  question_index: number;
  selected: string[];
}

export interface ExitPlanModeRequest {
  type: 'exit_plan_mode';
  session_id: string;
  request_id: string;
  planFilePath?: string;
}

export interface ExitPlanModeResponse {
  type: 'plan_approval_response';
  session_id: string;
  request_id: string;
  approved: boolean;
  feedback?: string;
}

// ============================================================================
// UI Types (internal to frontend)
// ============================================================================

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  cached_tokens: number;
  total_tokens: number;
}

export interface TurnStats {
  model?: string;
  duration_ms?: number;
  duration_api_ms?: number;
  input_tokens: number;
  output_tokens: number;
  cached_tokens: number;
  total_tokens: number;
  total_cost_usd?: number;
  num_turns?: number;
  start_time: Date;
  end_time: Date;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  isStreaming?: boolean;
  isThinking?: boolean;
  /** Important status messages that should not be collapsed */
  isStatusMessage?: boolean;
  toolUse?: {
    tool_name: string;
    tool_id: string;
    tool_input: unknown;
    /** If present, indicates this is a SubAgent tool call */
    parent_tool_use_id?: string | null;
  };
  toolResult?: {
    tool_id: string;
    content: string;
    is_error: boolean;
    /** If present, indicates this result is from a SubAgent */
    parent_tool_use_id?: string | null;
  };
  turnStats?: TurnStats;
}

// ============================================================================
// Session state
// ============================================================================

export interface SessionInfo {
  sessionId: string;
  cwd: string;
  model?: string;
  tools?: string[];
  agents?: string[];
  skills?: string[];
  slashCommands?: SlashCommand[];
  mcpServers?: McpServerStatus[];
  plugins?: PluginInfo[];
  claudeCodeVersion?: string;
}
