#!/usr/bin/env node

/**
 * MCP Proxy Server
 * This server proxies MCP Protocol commands to a server running on localhost:3000
 */
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import fs from 'fs';
import { z } from "zod";
import { Tools } from './types.js';
import { proxyRequest } from "./proxyRequest.js";
import { debugLog } from "./logger.js";
import { MCPSearch } from "./mcpSearch.js";
import { MCPInstall } from "./mcpInstall.js";

debugLog('Starting MCP Proxy Server script');

/**
 * Create an MCP server with all capabilities
 */
const server = new Server(
  {
    name: "MCP Proxy Server",
    version: "0.1.0",
  },
  {
    capabilities: {
      resources: {},
      tools: {},
      prompts: {},
    },
  }
);

debugLog('Server instance created');

debugLog('Setting up request handlers');

// Handler for listing resources
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  debugLog('ListResourcesRequestSchema handler called');
  const resources = await proxyRequest('resources/list', {});
  return resources as any; // The backend should already return { resources: [...] }
});

// Handler for reading resources
server.setRequestHandler(ReadResourceRequestSchema, async (request: any) => {
  debugLog('ReadResourceRequestSchema handler called', request);
  return await proxyRequest('resources/read', request.params);
});

// Handler for listing tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  debugLog('ListToolsRequestSchema handler called');
  try {
    const result = await proxyRequest<Tools>('tools/list', {});
    
    // Ensure the result has the expected format
    if (result && typeof result === 'object') {
      // If the result already has a tools array, return it directly
      if (Array.isArray(result.tools)) {
        result.tools.push(MCPSearch.tool);
        result.tools.push(MCPInstall.tool);
        debugLog('Received tools list with correct format');
        return result as any;
      }
      
      // If the result is an array, wrap it in a tools object
      if (Array.isArray(result)) {
        debugLog('Received tools as array, converting to expected format');
        result.push(MCPSearch.tool);
        result.push(MCPInstall.tool);
        return { tools: result } as any;
      }
    }
    
    // If we got here, the format is unexpected
    debugLog('Unexpected tools list format, returning empty list');
    return { tools: [] } as any;
  } catch (error) {
    console.error('Error fetching tools list:', error);
    return { tools: [] } as any;
  }
});

// Handler for calling tools
server.setRequestHandler(CallToolRequestSchema, async (request: any) => {
  debugLog('CallToolRequestSchema handler called', request);
  const params: { name: string, arguments: Record<string, any> } = request.params;
  if (params.name === MCPSearch.name) {
    return await MCPSearch.search(params.arguments.query);
  }
  if (params.name === MCPInstall.name) {
    fs.writeFileSync('/Users/edwardalvarado/mcp-dockmaster/apps/mcp-runner/install-args-2.json', JSON.stringify(params, null, 2));
    const x = await MCPInstall.install(params.arguments.name);
    fs.writeFileSync('/Users/edwardalvarado/mcp-dockmaster/apps/mcp-runner/install-result-2.json', JSON.stringify(x, null, 2));
    return x;
  }
  const callResult = await proxyRequest('tools/call', request.params);
  return callResult as any;
});

// Handler for listing prompts
server.setRequestHandler(ListPromptsRequestSchema, async () => {
  debugLog('ListPromptsRequestSchema handler called');
  const prompts = await proxyRequest('prompts/list', {});
  return prompts as any;  // The backend should already return { prompts: [...] }
});

// Handler for getting prompts
server.setRequestHandler(GetPromptRequestSchema, async (request: any) => {
  debugLog('GetPromptRequestSchema handler called', request);
  return await proxyRequest('prompts/get', request.params);
});

debugLog('All request handlers set up');

// Add a ping handler for testing
server.setRequestHandler(z.object({ method: z.literal('ping') }), async () => {
  debugLog('Ping handler called');
  return { message: 'pong' };
});

/**
 * Start the server using stdio transport.
 * This allows the server to communicate via standard input/output streams.
 */
async function main() {
  console.error('Starting MCP Proxy Server...');

  const transport = new StdioServerTransport();
  
  // Add transport event handlers
  transport.onmessage = (message) => {
    debugLog('Transport received message:', JSON.stringify(message));
  };
  
  transport.onerror = (error) => {
    debugLog('Transport error:', error);
  };
  
  transport.onclose = () => {
    debugLog('Transport closed');
  };
  
  await server.connect(transport);
  
  console.error('MCP Proxy Server started');
  // Initialize internal tools;
  await MCPSearch.init();
  await MCPInstall.init();
}

main().catch((error) => {
  console.error("Server error:", error);
  process.exit(1);
});
