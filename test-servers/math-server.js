#!/usr/bin/env node

// Simple MCP math server for testing
const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

// Helper to send JSON-RPC response
function send(response) {
  process.stdout.write(JSON.stringify(response) + '\n');
}

// Handle incoming messages
rl.on('line', (line) => {
  try {
    const msg = JSON.parse(line);
    
    if (msg.method === 'initialize') {
      send({
        jsonrpc: '2.0',
        id: msg.id,
        result: {
          protocolVersion: '0.1.0',
          capabilities: {},
          serverInfo: {
            name: 'Math Test Server',
            version: '1.0.0'
          }
        }
      });
    } else if (msg.method === 'initialized') {
      // Notification, no response needed
    } else if (msg.method === 'tools/list') {
      send({
        jsonrpc: '2.0',
        id: msg.id,
        result: {
          tools: [
            {
              name: 'add',
              description: 'Add two numbers',
              inputSchema: {
                type: 'object',
                properties: {
                  a: { type: 'number', description: 'First number' },
                  b: { type: 'number', description: 'Second number' }
                },
                required: ['a', 'b']
              }
            },
            {
              name: 'multiply',
              description: 'Multiply two numbers',
              inputSchema: {
                type: 'object',
                properties: {
                  a: { type: 'number', description: 'First number' },
                  b: { type: 'number', description: 'Second number' }
                },
                required: ['a', 'b']
              }
            }
          ]
        }
      });
    } else if (msg.method === 'tools/call') {
      const toolName = msg.params.name;
      const args = msg.params.arguments || {};
      
      if (toolName === 'add') {
        const result = (args.a || 0) + (args.b || 0);
        send({
          jsonrpc: '2.0',
          id: msg.id,
          result: {
            content: [{
              type: 'text',
              text: `${args.a} + ${args.b} = ${result}`
            }]
          }
        });
      } else if (toolName === 'multiply') {
        const result = (args.a || 0) * (args.b || 0);
        send({
          jsonrpc: '2.0',
          id: msg.id,
          result: {
            content: [{
              type: 'text',
              text: `${args.a} × ${args.b} = ${result}`
            }]
          }
        });
      } else {
        send({
          jsonrpc: '2.0',
          id: msg.id,
          error: {
            code: -32601,
            message: `Unknown tool: ${toolName}`
          }
        });
      }
    } else {
      send({
        jsonrpc: '2.0',
        id: msg.id,
        error: {
          code: -32601,
          message: `Method not found: ${msg.method}`
        }
      });
    }
  } catch (e) {
    // Ignore parse errors
  }
});

// Clean exit on EOF
rl.on('close', () => {
  process.exit(0);
});