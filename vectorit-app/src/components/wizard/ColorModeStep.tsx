import { useEffect } from "react";
import { useWizardStore } from "../../stores/wizardStore";
import { useAppStore } from "../../stores/appStore";

export function ColorModeStep() {
  const {
    colorMode,
    setColorMode,
    customColorCount,
    setCustomColorCount,
    paletteSuggestions,
    fetchPaletteSuggestions,
  } = useWizardStore();
  const { imagePath } = useAppStore();

  useEffect(() => {
    if (imagePath && paletteSuggestions.length === 0) {
      fetchPaletteSuggestions(imagePath);
    }
  }, [imagePath, paletteSuggestions.length, fetchPaletteSuggestions]);

  return (
    <div className="max-w-lg mx-auto">
      <h2 className="text-lg font-semibold text-gray-900 mb-4">Color Mode</h2>

      <div className="space-y-3 mb-6">
        <label
          className={`flex items-start gap-3 p-4 border rounded-lg cursor-pointer transition-colors ${
            colorMode === "automatic"
              ? "border-blue-500 bg-blue-50"
              : "border-gray-200 hover:border-gray-300"
          }`}
        >
          <input
            type="radio"
            name="colorMode"
            checked={colorMode === "automatic"}
            onChange={() => setColorMode("automatic")}
            className="mt-0.5"
          />
          <div>
            <div className="font-medium text-gray-900">Automatic</div>
            <div className="text-sm text-gray-500">
              Let VectorIt choose the optimal color count
            </div>
          </div>
        </label>

        <label
          className={`flex items-start gap-3 p-4 border rounded-lg cursor-pointer transition-colors ${
            colorMode === "custom"
              ? "border-blue-500 bg-blue-50"
              : "border-gray-200 hover:border-gray-300"
          }`}
        >
          <input
            type="radio"
            name="colorMode"
            checked={colorMode === "custom"}
            onChange={() => setColorMode("custom")}
            className="mt-0.5"
          />
          <div className="flex-1">
            <div className="font-medium text-gray-900">Custom</div>
            <div className="text-sm text-gray-500 mb-2">
              Choose the number of colors manually
            </div>
            {colorMode === "custom" && (
              <input
                type="number"
                min={2}
                max={256}
                value={customColorCount}
                onChange={(e) => setCustomColorCount(Number(e.target.value))}
                className="w-20 px-2 py-1 text-sm border border-gray-300 rounded"
              />
            )}
          </div>
        </label>
      </div>

      {/* Palette suggestions */}
      {paletteSuggestions.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-gray-700 mb-2">
            Suggested Palettes
          </h3>
          <div className="space-y-2">
            {paletteSuggestions.map((suggestion) => (
              <button
                key={suggestion.count}
                onClick={() => {
                  setColorMode("custom");
                  setCustomColorCount(suggestion.count);
                }}
                className={`w-full flex items-center gap-3 p-3 border rounded-lg text-left hover:bg-gray-50 transition-colors ${
                  colorMode === "custom" && customColorCount === suggestion.count
                    ? "border-blue-500 bg-blue-50"
                    : "border-gray-200"
                }`}
              >
                <span className="text-sm font-medium text-gray-600 w-12">
                  {suggestion.count}c
                </span>
                <div className="flex gap-0.5 flex-1">
                  {suggestion.colors.map((color, i) => (
                    <div
                      key={i}
                      className="w-5 h-5 rounded-sm border border-gray-200"
                      style={{ backgroundColor: color }}
                    />
                  ))}
                </div>
                <span className="text-xs text-gray-400">
                  {Math.round(suggestion.quality_score * 100)}%
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
