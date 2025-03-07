// create a store for the app

import { create } from "zustand";

interface AppStore {
  appState: "pending" | "ready" | "error";
  setAppState: (state: "pending" | "ready" | "error") => void;
}

export const useAppStore = create<AppStore>((set) => ({
  appState: "pending",
  setAppState: (state: "pending" | "ready" | "error") => set({ appState: state }),
}));
