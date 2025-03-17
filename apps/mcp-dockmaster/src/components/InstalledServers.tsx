import React, { useState, useEffect } from "react";
import MCPClient, { RuntimeServer, ServerToolInfo, RuntimeEnvConfig } from "../lib/mcpClient";
import { 
  dispatchServerStatusChanged, 
  dispatchServerUninstalled, 
  SERVER_STATUS_CHANGED, 
  SERVER_UNINSTALLED 
} from "../lib/events";
import "./InstalledServers.css";
import { ChevronDown, ChevronRight, Info, Settings } from "lucide-react";
import { CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Card } from "./ui/card";
import { cn } from "@/lib/utils";
import { Switch } from "./ui/switch";
import { Button } from "./ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { Badge } from "./ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Label } from "./ui/label";

// Add a simple notification component
interface NotificationProps {
  message: string;
  type: "success" | "error" | "info";
  onClose: () => void;
}

const Notification: React.FC<NotificationProps> = ({
  message,
  type,
  onClose,
}) => {
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

const InstalledServers: React.FC = () => {
  const [servers, setServers] = useState<RuntimeServer[]>([]);
  const [serverTools, setServerTools] = useState<ServerToolInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedServerId, setExpandedServerId] = useState<string | null>(null);
  const [envVarValues, setEnvVarValues] = useState<Record<string, string>>({});
  const [savingConfig, setSavingConfig] = useState(false);
  const [configPopupVisible, setConfigPopupVisible] = useState(false);
  const [transitioningServers, setTransitioningServers] = useState<Set<string>>(new Set());
  const [areToolsPaused, setAreToolsPaused] = useState(false);
  const [currentConfigTool, setCurrentConfigTool] =
    useState<RuntimeServer | null>(null);
  const [infoPopupVisible, setInfoPopupVisible] = useState(false);
  const [currentInfoServer, setCurrentInfoServer] = useState<RuntimeServer | null>(null);
  const [envOperationInProgress, setEnvOperationInProgress] = useState(false);
  const [notifications, setNotifications] = useState<
    Array<{ id: string; message: string; type: "success" | "error" | "info" }>
  >([]);

  useEffect(() => {
    loadData();

    // Create a named function for the event listener so we can remove it properly
    const handleFocus = () => {
      if (!envOperationInProgress) {
        loadData();
      }
    };

    // Reload when the window regains focus, but skip if ENV operation is in progress
    window.addEventListener("focus", handleFocus);

    return () => {
      window.removeEventListener("focus", handleFocus);
    };
  }, [envOperationInProgress]);

  // Add auto-refresh feature that runs every 2 seconds but only when server status and enabled state don't match
  useEffect(() => {
    // Helper function to check if any server needs refresh based on status/enabled mismatch
    const checkServersNeedRefresh = () => {
      // Skip refresh if ENV operation is in progress
      if (envOperationInProgress) {
        return;
      }
      
      // Check if any server has a mismatch between status and enabled state
      const serversNeedingRefresh = servers.filter(server => 
        (server.status !== 'running' && server.enabled) || // Not running but should be running
        (server.status !== 'stopped' && !server.enabled)   // Not stopped but should be stopped
      );
      
      if (serversNeedingRefresh.length > 0) {
        // Only refresh servers that need it
        MCPClient.listServers().then(newServers => {
          setServers(prevServers => {
            // Create a map of servers needing refresh
            const refreshIds = new Set(serversNeedingRefresh.map(s => s.id));
            
            // Clear transitioning state for refreshed servers
            setTransitioningServers(prev => {
              const newSet = new Set([...prev]);
              refreshIds.forEach(id => newSet.delete(id));
              return newSet;
            });
            
            // Update only servers that need refresh
            return prevServers.map(server => {
              if (refreshIds.has(server.id)) {
                const updatedServer = newServers.find(s => s.id === server.id);
                return updatedServer || server;
              }
              return server;
            }).sort((a, b) => a.id.localeCompare(b.id)); // Maintain consistent order
          });
        }).catch(error => {
          console.error("Failed to refresh servers:", error);
        });
      }
    };
    
    // Set up interval to check for refresh every 2 seconds
    const intervalId = setInterval(checkServersNeedRefresh, 2000);
    
    // Clean up interval on component unmount
    return () => {
      clearInterval(intervalId);
    };
  }, [servers]);

  // Effect to handle expanded tool changes
  useEffect(() => {
    if (expandedServerId) {
      console.log("Tool expanded:", expandedServerId);
      // Find the tool that was expanded
      const tool = serverTools.find((t) => t.server_id === expandedServerId);
      if (tool?.server_id) {
        console.log("Expanded tool has server_id:", tool.server_id);
        // Instead of calling loadData which would trigger another render,
        // just ensure we have the server tools for this server
        const server = servers.find((s) => s.id === tool.server_id);
        if (server && server.status === 'running') {
          discoverToolsForServer(tool.server_id);
        }
      }
    }
  }, [expandedServerId]);
  
  // Effect to reload data when tools visibility state changes
  useEffect(() => {
    console.log("Tools visibility changed:", areToolsPaused ? "Paused" : "Visible");
    loadData();
  }, [areToolsPaused]);

  // Add event listeners for tool status changes
  useEffect(() => {
    const handleServerStatusChanged = (event: CustomEvent) => {
      const { serverId } = event.detail;
      console.log("Server status changed:", serverId);

      // If serverId is 'all', refresh all data
      if (serverId === "all") {
        loadData();
        return;
      }

      // Otherwise, just refresh the specific server
      const server = servers.find((s) => s.id === serverId);
      if (server) {
        // Get updated server data
        MCPClient.listServers().then(newServers => {
          const updatedServer = newServers.find(s => s.id === serverId);
          if (updatedServer) {
            // Update only the changed server
            setServers(prevServers => 
              prevServers.map(s => 
                s.id === serverId ? updatedServer : s
              )
            );
          }
        }).catch(error => {
          console.error("Failed to update server status:", error);
        });
      }
    };

    // Add this new handler
    const handleServerUninstalled = (event: CustomEvent) => {
      const { toolId } = event.detail;
      console.log("Server uninstalled:", toolId);
      
      // Remove the uninstalled server from the list
      setServers(prevServers => 
        prevServers.filter(server => server.id !== toolId)
      );
    };

    document.addEventListener(
      SERVER_STATUS_CHANGED,
      handleServerStatusChanged as EventListener,
    );
    
    // Add this new event listener
    document.addEventListener(
      SERVER_UNINSTALLED,
      handleServerUninstalled as EventListener,
    );

    return () => {
      document.removeEventListener(
        SERVER_STATUS_CHANGED,
        handleServerStatusChanged as EventListener,
      );
      
      // Remove the new event listener
      document.removeEventListener(
        SERVER_UNINSTALLED,
        handleServerUninstalled as EventListener,
      );
    };
  }, [servers]);

  const loadData = async () => {
    setLoading(true);
    // Clear transitioning servers when loading new data
    setTransitioningServers(new Set());
    try {
      // Get servers and tools data separately
      const newServers = await MCPClient.listServers();
      const allServerTools = await MCPClient.listAllServerTools();
      
      // Update servers using a diff-based approach
      setServers(prevServers => {
        // Create a map of existing servers by ID for quick lookup
        const existingServersMap = new Map(
          prevServers.map(server => [server.id, server])
        );
        
        // Process new servers and update only what changed
        const updatedServers = newServers.map(newServer => {
          const existingServer = existingServersMap.get(newServer.id);
          
          // If server doesn't exist or has changes, use the new server data
          if (!existingServer || 
              existingServer.status !== newServer.status || 
              existingServer.enabled !== newServer.enabled ||
              existingServer.tool_count !== newServer.tool_count) {
            return newServer;
          }
          
          // Otherwise, keep the existing server to prevent unnecessary re-renders
          return existingServer;
        });
        
        // Sort servers to maintain consistent order
        return [...updatedServers].sort((a, b) => a.id.localeCompare(b.id));
      });
      
      // Set server tools
      setServerTools(allServerTools);
    } catch (error) {
      console.error("Failed to load data:", error);
    } finally {
      setLoading(false);
    }
  };

  const uninstallServer = async (id: string) => {
    try {
      // Update the UI optimistically
      setServers((prev) => prev.filter((server) => server.id !== id));
      
      // Call the backend API to uninstall the server
      const response = await MCPClient.uninstallServer({
        server_id: id,
      });
      
      if (response.success) {
        // Dispatch event that a server was uninstalled
        dispatchServerUninstalled(id);
        addNotification(`Server uninstalled successfully`, "success");
      } else {
        // If the API call fails, revert the UI change
        console.error("Failed to uninstall server:", response.message);
        addNotification(`Failed to uninstall server: ${response.message}`, "error");
        // Refresh the list to ensure UI is in sync with backend
        loadData();
      }
    } catch (error) {
      console.error("Error uninstalling server:", error);
      addNotification("Error uninstalling server", "error");
      // Refresh the list to ensure UI is in sync with backend
      loadData();
    }
  };

  const toggleToolStatus = async (id: string) => {
    try {
      // Find the current server to get its current enabled state
      const server = servers.find(server => server.id === id);
      if (!server) return;

      // Mark this server as transitioning
      setTransitioningServers(prev => new Set([...prev, id]));

      // Update the UI optimistically
      setServers(prev =>
        prev.map(server =>
          server.id === id ? { ...server, enabled: !server.enabled } : server,
        ),
      );

      // Call the backend API to update the server status
      const response = await MCPClient.updateServerStatus({
        server_id: id,
        enabled: !server.enabled,
      });

      if (response.success) {
        // Dispatch event that a server's status was changed
        dispatchServerStatusChanged(id);
      } else {
        // If the API call fails, revert the UI change
        console.error("Failed to update server status:", response.message);
        setServers(prev =>
          prev.map(server =>
            server.id === id ? { ...server, enabled: server.enabled } : server,
          ),
        );
        
        // Remove from transitioning servers
        setTransitioningServers(prev => {
          const newSet = new Set([...prev]);
          newSet.delete(id);
          return newSet;
        });
      }
    } catch (error) {
      console.error("Error toggling server status:", error);
      // Refresh the list to ensure UI is in sync with backend
      loadData();
      
      // Clear all transitioning servers on error
      setTransitioningServers(new Set());
    }
  };

  const discoverToolsForServer = async (
    serverId: string,
    e?: React.MouseEvent,
  ) => {
    if (e) {
      e.stopPropagation(); // Prevent the click from toggling the expanded state
    }

    try {
      const tools = await MCPClient.discoverTools({ server_id: serverId });
      console.log("Tools discovered:", tools);
      
      // Update only the server tools without reloading all servers
      // This prevents unnecessary reordering of servers
      const allServerTools = await MCPClient.listAllServerTools();
      setServerTools(allServerTools);
    } catch (error) {
      console.error(`Failed to discover tools for server ${serverId}:`, error);
    }
  };

  const startEditingEnvVars = (serverId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state

    // Find the server to get its current env vars
    const server = servers.find((s) => s.id === serverId);
    if (!server || !server.configuration?.env) return;

    // Initialize the env var values with current values
    const initialValues: Record<string, string> = {};
    Object.entries(server.configuration.env || {}).forEach(([key, value]) => {
      // If the value is an object with a description, it might not have a value yet
      if (typeof value === "object" && value !== null) {
        initialValues[key] = (value as RuntimeEnvConfig).default || "";
      } else {
        initialValues[key] = String(value);
      }
    });

    setEnvVarValues(initialValues);
    setCurrentConfigTool(server);
    setConfigPopupVisible(true);
  };

  const handleEnvVarChange = (key: string, value: string) => {
    setEnvVarValues((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const addNotification = (
    message: string,
    type: "success" | "error" | "info",
  ) => {
    const id = Date.now().toString();
    setNotifications((prev) => [...prev, { id, message, type }]);
  };

  const removeNotification = (id: string) => {
    setNotifications((prev) =>
      prev.filter((notification) => notification.id !== id),
    );
  };

  const saveEnvVars = async (serverId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state
    setSavingConfig(true);
    setEnvOperationInProgress(true); // Set flag to prevent auto-refresh

    try {
      // Find the server to update
      const server = servers.find((s) => s.id === serverId);
      if (!server) {
        console.error(`Server with ID ${serverId} not found`);
        addNotification(`Server with ID ${serverId} not found`, "error");
        return;
      }

      console.log(`Updating configuration for server: ${serverId}`, envVarValues);

      // Convert envVarValues to a flat key-value object for the API
      const flatConfig: Record<string, string> = {};
      Object.entries(envVarValues).forEach(([key, value]) => {
        flatConfig[key] = value;
      });

      // Update the server configuration
      const response = await MCPClient.updateServerConfig({
        server_id: serverId,
        config: flatConfig,
      });

      if (response.success) {
        console.log(`Server ${serverId} configuration updated successfully`);
        addNotification(
          `Configuration for ${server.name} updated successfully`,
          "success",
        );

        // Update the server in the local state with the new env values
        setServers((prev) =>
          prev.map((s) =>
            s.id === serverId
              ? {
                  ...s,
                  configuration: {
                    ...s.configuration,
                    env: Object.entries(envVarValues).reduce((acc, [key, value]) => {
                      acc[key] = {
                        default: value,
                        description: s.configuration?.env?.[key]?.description || "",
                        required: s.configuration?.env?.[key]?.required || false,
                      };
                      return acc;
                    }, {} as Record<string, RuntimeEnvConfig>),
                  },
                }
              : s,
          ),
        );

        // Restart the server with the new configuration if it's enabled
        if (server.enabled) {
          console.log(
            `Server ${serverId} is enabled, restarting with new configuration...`,
          );
          addNotification(`Restarting ${server.name}...`, "info");

          try {
            const restartResponse = await MCPClient.restartTool(serverId);
            if (restartResponse.success) {
              console.log(
                `Server ${serverId} restarted successfully with new configuration`,
              );
              addNotification(
                `${server.name} restarted successfully with new configuration`,
                "success",
              );
              // Dispatch event to update UI
              dispatchServerStatusChanged(serverId);
            } else {
              console.error(
                `Failed to restart server ${serverId}:`,
                restartResponse.message,
              );
              addNotification(
                `Failed to restart ${server.name}: ${restartResponse.message}`,
                "error",
              );
            }
          } catch (restartError) {
            console.error(`Error restarting server ${serverId}:`, restartError);
            addNotification(`Error restarting ${server.name}`, "error");
          }
        } else {
          console.log(`Server ${serverId} is disabled, not restarting`);
        }

        // Close the popup
        setConfigPopupVisible(false);
        setCurrentConfigTool(null);
      } else {
        console.error("Failed to update server configuration:", response.message);
        addNotification(
          `Failed to update configuration: ${response.message}`,
          "error",
        );
      }
    } catch (error) {
      console.error("Error updating server configuration:", error);
      addNotification("Error updating configuration", "error");
    } finally {
      setSavingConfig(false);
      setEnvOperationInProgress(false); // Reset flag to allow auto-refresh again
    }
  };

  const cancelEditingEnvVars = (e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state
    setConfigPopupVisible(false);
    setCurrentConfigTool(null);
    setEnvVarValues({});
    
    // Force a data reload when closing the ENV popup to ensure we have the latest server states
    loadData();
  };
  
  const openInfoPopup = (server: RuntimeServer, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent the click from toggling the expanded state
    setCurrentInfoServer(server);
    setInfoPopupVisible(true);
    
    // If the server is running, discover tools for it
    if (server.status === 'running') {
      discoverToolsForServer(server.id);
    }
  };
  
  const closeInfoPopup = () => {
    setInfoPopupVisible(false);
    setCurrentInfoServer(null);
  };

  const toggleExpandTool = (serverId: string, e: React.MouseEvent) => {
    // Don't toggle if clicking on status toggle
    const target = e.target as HTMLElement;
    if (target.closest(".app-status-indicator")) {
      return;
    }

    console.log("Toggling tool expansion for:", serverId);

    // If the tool is already expanded, collapse it
    if (expandedServerId === serverId) {
      console.log("Collapsing tool");
      setExpandedServerId(null);
    } else {
      console.log("Expanding tool");
      setExpandedServerId(serverId);

      // If the tool has a server_id and the server is running, refresh the tools
      const server = servers.find((s) => s.id === serverId);
      console.log("Server found:", server);

      if (server?.status === 'running') {
        console.log("Server is running, discovering tools");
        discoverToolsForServer(server.id);
      } else {
        console.log("Server is not running, no tools to discover");
      }
    }
  };

  const renderServerInfo = (server: RuntimeServer) => {
    // Only show server info when the tool is expanded
    if (expandedServerId !== server.id) {
      return null;
    }

    // Find tools for this server
    const toolsForServer = serverTools.filter((t) => t.server_id === server.id);

    return (
      <div className="server-info-container">
        <div className="server-header">
          <h4 className="!text-sm font-medium">Server Information</h4>
          <div className="server-status-badge">
            <Badge
              variant="outline"
              className={
                server.status === 'running'
                  ? "bg-emerald-500 text-white"
                  : server.status === 'starting'
                  ? "bg-yellow-500 text-white"
                  : server.status.startsWith("Error:")
                  ? "bg-red-500 text-white"
                  : "bg-red-500 text-white"
              }
            >
              {server.status === 'running' 
                ? "Running" 
                : server.status === 'stopped' 
                ? "Stopped" 
                : server.status === 'starting' 
                ? "Starting" 
                : server.status.startsWith("Error:") 
                ? server.status 
                : "Stopped"}
            </Badge>
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
            <h5 className="!text-sm font-medium">Available Tools</h5>
            <div className="server-tools-grid">
              {toolsForServer.map((tool) => (
                <div key={tool.proxy_id} className="server-tool-card gap-1">
                  <h6 className="!mb-0 !text-sm font-medium">{tool.name}</h6>
                  <p className="!text-xs">{tool.description}</p>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="empty-tools-message">
            <p>
              {server.status === 'running'
                ? hasUnsetRequiredEnvVars(server)
                  ? <span className="warning-text">This server requires you to set up certain environment variable(s). Without these settings, the list of tools may not appear.</span>
                  : 'No tools discovered from this server yet. Click "Refresh Tools" to discover available tools.'
                : server.status === 'starting'
                ? "Server is starting. Please wait..."
                : server.status.startsWith("Error:")
                ? <span className="error-text">Server error: {server.status.substring(7)}</span>
                : "Server is not running. Start the server to discover available tools."}
            </p>
          </div>
        )}
      </div>
    );
  };

  // Configuration popup component
  const renderInfoPopup = () => {
    if (!infoPopupVisible || !currentInfoServer) return null;
    
    // Filter tools for this server
    const toolsForServer = serverTools.filter((t) => t.server_id === currentInfoServer.id);
    
    return (
      <Dialog open={infoPopupVisible} onOpenChange={closeInfoPopup}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle>{currentInfoServer.name} Information</DialogTitle>
            <DialogDescription>
              Details about this server and its tools.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4 max-h-[60vh] overflow-y-auto">
            {/* Basic Information */}
            <div className="space-y-2">
              <h3 className="text-sm font-medium">Basic Information</h3>
              <div className="rounded-md border p-3 space-y-2">
                {currentInfoServer.description && (
                  <div className="grid grid-cols-4 items-start gap-4">
                    <Label className="text-right text-xs pt-1">Description</Label>
                    <div className="col-span-3 text-sm">{currentInfoServer.description}</div>
                  </div>
                )}
                <div className="grid grid-cols-4 items-start gap-4">
                  <Label className="text-right text-xs pt-1">ID</Label>
                  <div className="col-span-3 text-sm font-mono">{currentInfoServer.id}</div>
                </div>
                <div className="grid grid-cols-4 items-start gap-4">
                  <Label className="text-right text-xs pt-1">Status</Label>
                  <div className="col-span-3">
                    {currentInfoServer.status === 'running' ? (
                      <span className="text-green-500 text-sm">Running</span>
                    ) : (
                      <span className="text-red-500 text-sm">Stopped</span>
                    )}
                  </div>
                </div>
                <div className="grid grid-cols-4 items-start gap-4">
                  <Label className="text-right text-xs pt-1">Enabled</Label>
                  <div className="col-span-3 text-sm">
                    {currentInfoServer.enabled ? "Yes" : "No"}
                  </div>
                </div>
                <div className="grid grid-cols-4 items-start gap-4">
                  <Label className="text-right text-xs pt-1">Tools</Label>
                  <div className="col-span-3 text-sm">{currentInfoServer.tool_count} tools available</div>
                </div>
                <div className="grid grid-cols-4 items-start gap-4">
                  <Label className="text-right text-xs pt-1">Tools Type</Label>
                  <div className="col-span-3 text-sm">{currentInfoServer.tools_type}</div>
                </div>
                {currentInfoServer.sourceUrl && (
                  <div className="grid grid-cols-4 items-start gap-4">
                    <Label className="text-right text-xs pt-1">Source URL</Label>
                    <div className="col-span-3">
                      <a 
                        href={currentInfoServer.sourceUrl} 
                        target="_blank" 
                        rel="noopener noreferrer"
                        className="text-blue-500 hover:underline text-sm"
                      >
                        {currentInfoServer.sourceUrl}
                      </a>
                    </div>
                  </div>
                )}
                {currentInfoServer.entry_point && (
                  <div className="grid grid-cols-4 items-start gap-4">
                    <Label className="text-right text-xs pt-1">Entry Point</Label>
                    <div className="col-span-3 text-sm font-mono">{currentInfoServer.entry_point}</div>
                  </div>
                )}
              </div>
            </div>
            
            {/* Tools Section */}
            {toolsForServer.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Tools</h3>
                <div className="rounded-md border p-3 space-y-2">
                  {toolsForServer.map((tool) => (
                    <div key={tool.id}>
                      <div className="text-sm font-medium">{tool.name}</div>
                      <div className="text-sm font-small">{tool.description}</div>
                      {tool.inputSchema && (
                        <div className="text-xs font-mono">
                          {JSON.stringify(tool.inputSchema, null, 2)}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
            
            {/* Configuration */}
            {currentInfoServer.configuration && (
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Configuration</h3>
                <div className="rounded-md border p-3 space-y-2">
                  {currentInfoServer.configuration.command && (
                    <div className="grid grid-cols-4 items-start gap-4">
                      <Label className="text-right text-xs pt-1">Command</Label>
                      <div className="col-span-3 text-sm font-mono">{currentInfoServer.configuration.command}</div>
                    </div>
                  )}
                  {currentInfoServer.configuration.args && currentInfoServer.configuration.args.length > 0 && (
                    <div className="grid grid-cols-4 items-start gap-4">
                      <Label className="text-right text-xs pt-1">Arguments</Label>
                      <div className="col-span-3 text-sm font-mono">
                        {currentInfoServer.configuration.args.map((arg, index) => (
                          <div key={index}>{arg}</div>
                        ))}
                      </div>
                    </div>
                  )}
                  {currentInfoServer.configuration.env && Object.keys(currentInfoServer.configuration.env).length > 0 && (
                    <div className="grid grid-cols-4 items-start gap-4">
                      <Label className="text-right text-xs pt-1">Environment Variables</Label>
                      <div className="col-span-3 space-y-2">
                        {Object.entries(currentInfoServer.configuration.env).map(([key, value]) => (
                          <div key={key} className="text-sm">
                            <div className="font-medium">{key}</div>
                            {value.description && <div className="text-muted-foreground text-xs">{value.description}</div>}
                            <div className="text-xs mt-1">
                              {value.required ? 
                                <Badge variant="outline" className="bg-amber-100 text-amber-800 border-amber-300">Required</Badge> : 
                                <Badge variant="outline" className="bg-slate-100 text-slate-800 border-slate-300">Optional</Badge>
                              }
                              {value.default && <span className="ml-2">Default: <span className="font-mono">{value.default}</span></span>}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
            
            {/* Distribution */}
            {currentInfoServer.distribution && (
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Distribution</h3>
                <div className="rounded-md border p-3 space-y-2">
                  <div className="grid grid-cols-4 items-start gap-4">
                    <Label className="text-right text-xs pt-1">Type</Label>
                    <div className="col-span-3 text-sm">{currentInfoServer.distribution.type}</div>
                  </div>
                  <div className="grid grid-cols-4 items-start gap-4">
                    <Label className="text-right text-xs pt-1">Package</Label>
                    <div className="col-span-3">
                      {currentInfoServer.distribution.type === "npm" ? (
                        <a 
                          href={`https://www.npmjs.com/package/${currentInfoServer.distribution.package}`}
                          target="_blank" 
                          rel="noopener noreferrer"
                          className="text-blue-500 hover:underline text-sm font-mono"
                        >
                          {currentInfoServer.distribution.package}
                        </a>
                      ) : (
                        <span className="text-sm font-mono">{currentInfoServer.distribution.package}</span>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="destructive"
              onClick={() => {
                uninstallServer(currentInfoServer.id);
                closeInfoPopup();
              }}
            >
              Uninstall Server
            </Button>
            <Button variant="outline" onClick={closeInfoPopup}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  };

  const renderConfigPopup = () => {
    if (!configPopupVisible || !currentConfigTool) return null;

    const server = servers.find((s) => s.id === currentConfigTool.id);
    if (!server || !server.configuration?.env) return null;

    return (
      <div className="config-popup-overlay" onClick={cancelEditingEnvVars}>
        <div className="config-popup" onClick={(e) => e.stopPropagation()}>
          <div className="config-popup-header">
            <h3>Environment Variables - {server.name}</h3>
            <button
              className="close-popup-button"
              onClick={cancelEditingEnvVars}
            >
              ×
            </button>
          </div>

          <div className="config-popup-content">
            <div className="env-vars-editor">
              {Object.entries(server.configuration.env || {}).map(([key, value]) => {
                const description =
                  typeof value === "object" && value !== null
                    ? value.description
                    : "";
                // Get the default value if it exists in the object
                const defaultValue =
                  typeof value === "object" && value !== null
                    ? value.default
                    : value;

                return (
                  <div key={key} className="env-var-input-group">
                    <label htmlFor={`env-${key}`}>{key}</label>
                    <input
                      id={`env-${key}`}
                      type="text"
                      value={envVarValues[key] ?? defaultValue ?? ""}
                      onChange={(e) => handleEnvVarChange(key, e.target.value)}
                      placeholder={description || key}
                    />
                    {description && (
                      <div className="env-var-description">{description}</div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="config-popup-actions">
            <div className="flex justify-between w-full">
              <Button
                variant="destructive"
                onClick={(e) => {
                  e.stopPropagation();
                  uninstallServer(currentConfigTool.id);
                  cancelEditingEnvVars(e);
                }}
                disabled={savingConfig}
              >
                Uninstall
              </Button>
              <div className="flex gap-2">
                <button
                  className="save-env-vars-button"
                  onClick={(e) => saveEnvVars(currentConfigTool.id, e)}
                  disabled={savingConfig}
                >
                  {savingConfig ? "Saving..." : "Save"}
                </button>
                <button
                  className="cancel-env-vars-button"
                  onClick={cancelEditingEnvVars}
                  disabled={savingConfig}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  };

  // Helper function to check if a server has missing required ENV variables
  const hasUnsetRequiredEnvVars = (server: RuntimeServer): boolean => {
    if (!server.configuration?.env) return false;
    
    // Check if any required env vars are not set
    return Object.entries(server.configuration.env).some((entry) => {
      const value = entry[1];
      // If the env var is required and has no default value, it needs attention
      return value.required && 
        (!value.default || value.default.trim() === '');
    });
  };

  return (
    <div className="mx-auto flex h-full w-full max-w-4xl flex-col gap-8 px-6 py-10">
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
        <h1 className="text-2xl font-semibold tracking-tight">
          Servers Installed
        </h1>
        <p className="text-muted-foreground text-sm">
          Manage your installed AI applications and MCP tools.
        </p>
      </div>

      {loading ? (
        <div className="loading-message">Loading installed applications...</div>
      ) : servers.length === 0 ? (
        <div className="text-muted-foreground flex flex-col items-center justify-center gap-2 py-10">
          <p>You don&apos;t have any applications installed yet.</p>
          <p>Visit the AI App Store to discover and install applications.</p>
        </div>
      ) : (
        <div className="grid w-full grid-cols-2 gap-6">
          {servers.map((server) => (
            <Card
              key={server.id}
              className="gap-3 overflow-hidden border-slate-200 shadow-none"
            >
              <CardHeader className="">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-lg">{server.name}</CardTitle>
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "text-sm",
                          areToolsPaused
                            ? "text-slate-500"
                            : "text-emerald-600 dark:text-emerald-400",
                        )}
                      >
                        {areToolsPaused ? "Tools Hidden" : "Tools Visible"}
                      </span>
                      <Tooltip>
                        <TooltipTrigger>
                          <Switch
                            checked={!areToolsPaused}
                            onCheckedChange={(checked) => {
                              setAreToolsPaused(!checked);
                            }}
                            className="data-[state=checked]:bg-emerald-500"
                          />
                        </TooltipTrigger>
                        <TooltipContent>
                          {areToolsPaused ? "Show Tools" : "Hide Tools"}
                        </TooltipContent>
                      </Tooltip>
                    </div>
                    
                    <Tooltip>
                      <TooltipTrigger>
                        <Button
                          variant="ghost"
                          onClick={(e: React.MouseEvent) => {
                            openInfoPopup(server, e);
                          }}
                        >
                          <Info className="h-4 w-4" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>
                        Server Information
                      </TooltipContent>
                    </Tooltip>
                    
                    {server.configuration &&
                      server.configuration.env &&
                      Object.keys(server.configuration.env).length > 0 && (
                        <Tooltip>
                          <TooltipTrigger>
                            <Button
                              variant="ghost"
                              onClick={(e: React.MouseEvent) => {
                                e.stopPropagation();
                                startEditingEnvVars(server.id, e);
                              }}
                            >
                              <Settings className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>
                            Configure Environment Variables
                          </TooltipContent>
                        </Tooltip>
                      )}

                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "text-sm",
                          server.enabled
                            ? "text-emerald-600 dark:text-emerald-400"
                            : "text-slate-500",
                        )}
                      >
                        {server.enabled ? "Enabled" : "Disabled"}
                      </span>
                      <Switch
                        checked={server.enabled}
                        onCheckedChange={() => {
                          toggleToolStatus(server.id);
                        }}
                        className="data-[state=checked]:bg-emerald-500"
                      />
                    </div>
                  </div>
                </div>
              </CardHeader>

              <CardContent className="space-y-3 pb-0">
                <div className="flex items-center justify-between">
                  <CardDescription className="mt-1 line-clamp-2">
                    {server.description}
                  </CardDescription>
                  {hasUnsetRequiredEnvVars(server) && (
                    <Tooltip>
                      <TooltipTrigger>
                        <Badge 
                          variant="outline" 
                          className="bg-amber-100 text-amber-800 border-amber-300 ml-2"
                        >
                          Needs Attention
                        </Badge>
                      </TooltipTrigger>
                      <TooltipContent>
                        This server requires you to set up certain environment variable(s). Without these settings, the list of tools may not appear.
                      </TooltipContent>
                    </Tooltip>
                  )}
                </div>

                <div
                  className="server-status-indicator"
                  onClick={(e) => toggleExpandTool(server.id, e)}
                >
                  <div className="flex items-center gap-1">
                    <span
                      className={`server-status-dot ${
                        transitioningServers.has(server.id)
                          ? "transitioning"
                          : server.status === 'running' 
                            ? "running" 
                            : server.status === 'starting' 
                              ? "starting" 
                              : server.status.startsWith("Error:") 
                                ? "error" 
                                : "stopped"
                      }`}
                    ></span>
                    <span className="server-status-text">
                      Status: {transitioningServers.has(server.id)
                        ? "Updating..." 
                        : server.status === 'running' 
                          ? "Running" 
                          : server.status === 'stopped' 
                            ? "Stopped" 
                            : server.status === 'starting' 
                              ? "Starting..." 
                              : server.status.startsWith("Error:") 
                                ? server.status 
                                : "Stopped"}
                    </span>
                  </div>
                  <span className="flex items-center gap-1">
                    {expandedServerId === server.id ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                    {expandedServerId === server.id
                      ? "Hide details"
                      : "Show details"}
                  </span>
                </div>

                {renderServerInfo(server)}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {renderConfigPopup()}
      {renderInfoPopup()}
    </div>
  );
};

export default InstalledServers;
