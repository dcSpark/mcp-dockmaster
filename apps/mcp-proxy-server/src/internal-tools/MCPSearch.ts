// @ts-types="npm:@types/lunr"
import lunr from "npm:lunr";
import { proxyRequest } from "../proxyRequest.ts";
import { RegistryTool, Tool } from "../types.ts";

type ExtendedRegistryTool = RegistryTool & {
  installed: boolean;
}

export class MCPSearch {
  public static registry: Record<string, ExtendedRegistryTool> = {};
  public static name = 'search_mcp_servers_and_tools';
  public static isInitialized = false;
  private static idx: {
    search: (query: string) => any[];
  };

  private static adaptTool(tool: ExtendedRegistryTool): any {
    return {
      name: tool.name, 
      description: tool.description,
      short_description: tool.short_description, 
      id: tool.id,
      installed: tool.installed,
      categories: tool.categories,
      config: tool.config.env
    }
  }

  public static search(query: string, exact: boolean = false): {
    content: {
      type: 'text';
      text: string;
    }[];
  } {
    if (!MCPSearch.isInitialized) {
      throw new Error("MCPSearch is not initialized");
    }

    if (exact) {
      for (const [toolName, tool] of Object.entries(MCPSearch.registry)) {
        if (toolName.toLowerCase() === query.toLowerCase()) {
          return {
            content: [{
              type: 'text',
              text: JSON.stringify([MCPSearch.adaptTool(tool)])
            }]
          }
        }
      }
    
      return {
        content: [{
          type: 'text',
          text: JSON.stringify({ message: "No exact match found for " + query })
        }]
      }
    } else {
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
        .map(tool => MCPSearch.adaptTool(tool));

      return {
        content: [{
          type: 'text',
          text: JSON.stringify(tools),
        }]
      };
    }
  }
  
  private static setInitialized() {
    MCPSearch.isInitialized = true;
    /* Uncomment to test the search tool when compiling. */
    // console.log("MCPSearch initialized");
    // console.log('Example Search:', "sql database server");
    // const results = MCPSearch.search("sql database server");
    // const results = MCPSearch.search("deepfates/mcp-replicate", true);
    // results.content.forEach((result) => {
    //   try {
    //     let x = JSON.parse(result.text);
    //     if (Array.isArray(x)) {
    //       x = x.map((tool: any) => {
    //         tool.description = tool.description.substring(0, 40);
    //         return tool;
    //       });
    //     }
    //     result.text = JSON.stringify(x);
    //   } catch (e) {
    //     console.log(result)
    //     console.log("Error parsing JSON:", e);
    //   }
    // });
    // console.log(results);
  }

  public static async init() {
      let result: { tools: ExtendedRegistryTool[] } = { tools: [] };
      try {
        result = await proxyRequest<{ tools: ExtendedRegistryTool[] }>('registry/list', {});
      } catch (error) {
        console.error('Error initializing MCPSearch:', error);
        result = { tools: [] };
      }

    MCPSearch.idx = lunr(function (self: any) {
      self.ref('name');
      self.field('name');
      self.field('categories');
      self.field('tags');
      self.field('description');
      result.tools.forEach((tool: ExtendedRegistryTool) => {
        MCPSearch.registry[tool.name] = tool;
        self.add({
          name: tool.name,
          tags: tool.tags.join(', '),
          categories: tool.categories.join(', '),
          description: tool.description
        })
      });
    });
    MCPSearch.setInitialized();
  } 

  static tool: Tool = {
    "description": "Searches for MCP Servers & Tools available to be installed.",
    "short_description": "Searches for MCP Servers & Tools available to be installed.",
      "inputSchema": {
        "description": "Query to search for MCP Servers & Tools available to be installed.",
        "properties": {
          "query": {
            "description": "Query to search for MCP Servers & Tools available to be installed.",
            "title": "Query",
            "type": "string"
          },
          "exact": {
            "default": false,
            "description": "If true, the search will only return exact matches.",
            "title": "Exact",
            "type": "boolean"
          }
        },
        "required": [
          "query"
        ],
        "title": "Search MCP Servers & Tools",
        "type": "object"
      },
      "name": MCPSearch.name,
      "server_id": MCPSearch.name,
      "installed": true,
      "categories": ["mcp-dockmaster", "search"]
    };
}