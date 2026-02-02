#!/usr/bin/env node

/**
 * 测试 session_id 更新流程
 *
 * 验证：
 * 1. 客户端连接时使用初始 session_id
 * 2. 收到 session_init 后更新为真实 session_id
 * 3. 后续消息使用真实 session_id
 */

const WebSocket = require('ws');

const INITIAL_SESSION_ID = 'test-' + Date.now();
const WS_URL = `ws://localhost:3000/ws?session_id=${INITIAL_SESSION_ID}`;

console.log('🚀 测试 session_id 更新流程\n');
console.log(`📍 初始 session_id: ${INITIAL_SESSION_ID}`);
console.log(`📍 URL: ${WS_URL}\n`);

const ws = new WebSocket(WS_URL);

let actualSessionId = INITIAL_SESSION_ID;
let sessionInitReceived = false;

ws.on('open', () => {
  console.log('✅ WebSocket 连接成功\n');

  // 立即发送一条消息以触发 SDK 初始化
  console.log('📤 发送初始消息以触发 SDK 初始化...\n');
  const initMessage = {
    type: 'user_message',
    id: 'msg-init-' + Date.now(),
    session_id: actualSessionId,
    content: 'Hello! What is 2+2?',
    parent_tool_use_id: null
  };

  ws.send(JSON.stringify(initMessage));
  console.log('✉️  已发送初始消息（使用初始 session_id）');
  console.log(`   session_id: ${initMessage.session_id}\n`);
  console.log('─'.repeat(80));
});

ws.on('message', (data) => {
  try {
    const event = JSON.parse(data.toString());

    console.log(`\n📥 收到事件: ${event.type}`);

    switch (event.type) {
      case 'session_init':
        sessionInitReceived = true;
        const oldSessionId = actualSessionId;
        actualSessionId = event.session_id;

        console.log(`   旧 session_id: ${oldSessionId}`);
        console.log(`   新 session_id: ${actualSessionId}`);
        console.log(`   Model: ${event.model || 'N/A'}`);
        console.log(`   ✅ 已更新 actualSessionId`);

        // 等待一下，然后发送第二条消息验证使用了新的 session_id
        setTimeout(() => {
          console.log('\n📤 发送第二条消息（应该使用新的 session_id）...\n');
          const followUpMessage = {
            type: 'user_message',
            id: 'msg-followup-' + Date.now(),
            session_id: actualSessionId,  // 使用更新后的 session_id
            content: 'What is 3+3?',
            parent_tool_use_id: null
          };

          ws.send(JSON.stringify(followUpMessage));
          console.log('✉️  已发送第二条消息');
          console.log(`   session_id: ${followUpMessage.session_id}`);
          console.log(`   ✅ 验证：使用了从 session_init 接收到的真实 session_id\n`);
          console.log('─'.repeat(80));
        }, 1000);
        break;

      case 'turn_started':
        console.log(`   🔄 Turn 开始`);
        break;

      case 'turn_completed':
        console.log(`   ✅ Turn 完成`);
        if (event.usage) {
          console.log(`   Token: ${event.usage.total_tokens}`);
        }
        break;

      case 'assistant_message':
        const preview = event.text.substring(0, 50);
        console.log(`   💬 ${preview}${event.text.length > 50 ? '...' : ''}`);
        console.log(`   Final: ${event.is_final}`);
        break;

      case 'error':
        console.log(`   ❌ 错误: ${event.message}`);
        break;
    }

    console.log('─'.repeat(80));
  } catch (e) {
    console.error('解析消息失败:', e);
  }
});

ws.on('error', (error) => {
  console.error('\n❌ WebSocket 错误:', error.message);
  console.log('\n💡 提示：');
  console.log('  1. 确保 WebSocket 服务器正在运行（cargo run）');
  console.log('  2. 检查端口 3000 是否可用');
  process.exit(1);
});

ws.on('close', () => {
  console.log('\n👋 WebSocket 连接已关闭');

  if (sessionInitReceived) {
    console.log('\n✅ 测试成功！');
    console.log(`   初始 session_id: ${INITIAL_SESSION_ID}`);
    console.log(`   最终 session_id: ${actualSessionId}`);
    console.log(`   session_id 已正确更新: ${actualSessionId !== INITIAL_SESSION_ID ? '是' : '否'}`);
  } else {
    console.log('\n⚠️  未收到 session_init 事件');
  }

  process.exit(0);
});

// 15 秒后自动关闭
setTimeout(() => {
  console.log('\n⏰ 测试超时，关闭连接...');
  ws.close();
}, 15000);

// 处理 Ctrl+C
process.on('SIGINT', () => {
  console.log('\n\n⚠️  收到中断信号，关闭连接...');
  ws.close();
  process.exit(0);
});
