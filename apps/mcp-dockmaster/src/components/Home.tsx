import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Home.css";

import dockerIcon from "../assets/docker.svg";
import nodeIcon from "../assets/node.svg";
import pythonIcon from "../assets/python.svg";

import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@radix-ui/react-collapsible";
import { Button } from "./ui/button";
import {
  ArrowRight,
  ChevronDown,
  ChevronUp,
  Copy,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { toast } from "sonner";
import { Separator } from "./ui/separator";
import { Badge } from "./ui/badge";
import { cn } from "@/lib/utils";

interface PrerequisiteStatus {
  name: string;
  installed: boolean;
  loading: boolean;
  icon: string;
}

const mcpClientProxy = {
  claude: {
    mcpServers: {
      "mcp-dockmaster": {
        args: [
          "/path/to/mcp_dockmaster/apps/mcp-runner/build/index.js",
          "--stdio",
        ],
        command: "node",
      },
    },
  },
  cursor: {
    mcpServers: {
      "mcp-dockmaster": {
        args: [
          "/path/to/mcp_dockmaster/apps/mcp-runner/build/index.js",
          "--stdio",
        ],
        command: "node",
      },
    },
  },
};

const Home: React.FC = () => {
  const [prerequisites, setPrerequisites] = useState<PrerequisiteStatus[]>([
    { name: "Node.js", installed: false, loading: true, icon: nodeIcon },
    { name: "UV (Python)", installed: false, loading: true, icon: pythonIcon },
    { name: "Docker", installed: false, loading: true, icon: dockerIcon },
  ]);
  const [isChecking, setIsChecking] = useState(false);
  const [showMCPConfig, setShowMCPConfig] = useState(false);

  const checkPrerequisites = async () => {
    setIsChecking(true);
    setPrerequisites((prev) =>
      prev.map((item) => ({ ...item, loading: true })),
    );

    try {
      // Check if Node.js is installed
      const checkNode = async () => {
        try {
          const installed = await invoke<boolean>("check_node_installed");
          return installed;
        } catch (error) {
          console.error("Failed to check Node.js:", error);
          return false;
        }
      };

      // Check if uv is installed
      const checkUv = async () => {
        try {
          const installed = await invoke<boolean>("check_uv_installed");
          return installed;
        } catch (error) {
          console.error("Failed to check uv:", error);
          return false;
        }
      };

      // Check if Docker is installed
      const checkDocker = async () => {
        try {
          const installed = await invoke<boolean>("check_docker_installed");
          return installed;
        } catch (error) {
          console.error("Failed to check Docker:", error);
          return false;
        }
      };

      const [nodeInstalled, uvInstalled, dockerInstalled] = await Promise.all([
        checkNode(),
        checkUv(),
        checkDocker(),
      ]);

      setPrerequisites([
        {
          name: "Node.js",
          installed: nodeInstalled,
          loading: false,
          icon: nodeIcon,
        },
        {
          name: "UV (Python)",
          installed: uvInstalled,
          loading: false,
          icon: pythonIcon,
        },
        {
          name: "Docker",
          installed: dockerInstalled,
          loading: false,
          icon: dockerIcon,
        },
      ]);
    } catch (error) {
      console.error("Failed to check prerequisites:", error);
      setPrerequisites((prev) =>
        prev.map((item) => ({ ...item, loading: false })),
      );
    } finally {
      setIsChecking(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        toast.success("Configuration copied to clipboard!");
      })
      .catch((err) => {
        console.error("Failed to copy text: ", err);
        toast.error("Failed to copy text to clipboard");
      });
  };

  useEffect(() => {
    checkPrerequisites();
  }, []);

  return (
    <div className="mx-auto flex h-full w-full max-w-4xl flex-col gap-8 px-6 py-10">
      <div className="flex flex-col space-y-1.5">
        <h1 className="text-2xl font-semibold tracking-tight">
          Welcome to MPC Dockmaster
        </h1>
        <p className="text-muted-foreground text-sm">
          Select an option from the sidebar to get started.
        </p>
      </div>
      <div className="space-y-2">
        <h2 className="text-lg font-medium">Integrate with MCP Clients</h2>
        <p className="text-muted-foreground text-sm">
          Using the proxy tool, you will be able to integrate with MCP clients
          like Claude offering all the tools you configure in MPC Dockmaster.
        </p>

        <Collapsible
          className="mt-4 space-y-2"
          open={showMCPConfig}
          onOpenChange={setShowMCPConfig}
        >
          <CollapsibleTrigger asChild>
            <Button
              variant="outline"
              className="flex w-full items-center justify-between gap-2 shadow-none"
            >
              <span className="flex items-center gap-2">
                <ArrowRight className="h-4 w-4" />
                Show MCP Configuration
              </span>
              {showMCPConfig ? (
                <ChevronUp className="text-muted-foreground h-4 w-4 shrink-0" />
              ) : (
                <ChevronDown className="text-muted-foreground h-4 w-4 shrink-0" />
              )}
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="bg-muted/30 space-y-8 rounded-md border p-4">
            <div className="space-y-4">
              <div>
                <h4 className="font-semibold">Claude Configuration</h4>
                <p className="text-muted-foreground text-sm">
                  Use this configuration to connect Claude to your MCP servers:
                </p>
              </div>
              <div className="relative">
                <pre className="max-h-[300px] overflow-auto rounded-md bg-black p-4 text-sm text-white">
                  <code>{JSON.stringify(mcpClientProxy.claude, null, 2)}</code>
                </pre>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-muted-foreground absolute top-2 right-2 h-8 w-8 cursor-pointer p-0"
                  onClick={() =>
                    copyToClipboard(
                      JSON.stringify(mcpClientProxy.claude, null, 2),
                    )
                  }
                >
                  <Copy className="h-4 w-4" />
                  <span className="sr-only">Copy code</span>
                </Button>
              </div>
            </div>
            <Separator />
            <div className="space-y-4">
              <div>
                <h4 className="font-semibold">Cursor Configuration</h4>
                <p className="text-muted-foreground text-sm">
                  Use this configuration to connect Cursor to your MCP servers:
                </p>
              </div>
              <div className="relative">
                <pre className="max-h-[300px] overflow-auto rounded-md bg-black p-4 text-sm text-white">
                  <code>{JSON.stringify(mcpClientProxy.cursor, null, 2)}</code>
                </pre>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-muted-foreground absolute top-2 right-2 h-8 w-8 cursor-pointer p-0"
                  onClick={() =>
                    copyToClipboard(
                      JSON.stringify(mcpClientProxy.cursor, null, 2),
                    )
                  }
                >
                  <Copy className="h-4 w-4" />
                  <span className="sr-only">Copy code</span>
                </Button>
              </div>
            </div>
          </CollapsibleContent>
        </Collapsible>
      </div>
      <div className="space-y-4">
        <div className="flex flex-row items-center justify-between">
          <h2 className="text-lg font-medium">Runner Environment Support</h2>
          <Button
            disabled={isChecking}
            variant="outline"
            size="sm"
            className="flex items-center gap-2"
            onClick={checkPrerequisites}
          >
            {isChecking ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4" />
            )}
            {isChecking ? "Checking..." : "Refresh"}
          </Button>
        </div>
        <div className="space-y-4">
          <div className="grid gap-4">
            {prerequisites.map((prerequisite) => (
              <div
                key={prerequisite.name}
                className="hover:bg-muted/10 flex items-center justify-between rounded-lg border p-4 transition-colors"
              >
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "flex h-10 w-10 items-center justify-center rounded-full",
                      prerequisite.installed && "bg-green-500/10",
                      !prerequisite.installed && "bg-red-500/10",
                    )}
                  >
                    <img
                      src={prerequisite.icon}
                      alt={prerequisite.name}
                      className="h-5 w-5"
                    />
                  </div>
                  <div>
                    <p className="font-medium">{prerequisite.name}</p>
                    <p className="text-muted-foreground text-sm">
                      {prerequisite.installed
                        ? "Installed and running"
                        : "Not installed or not running"}
                    </p>
                  </div>
                </div>
                {prerequisite.loading ? (
                  <div className="flex items-center gap-2">
                    <span className="loading-indicator">Checking...</span>
                    <Loader2 className="h-4 w-4 animate-spin" />
                  </div>
                ) : (
                  <span className="status-indicator">
                    {prerequisite.installed ? (
                      <Badge className="bg-green-500 text-white hover:bg-green-600">
                        Active
                      </Badge>
                    ) : (
                      <Badge
                        variant="outline"
                        className="border-red-500 bg-red-500/10 text-red-500"
                      >
                        Inactive
                      </Badge>
                    )}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Home;
