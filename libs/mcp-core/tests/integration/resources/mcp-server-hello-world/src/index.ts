#!/usr/bin/env node

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ErrorCode,
  ListToolsRequestSchema,
  McpError,
} from "@modelcontextprotocol/sdk/types.js";
import { handlers, tools } from "./tools";

const server = new Server({
  name: "mcp-server-hello-world",
  version: "1.0.0",
}, {
  capabilities: {
    tools: {}
  }
});

const transport = new StdioServerTransport();
await server.connect(transport);

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return { tools };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const handler = handlers[request.params.name];
  if (handler) {
    try {
      const input = request.params.arguments
      return await handler(input);
    } catch (error) {
      return { toolResult: { error: (error as Error).message }, content: [], isError: true };
    }
  }
  return { toolResult: { error: "Method not found" }, content: [], isError: true };
});
