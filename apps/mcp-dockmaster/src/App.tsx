import "./index.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import Home from "./pages/home";
import InstalledServers from "./components/InstalledServers";
import Registry from "./components/Registry";
import About from "./pages/about";
import InitMpcOverlay from "./components/init-mpc-overlay";

import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarTrigger,
} from "./components/ui/sidebar";
import { Navigate, NavLink, Route, Routes, useMatch } from "react-router";
import {
  AboutIcon,
  HomeIcon,
  RegistryIcon,
  ServersIcon,
} from "./components/icons";
import { Toaster } from "./components/ui/sonner";
import { cn } from "./lib/utils";
import { TooltipProvider } from "./components/ui/tooltip";

function NavItem({
  icon: Icon,
  label,
  to,
}: {
  icon: React.ElementType;
  label: string;
  to: string;
}) {
  const match = useMatch(to);
  return (
    <NavLink
      to={to}
      className={cn(
        "flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
        match ? "bg-white" : "text-slate-600 opacity-80 hover:opacity-100",
      )}
    >
      <Icon className="size-5" />
      <span>{label}</span>
    </NavLink>
  );
}

function AppSidebar() {
  const items = [
    { id: "home", label: "Home", icon: HomeIcon, to: "/" },
    {
      id: "installed",
      label: "My Applications",
      icon: ServersIcon,
      to: "/installed",
    },
    {
      id: "registry",
      label: "AI App Store",
      icon: RegistryIcon,
      to: "/registry",
    },
    { id: "about", label: "About", icon: AboutIcon, to: "/about" },
  ];

  return (
    <Sidebar>
      <SidebarHeader>
        {/* <img src="/logo.png" alt="MPC Dockmaster" className="h-8 w-8" /> */}
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarMenu>
            {items.map((item) => (
              <SidebarMenuItem key={item.id}>
                <SidebarMenuButton asChild>
                  <NavItem icon={item.icon} label={item.label} to={item.to} />
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  );
}

export const queryClient = new QueryClient();

const AppRoutes = () => {
  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={0}>
        <InitMpcOverlay>
          <SidebarProvider>
            <AppSidebar />
            <SidebarInset>
              <SidebarTrigger className="absolute top-2 left-2" />
              <Routes>
                <Route path="/" element={<Home />} />
                <Route path="/installed" element={<InstalledServers />} />
                <Route path="/registry" element={<Registry />} />
                <Route path="/about" element={<About />} />
                <Route element={<Navigate replace to={"/"} />} path="*" />
              </Routes>
            </SidebarInset>
          </SidebarProvider>
          <Toaster position="top-right" theme="light" />
        </InitMpcOverlay>
      </TooltipProvider>
    </QueryClientProvider>
  );
};

export default AppRoutes;
