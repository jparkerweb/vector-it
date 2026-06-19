import { useCallback, useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { useWizardStore } from "../../stores/wizardStore";
import { Canvas } from "../Canvas";

export function ReviewStep() {
  const { vectorResult, isProcessing, vectorize, imageInfo } = useAppStore();
  const { selectedQuality, colorMode, customColorCount } = useWizardStore();
  const hasCanvasEdits = useAppStore((s) => s.hasCanvasEdits);
  const [reprocessing, setReprocessing] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);

  const doReprocess = useCallback(async () => {
    setReprocessing(true);
    await vectorize();
    setReprocessing(false);
  }, [vectorize]);

  const handleReprocess = useCallback(() => {
    if (hasCanvasEdits) {
      setShowConfirm(true);
    } else {
      doReprocess();
    }
  }, [hasCanvasEdits, doReprocess]);

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-900">Review</h2>
        <div className="flex items-center gap-3">
          <span className="text-sm text-gray-500">
            Quality: {selectedQuality} | Colors:{" "}
            {colorMode === "automatic" ? "Auto" : customColorCount}
          </span>
          <button
            onClick={handleReprocess}
            disabled={isProcessing || reprocessing}
            className="px-3 py-1.5 text-sm font-medium text-blue-600 border border-blue-300 rounded hover:bg-blue-50 disabled:opacity-40"
          >
            {reprocessing ? "Processing..." : "Re-process"}
          </button>
        </div>
      </div>
      <div className="flex-1 flex gap-4 min-h-0">
        {/* Original */}
        <div className="flex-1 flex flex-col items-center bg-gray-100 rounded-lg p-2">
          <span className="text-xs text-gray-500 mb-1">Original</span>
          {imageInfo && (
            <img
              src={`data:image/png;base64,${imageInfo.thumbnail_base64}`}
              alt="Original"
              className="max-w-full max-h-full object-contain"
            />
          )}
        </div>
        {/* Vector preview */}
        <div className="flex-1 flex flex-col items-center bg-gray-100 rounded-lg p-2">
          <span className="text-xs text-gray-500 mb-1">Vector</span>
          {vectorResult ? (
            <Canvas />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-400 text-sm">
              {isProcessing ? "Processing..." : "No result yet"}
            </div>
          )}
        </div>
      </div>

      {/* Reprocess confirmation dialog */}
      {showConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-sm mx-4">
            <h3 className="text-base font-semibold text-gray-900 mb-2">Discard edits?</h3>
            <p className="text-sm text-gray-600 mb-5">
              Re-processing will discard all your manual edits. This cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowConfirm(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => { setShowConfirm(false); doReprocess(); }}
                className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 transition-colors"
              >
                Discard &amp; Re-process
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
