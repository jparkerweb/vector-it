import { useState, useCallback } from "react";
import { useWizardStore } from "../../stores/wizardStore";
import { InlineColorPicker } from "../InlineColorPicker";

export function PaletteEditor() {
  const {
    customPalette,
    setCustomPalette,
    paletteSuggestions,
    setColorMode,
    setCustomColorCount,
  } = useWizardStore();
  const [dragIdx, setDragIdx] = useState<number | null>(null);

  const addColor = useCallback(() => {
    setCustomPalette([...customPalette, "#808080"]);
  }, [customPalette, setCustomPalette]);

  const removeColor = useCallback(
    (idx: number) => {
      setCustomPalette(customPalette.filter((_, i) => i !== idx));
    },
    [customPalette, setCustomPalette]
  );

  const updateColor = useCallback(
    (idx: number, color: string) => {
      const updated = [...customPalette];
      updated[idx] = color;
      setCustomPalette(updated);
    },
    [customPalette, setCustomPalette]
  );

  const handleDragStart = useCallback((idx: number) => {
    setDragIdx(idx);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);

  const handleDrop = useCallback(
    (targetIdx: number) => {
      if (dragIdx === null || dragIdx === targetIdx) return;
      const updated = [...customPalette];
      const [moved] = updated.splice(dragIdx, 1);
      updated.splice(targetIdx, 0, moved);
      setCustomPalette(updated);
      setDragIdx(null);
    },
    [dragIdx, customPalette, setCustomPalette]
  );

  const applyQuickPalette = useCallback(
    (colors: string[], count: number) => {
      setCustomPalette(colors);
      setColorMode("custom");
      setCustomColorCount(count);
    },
    [setCustomPalette, setColorMode, setCustomColorCount]
  );

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-medium text-gray-700">Custom Palette</h3>

      {/* Color grid */}
      <div className="flex flex-wrap gap-2 items-center">
        {customPalette.map((color, idx) => (
          <div
            key={idx}
            draggable
            onDragStart={() => handleDragStart(idx)}
            onDragOver={handleDragOver}
            onDrop={() => handleDrop(idx)}
            className="relative group"
          >
            <InlineColorPicker
              value={color}
              onChange={(c) => updateColor(idx, c)}
            />
            <button
              onClick={() => removeColor(idx)}
              className="absolute -top-1.5 -right-1.5 w-4 h-4 bg-red-500 text-white rounded-full text-[10px] leading-none flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
            >
              ×
            </button>
          </div>
        ))}

        {/* Add button */}
        <button
          onClick={addColor}
          className="w-10 h-10 rounded-md border-2 border-dashed border-gray-300 hover:border-blue-400 text-gray-400 hover:text-blue-500 flex items-center justify-center text-xl transition-colors"
        >
          +
        </button>
      </div>

      {/* Quick palettes */}
      {paletteSuggestions.length > 0 && (
        <div>
          <h4 className="text-xs font-medium text-gray-500 mb-2 uppercase tracking-wide">
            Quick Palettes
          </h4>
          <div className="space-y-1.5">
            {paletteSuggestions
              .filter((s) => [2, 4, 8, 12].includes(s.count))
              .map((suggestion) => (
                <button
                  key={suggestion.count}
                  onClick={() => applyQuickPalette(suggestion.colors, suggestion.count)}
                  className="w-full flex items-center gap-2 px-2 py-1.5 border border-gray-200 rounded hover:bg-gray-50 transition-colors"
                >
                  <span className="text-xs text-gray-500 w-8">
                    {suggestion.count}c
                  </span>
                  <div className="flex gap-0.5 flex-1">
                    {suggestion.colors.map((color, i) => (
                      <div
                        key={i}
                        className="w-4 h-4 rounded-sm border border-gray-200"
                        style={{ backgroundColor: color }}
                      />
                    ))}
                  </div>
                </button>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}
