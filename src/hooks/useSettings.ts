import { create } from "zustand";

interface Settings {
  claudeApiKey: string;
  overlayEnabled: boolean;
  captureInterval: number; // ms between captures
  overlayOpacity: number;
}

interface SettingsStore {
  settings: Settings;
  updateSettings: (partial: Partial<Settings>) => void;
}

export const useSettings = create<SettingsStore>((set) => ({
  settings: {
    claudeApiKey: "",
    overlayEnabled: true,
    captureInterval: 500, // 2 FPS
    overlayOpacity: 0.9,
  },
  updateSettings: (partial) =>
    set((s) => ({ settings: { ...s.settings, ...partial } })),
}));
