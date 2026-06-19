import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useAppStore } from "../stores/appStore";

const SWATCHES = [
  // Row 1: basics
  "#000000", "#333333", "#666666", "#999999", "#cccccc", "#ffffff",
  // Row 2: warm
  "#ff0000", "#ff4444", "#ff8800", "#ffcc00", "#ffff00", "#ffff88",
  // Row 3: cool
  "#0000ff", "#4488ff", "#00ccff", "#00ffcc", "#00ff00", "#88ff00",
  // Row 4: purples/pinks
  "#8800ff", "#cc44ff", "#ff00ff", "#ff0088", "#ff4466", "#cc8866",
  // Row 5: earth tones
  "#884400", "#aa6633", "#cc9966", "#ddbb88", "#556b2f", "#2f4f4f",
];

function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("");
}

interface InlineColorPickerProps {
  value: string;
  onChange: (color: string) => void;
}

export function InlineColorPicker({ value, onChange }: InlineColorPickerProps) {
  const [open, setOpen] = useState(false);
  const [hexInput, setHexInput] = useState(value);
  const containerRef = useRef<HTMLDivElement>(null);
  const vectorResult = useAppStore((s) => s.vectorResult);

  // Extract unique colors from canvas paths, with perceptual dedup
  const canvasColors = useMemo(() => {
    if (!vectorResult) return [];
    const seen = new Set<string>();
    const bucket = new Set<string>();
    const colors: string[] = [];
    const swatchSet = new Set(SWATCHES);
    for (const path of vectorResult.paths) {
      const { r, g, b } = path.fill_color;
      const hex = rgbToHex(r, g, b).toLowerCase();
      // Snap to nearest multiple of 8 for perceptual dedup
      const snap = (v: number) => Math.round(v / 8) * 8;
      const bucketKey = `${snap(r)},${snap(g)},${snap(b)}`;
      if (!bucket.has(bucketKey) && !seen.has(hex) && !swatchSet.has(hex)) {
        seen.add(hex);
        bucket.add(bucketKey);
        colors.push(hex);
      }
    }
    return colors;
  }, [vectorResult]);

  // Sync hex input when value changes externally
  useEffect(() => {
    setHexInput(value);
  }, [value]);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const pickColor = useCallback((color: string) => {
    onChange(color);
    setOpen(false);
  }, [onChange]);

  const handleHexSubmit = useCallback(() => {
    const cleaned = hexInput.startsWith("#") ? hexInput : `#${hexInput}`;
    if (/^#[0-9a-fA-F]{6}$/.test(cleaned)) {
      onChange(cleaned);
      setOpen(false);
    }
  }, [hexInput, onChange]);

  return (
    <div ref={containerRef} className="relative">
      {/* Color swatch button */}
      <button
        onClick={() => setOpen(!open)}
        className="w-7 h-7 rounded border border-gray-300 cursor-pointer hover:ring-2 hover:ring-blue-400 transition-all"
        style={{ backgroundColor: value }}
        title="Pick color"
      />

      {/* Dropdown */}
      {open && (
        <div
          className="absolute top-full left-0 mt-1 bg-white rounded-lg shadow-lg border border-gray-200 p-2 z-50 w-[204px]"
          onWheel={(e) => e.stopPropagation()}
        >
          {/* Swatch grid */}
          <div className="grid grid-cols-6 gap-1 mb-2">
            {SWATCHES.map((color) => (
              <button
                key={color}
                onClick={() => pickColor(color)}
                className={`w-7 h-7 rounded border transition-all ${
                  value === color
                    ? "border-blue-500 ring-2 ring-blue-300"
                    : "border-gray-200 hover:border-gray-400 hover:scale-110"
                }`}
                style={{ backgroundColor: color }}
                title={color}
              />
            ))}
          </div>

          {/* Canvas colors */}
          {canvasColors.length > 0 && (
            <>
              <div className="text-[10px] text-gray-400 uppercase tracking-wide mb-1">Canvas</div>
              <div className="grid grid-cols-6 gap-1 mb-2 max-h-[80px] overflow-y-auto overflow-x-hidden">
                {canvasColors.map((color) => (
                  <button
                    key={color}
                    onClick={() => pickColor(color)}
                    className={`w-7 h-7 rounded border transition-colors ${
                      value === color
                        ? "border-blue-500 ring-2 ring-blue-300"
                        : "border-gray-200 hover:border-gray-400"
                    }`}
                    style={{ backgroundColor: color }}
                    title={color}
                  />
                ))}
              </div>
            </>
          )}

          {/* Hex input */}
          <div className="flex gap-1">
            <input
              type="text"
              value={hexInput}
              onChange={(e) => setHexInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleHexSubmit()}
              placeholder="#000000"
              className="flex-1 text-xs px-2 py-1 border border-gray-200 rounded font-mono"
              maxLength={7}
            />
            <button
              onClick={handleHexSubmit}
              className="px-2 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              OK
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
