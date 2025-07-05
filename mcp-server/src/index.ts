#!/usr/bin/env node
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from '@modelcontextprotocol/sdk/types.js';
import axios, { AxiosInstance } from 'axios';
import { config } from 'dotenv';
import winston from 'winston';
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

// Load environment variables
config();

// Get directory of current module
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Configuration interface
interface Config {
  backend: {
    url: string;
    timeout: number;
  };
  server: {
    name: string;
    version: string;
    description: string;
  };
  useConfigFile?: boolean;
}

// Configure logging
const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.json(),
  transports: [
    new winston.transports.File({ 
      filename: '/tmp/jauauth-mcp.log',
      format: winston.format.combine(
        winston.format.timestamp(),
        winston.format.json()
      )
    })
  ],
});

// Load configuration
async function loadConfig(): Promise<Config> {
  // Default configuration
  let config: Config = {
    backend: {
      url: process.env.RUST_BACKEND_URL || 'http://localhost:7447',
      timeout: parseInt(process.env.API_TIMEOUT || '30000')
    },
    server: {
      name: 'JauAuth Router',
      version: '1.0.0',
      description: 'MCP router for multiple backend servers'
    },
    useConfigFile: process.env.USE_CONFIG_FILE === 'true'
  };

  // Check if we should use config file (controlled by dashboard toggle)
  if (config.useConfigFile) {
    try {
      const configPath = path.join(__dirname, '..', 'config.json');
      const configContent = await fs.readFile(configPath, 'utf-8');
      const fileConfig = JSON.parse(configContent);
      
      // Merge file config with environment config (env takes precedence)
      config = {
        ...config,
        backend: {
          ...fileConfig.backend,
          url: process.env.RUST_BACKEND_URL || fileConfig.backend?.url || config.backend.url,
          timeout: parseInt(process.env.API_TIMEOUT || fileConfig.backend?.timeout || config.backend.timeout)
        },
        server: {
          ...fileConfig.server,
          ...config.server
        }
      };
      
      logger.info('Loaded configuration from config.json');
    } catch (error) {
      logger.warn('Failed to load config.json, using defaults', { error });
    }
  }

  return config;
}

// Global configuration and backend instance
let appConfig: Config;
let backend: AxiosInstance;

class JauAuthMCPServer {
  private server: Server;
  private tools: Map<string, Tool> = new Map();
  private serverStatus: Map<string, boolean> = new Map();

  constructor(config: Config) {
    this.server = new Server(
      {
        name: config.server.name,
        version: config.server.version,
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.setupHandlers();
  }

  private setupHandlers() {
    // Handle tool listing
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      logger.info('Listing tools');
      
      try {
        // Fetch current tools from Rust backend
        await this.refreshTools();
        
        return {
          tools: Array.from(this.tools.values()),
        };
      } catch (error) {
        logger.error('Failed to list tools', { error });
        return { tools: [] };
      }
    });

    // Handle tool calls
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;
      logger.info('Calling tool', { name, args });

      try {
        // Special handling for router management tools
        if (name === 'router_status') {
          return await this.getRouterStatus();
        }

        if (name === 'router_list_servers') {
          return await this.listServers();
        }

        // Route all other tools to the Rust backend
        // Convert first underscore back to colon for backend routing (server_id:tool_name)
        const backendToolName = name.replace('_', ':');
        
        // Extract timeout parameter if provided
        let timeout = appConfig.backend.timeout; // Default timeout
        let cleanArgs = args;
        
        if (args && typeof args === 'object' && '__timeout' in args) {
          const timeoutParam = args.__timeout;
          
          // Handle special case: '*' means no timeout
          if (timeoutParam === '*') {
            timeout = 0; // 0 means no timeout in axios
          } else if (typeof timeoutParam === 'number' && timeoutParam > 0) {
            timeout = timeoutParam;
          } else if (typeof timeoutParam === 'string' && !isNaN(parseInt(timeoutParam))) {
            timeout = parseInt(timeoutParam);
          }
          
          // Remove __timeout from arguments before forwarding
          const { __timeout, ...restArgs } = args;
          cleanArgs = restArgs;
          
          logger.info(`Tool ${name} using custom timeout: ${timeout}ms`);
        }
        
        // Create request-specific axios instance with custom timeout
        const requestBackend = timeout === 0 
          ? axios.create({
              baseURL: appConfig.backend.url,
              headers: { 'Content-Type': 'application/json' },
              // No timeout
            })
          : axios.create({
              baseURL: appConfig.backend.url,
              timeout: timeout,
              headers: { 'Content-Type': 'application/json' },
            });
        
        // Only pass timeout_ms if user explicitly provided __timeout
        const hasExplicitTimeout = args && typeof args === 'object' && '__timeout' in args;
        
        const response = await requestBackend.post('/api/mcp/tool/call', {
          tool: backendToolName,
          arguments: cleanArgs,
          timeout_ms: hasExplicitTimeout ? timeout : undefined,
        });

        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(response.data.result, null, 2),
            },
          ],
        };
      } catch (error: any) {
        logger.error('Tool call failed', { name, error: error.message });
        
        // Provide more specific error for timeout
        let errorMessage = error.response?.data?.error || error.message;
        if (error.code === 'ECONNABORTED' || error.message.includes('timeout')) {
          errorMessage = `Request timeout: The operation took longer than expected. Consider using __timeout parameter for long-running operations.`;
        }
        
        return {
          content: [
            {
              type: 'text',
              text: `Error: ${errorMessage}`,
            },
          ],
          isError: true,
        };
      }
    });
  }

  private async refreshTools() {
    try {
      // Get current tools from Rust backend
      const response = await backend.get('/api/mcp/tools');
      const tools = response.data.tools || [];

      // Clear and update tools map
      this.tools.clear();

      // Always include router management tools
      this.tools.set('router_status', {
        name: 'router_status',
        description: 'Get the current status of all backend servers',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      });

      this.tools.set('router_list_servers', {
        name: 'router_list_servers',
        description: 'List all configured backend servers',
        inputSchema: {
          type: 'object',
          properties: {},
          required: [],
        },
      });

      // Add tools from backend servers
      for (const tool of tools) {
        // Replace colons with underscores to comply with Claude's naming requirements
        const safeName = tool.name.replace(/:/g, '_');
        
        // Enhance tool description with timeout parameter info
        const enhancedDescription = tool.description + 
          '\n\nNote: For long-running operations, you can add __timeout parameter (in milliseconds) to your arguments. ' +
          'Use __timeout: "*" for no timeout, or a number like __timeout: 300000 for 5 minutes.';
        
        // Add __timeout to the input schema if it has properties
        const enhancedSchema = tool.inputSchema ? {
          ...tool.inputSchema,
          properties: {
            ...tool.inputSchema.properties,
            __timeout: {
              type: ['string', 'number'],
              description: 'Optional timeout in milliseconds. Use "*" for no timeout, or a number like 300000 for 5 minutes. Default: 30000ms',
              examples: ['*', 300000, '60000']
            }
          }
        } : tool.inputSchema;
        
        this.tools.set(safeName, {
          ...tool,
          name: safeName,
          description: enhancedDescription,
          inputSchema: enhancedSchema
        });
      }

      logger.info(`Refreshed tools: ${this.tools.size} tools available`);
    } catch (error) {
      logger.error('Failed to refresh tools', { error });
    }
  }

  private async getRouterStatus() {
    try {
      const response = await backend.get('/api/mcp/status');
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: 'Failed to get router status',
          },
        ],
        isError: true,
      };
    }
  }

  private async listServers() {
    try {
      const response = await backend.get('/api/mcp/servers');
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: 'Failed to list servers',
          },
        ],
        isError: true,
      };
    }
  }

  async start() {
    logger.info('Starting JauAuth MCP server');

    // Initialize tools from backend
    await this.refreshTools();

    // Create stdio transport
    const transport = new StdioServerTransport();
    
    // Start the server
    await this.server.connect(transport);
    
    logger.info('JauAuth MCP server started successfully');

    // Refresh tools periodically
    setInterval(() => {
      this.refreshTools().catch((error) => {
        logger.error('Failed to refresh tools', { error });
      });
    }, 30000); // Every 30 seconds
  }
}

// Main function to start the server
async function main() {
  try {
    // Load configuration
    appConfig = await loadConfig();
    logger.info('Configuration loaded', { 
      backendUrl: appConfig.backend.url,
      useConfigFile: appConfig.useConfigFile 
    });
    
    // Create axios instance with loaded config
    backend = axios.create({
      baseURL: appConfig.backend.url,
      timeout: appConfig.backend.timeout,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    // Start the server
    const server = new JauAuthMCPServer(appConfig);
    await server.start();
  } catch (error) {
    logger.error('Failed to start MCP server', { error });
    console.error('Failed to start MCP server:', error);
    process.exit(1);
  }
}

// Start the application
main();