import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FinderToolProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  zoom: number;
  panX: number;
  panY: number;
  onEditApplied: () => void;
}

export function FinderTool({
  canvasRef,
  zoom,
  panX,
  panY,
  onEditApplied,
}: FinderToolProps) {
  const [artifacts, setArtifacts] = useState<[number, number][]>([]);
  const [isScanning, setIsScanning] = useState(false);

  const scanForArtifacts = useCallback(async () => {
    setIsScanning(true);
    try {
      const result = await invoke<[number, number][]>("find_artifacts");
      setArtifacts(result);
    } catch (e) {
      console.error("Failed to find artifacts:", e);
    } finally {
      setIsScanning(false);
    }
  }, []);

  // Scan on mount
  useEffect(() => {
    scanForArtifacts();
  }, [scanForArtifacts]);

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

      // Find nearest artifact pixel
      const [cx, cy] = coords;
      let nearest: [number, number] | null = null;
      let minDist = Infinity;

      for (const [ax, ay] of artifacts) {
        const dist = Math.abs(ax - cx) + Math.abs(ay - cy);
        if (dist < minDist && dist <= 3) {
          minDist = dist;
          nearest = [ax, ay];
        }
      }

      if (!nearest) return;

      try {
        await invoke("fix_artifact", { x: nearest[0], y: nearest[1] });
        onEditApplied();
        // Rescan after fix
        scanForArtifacts();
      } catch (err) {
        console.error("Failed to fix artifact:", err);
      }
    },
    [getImageCoords, artifacts, onEditApplied, scanForArtifacts]
  );

  return (
    <div className="relative">
      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={scanForArtifacts}
          disabled={isScanning}
          className="px-3 py-1 text-xs font-medium bg-orange-100 text-orange-700 rounded hover:bg-orange-200 disabled:opacity-50"
        >
          {isScanning ? "Scanning..." : "Rescan"}
        </button>
        <span className="text-xs text-gray-500">
          {artifacts.length} artifact{artifacts.length !== 1 ? "s" : ""} found
        </span>
      </div>

      {/* Overlay to show artifacts and capture clicks */}
      <div
        className="absolute inset-0"
        style={{ cursor: "pointer", zIndex: 10 }}
        onClick={handleClick}
      >
        {/* Render artifact highlights as pulsing red dots */}
        {artifacts.map(([x, y], i) => (
          <div
            key={i}
            className="absolute w-2 h-2 rounded-full bg-red-500 animate-pulse"
            style={{
              left: `${x * zoom + panX}px`,
              top: `${y * zoom + panY}px`,
              transform: "translate(-50%, -50%)",
            }}
          />
        ))}
      </div>
    </div>
  );
}
