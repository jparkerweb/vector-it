import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { useWizardStore } from "./wizardStore";

export interface ImageInfo {
  width: number;
  height: number;
  has_alpha: boolean;
  file_size_bytes: number;
  thumbnail_base64: string;
}

export interface Point {
  x: number;
  y: number;
}

export interface CubicBezier {
  p0: Point;
  p1: Point;
  p2: Point;
  p3: Point;
}

export interface BezierSegment {
  curve: CubicBezier;
  is_corner_start: boolean;
}

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface VectorPath {
  segments: BezierSegment[];
  fill_color: RgbColor;
  is_closed: boolean;
  stroke_color?: RgbColor;
  stroke_width?: number;
}

export interface LabColor {
  l: number;
  a: number;
  b: number;
}

export interface Palette {
  colors: LabColor[];
}

export interface VectorizationResult {
  paths: VectorPath[];
  palette: Palette;
  dimensions: [number, number];
  segmentation?: Segmentation;
}

export interface Segmentation {
  regions: Region[];
  label_map: number[];
  width: number;
  height: number;
}

export interface Region {
  id: number;
  color_index: number;
  pixel_count: number;
}

export type SegEdit =
  | { PaintPixels: { pixels: [number, number][]; target_region: number } }
  | { SplitRegion: { region_id: number; split_line: [Point, Point] } }
  | { MergeRegions: { source: number; target: number } };

export interface VectorizationConfig {
  color_count: number;
  smoothness: number;
  corner_threshold: number;
  simplify_tolerance: number;
  quality: "Low" | "Medium" | "High" | "Custom";
}

interface AppState {
  imagePath: string | null;
  imageInfo: ImageInfo | null;
  vectorResult: VectorizationResult | null;
  svgSource: string | null;
  isProcessing: boolean;
  error: string | null;
  hasCanvasEdits: boolean;
  loadImage: (path: string) => Promise<void>;
  vectorize: () => Promise<void>;
  exportSvg: (outputPath: string) => Promise<void>;
  clearError: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  imagePath: null,
  imageInfo: null,
  vectorResult: null,
  svgSource: null,
  isProcessing: false,
  hasCanvasEdits: false,
  error: null,

  loadImage: async (path: string) => {
    try {
      set({ isProcessing: true, error: null, svgSource: null });
      const info = await invoke<ImageInfo>("load_image", { path });
      set({ imagePath: path, imageInfo: info, vectorResult: null, isProcessing: false });
    } catch (e) {
      set({ error: String(e), isProcessing: false });
    }
  },

  vectorize: async () => {
    const { imagePath } = get();
    if (!imagePath) return;
    try {
      set({ isProcessing: true, error: null });
      const wizard = useWizardStore.getState();
      const colorCount =
        wizard.colorMode === "custom" ? wizard.customColorCount : 12;
      const config: VectorizationConfig = {
        color_count: colorCount,
        smoothness: 0.5,
        corner_threshold: 60.0,
        simplify_tolerance: 1.0,
        quality: wizard.selectedQuality,
      };
      const result = await invoke<VectorizationResult>("vectorize", {
        path: imagePath,
        config,
      });
      set({ vectorResult: result, isProcessing: false });
    } catch (e) {
      set({ error: String(e), isProcessing: false });
    }
  },

  exportSvg: async (outputPath: string) => {
    const { vectorResult } = get();
    if (!vectorResult) return;
    try {
      set({ isProcessing: true, error: null });
      await invoke("export_svg", { result: vectorResult, outputPath });
      set({ isProcessing: false });
    } catch (e) {
      set({ error: String(e), isProcessing: false });
    }
  },

  clearError: () => set({ error: null }),
}));
