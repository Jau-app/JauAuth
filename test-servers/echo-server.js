#!/usr/bin/env node

// Simple MCP echo server for testing
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
            name: 'Echo Test Server',
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
              name: 'echo',
              description: 'Echo back the input message',
              inputSchema: {
                type: 'object',
                properties: {
                  message: {
                    type: 'string',
                    description: 'Message to echo'
                  }
                },
                required: ['message']
              }
            }
          ]
        }
      });
    } else if (msg.method === 'tools/call') {
      const toolName = msg.params.name;
      const args = msg.params.arguments || {};
      
      if (toolName === 'echo') {
        send({
          jsonrpc: '2.0',
          id: msg.id,
          result: {
            content: [{
              type: 'text',
              text: `Echo: ${args.message || 'No message provided'}`
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