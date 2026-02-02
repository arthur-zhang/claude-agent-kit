#!/usr/bin/env node

/**
 * Test script for UserSessionInit message
 *
 * This script tests the new two-phase initialization flow:
 * 1. Connect to WebSocket
 * 2. Send UserSessionInit message
 * 3. Wait for SessionInit response
 * 4. Send a test user message
 */

const WebSocket = require('ws');

const WS_URL = 'ws://localhost:3000/ws';
const SESSION_ID = `test-${Date.now()}`;

console.log('🧪 Testing UserSessionInit flow...\n');

const ws = new WebSocket(`${WS_URL}?session_id=${SESSION_ID}`);

ws.on('open', () => {
  console.log('✅ WebSocket connected');

  // Send UserSessionInit message
  const initMessage = {
    type: 'user_session_init',
    id: `init-${Date.now()}`,
    session_id: SESSION_ID,
    cwd: process.cwd(),
    model: 'claude-sonnet-4',
    permission_mode: 'manual',
    max_turns: 10,
    max_budget_usd: 1.0,
    user: 'test-user'
  };

  console.log('📤 Sending UserSessionInit:', JSON.stringify(initMessage, null, 2));
  ws.send(JSON.stringify(initMessage));
});

ws.on('message', (data) => {
  try {
    const message = JSON.parse(data.toString());
    console.log('📥 Received:', message.type);

    if (message.type === 'session_init') {
      console.log('✅ Session initialized successfully!');
      console.log('   Session ID:', message.session_id);
      console.log('   CWD:', message.cwd);
      console.log('   Model:', message.model);
      console.log('   Tools:', message.tools?.length || 0);

      // Send a test message
      setTimeout(() => {
        const testMessage = {
          type: 'user_message',
          id: `msg-${Date.now()}`,
          session_id: SESSION_ID,
          content: 'Hello! This is a test message.'
        };

        console.log('\n📤 Sending test message...');
        ws.send(JSON.stringify(testMessage));
      }, 1000);
    } else if (message.type === 'error') {
      console.error('❌ Error:', message.message);
      if (message.is_fatal) {
        console.error('   Fatal error - closing connection');
        ws.close();
      }
    } else if (message.type === 'assistant_message') {
      console.log('💬 Assistant:', message.text.substring(0, 100) + '...');
    } else if (message.type === 'turn_completed') {
      console.log('✅ Turn completed');
      console.log('   Tokens used:', message.usage.total_tokens);

      // Test complete - close connection
      setTimeout(() => {
        console.log('\n✅ Test completed successfully!');
        ws.close();
      }, 1000);
    }
  } catch (error) {
    console.error('❌ Failed to parse message:', error);
  }
});

ws.on('error', (error) => {
  console.error('❌ WebSocket error:', error.message);
});

ws.on('close', () => {
  console.log('🔌 WebSocket disconnected');
  process.exit(0);
});

// Timeout after 30 seconds
setTimeout(() => {
  console.error('❌ Test timeout - closing connection');
  ws.close();
  process.exit(1);
}, 30000);
