import React, { useState, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import MCPClient from "../lib/mcpClient";
import { getAvailableTools } from "../lib/registry";
import { TOOL_UNINSTALLED, TOOL_INSTALLED, dispatchToolInstalled, dispatchToolUninstalled } from "../lib/events";
import "./Registry.css";

// Import runner icons
import dockerIcon from "../assets/docker.svg";
import nodeIcon from "../assets/node.svg";
import pythonIcon from "../assets/python.svg";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./ui/card";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Skeleton } from "./ui/skeleton";
import { Search } from "lucide-react";

interface RegistryTool {
  id: string;
  name: string;
  description: string;
  fullDescription: string;
  publisher: {
    id: string;
    name: string;
    url: string;
  };
  runtime: string;
  installed: boolean;
  isOfficial?: boolean;
  sourceUrl?: string;
  distribution?: {
    type: string;
    package: string;
  };
  config?: {
    command: string;
    args: string[];
    env: Record<string, any>;
  };
  license?: string;
}

const Registry: React.FC = () => {
  const [availableTools, setAvailableTools] = useState<RegistryTool[]>([]);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<string | null>(null);
  const [uninstalling, setUninstalling] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");

  // Load tools on initial mount
  useEffect(() => {
    loadAvailableTools();

    // Add event listener for visibility change to reload tools when component becomes visible
    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        loadAvailableTools();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    document.addEventListener("visibilitychange", handleVisibilityChange);

    // Also reload when the window regains focus
    window.addEventListener("focus", loadAvailableTools);

    window.addEventListener("focus", loadAvailableTools);

    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", loadAvailableTools);
    };
  }, []);

  // Listen for tool installation/uninstallation events
  useEffect(() => {
    // When a tool is uninstalled, update its status in the registry
    const handleToolUninstalled = (event: CustomEvent<{ toolId: string }>) => {
      const { toolId } = event.detail;
      setAvailableTools((prev) => prev.map((tool) => (tool.id === toolId ? { ...tool, installed: false } : tool)));
    };

    // When a tool is installed elsewhere, update its status in the registry
    const handleToolInstalled = (event: CustomEvent<{ toolId: string }>) => {
      const { toolId } = event.detail;
      setAvailableTools((prev) => prev.map((tool) => (tool.id === toolId ? { ...tool, installed: true } : tool)));
    };

    // Add event listeners
    document.addEventListener(TOOL_UNINSTALLED, handleToolUninstalled as EventListener);
    document.addEventListener(TOOL_INSTALLED, handleToolInstalled as EventListener);

    // Clean up event listeners on unmount
    return () => {
      document.removeEventListener(TOOL_UNINSTALLED, handleToolUninstalled as EventListener);
      document.removeEventListener(TOOL_INSTALLED, handleToolInstalled as EventListener);
    };
  }, []);

  const loadAvailableTools = async () => {
    setLoading(true);
    try {
      // Get tools from registry
      const registryTools = await getAvailableTools();

      // Get installed tools to check status
      const installedTools = await MCPClient.listTools();

      // Create a map to ensure we don't have duplicate tools by name
      const uniqueToolsMap = new Map();

      // Process registry tools and mark as installed if they match installed tools
      registryTools.forEach((tool) => {
        // Check if tool is installed by ID or by name (in case IDs don't match)
        const isInstalled = installedTools.some(
          (installedTool) =>
            installedTool.id === tool.id || installedTool.name.toLowerCase() === tool.name.toLowerCase()
        );

        // Use lowercase name as key to avoid case-sensitivity issues
        const key = tool.name.toLowerCase();

        // Only add if not already in the map
        if (!uniqueToolsMap.has(key)) {
          uniqueToolsMap.set(key, {
            ...tool,
            installed: isInstalled,
          });
        }
      });

      // Convert map values to array
      const toolsWithStatus = Array.from(uniqueToolsMap.values());

      setAvailableTools(toolsWithStatus);
    } catch (error) {
      console.error("Failed to load available tools:", error);
    } finally {
      setLoading(false);
    }
  };

  const installTool = async (tool: RegistryTool) => {
    // Double-check if the tool is already installed before proceeding
    const installedTools = await MCPClient.listTools();
    const isAlreadyInstalled = installedTools.some(
      (installedTool) => installedTool.id === tool.id || installedTool.name.toLowerCase() === tool.name.toLowerCase()
    );

    if (isAlreadyInstalled) {
      // Tool is already installed, update UI and don't try to install again
      setAvailableTools((prev) => prev.map((item) => (item.id === tool.id ? { ...item, installed: true } : item)));
      return;
    }

    setInstalling(tool.id);
    try {
      // For now, use a default entry point based on the tool type
      const entryPoint = getDefaultEntryPoint(tool.name);

      // Prepare authentication if needed
      let authentication = null;
      if (tool.config && tool.config.env) {
        // For now, we don't have a way to collect env vars from the user
        // In a real implementation, you would prompt the user for these values
        authentication = { env: tool.config.env };
      }

      console.log("Registering tool:", JSON.stringify(tool, null, 2), tool.runtime, entryPoint, authentication);
      const response = await MCPClient.registerTool({
        tool_id: tool.id,
        tool_name: tool.name,
        description: tool.description,
        tool_type: tool.runtime,
        configuration: tool.config,
        distribution: tool.distribution,
        authentication: authentication,
      });

      if (response.success) {
        // Update tool as installed
        setAvailableTools((prev) => prev.map((item) => (item.id === tool.id ? { ...item, installed: true } : item)));

        // Dispatch event that a tool was installed
        dispatchToolInstalled(tool.id);
      }
    } catch (error) {
      console.error("Failed to install tool:", error);
    } finally {
      setInstalling(null);
    }
  };

  const uninstallTool = async (id: string) => {
    try {
      setUninstalling(id);

      // Update the UI optimistically
      setAvailableTools((prev) => prev.map((tool) => (tool.id === id ? { ...tool, installed: false } : tool)));

      // Get the tool from the registry
      const registryTool = availableTools.find((tool) => tool.id === id);
      if (!registryTool) {
        console.error("Tool not found in registry:", id);
        return;
      }

      // Get the actual tool ID from the backend by matching names
      const installedTools = await MCPClient.listTools();
      const matchingTool = installedTools.find((tool) => tool.name.toLowerCase() === registryTool.name.toLowerCase());

      if (!matchingTool) {
        console.error("Tool not found in installed tools:", registryTool.name);
        // Revert UI change
        setAvailableTools((prev) => prev.map((tool) => (tool.id === id ? { ...tool, installed: true } : tool)));
        return;
      }

      // Use the actual tool ID from the backend
      const actualToolId = matchingTool.id;

      // Call the backend API to uninstall the tool
      const response = await MCPClient.uninstallTool({
        tool_id: actualToolId,
      });

      if (response.success) {
        // Dispatch event that a tool was uninstalled with the registry ID for UI updates
        dispatchToolUninstalled(id);
      } else {
        // If the API call fails, revert the UI change
        console.error("Failed to uninstall tool:", response.message);
        setAvailableTools((prev) => prev.map((tool) => (tool.id === id ? { ...tool, installed: true } : tool)));
      }
    } catch (error) {
      console.error("Error uninstalling tool:", error);
      // Refresh the list to ensure UI is in sync with backend
      loadAvailableTools();
    } finally {
      setUninstalling(null);
    }
  };

  // Helper function to get a default entry point based on tool type and name
  const getDefaultEntryPoint = (toolName: string): string => {
    // Try to find the tool in the available tools to get its distribution info
    const tool = availableTools.find((t) => t.name === toolName);

    if (tool && tool.distribution && tool.config) {
      // Run the command with the args if provided
      if (tool.config.command && tool.config.args) {
        return `${tool.config.command} ${tool.config.args.join(" ")}`;
      }

      // Fallback 1: If the tool is an npm package, use npx to run it
      if (tool.distribution.type === "npm" && tool.distribution.package) {
        return `npx -y ${tool.distribution.package}`;
      }

      // Fallback 2: If the tool is a docker image, use the image name
      if (tool.distribution.type === "dockerhub" && tool.distribution.package) {
        return `docker run --name ${tool.id} ${tool.distribution.package}`;
      }
    }

    throw new Error(`No entry point found for tool: ${toolName}`);
  };

  const getRunnerIcon = (runtime: string) => {
    switch (runtime.toLowerCase()) {
      case "docker":
        return <img src={dockerIcon} alt="Docker" className="runner-icon" title="Docker" />;
      case "node":
        return <img src={nodeIcon} alt="Node.js" className="runner-icon" title="Node.js" />;
      case "python":
        return <img src={pythonIcon} alt="Python/UV" className="runner-icon" title="Python/UV" />;
      default:
        return <span className="runner-icon unknown">?</span>;
    }
  };
  const parentRef = React.useRef<HTMLDivElement>(null);

  const filteredTools = searchTerm
    ? availableTools.filter(
        (tool) =>
          tool.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          tool.description.toLowerCase().includes(searchTerm.toLowerCase())
      )
    : availableTools;

  // Set up the virtualizer
  const rowVirtualizer = useVirtualizer({
    count: filteredTools.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 170, // Estimated height of each tool item in pixels
    overscan: 1, // Number of items to render beyond the visible area
    gap: 16,
  });

  return (
    <div className="h-full px-6 flex flex-col gap-8 py-10 max-w-4xl mx-auto w-full">
      <div className="flex flex-col space-y-1.5 ">
        <h1 className="font-semibold tracking-tight text-2xl">AI App Store</h1>
        <p className="text-sm text-muted-foreground">Discover and install AI applications and MCP tools.</p>
      </div>

      <div className="relative flex-1 max-h-12 h-full min-h-12 shrink-0">
        <div className="absolute top-4 left-3 flex items-center pointer-events-none">
          <Search size={18} className="text-gray-400" />
        </div>
        <input
          type="text"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="size-full pl-10 pr-4 py-2 bg-background text-foreground border placeholder:text-muted-foreground  rounded-lg focus:outline-none focus:ring-1 focus:ring-neutral-900/60   "
          placeholder="Search for tools..."
          aria-label="Search for tools"
        />
      </div>
      {loading ? (
        <div className="flex justify-center items-center flex-col gap-3 ">
          {Array.from({ length: 3 }).map((_, index) => (
            <Skeleton key={index} className="w-full h-40 bg-muted rounded-md" />
          ))}
        </div>
      ) : (
        <>
          {filteredTools.length === 0 ? (
            <div className="py-10 text-center text-sm text-muted-foreground">
              <p>No tools found matching your search criteria.</p>
            </div>
          ) : (
            <div ref={parentRef} style={{ height: "100%", overflow: "auto", contain: "strict" }}>
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  width: "100%",
                  position: "relative",
                  overflow: "hidden",
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const tool = filteredTools[virtualRow.index];
                  return (
                    <div
                      key={tool.id}
                      // className=" top-0 left-0 pr-4"
                      style={{
                        position: "absolute",
                        width: "100%",
                        height: `${virtualRow.size}px`,
                        transform: `translateY(${virtualRow.start}px)`,
                        boxSizing: "border-box",
                      }}
                    >
                      <Card className="overflow-hidden gap-4 w-full border-slate-200 shadow-none ">
                        <CardHeader className="pb-0">
                          <div className="flex justify-between items-start">
                            <div className="flex items-center gap-3">
                              <div className="bg-muted rounded-md p-1 size-8 ">{getRunnerIcon(tool.runtime)}</div>

                              <CardTitle className="text-lg">{tool.name}</CardTitle>
                              {tool.installed && (
                                <Badge variant="outline" className="ml-auto">
                                  Installed
                                </Badge>
                              )}
                            </div>
                          </div>
                        </CardHeader>
                        <CardContent>
                          <CardDescription className="line-clamp-2">{tool.description}</CardDescription>
                        </CardContent>
                        <CardFooter className="pt-0 flex justify-between items-center">
                          {tool.publisher && (
                            <div className="flex items-center gap-1 text-sm text-muted-foreground">
                              <span>By </span>{" "}
                              <a
                                href={tool.publisher.url}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="underline text-foreground"
                              >
                                {tool.publisher.name}
                              </a>
                            </div>
                          )}
                          {tool.installed ? (
                            <Button
                              variant="destructive"
                              onClick={() => uninstallTool(tool.id)}
                              disabled={uninstalling === tool.id}
                              type="button"
                            >
                              {uninstalling === tool.id ? "Uninstalling..." : "Uninstall"}
                            </Button>
                          ) : (
                            <Button
                              variant="outline"
                              type="button"
                              onClick={() => !tool.installed && installTool(tool)}
                              disabled={tool.installed || installing === tool.id}
                            >
                              {tool.installed ? "Installed" : installing === tool.id ? "Installing..." : "Install"}
                            </Button>
                          )}
                        </CardFooter>
                      </Card>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default Registry;
