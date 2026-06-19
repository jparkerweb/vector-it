import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAdvancedStore } from "../../stores/advancedStore";
import { useAppStore } from "../../stores/appStore";

export function AdvancedPanel() {
  const {
    isAdvancedMode,
    smoothness,
    cornerThreshold,
    colorCount,
    aaSensitivity,
    setSmoothness,
    setCornerThreshold,
    setColorCount,
    setAaSensitivity,
    resetToDefaults,
    toggleMode,
  } = useAdvancedStore();

  const { imagePath } = useAppStore();
  const hasCanvasEdits = useAppStore((s) => s.hasCanvasEdits);
  const [showConfirm, setShowConfirm] = useState(false);

  const doApply = useCallback(async () => {
    if (!imagePath) return;
    try {
      const config = {
        color_count: colorCount,
        smoothness,
        corner_threshold: cornerThreshold,
        simplify_tolerance: 2.0 - smoothness * 1.8,
        quality: "Custom" as const,
      };
      const result = await invoke("vectorize", {
        path: imagePath,
        config,
      });
      useAppStore.setState({ vectorResult: result as any });
    } catch (e) {
      console.error("Failed to re-vectorize:", e);
    }
  }, [imagePath, smoothness, cornerThreshold, colorCount]);

  const handleApply = useCallback(() => {
    if (hasCanvasEdits) {
      setShowConfirm(true);
    } else {
      doApply();
    }
  }, [hasCanvasEdits, doApply]);

  if (!isAdvancedMode) {
    return (
      <button
        onClick={toggleMode}
        className="px-3 py-1.5 text-xs font-medium text-gray-600 hover:text-gray-900 bg-gray-100 hover:bg-gray-200 rounded transition-colors"
      >
        Advanced
      </button>
    );
  }

  return (
    <div className="w-64 bg-white border-l border-gray-200 p-4 flex flex-col gap-4 overflow-y-auto">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-800">Advanced Parameters</h3>
        <button
          onClick={toggleMode}
          className="text-xs text-gray-400 hover:text-gray-600"
        >
          ✕
        </button>
      </div>

      {/* Smoothness */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium text-gray-600">
          Smoothness: {smoothness.toFixed(2)}
        </label>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={smoothness}
          onChange={(e) => setSmoothness(parseFloat(e.target.value))}
          className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-[10px] text-gray-400">
          <span>Angular</span>
          <span>Smooth</span>
        </div>
      </div>

      {/* Corner Threshold */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium text-gray-600">
          Corner Threshold: {cornerThreshold}°
        </label>
        <input
          type="range"
          min={30}
          max={120}
          step={5}
          value={cornerThreshold}
          onChange={(e) => setCornerThreshold(parseInt(e.target.value))}
          className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-[10px] text-gray-400">
          <span>30°</span>
          <span>120°</span>
        </div>
      </div>

      {/* Color Count */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium text-gray-600">
          Color Count: {colorCount}
        </label>
        <input
          type="range"
          min={2}
          max={256}
          step={1}
          value={colorCount}
          onChange={(e) => setColorCount(parseInt(e.target.value))}
          className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-[10px] text-gray-400">
          <span>2</span>
          <span>256</span>
        </div>
      </div>

      {/* AA Sensitivity */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium text-gray-600">
          AA Sensitivity: {aaSensitivity.toFixed(1)}
        </label>
        <input
          type="range"
          min={0}
          max={1}
          step={0.1}
          value={aaSensitivity}
          onChange={(e) => setAaSensitivity(parseFloat(e.target.value))}
          className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div className="flex justify-between text-[10px] text-gray-400">
          <span>Low</span>
          <span>High</span>
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex flex-col gap-2 mt-2 pt-2 border-t border-gray-100">
        <button
          onClick={handleApply}
          className="w-full px-3 py-2 text-xs font-medium bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
        >
          Apply
        </button>
        <button
          onClick={resetToDefaults}
          className="w-full px-3 py-2 text-xs font-medium text-gray-600 bg-gray-100 rounded hover:bg-gray-200 transition-colors"
        >
          Reset to Defaults
        </button>
      </div>

      {/* Reprocess confirmation dialog */}
      {showConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-sm mx-4">
            <h3 className="text-base font-semibold text-gray-900 mb-2">Discard edits?</h3>
            <p className="text-sm text-gray-600 mb-5">
              Applying new parameters will discard all your manual edits. This cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowConfirm(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => { setShowConfirm(false); doApply(); }}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 transition-colors"
              >
                Discard &amp; Apply
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
