import React, { useState, useEffect } from "react";
import MCPClient from "../lib/mcpClient";
import { dispatchToolStatusChanged, TOOL_STATUS_CHANGED } from "../lib/events";
import "./InstalledServers.css";
import { ChevronDown, ChevronRight } from "lucide-react";
import { CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./ui/card";
import { Card } from "./ui/card";
import { cn } from "@/lib/utils";
import { Switch } from "./ui/switch";
import { Button } from "./ui/button";
import { Settings, Settings2 } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { Dialog, DialogContent, DialogTrigger } from "./ui/dialog";

// Add a simple notification component
interface NotificationProps {
  message: string;
  type: "success" | "error" | "info";
  onClose: () => void;
}

const Notification: React.FC<NotificationProps> = ({ message, type, onClose }) => {
  useEffect(() => {
    const timer = setTimeout(() => {
      onClose();
    }, 5000);
    return () => clearTimeout(timer);
  }, [onClose]);

  return (
    <div className={`notification ${type}`}>
      {message}
      <button onClick={onClose} className="close-btn">
        ×
      </button>
    </div>
  );
};

interface InstalledTool {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  server_id?: string;
  process_running?: boolean;
  server_name?: string;
  config?: {
    env: Record<string, any>;
  };
}

interface Server {
  id: string;
  name: string;
  tool_count: number;
  enabled: boolean;
  process_running: boolean;
}

interface ServerTool {
  id: string;
  name: string;
  description: string;
  server_id: string;
  proxy_id: string;
}

const InstalledServers: React.FC = () => {
  const [installedTools, setInstalledTools] = useState<InstalledTool[]>([]);
  const [servers, setServers] = useState<Server[]>([]);
  const [serverTools, setServerTools] = useState<ServerTool[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedToolId, setExpandedToolId] = useState<string | null>(null);
  const [envVarValues, setEnvVarValues] = useState<Record<string, string>>({});
  const [savingConfig, setSavingConfig] = useState(false);
  const [configPopupVisible, setConfigPopupVisible] = useState(false);
  const [currentConfigTool, setCurrentConfigTool] = useState<InstalledTool | null>(null);
  const [notifications, setNotifications] = useState<
    Array<{ id: string; message: string; type: "success" | "error" | "info" }>
  >([]);

  useEffect(() => {
    loadData();

    // Reload when the window regains focus
    window.addEventListener("focus", loadData);

    return () => {
      window.removeEventListener("focus", loadData);
    };
  }, []);

  // Effect to handle expanded tool changes
  useEffect(() => {
    if (expandedToolId) {
      console.log("Tool expanded:", expandedToolId);
      // Find the tool that was expanded
      const tool = installedTools.find((t) => t.id === expandedToolId);
      if (tool?.server_id) {
        console.log("Expanded tool has server_id:", tool.server_id);
        // Instead of calling loadData which would trigger another render,
        // just ensure we have the server tools for this server
        const server = servers.find((s) => s.id === tool.server_id);
        if (server && server.process_running) {
          discoverToolsForServer(tool.server_id);
        }
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [expandedToolId]);

  // Add event listeners for tool status changes
  useEffect(() => {
    const handleToolStatusChanged = (event: CustomEvent) => {
      const { toolId } = event.detail;
      console.log("Tool status changed:", toolId);

      // If toolId is 'all', refresh all data
      if (toolId === "all") {
        loadData();
        return;
      }

      // Otherwise, just refresh the specific tool
      const tool = installedTools.find((t) => t.id === toolId);
      if (tool) {
        loadData();
      }
    };

    document.addEventListener(TOOL_STATUS_CHANGED, handleToolStatusChanged as EventListener);

    return () => {
      document.removeEventListener(TOOL_STATUS_CHANGED, handleToolStatusChanged as EventListener);
    };
  }, [installedTools]);

  const loadData = async () => {
    setLoading(true);
    try {
      // Get all data from MCP client
      const allData = await MCPClient.getAllServerData();

      // Set servers (filtered to only active servers)
      const activeServers = allData.servers.filter((server: any) => server.process_running);
      setServers(activeServers);

      // Set server tools - make sure we have all tools from all servers
      const allServerTools = allData.tools || [];
      setServerTools(allServerTools);

      // Get installed tools
      const tools = await MCPClient.listTools();

      // Transform to our internal format with enabled state
      // Ensure we don't have duplicates by using a Map with tool name as key
      const toolsMap = new Map();

      tools.forEach((tool: any) => {
        const toolId = tool.id || `tool_${Math.random().toString(36).substr(2, 9)}_${Date.now()}`;
        // Find server info for this tool
        const serverTool = allServerTools.find((st: any) => st.server_id === tool.id);
        const server = serverTool ? activeServers.find((s: any) => s.id === serverTool.server_id) : null;

        // Only add if not already in the map (avoid duplicates)
        if (!toolsMap.has(tool.name.toLowerCase())) {
          toolsMap.set(tool.name.toLowerCase(), {
            id: toolId,
            name: tool.name,
            description: tool.description,
            enabled: tool.enabled !== undefined ? tool.enabled : true,
            server_id: serverTool?.server_id,
            server_name: server?.name,
            process_running: server?.process_running,
            config: tool.configuration || { env: {} },
          });
        }
      });

      // Convert map values to array
      const installedTools = Array.from(toolsMap.values());

      setInstalledTools(installedTools);
    } catch (error) {
      console.error("Failed to load data:", error);
    } finally {
      setLoading(false);
    }
  };

  const toggleToolStatus = async (id: string) => {
    try {
      // Find the current tool to get its current enabled state
      const tool = installedTools.find((tool) => tool.id === id);
      if (!tool) return;

      // Update the UI optimistically
      setInstalledTools((prev) => prev.map((tool) => (tool.id === id ? { ...tool, enabled: !tool.enabled } : tool)));

      // Call the backend API to update the tool status
      const response = await MCPClient.updateToolStatus({
        tool_id: id,
        enabled: !tool.enabled,
      });

      if (response.success) {
        // Dispatch event that a tool's status was changed
        dispatchToolStatusChanged(id);
      } else {
        // If the API call fails, revert the UI change
        console.error("Failed to update tool status:", response.message);
        setInstalledTools((prev) => prev.map((tool) => (tool.id === id ? { ...tool, enabled: tool.enabled } : tool)));
      }
    } catch (error) {
      console.error("Error toggling tool status:", error);
      // Refresh the list to ensure UI is in sync with backend
      loadData();
    }
  };

  const discoverToolsForServer = async (serverId: string, e?: React.MouseEvent) => {
    if (e) {
      e.stopPropagation(); // Prevent the click from toggling the expanded state
    }

    try {
      await MCPClient.discoverTools({ server_id: serverId });
      // Reload all data after discovery
      loadData();
    } catch (error) {
      console.error(`Failed to discover tools for server ${serverId}:`, error);
    }
  };

  const startEditingEnvVars = (toolId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state

    // Find the tool to get its current env vars
    const tool = installedTools.find((t) => t.id === toolId);
    if (!tool || !tool.config) return;

    // Initialize the env var values with current values
    const initialValues: Record<string, string> = {};
    Object.entries(tool.config.env).forEach(([key, value]) => {
      // If the value is an object with a description, it might not have a value yet
      if (typeof value === "object" && value !== null) {
        initialValues[key] = "";
      } else {
        initialValues[key] = String(value);
      }
    });

    setEnvVarValues(initialValues);
    setCurrentConfigTool(tool);
    setConfigPopupVisible(true);
  };

  const handleEnvVarChange = (key: string, value: string) => {
    setEnvVarValues((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const addNotification = (message: string, type: "success" | "error" | "info") => {
    const id = Date.now().toString();
    setNotifications((prev) => [...prev, { id, message, type }]);
  };

  const removeNotification = (id: string) => {
    setNotifications((prev) => prev.filter((notification) => notification.id !== id));
  };

  const saveEnvVars = async (toolId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state
    setSavingConfig(true);

    try {
      // Find the tool to update
      const tool = installedTools.find((t) => t.id === toolId);
      if (!tool) {
        console.error(`Tool with ID ${toolId} not found`);
        addNotification(`Tool with ID ${toolId} not found`, "error");
        return;
      }

      console.log(`Updating configuration for tool: ${toolId}`, envVarValues);

      // Update the tool configuration
      const response = await MCPClient.updateToolConfig({
        tool_id: toolId,
        config: {
          env: envVarValues,
        },
      });

      if (response.success) {
        console.log(`Tool ${toolId} configuration updated successfully`);
        addNotification(`Configuration for ${tool.name} updated successfully`, "success");

        // Update the tool in the local state
        setInstalledTools((prev) =>
          prev.map((t) =>
            t.id === toolId
              ? {
                  ...t,
                  config: {
                    ...t.config,
                    env: { ...envVarValues },
                  },
                }
              : t
          )
        );

        // Restart the tool with the new configuration if it's enabled
        if (tool.enabled) {
          console.log(`Tool ${toolId} is enabled, restarting with new configuration...`);
          addNotification(`Restarting ${tool.name}...`, "info");

          try {
            const restartResponse = await MCPClient.restartTool(toolId);
            if (restartResponse.success) {
              console.log(`Tool ${toolId} restarted successfully with new configuration`);
              addNotification(`${tool.name} restarted successfully with new configuration`, "success");
              // Dispatch event to update UI
              dispatchToolStatusChanged(toolId);
            } else {
              console.error(`Failed to restart tool ${toolId}:`, restartResponse.message);
              addNotification(`Failed to restart ${tool.name}: ${restartResponse.message}`, "error");
            }
          } catch (restartError) {
            console.error(`Error restarting tool ${toolId}:`, restartError);
            addNotification(`Error restarting ${tool.name}`, "error");
          }
        } else {
          console.log(`Tool ${toolId} is disabled, not restarting`);
        }

        // Close the popup
        setConfigPopupVisible(false);
        setCurrentConfigTool(null);
      } else {
        console.error("Failed to update tool configuration:", response.message);
        addNotification(`Failed to update configuration: ${response.message}`, "error");
      }
    } catch (error) {
      console.error("Error updating tool configuration:", error);
      addNotification("Error updating configuration", "error");
    } finally {
      setSavingConfig(false);
    }
  };

  const cancelEditingEnvVars = (e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state
    setConfigPopupVisible(false);
    setCurrentConfigTool(null);
    setEnvVarValues({});
  };

  const toggleExpandTool = (toolId: string, e: React.MouseEvent) => {
    // Don't toggle if clicking on status toggle
    const target = e.target as HTMLElement;
    if (target.closest(".app-status-indicator")) {
      return;
    }

    console.log("Toggling tool expansion for:", toolId);

    // If the tool is already expanded, collapse it
    if (expandedToolId === toolId) {
      console.log("Collapsing tool");
      setExpandedToolId(null);
    } else {
      console.log("Expanding tool");
      setExpandedToolId(toolId);

      // If the tool has a server_id and the server is running, refresh the tools
      const tool = installedTools.find((t) => t.id === toolId);
      console.log("Tool found:", tool);

      if (tool?.server_id) {
        console.log("Tool has server_id:", tool.server_id);
        // Force refresh tools for this server regardless of running state
        discoverToolsForServer(tool.server_id);
      } else {
        console.log("Tool does not have server_id");
      }
    }
  };

  const renderServerInfo = (tool: InstalledTool) => {
    // Only show server info when the tool is expanded
    if (expandedToolId !== tool.id) {
      return null;
    }

    // If the tool doesn't have a server_id, show a fallback message
    if (!tool.server_id) {
      return (
        <div className="server-info-container">
          <div className="empty-tools-message">
            <p>
              This application is not associated with an MCP server. It may be a standalone application or a built-in
              tool.
            </p>
          </div>
        </div>
      );
    }

    // Find the server for this tool
    const server = servers.find((s) => s.id === tool.server_id);
    if (!server) {
      return (
        <div className="server-info-container">
          <div className="empty-tools-message">
            <p>No server information available for this tool.</p>
          </div>
        </div>
      );
    }

    // Find tools for this server
    const toolsForServer = serverTools.filter((t) => t.server_id === server.id);

    return (
      <div className="server-info-container">
        <div className="server-header">
          <h4>Server Information</h4>
          <div className="server-status-badge">
            <span className={server.process_running ? "running" : "stopped"}>
              {server.process_running ? "Running" : "Stopped"}
            </span>
          </div>
          <button
            className="discover-button"
            onClick={(e) => {
              e.stopPropagation();
              discoverToolsForServer(server.id, e);
            }}
          >
            Refresh Tools
          </button>
        </div>
        {toolsForServer.length > 0 ? (
          <div className="server-tools">
            <h5>Available Tools</h5>
            <div className="server-tools-grid">
              {toolsForServer.map((tool) => (
                <div key={tool.proxy_id} className="server-tool-card">
                  <h6>{tool.name}</h6>
                  <p>{tool.description}</p>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="empty-tools-message">
            <p>
              {server.process_running
                ? 'No tools discovered from this server yet. Click "Refresh Tools" to discover available tools.'
                : "Server is not running. Start the server to discover available tools."}
            </p>
          </div>
        )}
      </div>
    );
  };

  // Configuration popup component
  const renderConfigPopup = () => {
    if (!configPopupVisible || !currentConfigTool) return null;

    const tool = installedTools.find((t) => t.id === currentConfigTool.id);
    if (!tool || !tool.config) return null;

    return (
      <div className="config-popup-overlay" onClick={cancelEditingEnvVars}>
        <div className="config-popup" onClick={(e) => e.stopPropagation()}>
          <div className="config-popup-header">
            <h3>Environment Variables - {tool.name}</h3>
            <button className="close-popup-button" onClick={cancelEditingEnvVars}>
              ×
            </button>
          </div>

          <div className="config-popup-content">
            <div className="env-vars-editor">
              {Object.entries(tool.config.env).map(([key, value]) => {
                const description = typeof value === "object" && value !== null ? value.description : "";

                return (
                  <div key={key} className="env-var-input-group">
                    <label htmlFor={`env-${key}`}>{key}</label>
                    <input
                      id={`env-${key}`}
                      type="text"
                      value={envVarValues[key] || ""}
                      onChange={(e) => handleEnvVarChange(key, e.target.value)}
                      placeholder={description || key}
                    />
                    {description && <div className="env-var-description">{description}</div>}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="config-popup-actions">
            <button
              className="save-env-vars-button"
              onClick={(e) => saveEnvVars(currentConfigTool.id, e)}
              disabled={savingConfig}
            >
              {savingConfig ? "Saving..." : "Save"}
            </button>
            <button className="cancel-env-vars-button" onClick={cancelEditingEnvVars} disabled={savingConfig}>
              Cancel
            </button>
          </div>
        </div>
      </div>
    );
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "Running":
        return "bg-emerald-500";
      case "Stopped":
        return "bg-amber-500";
      case "Error":
        return "bg-red-500";
      default:
        return "bg-slate-500";
    }
  };
  return (
    <div className="h-full px-6 flex flex-col gap-8 py-10 max-w-4xl mx-auto w-full">
      {/* Notifications */}
      <div className="notification-container">
        {notifications.map((notification) => (
          <Notification
            key={notification.id}
            message={notification.message}
            type={notification.type}
            onClose={() => removeNotification(notification.id)}
          />
        ))}
      </div>
      <div className="flex flex-col space-y-1.5">
        <h1 className="font-semibold tracking-tight text-2xl">My Applications</h1>
        <p className="text-sm text-muted-foreground">Manage your installed AI applications and MCP tools.</p>
      </div>

      {loading ? (
        <div className="loading-message">Loading installed applications...</div>
      ) : installedTools.length === 0 ? (
        <div className="py-10 flex flex-col gap-2 items-center justify-center text-muted-foreground">
          <p>You don't have any applications installed yet.</p>
          <p>Visit the AI App Store to discover and install applications.</p>
        </div>
      ) : (
        // <div className="tools-grid">
        //   {installedTools.map(tool => (
        //     <div
        //       key={tool.id}
        //       className={`tool-card ${tool.enabled ? 'enabled' : 'disabled'} ${expandedToolId === tool.id ? 'expanded' : ''}`}
        //     >
        //       <div className="tool-header">
        //         <h3 className="tool-title">{tool.name}</h3>
        //         <div className="tool-status">
        //           {tool.config && tool.config.env && Object.keys(tool.config.env).length > 0 && (
        //             <button
        //               className="config-button"
        //               onClick={(e) => {
        //                 e.stopPropagation();
        //                 startEditingEnvVars(tool.id, e);
        //               }}
        //               title="Configure Environment Variables"
        //             >
        //               <WrenchIcon />
        //             </button>
        //           )}
        //           <span
        //             className={`app-status-indicator ${tool.enabled ? 'active' : 'inactive'}`}
        //             onClick={(e) => {
        //               e.stopPropagation();
        //               toggleToolStatus(tool.id);
        //             }}
        //             role="switch"
        //             aria-checked={tool.enabled}
        //             tabIndex={0}
        //           ></span>
        //           <span>{tool.enabled ? 'Enabled' : 'Disabled'}</span>
        <div className="grid grid-cols-2 gap-6 w-full">
          {installedTools.map((tool) => (
            <Card className="overflow-hidden border-slate-200 shadow-none ">
              <CardHeader className="pb-3">
                <div className="flex justify-between items-start">
                  <CardTitle className="text-lg">{tool.name}</CardTitle>
                  <div className="flex items-center gap-2">
                    {tool.config && Object.keys(tool.config.env).length > 0 && (
                      <Tooltip>
                        <TooltipTrigger>
                          <Button
                            variant="ghost"
                            onClick={(e) => {
                              e.stopPropagation();
                              startEditingEnvVars(tool.id, e);
                            }}
                          >
                            <Settings className="w-4 h-4" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>Configure Environment Variables</TooltipContent>
                      </Tooltip>
                    )}

                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "text-sm",
                          tool.enabled ? "text-emerald-600 dark:text-emerald-400" : "text-slate-500"
                        )}
                      >
                        {tool.enabled ? "Enabled" : "Disabled"}
                      </span>
                      <Switch
                        checked={tool.enabled}
                        onCheckedChange={() => {
                          toggleToolStatus(tool.id);
                        }}
                        className="data-[state=checked]:bg-emerald-500"
                      />
                    </div>
                  </div>
                </div>
              </CardHeader>

              <CardContent className="pb-3 space-y-3">
                <CardDescription className="line-clamp-2 mt-1">{tool.description}</CardDescription>
                {/* {tool.server_id ? (
                  <div className="flex items-center gap-1.5">
                    <div
                      className={`h-2 w-2 rounded-full ${getStatusColor(tool.process_running ? "Running" : "Stopped")}`}
                    ></div>
                    <span className="text-sm">Status: {tool.process_running ? "Running" : "Stopped"}</span>
                  </div>
                ) : null} */}

                <div className="server-status-indicator" onClick={(e) => toggleExpandTool(tool.id, e)}>
                  {tool.server_id ? (
                    <div className="flex items-center gap-1">
                      <span className={`server-status-dot ${tool.process_running ? "running" : "stopped"}`}></span>
                      <span className="server-status-text">Status: {tool.process_running ? "Running" : "Stopped"}</span>
                    </div>
                  ) : (
                    <span className="server-status-text">Click to view details</span>
                  )}
                  <span className="flex items-center gap-1">
                    {expandedToolId === tool.id ? (
                      <ChevronDown className="w-4 h-4" />
                    ) : (
                      <ChevronRight className="w-4 h-4" />
                    )}
                    {expandedToolId === tool.id ? "Hide details" : "Show details"}
                  </span>
                </div>

                {renderServerInfo(tool)}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {renderConfigPopup()}
    </div>
  );
};

export default InstalledServers;
