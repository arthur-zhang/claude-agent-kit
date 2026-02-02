#!/usr/bin/env node

/**
 * WebSocket 统一事件系统测试脚本
 *
 * 用法：
 *   1. 启动 WebSocket 服务器：cargo run
 *   2. 运行此脚本：node test-unified-events.js
 */

const WebSocket = require('ws');

const INITIAL_SESSION_ID = 'test-' + Date.now();
const WS_URL = `ws://localhost:3000/ws?session_id=${INITIAL_SESSION_ID}`;

console.log('🚀 连接到 WebSocket 服务器...');
console.log(`📍 URL: ${WS_URL}\n`);

const ws = new WebSocket(WS_URL);

// 记录接收到的事件
const receivedEvents = [];
// 存储从 session_init 接收到的真实 session_id
let actualSessionId = INITIAL_SESSION_ID;
// 标记是否已发送测试消息
let testMessageSent = false;

ws.on('open', () => {
  console.log('✅ WebSocket 连接成功\n');
  console.log('📨 等待接收事件...\n');
  console.log('─'.repeat(80));

  // 不再使用固定延迟发送消息，而是在收到 session_init 后发送

  // 10 秒后关闭连接并显示统计
  setTimeout(() => {
    console.log('\n📊 测试完成，统计信息：\n');
    console.log(`总共接收到 ${receivedEvents.length} 个事件\n`);

    // 按类型统计
    const eventCounts = {};
    receivedEvents.forEach(event => {
      eventCounts[event.type] = (eventCounts[event.type] || 0) + 1;
    });

    console.log('事件类型统计：');
    Object.entries(eventCounts).forEach(([type, count]) => {
      console.log(`  - ${type}: ${count}`);
    });

    console.log('\n✅ 验证结果：');

    // 验证必须的事件
    const requiredEvents = ['session_init', 'turn_started'];
    const missingEvents = requiredEvents.filter(type => !eventCounts[type]);

    if (missingEvents.length === 0) {
      console.log('  ✓ 所有必需事件都已接收');
    } else {
      console.log('  ✗ 缺少以下事件:', missingEvents.join(', '));
    }

    // 验证事件顺序
    if (receivedEvents[0]?.type === 'session_init') {
      console.log('  ✓ 第一个事件是 session_init');
    } else {
      console.log('  ✗ 第一个事件不是 session_init');
    }

    if (eventCounts['turn_started'] && eventCounts['turn_completed']) {
      console.log('  ✓ Turn 生命周期完整（started + completed）');
    }

    if (eventCounts['token_usage']) {
      console.log('  ✓ 包含 Token 使用统计');
    }

    ws.close();
  }, 10000);
});

ws.on('message', (data) => {
  try {
    const event = JSON.parse(data.toString());
    receivedEvents.push(event);

    // 美化输出
    console.log(`\n📥 [${event.type}]`);

    switch (event.type) {
      case 'session_init':
        // 保存从服务器接收到的真实 session_id
        actualSessionId = event.session_id;
        console.log(`   Session ID: ${event.session_id}`);
        console.log(`   Model: ${event.model || 'N/A'}`);
        console.log(`   ✅ 已更新 actualSessionId 为: ${actualSessionId}`);

        // 收到 session_init 后发送测试消息
        if (!testMessageSent) {
          testMessageSent = true;
          setTimeout(() => {
            console.log('\n📤 发送用户消息...\n');

            const userMessage = {
              type: 'user_message',
              id: 'msg-' + Date.now(),
              session_id: actualSessionId,  // 使用真实的 session_id
              content: 'Hello! Please tell me what is 2+2?',
              parent_tool_use_id: null
            };

            ws.send(JSON.stringify(userMessage));
            console.log('✉️  已发送:', JSON.stringify(userMessage, null, 2));
            console.log('─'.repeat(80));
          }, 500);
        }
        break;

      case 'turn_started':
        console.log(`   🔄 Turn 开始`);
        break;

      case 'turn_completed':
        console.log(`   ✅ Turn 完成`);
        if (event.usage) {
          console.log(`   Token 使用: ${event.usage.total_tokens} (输入: ${event.usage.input_tokens}, 输出: ${event.usage.output_tokens})`);
        }
        break;

      case 'turn_failed':
        console.log(`   ❌ Turn 失败: ${event.error}`);
        break;

      case 'assistant_message':
        const preview = event.text.substring(0, 50);
        console.log(`   💬 ${preview}${event.text.length > 50 ? '...' : ''}`);
        console.log(`   Final: ${event.is_final}`);
        break;

      case 'assistant_reasoning':
        const reasoningPreview = event.text.substring(0, 50);
        console.log(`   🤔 ${reasoningPreview}${event.text.length > 50 ? '...' : ''}`);
        break;

      case 'tool_started':
        console.log(`   🔧 工具: ${event.tool_name}`);
        console.log(`   ID: ${event.tool_id}`);
        break;

      case 'tool_completed':
        console.log(`   ${event.success ? '✅' : '❌'} 工具完成: ${event.tool_id}`);
        if (event.result) {
          const resultPreview = event.result.substring(0, 50);
          console.log(`   结果: ${resultPreview}${event.result.length > 50 ? '...' : ''}`);
        }
        break;

      case 'control_request':
        console.log(`   🔐 权限请求: ${event.tool_name}`);
        console.log(`   风险级别: ${event.context.risk_level}`);
        console.log(`   描述: ${event.context.description}`);

        // 自动批准（测试用）
        setTimeout(() => {
          const response = {
            type: 'permission_response',
            id: 'resp-' + Date.now(),
            session_id: actualSessionId,  // 使用真实的 session_id
            request_id: event.request_id,
            decision: 'allow',
            explanation: 'Auto-approved for testing'
          };
          ws.send(JSON.stringify(response));
          console.log(`   ✓ 已自动批准权限请求`);
        }, 100);
        break;

      case 'token_usage':
        console.log(`   📊 Token: ${event.usage.total_tokens}`);
        if (event.usage_percent) {
          console.log(`   使用率: ${(event.usage_percent * 100).toFixed(1)}%`);
        }
        break;

      case 'context_compaction':
        console.log(`   🗜️  上下文压缩: ${event.tokens_before} → ${event.tokens_after}`);
        break;

      case 'error':
        console.log(`   ❌ 错误: ${event.message}`);
        console.log(`   Fatal: ${event.is_fatal}`);
        break;

      default:
        console.log(`   数据:`, JSON.stringify(event, null, 2));
    }

    console.log('─'.repeat(80));
  } catch (e) {
    console.error('解析消息失败:', e);
  }
});

ws.on('error', (error) => {
  console.error('❌ WebSocket 错误:', error.message);
  console.log('\n💡 提示：');
  console.log('  1. 确保 WebSocket 服务器正在运行（cargo run）');
  console.log('  2. 检查端口 3000 是否可用');
  process.exit(1);
});

ws.on('close', () => {
  console.log('\n👋 WebSocket 连接已关闭');
  process.exit(0);
});

// 处理 Ctrl+C
process.on('SIGINT', () => {
  console.log('\n\n⚠️  收到中断信号，关闭连接...');
  ws.close();
  process.exit(0);
});
