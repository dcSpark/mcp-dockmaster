import { invoke } from '@tauri-apps/api/core';

interface ToolRegistrationRequest {
  tool_id: string;
  tool_name: string;
  description: string;
  authentication?: any;
  tool_type: string;  // "nodejs", "python", "docker"
  configuration?: {
    command: string;
    args: string[];
    env: Record<string, string>;
  };
  distribution?: {
    type: string;
    package: string;
  };
}

interface ToolRegistrationResponse {
  success: boolean;
  message: string;
  tool_id?: string;
}

interface ToolExecutionRequest {
  tool_id: string;
  parameters: any;
}

interface ToolExecutionResponse {
  success: boolean;
  result?: any;
  error?: string;
}

interface ToolUpdateRequest {
  tool_id: string;
  enabled: boolean;
}

interface ToolUpdateResponse {
  success: boolean;
  message: string;
}

interface ToolConfigUpdateRequest {
  tool_id: string;
  configuration: {
    command: string;
    args?: string[];
    env?: Record<string, string>;
  };
}

interface ToolConfigUpdateResponse {
  success: boolean;
  message: string;
}

interface ToolUninstallRequest {
  tool_id: string;
}

interface ToolUninstallResponse {
  success: boolean;
  message: string;
}

interface DiscoverServerToolsRequest {
  server_id: string;
}

interface DiscoverServerToolsResponse {
  success: boolean;
  tools?: any[];
  error?: string;
}

/**
 * MCP Client for interacting with the MCP Server Proxy
 */
export class MCPClient {
  /**
   * Register a new tool with the MCP server
   */
  static async registerTool(request: ToolRegistrationRequest): Promise<ToolRegistrationResponse> {
    return await invoke<ToolRegistrationResponse>('register_tool', { request });
  }

  /**
   * List all registered tools
   */
  static async listTools(): Promise<any[]> {
    return await invoke<any[]>('list_tools');
  }

  /**
   * Execute a registered tool
   */
  static async executeTool(request: ToolExecutionRequest): Promise<ToolExecutionResponse> {
    return await invoke<ToolExecutionResponse>('execute_tool', { request });
  }

  /**
   * Update a tool's status (enabled/disabled)
   */
  static async updateToolStatus(request: ToolUpdateRequest): Promise<ToolUpdateResponse> {
    return await invoke<ToolUpdateResponse>('update_tool_status', { request });
  }

  /**
   * Update a tool's configuration (environment variables)
   */
  static async updateToolConfig(request: ToolConfigUpdateRequest): Promise<ToolConfigUpdateResponse> {
    return await invoke<ToolConfigUpdateResponse>('update_tool_config', { request });
  }

  static async restartTool(toolId: string): Promise<ToolUpdateResponse> {
    return await invoke<ToolUpdateResponse>('restart_tool_command', { toolId });
  }

  /**
   * Uninstall a registered tool
   */
  static async uninstallTool(request: ToolUninstallRequest): Promise<ToolUninstallResponse> {
    return await invoke<ToolUninstallResponse>('uninstall_tool', { request });
  }

  /**
   * Discover tools from a specific MCP server
   */
  static async discoverTools(request: DiscoverServerToolsRequest): Promise<DiscoverServerToolsResponse> {
    return await invoke<DiscoverServerToolsResponse>('discover_tools', { request });
  }
  
  /**
   * List all available tools from all running MCP servers
   */
  static async listAllServerTools(): Promise<any[]> {
    return await invoke<any[]>('list_all_server_tools');
  }
  
  /**
   * Execute a tool from an MCP server through the proxy
   */
  static async executeProxyTool(request: ToolExecutionRequest): Promise<ToolExecutionResponse> {
    return await invoke<ToolExecutionResponse>('execute_proxy_tool', { request });
  }
  
  /**
   * Get Claude configuration for MCP servers
   */
  static async getClaudeConfig(): Promise<any> {
    return await invoke<any>('get_claude_config');
  }

  /**
   * Get all server data in a single call to avoid lock issues
   * Returns servers, tools, and Claude configuration in a single response
   */
  static async getAllServerData(): Promise<any> {
    return await invoke<any>('get_all_server_data');
  }
}

export default MCPClient;  