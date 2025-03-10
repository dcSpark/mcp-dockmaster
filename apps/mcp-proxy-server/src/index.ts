/**
 * MCP Proxy Server
 * This server proxies MCP Protocol commands to a server running on localhost:3000
 */
import { Server } from "npm:@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "npm:@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
} from "npm:@modelcontextprotocol/sdk/types.js";
import { z } from "npm:zod";
import { Tools } from './types.ts';
import { proxyRequest } from "./proxyRequest.ts";
import { debugLog } from "./logger.ts";
import { initInternalTools, injectInternalTools, runInternalTool } from "./internal-tools/index.ts";

debugLog('Starting MCP Proxy Server script');

// Define the schema for tools/list_changed notification
const ToolsListChangedSchema = z.object({
  method: z.literal("notifications/tools/list_changed"),
});

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
      tools: {
        "listChanged": true,
      },
      prompts: {},
    },
  }
);

debugLog('Server instance created');

// Add notification handler for tools/list_changed
server.setNotificationHandler(ToolsListChangedSchema, async () => {
  debugLog('Received tools/list_changed notification');
  // Notify any connected clients that they should refresh their tool list
  server.notification({
    method: "notifications/tools/list_changed",
    params: {}
  });
});

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
        injectInternalTools(result);
        result.tools.forEach((tool: any) => {
          // This is temporal patch. 
          // Claude desktop breaks with "invalid names"
          tool.name = tool.name.replace(/[^a-zA-Z0-9_-]/g, '_');
          tool.name = tool.name.substring(0, 64);
        });
        debugLog('Received tools list with correct format');
        return result as any;
      }
      
      // If the result is an array, wrap it in a tools object
      if (Array.isArray(result)) {
        debugLog('Received tools as array, converting to expected format');
        injectInternalTools({ tools: result });
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
  const result = await runInternalTool(params);

  if (result.isInternalTool) {
    return result.result;
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
  await initInternalTools();

}

main().catch((error) => {
  console.error("Server error:", error);
  Deno.exit(1);
});
