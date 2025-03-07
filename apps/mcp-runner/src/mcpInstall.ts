import lunr from "lunr";
import { proxyRequest } from "./proxyRequest.js";
import { RegistryTool, Tool, Tools } from "./types.js";

export class MCPInstall {
  // public static registry: Record<string, Tool> = {};
  public static name = 'install_mcp_servers_and_tools';
  public static isInitialized = false;

  public static async install(name: string) {
    if (!MCPInstall.isInitialized) {
      throw new Error("MCPInstall is not initialized");
    }
    const result = await proxyRequest('registry/install', {
      tool: name
    });
    return {
      content: [{
        type: 'text',
        text: JSON.stringify(result),
      }]
    };
  }
  
  private static setInitialized() {
    MCPInstall.isInitialized = true;
  }

  public static async init() {
    // const result = await proxyRequest<{ tools: RegistryTool[] }>('registry/list', {});
    // result.tools.forEach((tool: any) => {
    //   MCPInstall.registry[tool.name] = tool;
    // });
    MCPInstall.setInitialized();
  } 

  static tool: Tool = {
    "fullDescription": "Installs MCP Servers & Tools available to be installed.",
    "description": "Installs MCP Servers & Tools available to be installed.",
      "inputSchema": {
        "description": "Name of the MCP Server or Tool to install.",
        "properties": {
          "name": {
            "description": "Name of the MCP Server or Tool to install.",
            "title": "Name",
            "type": "string"
          }
        },
        "required": [
          "name"
        ],
        "title": "Install MCP Server",
        "type": "object"
      },
      "name": MCPInstall.name,
      "server_id": MCPInstall.name
    };
}