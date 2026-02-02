#!/usr/bin/env node

/**
 * 验证所有事件的 session_id 是否正确
 */

const WebSocket = require('ws');

const INITIAL_SESSION_ID = 'test-' + Date.now();
const WS_URL = `ws://localhost:3000/ws?session_id=${INITIAL_SESSION_ID}`;

console.log('🔍 验证所有事件的 session_id\n');
console.log(`📍 初始 session_id: ${INITIAL_SESSION_ID}\n`);

const ws = new WebSocket(WS_URL);

let actualSessionId = null;
const receivedEvents = [];

ws.on('open', () => {
  console.log('✅ WebSocket 连接成功\n');

  // 发送消息触发 SDK 初始化
  const message = {
    type: 'user_message',
    id: 'msg-' + Date.now(),
    session_id: INITIAL_SESSION_ID,
    content: 'Hello! What is 2+2?',
    parent_tool_use_id: null
  };

  ws.send(JSON.stringify(message));
  console.log('📤 已发送初始消息\n');
});

ws.on('message', (data) => {
  try {
    const event = JSON.parse(data.toString());
    receivedEvents.push(event);

    console.log(`📥 [${event.type}]`);
    console.log(`   session_id: ${event.session_id}`);

    // 保存第一次收到的真实 session_id
    if (event.type === 'session_init' && !actualSessionId) {
      actualSessionId = event.session_id;
      console.log(`   ✅ 这是真实的 session_id`);
    } else if (actualSessionId) {
      // 验证后续事件的 session_id 是否正确
      if (event.session_id === actualSessionId) {
        console.log(`   ✅ session_id 正确`);
      } else {
        console.log(`   ❌ session_id 错误！应该是 ${actualSessionId}`);
      }
    }

    console.log('');
  } catch (e) {
    console.error('解析消息失败:', e);
  }
});

ws.on('error', (error) => {
  console.error('\n❌ WebSocket 错误:', error.message);
  process.exit(1);
});

ws.on('close', () => {
  console.log('\n👋 WebSocket 连接已关闭\n');

  // 统计结果
  console.log('📊 统计结果：');
  console.log(`   总共收到 ${receivedEvents.length} 个事件`);
  console.log(`   初始 session_id: ${INITIAL_SESSION_ID}`);
  console.log(`   真实 session_id: ${actualSessionId || '未收到'}\n`);

  if (actualSessionId) {
    const wrongEvents = receivedEvents.filter(e => e.session_id !== actualSessionId);
    if (wrongEvents.length === 0) {
      console.log('✅ 所有事件的 session_id 都正确！');
    } else {
      console.log(`❌ 有 ${wrongEvents.length} 个事件的 session_id 不正确：`);
      wrongEvents.forEach(e => {
        console.log(`   - ${e.type}: ${e.session_id}`);
      });
    }
  }

  process.exit(0);
});

// 10 秒后自动关闭
setTimeout(() => {
  console.log('\n⏰ 测试完成，关闭连接...');
  ws.close();
}, 10000);

// 处理 Ctrl+C
process.on('SIGINT', () => {
  console.log('\n\n⚠️  收到中断信号，关闭连接...');
  ws.close();
  process.exit(0);
});
