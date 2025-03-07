import lunr from "lunr";
import { proxyRequest } from "./proxyRequest.js";
import { RegistryTool, Tool, Tools } from "./types.js";

export class MCPSearch {
  public static registry: Record<string, Tool> = {};
  public static name = 'search_mcp_servers_and_tools';
  public static isInitialized = false;
  private static idx: {
    search: (query: string) => any[];
  };

  public static search(query: string): {
    content: {
      type: 'text';
      text: string;
    }[];
  } {
    if (!MCPSearch.isInitialized) {
      throw new Error("MCPSearch is not initialized");
    }
    const results: {
      ref: string;
      score: number;
      matchData: {
        metadata: Record<string, any>;
      };
    }[] = MCPSearch.idx.search(query);
   
    const tools = results
      .slice(0, 10)
      .map((result) => MCPSearch.registry[result.ref])
      .map(tool => ({
        name: tool.name, fullDescription: tool.fullDescription, id: tool.name
      }));

    return {
      content: [{
        type: 'text',
        text: JSON.stringify(tools),
      }]
    };
  }
  
  private static setInitialized() {
    MCPSearch.isInitialized = true;
    // console.log("MCPSearch initialized");
    // console.log('Example Search:', "sql database server");
    // console.log(MCPSearch.search("sql database server"));
  }

  public static async init() {
    // const result = await proxyRequest<Tools>('tools/list', {});
    const result = await proxyRequest<{ tools: RegistryTool[] }>('registry/list', {});
    // console.log('result registry/list', result);
    MCPSearch.idx = lunr(function (self: any) {
      self.ref('name');
      self.field('name');
      self.field('fullDescription');
      result.tools.forEach((tool: any) => {
        // console.log('[Added Search]', tool.name, tool.description.substring(0, 40) + '...');
        MCPSearch.registry[tool.name] = tool;
        self.add(tool)
      });
    });
    MCPSearch.setInitialized();
  } 

  static tool: Tool = {
    "fullDescription": "Searches for MCP Servers & Tools available to be installed.",
    "description": "Searches for MCP Servers & Tools available to be installed.",
      "inputSchema": {
        "description": "Query to search for MCP Servers & Tools available to be installed.",
        "properties": {
          "query": {
            "default": "test",
            "description": "Query to search for MCP Servers & Tools available to be installed.",
            "title": "Query",
            "type": "string"
          }
        },
        "required": [
          "query"
        ],
        "title": "Search MCP Servers & Tools",
        "type": "object"
      },
      "name": MCPSearch.name,
      "server_id": MCPSearch.name
    };
}