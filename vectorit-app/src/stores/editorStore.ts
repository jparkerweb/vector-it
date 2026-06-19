import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { SegEdit, Segmentation } from "./appStore";

interface EditorState {
  editHistory: SegEdit[];
  canUndo: boolean;
  currentSegmentation: Segmentation | null;
  activeTool: "pencil" | "eyedropper" | "finder" | "zap" | null;
  activeRegionId: number | null;
  activeColor: string | null;

  initEditor: (segmentation: Segmentation) => Promise<void>;
  applyEdit: (edit: SegEdit) => Promise<void>;
  undo: () => Promise<void>;
  resetAll: () => Promise<void>;
  setActiveTool: (tool: "pencil" | "eyedropper" | "finder" | "zap" | null) => void;
  setActiveRegion: (regionId: number, color: string) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  editHistory: [],
  canUndo: false,
  currentSegmentation: null,
  activeTool: null,
  activeRegionId: null,
  activeColor: null,

  initEditor: async (segmentation: Segmentation) => {
    try {
      await invoke("init_editor", { segmentation });
      set({
        currentSegmentation: segmentation,
        editHistory: [],
        canUndo: false,
      });
    } catch (e) {
      console.error("Failed to initialize editor:", e);
    }
  },

  applyEdit: async (edit: SegEdit) => {
    try {
      const result = await invoke<Segmentation>("apply_edit", { edit });
      set((s) => ({
        currentSegmentation: result,
        editHistory: [...s.editHistory, edit],
        canUndo: true,
      }));
    } catch (e) {
      console.error("Failed to apply edit:", e);
    }
  },

  undo: async () => {
    try {
      const success = await invoke<boolean>("undo_edit");
      if (success) {
        const seg = await invoke<Segmentation>("get_segmentation");
        set((s) => ({
          currentSegmentation: seg,
          editHistory: s.editHistory.slice(0, -1),
          canUndo: s.editHistory.length > 1,
        }));
      } else {
        set({ canUndo: false });
      }
    } catch (e) {
      console.error("Failed to undo:", e);
    }
  },

  resetAll: async () => {
    try {
      await invoke("reset_edits");
      const seg = await invoke<Segmentation>("get_segmentation");
      set({
        currentSegmentation: seg,
        editHistory: [],
        canUndo: false,
      });
    } catch (e) {
      console.error("Failed to reset:", e);
    }
  },

  setActiveTool: (tool) => set({ activeTool: tool }),
  setActiveRegion: (regionId, color) =>
    set({ activeRegionId: regionId, activeColor: color }),
}));
