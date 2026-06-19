import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EyedropperToolProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  zoom: number;
  panX: number;
  panY: number;
  onColorSampled: (regionId: number, color: string) => void;
}

interface SampleResult {
  region_id: number;
  color_hex: string;
}

export function EyedropperTool({
  canvasRef,
  zoom,
  panX,
  panY,
  onColorSampled,
}: EyedropperToolProps) {
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

  const handleClick = useCallback(
    async (e: React.MouseEvent) => {
      const coords = getImageCoords(e);
      if (!coords) return;

      const useOriginal = e.button === 2; // Right-click samples from original

      try {
        const result = await invoke<SampleResult>("sample_color", {
          x: coords[0],
          y: coords[1],
          fromOriginal: useOriginal,
        });
        onColorSampled(result.region_id, result.color_hex);
      } catch (err) {
        console.error("Failed to sample color:", err);
      }
    },
    [getImageCoords, onColorSampled]
  );

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      handleClick(e);
    },
    [handleClick]
  );

  return (
    <div
      className="absolute inset-0"
      style={{ cursor: "crosshair", zIndex: 10 }}
      onClick={handleClick}
      onContextMenu={handleContextMenu}
    />
  );
}
