import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PencilToolProps {
  activeRegionId: number | null;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  zoom: number;
  panX: number;
  panY: number;
  onEditApplied: () => void;
}

const BRUSH_SIZES = [1, 3, 5, 7] as const;

export function PencilTool({
  activeRegionId,
  canvasRef,
  zoom,
  panX,
  panY,
  onEditApplied,
}: PencilToolProps) {
  const [brushSize, setBrushSize] = useState<(typeof BRUSH_SIZES)[number]>(1);
  const [isDrawing, setIsDrawing] = useState(false);
  const pixelBuffer = useRef<[number, number][]>([]);

  const getImageCoords = useCallback(
    (e: React.MouseEvent): [number, number] | null => {
      const canvas = canvasRef.current;
      if (!canvas) return null;
      const rect = canvas.getBoundingClientRect();
      const x = Math.floor((e.clientX - rect.left - panX) / zoom);
      const y = Math.floor((e.clientY - rect.top - panY) / zoom);
      return [x, y];
    },
    [canvasRef, zoom, panX, panY]
  );

  const collectBrushPixels = useCallback(
    (cx: number, cy: number): [number, number][] => {
      const pixels: [number, number][] = [];
      const radius = Math.floor(brushSize / 2);
      for (let dy = -radius; dy <= radius; dy++) {
        for (let dx = -radius; dx <= radius; dx++) {
          pixels.push([cx + dx, cy + dy]);
        }
      }
      return pixels;
    },
    [brushSize]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (activeRegionId === null) return;
      const coords = getImageCoords(e);
      if (!coords) return;

      setIsDrawing(true);
      pixelBuffer.current = collectBrushPixels(coords[0], coords[1]);
    },
    [activeRegionId, getImageCoords, collectBrushPixels]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!isDrawing) return;
      const coords = getImageCoords(e);
      if (!coords) return;

      const newPixels = collectBrushPixels(coords[0], coords[1]);
      pixelBuffer.current.push(...newPixels);
    },
    [isDrawing, getImageCoords, collectBrushPixels]
  );

  const handleMouseUp = useCallback(async () => {
    if (!isDrawing || activeRegionId === null) return;
    setIsDrawing(false);

    const pixels = pixelBuffer.current;
    pixelBuffer.current = [];

    if (pixels.length === 0) return;

    try {
      await invoke("apply_edit", {
        edit: {
          PaintPixels: {
            pixels,
            target_region: activeRegionId,
          },
        },
      });
      onEditApplied();
    } catch (e) {
      console.error("Failed to apply paint edit:", e);
    }
  }, [isDrawing, activeRegionId, onEditApplied]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-500 font-medium">Brush:</span>
        {BRUSH_SIZES.map((size) => (
          <button
            key={size}
            onClick={() => setBrushSize(size)}
            className={`w-7 h-7 flex items-center justify-center rounded text-xs font-medium transition-colors ${
              brushSize === size
                ? "bg-blue-500 text-white"
                : "bg-gray-200 text-gray-600 hover:bg-gray-300"
            }`}
            title={`${size}px brush`}
          >
            {size}
          </button>
        ))}
      </div>

      {/* Invisible overlay to capture drawing events */}
      <div
        className="absolute inset-0"
        style={{ cursor: "crosshair", zIndex: 10 }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      />
    </div>
  );
}
