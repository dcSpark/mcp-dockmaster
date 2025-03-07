import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dispatchToolStatusChanged } from "../lib/events";
import { useAppStore } from "@/store/app";
import { Progress } from "./ui/progress";

interface LoadingOverlayProps {
  children: React.ReactNode;
}

const LoadingOverlay: React.FC<LoadingOverlayProps> = ({ children }) => {
  const [progress, setProgress] = useState(0);

  const appState = useAppStore((state) => state.appState);
  const setAppState = useAppStore((state) => state.setAppState);

  useEffect(() => {
    setAppState("pending");
    // Set up a listener for the initialization complete event
    const unlisten = listen("mcp-initialization-complete", () => {
      console.log("Received initialization complete event");

      // Trigger a refresh of all tools
      dispatchToolStatusChanged("all");
      setAppState("ready");
    });

    // Poll for initialization status
    const interval = setInterval(async () => {
      try {
        const isComplete = await invoke<boolean>("check_initialization_complete");
        if (isComplete) {
          console.log("Initialization is complete");

          // Trigger a refresh of all tools
          dispatchToolStatusChanged("all");

          setAppState("ready");
          clearInterval(interval);
        } else {
          // Increment progress for visual feedback
          setProgress((prev) => Math.min(prev + 5, 90));
        }
      } catch (error) {
        console.error("Error checking initialization status:", error);
        setAppState("error");
      }
    }, 500);

    return () => {
      unlisten.then((fn) => fn());
      clearInterval(interval);
    };
  }, []);

  if (appState === "pending") {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <div className="bg-white rounded-lg  p-6 text-center">
          <h2 className="text-xl font-semibold text-gray-800">Initializing MCP Services</h2>
          <p className="mt-2 text-gray-600 text-sm">We're setting things up for you. Please hold on...</p>
          <div className="mt-4 w-full bg-gray-200 rounded-full h-2">
            <Progress value={progress} max={100} />
          </div>
          <p className="mt-2 text-sm text-gray-500">{progress}% completed</p>
        </div>
      </div>
    );
  }

  if (appState === "error") {
    return (
      <div className="flex items-center justify-center h-full w-full bg-red-100">
        <div className="bg-white rounded-lg p-6 text-center shadow-md">
          <h2 className="text-2xl font-semibold text-red-600">Error Initializing MCP Services</h2>
          <p className="mt-2 text-gray-700">Please check your MCP configuration and try again.</p>
        </div>
      </div>
    );
  }

  if (appState === "ready") {
    return children;
  }

  return null;
};

export default LoadingOverlay;
