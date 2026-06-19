import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ZapToolProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  zoom: number;
  panX: number;
  panY: number;
  onEditApplied: () => void;
}

export function ZapTool({
  canvasRef,
  zoom,
  panX,
  panY,
  onEditApplied,
}: ZapToolProps) {
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(
    null
  );

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

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const coords = getImageCoords(e);
      if (coords) {
        setHoverPos({ x: coords[0], y: coords[1] });
      }
    },
    [getImageCoords]
  );

  const handleClick = useCallback(
    async (e: React.MouseEvent) => {
      const coords = getImageCoords(e);
      if (!coords) return;

      try {
        await invoke("zap_region", {
          x: coords[0],
          y: coords[1],
          minRegionSize: 4,
        });
        onEditApplied();
      } catch (err) {
        console.error("Failed to zap region:", err);
      }
    },
    [getImageCoords, onEditApplied]
  );

  const handleMouseLeave = useCallback(() => {
    setHoverPos(null);
  }, []);

  return (
    <div className="relative">
      {/* Overlay for capturing mouse events */}
      <div
        className="absolute inset-0"
        style={{ cursor: "crosshair", zIndex: 10 }}
        onMouseMove={handleMouseMove}
        onClick={handleClick}
        onMouseLeave={handleMouseLeave}
      >
        {/* Show split preview line on hover */}
        {hoverPos && (
          <svg
            className="absolute inset-0 w-full h-full pointer-events-none"
            style={{ zIndex: 11 }}
          >
            <line
              x1={(hoverPos.x - 10) * zoom + panX}
              y1={hoverPos.y * zoom + panY}
              x2={(hoverPos.x + 10) * zoom + panX}
              y2={hoverPos.y * zoom + panY}
              stroke="#ef4444"
              strokeWidth={1.5}
              strokeDasharray="4 2"
              opacity={0.8}
            />
          </svg>
        )}
      </div>
    </div>
  );
}
