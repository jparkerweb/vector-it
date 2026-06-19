import { create } from "zustand";
import { persist } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "./appStore";

export type ExportFormat = "svg" | "eps" | "pdf" | "dxf";

interface SettingsState {
  lastExportFormat: ExportFormat;
  lastExportDir: string;
  lastFilenamePattern: string;
  hasExportedOnce: boolean;
  lastExportPath: string | null;

  setLastExport: (format: ExportFormat, dir: string, path: string) => void;
  setFilenamePattern: (pattern: string) => void;

  quickSave: () => Promise<string | null>;
  saveAs: () => Promise<string | null>;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      lastExportFormat: "svg" as ExportFormat,
      lastExportDir: "",
      lastFilenamePattern: "{original_name}_vector",
      hasExportedOnce: false,
      lastExportPath: null,

      setLastExport: (format, dir, path) =>
        set({
          lastExportFormat: format,
          lastExportDir: dir,
          hasExportedOnce: true,
          lastExportPath: path,
        }),

      setFilenamePattern: (pattern) => set({ lastFilenamePattern: pattern }),

      quickSave: async () => {
        const state = get();
        const { vectorResult, svgSource } = useAppStore.getState();
        if (!vectorResult && !svgSource) return null;

        if (!state.hasExportedOnce || !state.lastExportPath) {
          return get().saveAs();
        }

        try {
          if (svgSource) {
            await invoke("write_file", { path: state.lastExportPath, content: svgSource });
          } else {
            await invoke(`export_${state.lastExportFormat}`, {
              result: vectorResult,
              outputPath: state.lastExportPath,
            });
          }
          return state.lastExportPath;
        } catch {
          return null;
        }
      },

      saveAs: async () => {
        const { vectorResult, svgSource } = useAppStore.getState();
        if (!vectorResult && !svgSource) return null;

        const state = get();
        const format = svgSource ? "svg" as ExportFormat : state.lastExportFormat;
        const filters: Record<
          ExportFormat,
          { name: string; extensions: string[] }
        > = {
          svg: { name: "SVG", extensions: ["svg"] },
          eps: { name: "EPS", extensions: ["eps"] },
          pdf: { name: "PDF", extensions: ["pdf"] },
          dxf: { name: "DXF", extensions: ["dxf"] },
        };

        let defaultDir: string | undefined;
        if (state.lastExportDir) {
          defaultDir = state.lastExportDir;
        } else {
          try {
            defaultDir = await invoke<string>("get_documents_dir");
          } catch {
            // fall through
          }
        }

        const path = await save({
          filters: [filters[format]],
          defaultPath: defaultDir
            ? `${defaultDir}/output.${format}`
            : `output.${format}`,
        });

        if (path) {
          try {
            if (svgSource) {
              await invoke("write_file", { path, content: svgSource });
            } else {
              await invoke(`export_${format}`, {
                result: vectorResult,
                outputPath: path,
              });
            }
            const dir = path.substring(
              0,
              Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"))
            );
            set({
              lastExportFormat: format,
              lastExportDir: dir,
              lastExportPath: path,
              hasExportedOnce: true,
            });
            return path;
          } catch {
            return null;
          }
        }
        return null;
      },
    }),
    {
      name: "vectorit-settings",
      partialize: (state) => ({
        lastExportFormat: state.lastExportFormat,
        lastExportDir: state.lastExportDir,
        lastFilenamePattern: state.lastFilenamePattern,
      }),
    }
  )
);
