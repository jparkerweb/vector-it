import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../../stores/appStore";

type ExportFormat = "svg" | "eps" | "pdf";

export function ExportStep() {
  const { vectorResult, isProcessing } = useAppStore();
  const [format, setFormat] = useState<ExportFormat>("svg");
  const [exported, setExported] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const handleExport = async () => {
    if (!vectorResult) return;

    const filters: Record<ExportFormat, { name: string; extensions: string[] }> = {
      svg: { name: "SVG", extensions: ["svg"] },
      eps: { name: "EPS", extensions: ["eps"] },
      pdf: { name: "PDF", extensions: ["pdf"] },
    };

    const path = await save({
      filters: [filters[format]],
      defaultPath: `output.${format}`,
    });

    if (path) {
      const { invoke } = await import("@tauri-apps/api/core");
      const command = `export_${format}`;
      try {
        await invoke(command, { result: vectorResult, outputPath: path });
        setExported(true);
        setExportError(null);
      } catch (e) {
        setExportError(String(e));
        setExported(false);
      }
    }
  };

  return (
    <div className="max-w-lg mx-auto">
      <h2 className="text-lg font-semibold text-gray-900 mb-4">Export</h2>

      <div className="space-y-4">
        {/* Format selection */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Format
          </label>
          <select
            value={format}
            onChange={(e) => {
              setFormat(e.target.value as ExportFormat);
              setExported(false);
            }}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
          >
            <option value="svg">SVG — Scalable Vector Graphics</option>
            <option value="eps">EPS — Encapsulated PostScript</option>
            <option value="pdf">PDF — Portable Document Format</option>
          </select>
        </div>

        {/* Export button */}
        <button
          onClick={handleExport}
          disabled={!vectorResult || isProcessing}
          className="w-full px-4 py-3 text-sm font-medium text-white bg-green-600 rounded-lg hover:bg-green-700 disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Export as {format.toUpperCase()}
        </button>

        {exported && (
          <p className="text-sm text-green-600 text-center">
            ✓ Exported successfully!
          </p>
        )}

        {exportError && (
          <p className="text-sm text-red-600 text-center">
            ✗ Export failed: {exportError}
          </p>
        )}
      </div>
    </div>
  );
}
