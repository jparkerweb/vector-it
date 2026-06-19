import { create } from "zustand";

interface AdvancedState {
  isAdvancedMode: boolean;
  smoothness: number;
  cornerThreshold: number;
  colorCount: number;
  aaSensitivity: number;

  toggleMode: () => void;
  setSmoothness: (value: number) => void;
  setCornerThreshold: (value: number) => void;
  setColorCount: (value: number) => void;
  setAaSensitivity: (value: number) => void;
  resetToDefaults: () => void;
}

const DEFAULTS = {
  smoothness: 0.5,
  cornerThreshold: 60,
  colorCount: 12,
  aaSensitivity: 0.5,
};

export const useAdvancedStore = create<AdvancedState>((set) => ({
  isAdvancedMode: false,
  ...DEFAULTS,

  toggleMode: () => set((s) => ({ isAdvancedMode: !s.isAdvancedMode })),
  setSmoothness: (value) => set({ smoothness: value }),
  setCornerThreshold: (value) => set({ cornerThreshold: value }),
  setColorCount: (value) => set({ colorCount: value }),
  setAaSensitivity: (value) => set({ aaSensitivity: value }),
  resetToDefaults: () => set(DEFAULTS),
}));
