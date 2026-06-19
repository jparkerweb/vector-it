import { useState, useCallback, useEffect, useRef } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { DropZone } from "./components/DropZone";
import { Canvas } from "./components/Canvas";
import { useAppStore } from "./stores/appStore";
import { useProgress } from "./hooks/useProgress";
import { useSettingsStore } from "./stores/settingsStore";
import { InlineColorPicker } from "./components/InlineColorPicker";

type ExportFormat = "svg" | "eps" | "pdf" | "dxf";
type QualityLevel = "Low" | "Medium" | "High";
type PathMode = "polygon" | "spline";

interface Preset {
  label: string;
  description: string;
  quality: QualityLevel;
  colorCount: number;
  pathMode: PathMode;
  speckleFilter: number;
  colorPrecision: number;
  cornerThreshold: number;
}

const PRESETS: Record<string, Preset> = {
  logo: {
    label: "🎯 Logo / Icon",
    description: "Clean shapes, few colors",
    quality: "High", colorCount: 8, pathMode: "polygon",
    speckleFilter: 4, colorPrecision: 6, cornerThreshold: 60,
  },
  illustration: {
    label: "🎨 Illustration",
    description: "Smooth curves, rich colors",
    quality: "High", colorCount: 16, pathMode: "spline",
    speckleFilter: 4, colorPrecision: 6, cornerThreshold: 120,
  },
  photo: {
    label: "📷 Photo",
    description: "Many colors, smooth gradients",
    quality: "Medium", colorCount: 24, pathMode: "spline",
    speckleFilter: 8, colorPrecision: 8, cornerThreshold: 180,
  },
  pixelArt: {
    label: "👾 Pixel Art",
    description: "Sharp edges, exact colors",
    quality: "High", colorCount: 16, pathMode: "polygon",
    speckleFilter: 1, colorPrecision: 8, cornerThreshold: 0,
  },
  minimal: {
    label: "✂️ Minimal",
    description: "Few paths, simple shapes",
    quality: "Low", colorCount: 4, pathMode: "polygon",
    speckleFilter: 10, colorPrecision: 4, cornerThreshold: 60,
  },
  detailed: {
    label: "🔍 Detailed",
    description: "Maximum fidelity",
    quality: "High", colorCount: 32, pathMode: "polygon",
    speckleFilter: 2, colorPrecision: 8, cornerThreshold: 60,
  },
};

function App() {
  const { imageInfo, vectorResult, isProcessing, error, clearError, imagePath, svgSource } =
    useAppStore();
  const { percent, isActive } = useProgress();

  const [quality, setQuality] = useState<QualityLevel>("High");
  const [colorCount, setColorCount] = useState(12);
  const [format, setFormat] = useState<ExportFormat>("svg");
  const [exported, setExported] = useState(false);
  const [copied, setCopied] = useState(false);
  const [pathMode, setPathMode] = useState<PathMode>("polygon");
  const [speckleFilter, setSpeckleFilter] = useState(4);
  const [colorPrecision, setColorPrecision] = useState(6);
  const [cornerThreshold, setCornerThreshold] = useState(60);
  const [antiAlias, setAntiAlias] = useState(true);
  const [bgColor, setBgColor] = useState<string | null>(null); // null = transparent
  const [activePreset, setActivePreset] = useState<string | null>("logo");
  const [autoRender, setAutoRender] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [showReprocessConfirm, setShowReprocessConfirm] = useState(false);
  const pendingReprocessRef = useRef<(() => void) | null>(null);
  const skipAutoRenderRef = useRef(false);
  // Tracks the settings that produced the current vectorResult
  const committedSettingsRef = useRef({
    quality: "High" as QualityLevel,
    colorCount: 12,
    pathMode: "polygon" as PathMode,
    speckleFilter: 4,
    colorPrecision: 6,
    cornerThreshold: 60,
    activePreset: "logo" as string | null,
  });
  const [resizeW, setResizeW] = useState<number | null>(null);
  const [resizeH, setResizeH] = useState<number | null>(null);
  const [lockAspect, setLockAspect] = useState(true);

  // Sync resize inputs when a new image is loaded or vectorResult arrives
  useEffect(() => {
    if (imageInfo) {
      setResizeW(imageInfo.width);
      setResizeH(imageInfo.height);
    }
  }, [imageInfo?.width, imageInfo?.height]);

  useEffect(() => {
    if (vectorResult) {
      setResizeW(vectorResult.dimensions[0]);
      setResizeH(vectorResult.dimensions[1]);
    }
  }, [vectorResult?.dimensions[0], vectorResult?.dimensions[1]]);

  const handleResize = useCallback(() => {
    if (!vectorResult || !resizeW || !resizeH) return;
    const [oldW, oldH] = vectorResult.dimensions;
    if (resizeW === oldW && resizeH === oldH) return;
    const scaleX = resizeW / oldW;
    const scaleY = resizeH / oldH;
    const scaledPaths = vectorResult.paths.map((path) => ({
      ...path,
      segments: path.segments.map((seg) => ({
        ...seg,
        curve: {
          p0: { x: seg.curve.p0.x * scaleX, y: seg.curve.p0.y * scaleY },
          p1: { x: seg.curve.p1.x * scaleX, y: seg.curve.p1.y * scaleY },
          p2: { x: seg.curve.p2.x * scaleX, y: seg.curve.p2.y * scaleY },
          p3: { x: seg.curve.p3.x * scaleX, y: seg.curve.p3.y * scaleY },
        },
      })),
    }));
    useAppStore.setState({
      vectorResult: { ...vectorResult, paths: scaledPaths, dimensions: [resizeW, resizeH] },
    });
  }, [vectorResult, resizeW, resizeH]);

  const handleTrim = useCallback(() => {
    if (!vectorResult || vectorResult.paths.length === 0) return;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const path of vectorResult.paths) {
      for (const seg of path.segments) {
        for (const pt of [seg.curve.p0, seg.curve.p1, seg.curve.p2, seg.curve.p3]) {
          if (pt.x < minX) minX = pt.x;
          if (pt.y < minY) minY = pt.y;
          if (pt.x > maxX) maxX = pt.x;
          if (pt.y > maxY) maxY = pt.y;
        }
      }
    }
    // Add a small 1px padding
    minX = Math.max(0, Math.floor(minX) - 1);
    minY = Math.max(0, Math.floor(minY) - 1);
    maxX = Math.ceil(maxX) + 1;
    maxY = Math.ceil(maxY) + 1;
    const newW = maxX - minX;
    const newH = maxY - minY;
    if (newW <= 0 || newH <= 0) return;
    // Translate all paths so the bounding box starts at (0,0) + padding
    const offsetX = minX;
    const offsetY = minY;
    const trimmedPaths = vectorResult.paths.map((path) => ({
      ...path,
      segments: path.segments.map((seg) => ({
        ...seg,
        curve: {
          p0: { x: seg.curve.p0.x - offsetX, y: seg.curve.p0.y - offsetY },
          p1: { x: seg.curve.p1.x - offsetX, y: seg.curve.p1.y - offsetY },
          p2: { x: seg.curve.p2.x - offsetX, y: seg.curve.p2.y - offsetY },
          p3: { x: seg.curve.p3.x - offsetX, y: seg.curve.p3.y - offsetY },
        },
      })),
    }));
    setResizeW(newW);
    setResizeH(newH);
    useAppStore.setState({
      vectorResult: { ...vectorResult, paths: trimmedPaths, dimensions: [newW, newH] },
    });
  }, [vectorResult]);

  const handlePad = useCallback(() => {
    if (!vectorResult) return;
    const pad = 10;
    const [oldW, oldH] = vectorResult.dimensions;
    const newW = oldW + pad * 2;
    const newH = oldH + pad * 2;
    const paddedPaths = vectorResult.paths.map((path) => ({
      ...path,
      segments: path.segments.map((seg) => ({
        ...seg,
        curve: {
          p0: { x: seg.curve.p0.x + pad, y: seg.curve.p0.y + pad },
          p1: { x: seg.curve.p1.x + pad, y: seg.curve.p1.y + pad },
          p2: { x: seg.curve.p2.x + pad, y: seg.curve.p2.y + pad },
          p3: { x: seg.curve.p3.x + pad, y: seg.curve.p3.y + pad },
        },
      })),
    }));
    setResizeW(newW);
    setResizeH(newH);
    useAppStore.setState({
      vectorResult: { ...vectorResult, paths: paddedPaths, dimensions: [newW, newH] },
    });
  }, [vectorResult]);

  const applyPreset = useCallback((key: string) => {
    const p = PRESETS[key];
    if (!p) return;
    setActivePreset(key);
    setQuality(p.quality);
    setColorCount(p.colorCount);
    setPathMode(p.pathMode);
    setSpeckleFilter(p.speckleFilter);
    setColorPrecision(p.colorPrecision);
    setCornerThreshold(p.cornerThreshold);
  }, []);

  const handleOpen = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "svg"] }],
    });
    if (selected) {
      setExported(false);
      const isSvg = selected.toLowerCase().endsWith(".svg");

      if (isSvg) {
        useAppStore.setState({ isProcessing: true, error: null });
        try {
          const result = await invoke<{ svg_content: string; vector_result: any }>("load_svg_file", { path: selected });
          const dims = result.vector_result.dimensions;
          useAppStore.setState({
            imagePath: selected,
            imageInfo: { width: dims[0], height: dims[1], has_alpha: true, file_size_bytes: 0, thumbnail_base64: "" },
            vectorResult: result.vector_result,
            svgSource: result.svg_content,
            isProcessing: false,
          });
        } catch (e) {
          useAppStore.setState({ error: String(e), isProcessing: false });
        }
      } else {
        useAppStore.setState({ isProcessing: true, error: null, svgSource: null });
        try {
          const info = await invoke<any>("load_image", { path: selected });
          useAppStore.setState({ imagePath: selected, imageInfo: info, vectorResult: null });
          // Auto-vectorize
          const config = {
            color_count: colorCount,
            smoothness: 0.5,
            corner_threshold: cornerThreshold as number,
            simplify_tolerance: 1.0,
            quality,
            path_mode: pathMode,
            speckle_filter: speckleFilter,
            color_precision: colorPrecision,
          };
          const result = await invoke<any>("vectorize", { path: selected, config });
          useAppStore.setState({ vectorResult: result, isProcessing: false });
          committedSettingsRef.current = {
            quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold, activePreset,
          };
        } catch (e) {
          useAppStore.setState({ error: String(e), isProcessing: false });
        }
      }
    }
  }, [quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold, activePreset]);

  const handleReprocess = useCallback(async () => {
    if (!imagePath) return;
    setExported(false);
    useAppStore.setState({ isProcessing: true, error: null });
    try {
      const config = {
        color_count: colorCount,
        smoothness: 0.5,
        corner_threshold: cornerThreshold as number,
        simplify_tolerance: 1.0,
        quality,
        path_mode: pathMode,
        speckle_filter: speckleFilter,
        color_precision: colorPrecision,
      };
      const result = await invoke<any>("vectorize", { path: imagePath, config });
      useAppStore.setState({ vectorResult: result, isProcessing: false });
      // Mark these settings as the ones that produced the current result
      committedSettingsRef.current = {
        quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold, activePreset,
      };
    } catch (e) {
      useAppStore.setState({ error: String(e), isProcessing: false });
    }
  }, [imagePath, quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold, activePreset]);

  /** Wraps handleReprocess with a confirmation if the user has unsaved edits. */
  const guardedReprocess = useCallback(() => {
    if (useAppStore.getState().hasCanvasEdits) {
      pendingReprocessRef.current = () => {
        handleReprocess();
      };
      setShowReprocessConfirm(true);
    } else {
      handleReprocess();
    }
  }, [handleReprocess]);

  const confirmReprocess = useCallback(() => {
    setShowReprocessConfirm(false);
    // Skip the auto-render that fires when showReprocessConfirm changes,
    // because the pending reprocess already handles it.
    skipAutoRenderRef.current = true;
    // User explicitly discarded edits — clear immediately so subsequent
    // setting changes don't trigger the modal again before vectorResult arrives.
    useAppStore.setState({ hasCanvasEdits: false });
    pendingReprocessRef.current?.();
    pendingReprocessRef.current = null;
  }, []);

  const cancelReprocess = useCallback(() => {
    setShowReprocessConfirm(false);
    pendingReprocessRef.current = null;
    // Restore settings to the last successfully processed values
    const s = committedSettingsRef.current;
    skipAutoRenderRef.current = true;
    setQuality(s.quality);
    setColorCount(s.colorCount);
    setPathMode(s.pathMode);
    setSpeckleFilter(s.speckleFilter);
    setColorPrecision(s.colorPrecision);
    setCornerThreshold(s.cornerThreshold);
    setActivePreset(s.activePreset);
  }, []);

  // Clipboard paste: Ctrl+V / Cmd+V with an image on the clipboard
  useEffect(() => {
    const handlePaste = async (e: ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;
      for (const item of Array.from(items)) {
        if (item.type.startsWith("image/")) {
          e.preventDefault();
          const blob = item.getAsFile();
          if (!blob) return;
          const arrayBuf = await blob.arrayBuffer();
          const bytes = Array.from(new Uint8Array(arrayBuf));

          setExported(false);
          useAppStore.setState({ isProcessing: true, error: null });
          try {
            const res = await invoke<{ path: string; info: any }>("paste_image", { pngData: bytes });
            useAppStore.setState({ imagePath: res.path, imageInfo: res.info, vectorResult: null });
            const config = {
              color_count: colorCount,
              smoothness: 0.5,
              corner_threshold: cornerThreshold as number,
              simplify_tolerance: 1.0,
              quality,
              path_mode: pathMode,
              speckle_filter: speckleFilter,
              color_precision: colorPrecision,
            };
            const result = await invoke<any>("vectorize", { path: res.path, config });
            useAppStore.setState({ vectorResult: result, isProcessing: false });
          } catch (err) {
            useAppStore.setState({ error: String(err), isProcessing: false });
          }
          return;
        }
      }
    };
    window.addEventListener("paste", handlePaste);
    return () => window.removeEventListener("paste", handlePaste);
  }, [quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold]);

  // Auto-render: debounced re-process when any setting changes
  useEffect(() => {
    if (!autoRender || !imagePath || !vectorResult || showReprocessConfirm) return;
    if (skipAutoRenderRef.current) {
      skipAutoRenderRef.current = false;
      return;
    }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      guardedReprocess();
    }, 500);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [autoRender, quality, colorCount, pathMode, speckleFilter, colorPrecision, cornerThreshold, guardedReprocess, showReprocessConfirm]);

  // Ctrl+S: Quick Save
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        if (!vectorResult && !svgSource) return;
        const path = await useSettingsStore.getState().quickSave();
        if (path) setExported(true);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [vectorResult, svgSource]);

  const handleExport = useCallback(async () => {
    if (!vectorResult && !svgSource) return;

    // SVG source mode: only export as SVG (write directly)
    if (svgSource) {
      const path = await save({ filters: [{ name: "SVG", extensions: ["svg"] }], defaultPath: "output.svg" });
      if (path) {
        try {
          await invoke("write_file", { path, content: svgSource });
          const dir = path.substring(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));
          useSettingsStore.getState().setLastExport("svg", dir, path);
          setExported(true);
        } catch (e) {
          useAppStore.setState({ error: String(e) });
        }
      }
      return;
    }

    const filters: Record<ExportFormat, { name: string; extensions: string[] }> = {
      svg: { name: "SVG", extensions: ["svg"] },
      eps: { name: "EPS", extensions: ["eps"] },
      pdf: { name: "PDF", extensions: ["pdf"] },
      dxf: { name: "DXF", extensions: ["dxf"] },
    };
    const path = await save({ filters: [filters[format]], defaultPath: `output.${format}` });
    if (path) {
      try {
        if (format === "dxf") {
          await invoke("export_dxf", { result: vectorResult, outputPath: path, lineOnly: false, segmentsPerCurve: null });
        } else {
          await invoke(`export_${format}`, { result: vectorResult, outputPath: path });
        }
        const dir = path.substring(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));
        useSettingsStore.getState().setLastExport(format, dir, path);
        setExported(true);
      } catch (e) {
        useAppStore.setState({ error: String(e) });
      }
    }
  }, [vectorResult, svgSource, format]);

  return (
    <div className="flex flex-col h-screen bg-gray-50">
      {/* Top toolbar */}
      <div className="flex items-center gap-3 px-4 py-2.5 bg-white border-b border-gray-200 shadow-sm">
        <button
          onClick={handleOpen}
          disabled={isProcessing}
          className="px-4 py-1.5 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 font-medium text-sm"
        >
          Open
        </button>
        <button
          onClick={() => {
            // Programmatically trigger paste from clipboard
            document.dispatchEvent(new Event("paste-trigger"));
            navigator.clipboard.read().then(async (items) => {
              // Check for SVG content (image/svg+xml or text with SVG)
              for (const item of items) {
                if (item.types.includes("image/svg+xml")) {
                  const blob = await item.getType("image/svg+xml");
                  const svgText = await blob.text();
                  if (svgText.includes("<svg")) {
                    setExported(false);
                    useAppStore.setState({ isProcessing: true, error: null });
                    try {
                      const result = await invoke<{ svg_content: string; vector_result: any }>("parse_svg_content", { svgContent: svgText });
                      const dims = result.vector_result.dimensions;
                      useAppStore.setState({
                        imagePath: null,
                        imageInfo: { width: dims[0], height: dims[1], has_alpha: true, file_size_bytes: svgText.length, thumbnail_base64: "" },
                        vectorResult: result.vector_result,
                        svgSource: result.svg_content,
                        isProcessing: false,
                      });
                    } catch (e) {
                      useAppStore.setState({ error: String(e), isProcessing: false });
                    }
                    return;
                  }
                }
              }

              // Check for raster image
              for (const item of items) {
                const imageType = item.types.find(t => t.startsWith("image/"));
                if (imageType) {
                  const blob = await item.getType(imageType);
                  const arrayBuf = await blob.arrayBuffer();
                  const bytes = Array.from(new Uint8Array(arrayBuf));
                  setExported(false);
                  useAppStore.setState({ isProcessing: true, error: null, svgSource: null });
                  try {
                    const res = await invoke<{ path: string; info: any }>("paste_image", { pngData: bytes });
                    useAppStore.setState({ imagePath: res.path, imageInfo: res.info, vectorResult: null });
                    const config = {
                      color_count: colorCount,
                      smoothness: 0.5,
                      corner_threshold: cornerThreshold as number,
                      simplify_tolerance: 1.0,
                      quality,
                      path_mode: pathMode,
                      speckle_filter: speckleFilter,
                      color_precision: colorPrecision,
                    };
                    const result = await invoke<any>("vectorize", { path: res.path, config });
                    useAppStore.setState({ vectorResult: result, isProcessing: false });
                  } catch (err) {
                    useAppStore.setState({ error: String(err), isProcessing: false });
                  }
                  return;
                }
              }

              // Check for SVG as plain text
              try {
                const text = await navigator.clipboard.readText();
                if (text.trim().startsWith("<svg") || text.trim().startsWith("<?xml")) {
                  setExported(false);
                  useAppStore.setState({ isProcessing: true, error: null });
                  try {
                    const result = await invoke<{ svg_content: string; vector_result: any }>("parse_svg_content", { svgContent: text });
                    const dims = result.vector_result.dimensions;
                    useAppStore.setState({
                      imagePath: null,
                      imageInfo: { width: dims[0], height: dims[1], has_alpha: true, file_size_bytes: text.length, thumbnail_base64: "" },
                      vectorResult: result.vector_result,
                      svgSource: result.svg_content,
                      isProcessing: false,
                    });
                  } catch (e) {
                    useAppStore.setState({ error: String(e), isProcessing: false });
                  }
                  return;
                }
              } catch { /* text read failed, fall through */ }

              useAppStore.setState({ error: "No image found on clipboard" });
            }).catch(() => {
              useAppStore.setState({ error: "Could not read clipboard. Try Ctrl+V instead." });
            });
          }}
          disabled={isProcessing}
          className="px-4 py-1.5 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 disabled:opacity-50 font-medium text-sm border border-gray-300"
          title="Paste image from clipboard (or press Ctrl+V)"
        >
          📋 Paste
        </button>
        <button
          onClick={() => {
            setExported(false);
            useAppStore.setState({
              imagePath: null,
              imageInfo: { width: 800, height: 600, has_alpha: true, file_size_bytes: 0, thumbnail_base64: "" },
              vectorResult: { paths: [], palette: { colors: [] }, dimensions: [800, 600], segmentation: { regions: [], label_map: [], width: 800, height: 600 } },
              svgSource: null,
              isProcessing: false,
              error: null,
              hasCanvasEdits: false,
            });
          }}
          disabled={isProcessing}
          className="px-4 py-1.5 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 disabled:opacity-50 font-medium text-sm border border-gray-300"
          title="Start with a blank canvas"
        >
          🗒️ Blank
        </button>

        {imageInfo && (
          <>
            <div className="h-5 w-px bg-gray-300" />
            <span className="text-xs text-gray-500">
              {imageInfo.width} × {imageInfo.height}px
            </span>
          </>
        )}

        <div className="flex-1" />

        {(vectorResult || svgSource) && (
          <div className="flex items-center gap-2">
            {/* Trim/Pad/Resize only for vectorized results, not raw SVG sources */}
            {vectorResult && !svgSource && (
              <>
                <button
                  onClick={handleTrim}
                  className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors border border-gray-300"
                  title="Trim canvas to fit paths"
                >✂ Trim</button>
                <button
                  onClick={handlePad}
                  className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors border border-gray-300"
                  title="Add 10px padding around canvas"
                >⬜ Pad</button>
                {/* Resize controls */}
                <div className="flex items-center gap-1">
                  <input
                    type="number"
                    value={resizeW ?? ""}
                    onChange={(e) => {
                      const w = Math.max(1, Number(e.target.value) || 1);
                      setResizeW(w);
                      if (lockAspect && vectorResult) {
                        const aspect = vectorResult.dimensions[1] / vectorResult.dimensions[0];
                        setResizeH(Math.round(w * aspect));
                      }
                    }}
                    onBlur={handleResize}
                    onKeyDown={(e) => { if (e.key === "Enter") handleResize(); }}
                    className="w-16 px-1.5 py-1 border border-gray-300 rounded text-xs text-center bg-white"
                    title="Output width"
                  />
                  <span className="text-xs text-gray-400">×</span>
                  <input
                    type="number"
                    value={resizeH ?? ""}
                    onChange={(e) => {
                      const h = Math.max(1, Number(e.target.value) || 1);
                      setResizeH(h);
                      if (lockAspect && vectorResult) {
                        const aspect = vectorResult.dimensions[0] / vectorResult.dimensions[1];
                        setResizeW(Math.round(h * aspect));
                      }
                    }}
                    onBlur={handleResize}
                    onKeyDown={(e) => { if (e.key === "Enter") handleResize(); }}
                    className="w-16 px-1.5 py-1 border border-gray-300 rounded text-xs text-center bg-white"
                    title="Output height"
                  />
                  <button
                    onClick={() => setLockAspect(!lockAspect)}
                    className={`px-1.5 py-1 text-xs rounded border transition-colors ${
                      lockAspect ? "bg-blue-50 border-blue-300 text-blue-600" : "bg-white border-gray-300 text-gray-400"
                    }`}
                    title={lockAspect ? "Aspect ratio locked" : "Aspect ratio unlocked"}
                  >
                    {lockAspect ? "🔗" : "🔓"}
                  </button>
                </div>

                <div className="h-5 w-px bg-gray-300" />
              </>
            )}

            {!svgSource && (
              <select
                value={format}
                onChange={(e) => { setFormat(e.target.value as ExportFormat); setExported(false); }}
                className="px-2 py-1.5 border border-gray-300 rounded-md text-sm bg-white"
              >
                <option value="svg">SVG</option>
                <option value="eps">EPS</option>
                <option value="pdf">PDF</option>
                <option value="dxf">DXF</option>
              </select>
            )}
            <button
              onClick={handleExport}
              className="px-4 py-1.5 bg-green-600 text-white rounded-md hover:bg-green-700 font-medium text-sm"
            >
              Export
            </button>
            <button
              onClick={async () => {
                if (!vectorResult && !svgSource) return;
                try {
                  let svgString: string;
                  if (svgSource) {
                    svgString = svgSource;
                  } else {
                    svgString = await invoke<string>("render_svg_string", { result: vectorResult });
                  }
                  await navigator.clipboard.writeText(svgString);
                  setCopied(true);
                  setTimeout(() => setCopied(false), 2000);
                } catch (e) {
                  useAppStore.setState({ error: String(e) });
                }
              }}
              className="px-3 py-1.5 bg-blue-600 text-white rounded-md hover:bg-blue-700 font-medium text-sm"
              title="Copy SVG to clipboard"
            >
              {copied ? "✓ Copied" : "📋 Copy"}
            </button>
            <button
              onClick={async () => {
                const path = await useSettingsStore.getState().quickSave();
                if (path) setExported(true);
              }}
              className="px-3 py-1.5 bg-green-500 text-white rounded-md hover:bg-green-600 font-medium text-sm"
              title="Quick Save (Ctrl+S)"
            >
              ⚡ Save
            </button>
            {exported && <span className="text-green-600 text-xs">✓ Saved</span>}
            {useSettingsStore.getState().lastExportPath && (
              <span
                draggable
                onDragStart={async (e) => {
                  const lastPath = useSettingsStore.getState().lastExportPath;
                  if (lastPath) {
                    e.dataTransfer.setData("text/plain", lastPath);
                    try {
                      await invoke("start_drag", { filePath: lastPath });
                    } catch { /* drag validation only */ }
                  }
                }}
                className="px-2 py-1 cursor-grab text-gray-500 hover:text-gray-700 text-sm"
                title="Drag exported file to other apps"
              >
                📎
              </span>
            )}
          </div>
        )}
      </div>

      {/* Progress bar — fixed height to avoid layout shift */}
      <div className="h-1 bg-gray-200 relative">
        {isActive && (
          <div
            className="h-full bg-blue-500 transition-all duration-300 ease-out"
            style={{ width: `${percent}%` }}
          />
        )}
      </div>

      {/* Error banner */}
      {error && (
        <div className="flex items-center gap-2 px-4 py-2 bg-red-50 border-b border-red-200 text-red-700 text-sm">
          <span className="flex-1">{error}</span>
          <button onClick={clearError} className="text-red-400 hover:text-red-600 font-bold">✕</button>
        </div>
      )}

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden">
        {!imageInfo ? (
          <div className="flex-1 p-8">
            <DropZone />
          </div>
        ) : (
          <>
            {/* Canvas - takes most of the space */}
            <div className="flex-1 flex flex-col overflow-hidden">
              <Canvas antiAlias={antiAlias} bgColor={bgColor} />
            </div>

            {/* Settings sidebar */}
            <div className="w-64 bg-white border-l border-gray-200 flex flex-col overflow-auto">
              {svgSource ? (
                <div className="p-4 space-y-4">
                  <div className="flex items-center gap-2">
                    <span className="text-lg">📐</span>
                    <h3 className="text-sm font-semibold text-gray-700">SVG Source</h3>
                  </div>
                  <p className="text-xs text-gray-500">
                    This file was loaded as a vector SVG. Processing controls are not needed.
                  </p>
                  <div className="text-[11px] text-gray-400 space-y-0.5 pt-2 border-t border-gray-100">
                    <div>Dimensions: {imageInfo?.width} × {imageInfo?.height}</div>
                    <div>Size: {(svgSource.length / 1024).toFixed(1)} KB</div>
                  </div>
                </div>
              ) : !imagePath && vectorResult ? (
                <div className="p-4 space-y-4">
                  <div className="flex items-center gap-2">
                    <span className="text-lg">🗒️</span>
                    <h3 className="text-sm font-semibold text-gray-700">Blank Canvas</h3>
                  </div>
                  <p className="text-xs text-gray-500">
                    Drawing on a blank canvas. Use the editing tools to create your SVG.
                  </p>
                  <div className="text-[11px] text-gray-400 space-y-0.5 pt-2 border-t border-gray-100">
                    <div>Dimensions: {imageInfo?.width} × {imageInfo?.height}</div>
                    <div>Paths: {vectorResult.paths.length}</div>
                  </div>
                </div>
              ) : (
              <>
              <div className="p-4 space-y-5">
                {/* Auto-Render toggle */}
                <div className="flex items-center justify-between">
                  <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Auto-Render</h3>
                  <button
                    onClick={() => setAutoRender(!autoRender)}
                    className={`relative w-9 h-5 rounded-full transition-colors ${autoRender ? "bg-blue-600" : "bg-gray-300"}`}
                  >
                    <span
                      className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full transition-transform ${autoRender ? "translate-x-4" : ""}`}
                    />
                  </button>
                </div>

                <hr className="border-gray-100" />
                <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Preset</h3>

                {/* Preset buttons */}
                <div className="grid grid-cols-2 gap-1.5">
                  {Object.entries(PRESETS).map(([key, preset]) => (
                    <button
                      key={key}
                      onClick={() => applyPreset(key)}
                      className={`px-2 py-1.5 text-[11px] rounded-md border transition-colors text-left ${
                        activePreset === key
                          ? "border-blue-500 bg-blue-50 text-blue-700 font-medium"
                          : "border-gray-200 bg-white text-gray-600 hover:border-gray-300 hover:bg-gray-50"
                      }`}
                      title={preset.description}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>

                <hr className="border-gray-100" />
                <h3 className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Fine-tune</h3>

                {/* Quality */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">Quality</label>
                  <select
                    value={quality}
                    onChange={(e) => { setQuality(e.target.value as QualityLevel); setActivePreset(null); }}
                    className="w-full px-2.5 py-1.5 border border-gray-300 rounded-md text-sm bg-white"
                  >
                    <option value="Low">Low — Fast</option>
                    <option value="Medium">Medium</option>
                    <option value="High">High — Best</option>
                  </select>
                </div>

                {/* Color Count */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">
                    Colors: {colorCount}
                  </label>
                  <input
                    type="range"
                    min={2}
                    max={32}
                    step={1}
                    value={colorCount}
                    onChange={(e) => { setColorCount(parseInt(e.target.value)); setActivePreset(null); }}
                    className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                  />
                  <div className="flex justify-between text-[10px] text-gray-400 mt-0.5">
                    <span>2</span>
                    <span>32</span>
                  </div>
                </div>

                {/* Path Mode */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">Path Mode</label>
                  <select
                    value={pathMode}
                    onChange={(e) => { setPathMode(e.target.value as PathMode); setActivePreset(null); }}
                    className="w-full px-2.5 py-1.5 border border-gray-300 rounded-md text-sm bg-white"
                  >
                    <option value="polygon">Polygon — Sharp edges</option>
                    <option value="spline">Spline — Smooth curves</option>
                  </select>
                </div>

                {/* Speckle Filter */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">
                    Speckle Filter: {speckleFilter}px
                  </label>
                  <input
                    type="range"
                    min={1}
                    max={20}
                    step={1}
                    value={speckleFilter}
                    onChange={(e) => { setSpeckleFilter(parseInt(e.target.value)); setActivePreset(null); }}
                    className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                  />
                  <div className="flex justify-between text-[10px] text-gray-400 mt-0.5">
                    <span>1 (keep detail)</span>
                    <span>20 (clean)</span>
                  </div>
                </div>

                {/* Color Precision */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">
                    Color Precision: {colorPrecision}
                  </label>
                  <input
                    type="range"
                    min={1}
                    max={8}
                    step={1}
                    value={colorPrecision}
                    onChange={(e) => { setColorPrecision(parseInt(e.target.value)); setActivePreset(null); }}
                    className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                  />
                  <div className="flex justify-between text-[10px] text-gray-400 mt-0.5">
                    <span>1 (coarse)</span>
                    <span>8 (fine)</span>
                  </div>
                </div>

                {/* Corner Threshold */}
                <div>
                  <label className="block text-xs font-medium text-gray-600 mb-1.5">
                    Corner Threshold: {cornerThreshold}°
                  </label>
                  <input
                    type="range"
                    min={0}
                    max={180}
                    step={5}
                    value={cornerThreshold}
                    onChange={(e) => { setCornerThreshold(parseInt(e.target.value)); setActivePreset(null); }}
                    className="w-full h-1.5 bg-gray-200 rounded-lg appearance-none cursor-pointer"
                  />
                  <div className="flex justify-between text-[10px] text-gray-400 mt-0.5">
                    <span>0° (all corners)</span>
                    <span>180° (smooth)</span>
                  </div>
                </div>

                {/* Anti-aliasing toggle */}
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-gray-600">Anti-aliasing</label>
                  <button
                    onClick={() => setAntiAlias(!antiAlias)}
                    className={`relative w-9 h-5 rounded-full transition-colors ${antiAlias ? "bg-blue-600" : "bg-gray-300"}`}
                  >
                    <span
                      className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full transition-transform ${antiAlias ? "translate-x-4" : ""}`}
                    />
                  </button>
                </div>

                {/* Background color */}
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-gray-600">Background</label>
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={() => setBgColor(bgColor === null ? "#ffffff" : null)}
                      className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${
                        bgColor === null ? "bg-blue-50 border-blue-300 text-blue-600" : "bg-white border-gray-300 text-gray-500"
                      }`}
                      title="Toggle transparent background"
                    >
                      {bgColor === null ? "Transparent" : "Color"}
                    </button>
                    {bgColor !== null && (
                      <InlineColorPicker value={bgColor} onChange={setBgColor} />
                    )}
                  </div>
                </div>

                {/* Re-process */}
                <button
                  onClick={guardedReprocess}
                  disabled={isProcessing}
                  className="w-full px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 transition-colors"
                >
                  {isProcessing ? "Processing..." : "Re-process"}
                  </button>
              </div>

              {/* Info footer */}
              {vectorResult && (
                <div className="mt-auto p-4 border-t border-gray-100">
                  <div className="text-[11px] text-gray-400 space-y-0.5">
                    <div>Paths: {vectorResult.paths?.length ?? 0}</div>
                    <div>Quality: {quality}</div>
                    <div>Colors: {colorCount}</div>
                  </div>
                </div>
              )}
              </>
              )}
            </div>
          </>
        )}
      </div>

      {/* Reprocess confirmation dialog */}
      {showReprocessConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-sm mx-4">
            <h3 className="text-base font-semibold text-gray-900 mb-2">Discard edits?</h3>
            <p className="text-sm text-gray-600 mb-5">
              Re-processing will discard all your manual edits. This cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={cancelReprocess}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmReprocess}
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

export default App;
