const availableTools: RegistryTool[] = [
  {
    id: "helius-proxy",
    name: "Helius Proxy",
    description: "A simple tool that proxies requests to Helius",
    publisher: {
      id: "dcspark",
      name: "dcspark",
      url: "https://www.dcspark.com/",
    },
    isOfficial: true,
    sourceUrl: "https://github.com/dcspark/mcp-server-helius",
    distribution: {
      type: "npm",
      package: "@mcp-dockmaster/mcp-server-helius",
    },
    license: "MIT",
    runtime: "node",
    config: {
      command: "npx",
      args: ["-y", "@mcp-dockmaster/mcp-server-helius"],
      env: {
        "HELIUS_API_KEY": {
          description: "Your Helius API key. See: https://www.helius.xyz/api",
        }
      }
    }
  },
  {
    id: "jupiter-proxy",
    name: "Jupiter Proxy",
    description: "A tool that provides access to Jupiter's Solana swap services",
    publisher: {
      id: "jupiter",
      name: "Jupiter",
      url: "https://jup.ag/",
    },
    isOfficial: false,
    sourceUrl: "local",
    distribution: {
      type: "npm",
      package: "file:/Users/nicolasarqueros/mcp-dockmaster/develop/mcp-server-jupiter",
    },
    license: "MIT",
    runtime: "node",
    config: {
      command: "npx",
      args: ["-y", "file:/Users/nicolasarqueros/mcp-dockmaster/develop/mcp-server-jupiter"],
      env: {
        "JUPITER_API_KEY": {
          description: "Your Jupiter API key. See: https://jup.ag/api",
        }
      }
    }
  },
  {
    id: "brave-search-ref",
    name: "Brave Search",
    description: "Web and local search using Brave's Search API. A Model Context Protocol reference server.",
    publisher: {
      id: "modelcontextprotocol",
      name: "Anthropic, PBC",
      url: "https://modelcontextprotocol.io/",
    },
    isOfficial: false,
    sourceUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/brave-search",
    distribution: {
      type: "npm",
      package: "@modelcontextprotocol/server-brave-search",
    },
    license: "MIT",
    runtime: "node",
    config: {
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-brave-search"],
      env: {
        "BRAVE_API_KEY": {
          description: "Your Brave Search API key. See: https://brave.com/search/api",
        }
      }
    }
  },
  {
    id: "github-ref",
    name: "GitHub",
    description: "GitHub repository access and management. A Model Context Protocol reference server.",
    publisher: {
      id: "modelcontextprotocol",
      name: "Anthropic, PBC",
      url: "https://modelcontextprotocol.io/",
    },
    isOfficial: false,
    sourceUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/github",
    distribution: {
      type: "npm",
      package: "@modelcontextprotocol/server-github",
    },
    license: "MIT",
    runtime: "node",
    config: {
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-github"],
      env: {
        "GITHUB_PERSONAL_ACCESS_TOKEN": {
          description: "Your GitHub Personal Access Token. Find it at: https://github.com/settings/tokens",
        }
      }
    }
  },
  {
    id: "memory-ref",
    name: "Memory",
    description: "Knowledge graph-based persistent memory system. A Model Context Protocol reference server.",
    publisher: {
      id: "modelcontextprotocol",
      name: "Anthropic, PBC",
      url: "https://modelcontextprotocol.io/",
    },
    isOfficial: false,
    sourceUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
    distribution: {
      type: "npm",
      package: "@modelcontextprotocol/server-memory",
    },
    license: "MIT",
    runtime: "node",
    config: {
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-memory"],
      env: {}
    }
  }
]

interface RegistryTool {
  id: string;
  name: string;
  description: string;
  publisher: {
    id: string;
    name: string;
    url: string;
  };
  isOfficial: boolean;
  sourceUrl: string;
  distribution: {
    type: string;
    package: string;
  };
  license: string;
  runtime: string;
  config: {
    command: string;
    args: string[];
    env: Record<string, any>;
  };
}

/**
 * Get all available tools from the registry
 */
export const getAvailableTools = async (): Promise<RegistryTool[]> => {
  return availableTools;
};

/**
 * Get a specific tool by ID
 */
export const getToolById = async (id: string): Promise<RegistryTool | null> => {
  return availableTools.find(tool => tool.id === id) || null;
};

export default {
  getAvailableTools,
  getToolById
};