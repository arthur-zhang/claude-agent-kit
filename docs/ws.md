## 新建 session

```json
{
  "type": "user_session_init",
  "cwd": "/tmp",
  "model": "opus",
  "permission_mode": null,
  "max_turns": null,
  "max_budget_usd": null,
  "user": null,
  "disallowed_tools": null,
  "max_thinking_tokens": 10000,
  "resume": null
}
```
响应
```json
{
  "type": "user_session_init_response",
  "success": true,
  "resumed": false
}
```

## resume 请求
```json
{
  "type": "user_session_init",
  "cwd": "/tmp",
  "model": "opus",
  "permission_mode": null,
  "max_turns": null,
  "max_budget_usd": null,
  "user": null,
  "disallowed_tools": null,
  "max_thinking_tokens": 10000,
  "resume": "9b8af48c-dbda-47fa-8076-81f90ed4a135"
}
```
响应
```json
{
  "type": "user_session_init_response",
  "success": false,
  "error": "No conversation found with session ID: 9b8af48c-dbda-47fa-8076-81f90ed4a135",
  "resumed": true
}

```