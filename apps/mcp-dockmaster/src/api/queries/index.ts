import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { queryKeys } from "../constants";

export const useNodeInstalled = () => {
  return useQuery({
    queryKey: queryKeys.nodeInstalled,
    queryFn: async () => {
      const installed = await invoke<boolean>("check_node_installed");
      return installed;
    },
  });
};

export const useUvInstalled = () => {
  return useQuery({
    queryKey: queryKeys.uvInstalled,
    queryFn: async () => {
      const installed = await invoke<boolean>("check_uv_installed");
      return installed;
    },
  });
};

export const useDockerInstalled = () => {
  return useQuery({
    queryKey: queryKeys.dockerInstalled,
    queryFn: async () => {
      const installed = await invoke<boolean>("check_docker_installed");
      return installed;
    },
  });
};
