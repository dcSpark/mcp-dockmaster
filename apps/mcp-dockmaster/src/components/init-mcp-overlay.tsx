import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { dispatchServerStatusChanged } from "../lib/events";
import { useAppStore } from "@/store/app";
import { Progress } from "./ui/progress";
import { TermsConsentDialog } from "./terms-consent-dialog";
import { getUserConsent } from "../lib/localStorage";
import { toast } from "sonner";

const INITIALIZATION_TOAST_ID = "MCP-INITIALIZATION-TOAST";

interface LoadingOverlayProps {
  children: React.ReactNode;
}

const InitMCPOverlay: React.FC<LoadingOverlayProps> = ({
  children,
}: {
  children: React.ReactNode;
}) => {
  const [progress, setProgress] = useState(0);
  const [showTerms, setShowTerms] = useState(false);

  const appState = useAppStore((state) => state.appState);
  const setAppState = useAppStore((state) => state.setAppState);
  const userConsented = useAppStore((state) => state.userConsented);
  const setUserConsented = useAppStore((state) => state.setUserConsented);

  useEffect(() => {
    setAppState("pending");
    const unlisten = listen("mcp-initialization-complete", () => {
      console.log("Received initialization complete event");

      dispatchServerStatusChanged("all");
      setAppState("ready");
      toast.success("MCP services initialized!", {
        id: INITIALIZATION_TOAST_ID,
        description: null,
      });
    });

    // Poll for initialization status
    const interval = setInterval(async () => {
      try {
        const isComplete = await invoke<boolean>(
          "check_initialization_complete",
        );
        if (isComplete) {
          console.log("Initialization is complete");

          dispatchServerStatusChanged("all");

          setAppState("ready");
          toast.success("MCP services initialized!", {
            id: INITIALIZATION_TOAST_ID,
            description: null,
          });
          clearInterval(interval);
        } else {
          toast.loading("Initializing MCP Services", {
            description: (
              <div className="flex w-full flex-col gap-2">
                <p className="mt-2 text-xs text-slate-600">
                  We&apos;re setting things up for you...
                </p>
                <Progress value={progress} max={100} />
              </div>
            ),
            id: INITIALIZATION_TOAST_ID,
          });
          setProgress((prev) => Math.min(prev + 5, 90));
        }
      } catch (error) {
        console.error("Error checking initialization status:", error);
        setAppState("error");
        toast.error("Error initializing MCP Services", {
          id: INITIALIZATION_TOAST_ID,
          description: "Please check your MCP configuration and try again.",
        });
      }
    }, 500);

    return () => {
      unlisten.then((fn) => fn());
      clearInterval(interval);
    };
  }, [progress]);

  // Check if user has consented when app is ready
  useEffect(() => {
    if (appState === "ready") {
      const consent = getUserConsent();
      if (!consent || !consent.termsAccepted) {
        setShowTerms(true);
      } else {
        setUserConsented(true);
      }
    }
  }, [appState]);

  const handleTermsAccepted = () => {
    setShowTerms(false);
    setUserConsented(true);
  };

  if (!userConsented) {
    return (
      <TermsConsentDialog open={showTerms} onAccept={handleTermsAccepted} />
    );
  }
  return children;
};

export default InitMCPOverlay;
