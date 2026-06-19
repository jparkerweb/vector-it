import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type WizardStep = "imageType" | "quality" | "colorMode" | "review" | "export";

export type ImageTypeChoice = "photo" | "logo_smooth" | "logo_sharp";
export type QualityChoice = "Low" | "Medium" | "High";

export interface DetectionResult {
  image_type: "Photo" | "AntiAliased" | "Aliased";
  confidence: number;
}

export interface PaletteSuggestion {
  count: number;
  colors: string[];
  quality_score: number;
}

interface WizardState {
  currentStep: WizardStep;
  steps: WizardStep[];

  // Step selections
  detectedType: DetectionResult | null;
  selectedImageType: ImageTypeChoice | null;
  selectedQuality: QualityChoice;
  colorMode: "automatic" | "custom";
  customColorCount: number;
  paletteSuggestions: PaletteSuggestion[];
  customPalette: string[];

  // Actions
  setStep: (step: WizardStep) => void;
  nextStep: () => void;
  prevStep: () => void;
  setImageType: (type_: ImageTypeChoice) => void;
  setQuality: (quality: QualityChoice) => void;
  setColorMode: (mode: "automatic" | "custom") => void;
  setCustomColorCount: (count: number) => void;
  setCustomPalette: (palette: string[]) => void;
  detectImageType: (imagePath: string) => Promise<void>;
  fetchPaletteSuggestions: (imagePath: string) => Promise<void>;
  reset: () => void;
}

const STEPS: WizardStep[] = ["imageType", "quality", "colorMode", "review", "export"];

export const useWizardStore = create<WizardState>((set, get) => ({
  currentStep: "imageType",
  steps: STEPS,

  detectedType: null,
  selectedImageType: null,
  selectedQuality: "Medium",
  colorMode: "automatic",
  customColorCount: 12,
  paletteSuggestions: [],
  customPalette: [],

  setStep: (step) => set({ currentStep: step }),

  nextStep: () => {
    const { currentStep, steps } = get();
    const idx = steps.indexOf(currentStep);
    if (idx < steps.length - 1) {
      set({ currentStep: steps[idx + 1] });
    }
  },

  prevStep: () => {
    const { currentStep, steps } = get();
    const idx = steps.indexOf(currentStep);
    if (idx > 0) {
      set({ currentStep: steps[idx - 1] });
    }
  },

  setImageType: (type_) => set({ selectedImageType: type_ }),
  setQuality: (quality) => set({ selectedQuality: quality }),
  setColorMode: (mode) => set({ colorMode: mode }),
  setCustomColorCount: (count) => set({ customColorCount: count }),
  setCustomPalette: (palette) => set({ customPalette: palette }),

  detectImageType: async (imagePath: string) => {
    try {
      const result = await invoke<DetectionResult>("detect_type", { path: imagePath });
      set({ detectedType: result });

      // Auto-select based on detection
      const mapping: Record<string, ImageTypeChoice> = {
        Photo: "photo",
        AntiAliased: "logo_smooth",
        Aliased: "logo_sharp",
      };
      set({ selectedImageType: mapping[result.image_type] ?? "logo_smooth" });
    } catch {
      set({ selectedImageType: "logo_smooth" });
    }
  },

  fetchPaletteSuggestions: async (imagePath: string) => {
    try {
      const suggestions = await invoke<PaletteSuggestion[]>("suggest_palette", {
        path: imagePath,
        maxColors: 16,
      });
      set({ paletteSuggestions: suggestions });
    } catch {
      set({ paletteSuggestions: [] });
    }
  },

  reset: () =>
    set({
      currentStep: "imageType",
      detectedType: null,
      selectedImageType: null,
      selectedQuality: "Medium",
      colorMode: "automatic",
      customColorCount: 12,
      paletteSuggestions: [],
      customPalette: [],
    }),
}));
