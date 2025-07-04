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
        const response = await backend.post('/api/mcp/tool/call', {
          tool: backendToolName,
          arguments: args,
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
        
        return {
          content: [
            {
              type: 'text',
              text: `Error: ${error.response?.data?.error || error.message}`,
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
        this.tools.set(safeName, {
          ...tool,
          name: safeName
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