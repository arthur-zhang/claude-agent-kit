可以

1 tool call
协议设计 - 第九部分：配置管理消息

19. 配置更新 (config_update)

客户端 → 服务器

更新会话配置：

{
  "type": "config_update",
  "id": "msg_config123",
  "session_id": "sess_xyz789",
  "config": {
    "model": "claude-opus-4",
    "max_turns": 200,
    "permission_mode": "plan"
  }
}
服务器响应：

{
  "type": "config_updated",
  "id": "msg_config_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_config123",
  "config": {
    "model": "claude-opus-4",
    "max_turns": 200,
    "permission_mode": "plan"
  }
}
20. 配置查询 (config_query)

客户端 → 服务器

{
  "type": "config_query",
  "id": "msg_query123",
  "session_id": "sess_xyz789"
}
服务器响应：

{
  "type": "config_response",
  "id": "msg_query_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_query123",
  "config": {
    "model": "claude-sonnet-4-5",
    "max_turns": 100,
    "permission_mode": "default",
    "working_directory": "/Users/arthur/project"
  }
}
可配置项：

model: 使用的模型
max_turns: 最大轮次
permission_mode: 权限模式（default, acceptEdits, plan, bypassPermissions）
working_directory: 工作目录