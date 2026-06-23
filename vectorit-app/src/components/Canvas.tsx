import { useRef, useEffect, useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore, VectorPath, VectorizationResult, BezierSegment, Point, RgbColor } from "../stores/appStore";
import { InlineColorPicker } from "./InlineColorPicker";
import { ShapePicker } from "./ShapePicker";
import {
  type ShapeType,
  generateEllipse,
  generateRect,
  generateTriangle,
  generateStar,
  generatePolygon,
  generateHeart,
} from "../utils/shapeGenerators";

type CanvasMode = "pan" | "select" | "draw" | "erase" | "recolor" | "reshape" | "shape";

// Identifies a draggable control point within a path
interface ControlPointRef {
  segIndex: number;
  point: "p0" | "p1" | "p2" | "p3";
}

// Ramer-Douglas-Peucker line simplification
function rdpSimplify(points: Point[], epsilon: number): Point[] {
  if (points.length <= 2) return points;
  let maxDist = 0;
  let maxIdx = 0;
  const start = points[0];
  const end = points[points.length - 1];
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const lenSq = dx * dx + dy * dy;

  for (let i = 1; i < points.length - 1; i++) {
    let dist: number;
    if (lenSq === 0) {
      dist = Math.hypot(points[i].x - start.x, points[i].y - start.y);
    } else {
      const t = Math.max(0, Math.min(1, ((points[i].x - start.x) * dx + (points[i].y - start.y) * dy) / lenSq));
      const px = start.x + t * dx;
      const py = start.y + t * dy;
      dist = Math.hypot(points[i].x - px, points[i].y - py);
    }
    if (dist > maxDist) { maxDist = dist; maxIdx = i; }
  }

  if (maxDist > epsilon) {
    const left = rdpSimplify(points.slice(0, maxIdx + 1), epsilon);
    const right = rdpSimplify(points.slice(maxIdx), epsilon);
    return [...left.slice(0, -1), ...right];
  }
  return [start, end];
}

// Convert a polyline to cubic bezier segments
function pointsToBezierSegments(pts: Point[]): BezierSegment[] {
  if (pts.length < 2) return [];
  const segments: BezierSegment[] = [];
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i];
    const p3 = pts[i + 1];
    const p1 = { x: p0.x + (p3.x - p0.x) / 3, y: p0.y + (p3.y - p0.y) / 3 };
    const p2 = { x: p0.x + (p3.x - p0.x) * 2 / 3, y: p0.y + (p3.y - p0.y) * 2 / 3 };
    segments.push({ curve: { p0, p1, p2, p3 }, is_corner_start: false });
  }
  return segments;
}

function floodFillEmpty(imageData: ImageData, startX: number, startY: number): Set<number> {
  const { width, height, data } = imageData;
  if (startX < 0 || startY < 0 || startX >= width || startY >= height) {
    return new Set();
  }

  const startOffset = (startY * width + startX) * 4;
  if (data[startOffset] !== 0 || data[startOffset + 1] !== 0 || data[startOffset + 2] !== 0) {
    return new Set();
  }

  const visited = new Uint8Array(width * height);
  const queue: number[] = [startY * width + startX];
  const pixels = new Set<number>();
  visited[startY * width + startX] = 1;

  for (let head = 0; head < queue.length; head++) {
    const idx = queue[head];
    pixels.add(idx);
    const x = idx % width;
    const y = Math.floor(idx / width);

    const neighbors = [
      [x - 1, y],
      [x + 1, y],
      [x, y - 1],
      [x, y + 1],
    ];

    for (const [nx, ny] of neighbors) {
      if (nx < 0 || ny < 0 || nx >= width || ny >= height) continue;
      const nidx = ny * width + nx;
      if (visited[nidx]) continue;
      visited[nidx] = 1;
      const offset = nidx * 4;
      if (data[offset] === 0 && data[offset + 1] === 0 && data[offset + 2] === 0) {
        queue.push(nidx);
      }
    }
  }

  return pixels;
}

function traceBoundary(pixels: Set<number>, width: number, height: number): Point[] {
  if (pixels.size === 0) return [];

  type GridPoint = { x: number; y: number };
  type Edge = { start: GridPoint; end: GridPoint; used: boolean };

  const pointKey = ({ x, y }: GridPoint) => `${x},${y}`;
  const edges: Edge[] = [];
  const outgoing = new Map<string, number[]>();

  const addEdge = (start: GridPoint, end: GridPoint) => {
    const index = edges.push({ start, end, used: false }) - 1;
    const key = pointKey(start);
    const bucket = outgoing.get(key);
    if (bucket) bucket.push(index);
    else outgoing.set(key, [index]);
  };

  const hasPixel = (x: number, y: number) =>
    x >= 0 && y >= 0 && x < width && y < height && pixels.has(y * width + x);

  for (const idx of pixels) {
    const x = idx % width;
    const y = Math.floor(idx / width);
    if (!hasPixel(x, y - 1)) addEdge({ x, y }, { x: x + 1, y });
    if (!hasPixel(x + 1, y)) addEdge({ x: x + 1, y }, { x: x + 1, y: y + 1 });
    if (!hasPixel(x, y + 1)) addEdge({ x: x + 1, y: y + 1 }, { x, y: y + 1 });
    if (!hasPixel(x - 1, y)) addEdge({ x, y: y + 1 }, { x, y });
  }

  const dirIndex = (edge: Edge) => {
    const dx = edge.end.x - edge.start.x;
    const dy = edge.end.y - edge.start.y;
    if (dx === 1) return 0;
    if (dy === 1) return 1;
    if (dx === -1) return 2;
    return 3;
  };

  const chooseNextEdge = (currentDir: number, candidates: number[]) => {
    for (const turn of [1, 0, 3, 2]) {
      const wanted = (currentDir + turn) % 4;
      const match = candidates.find((index) => dirIndex(edges[index]) === wanted);
      if (match !== undefined) return match;
    }
    return candidates[0];
  };

  const polygonArea = (points: Point[]) => {
    let area = 0;
    for (let i = 0; i < points.length - 1; i++) {
      area += points[i].x * points[i + 1].y - points[i + 1].x * points[i].y;
    }
    return area / 2;
  };

  const loops: Point[][] = [];
  for (let startEdge = 0; startEdge < edges.length; startEdge++) {
    if (edges[startEdge].used) continue;

    const loop: Point[] = [];
    const startKey = pointKey(edges[startEdge].start);
    let currentEdgeIndex = startEdge;

    while (true) {
      const edge = edges[currentEdgeIndex];
      if (edge.used) break;
      edge.used = true;
      loop.push({ x: edge.start.x, y: edge.start.y });

      const nextCandidates = (outgoing.get(pointKey(edge.end)) ?? []).filter(
        (index) => !edges[index].used
      );
      if (nextCandidates.length === 0 || pointKey(edge.end) === startKey) {
        loop.push({ x: edge.end.x, y: edge.end.y });
        break;
      }

      currentEdgeIndex = chooseNextEdge(dirIndex(edge), nextCandidates);
    }

    if (loop.length > 2) {
      const first = loop[0];
      const last = loop[loop.length - 1];
      loops.push(
        first.x === last.x && first.y === last.y ? loop : [...loop, { ...first }]
      );
    }
  }

  if (loops.length === 0) return [];

  const largestLoop = loops.reduce((best, loop) =>
    Math.abs(polygonArea(loop)) > Math.abs(polygonArea(best)) ? loop : best
  );
  const closed = rdpSimplify(largestLoop, 0.75);
  if (closed.length < 3) return [];

  const first = closed[0];
  const last = closed[closed.length - 1];
  return first.x === last.x && first.y === last.y ? closed : [...closed, { ...first }];
}

function polygonToBezierSegments(points: Point[]): BezierSegment[] {
  if (points.length < 4) return [];
  const segments: BezierSegment[] = [];
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i];
    const p3 = points[i + 1];
    const p1 = { x: p0.x + (p3.x - p0.x) / 3, y: p0.y + (p3.y - p0.y) / 3 };
    const p2 = { x: p0.x + ((p3.x - p0.x) * 2) / 3, y: p0.y + ((p3.y - p0.y) * 2) / 3 };
    segments.push({ curve: { p0, p1, p2, p3 }, is_corner_start: true });
  }
  return segments;
}

// De Casteljau split: split a cubic bezier at parameter t into two cubic beziers
function splitBezierAt(curve: { p0: Point; p1: Point; p2: Point; p3: Point }, t: number): [typeof curve, typeof curve] {
  const { p0, p1, p2, p3 } = curve;
  // First level
  const a = { x: p0.x + (p1.x - p0.x) * t, y: p0.y + (p1.y - p0.y) * t };
  const b = { x: p1.x + (p2.x - p1.x) * t, y: p1.y + (p2.y - p1.y) * t };
  const c = { x: p2.x + (p3.x - p2.x) * t, y: p2.y + (p3.y - p2.y) * t };
  // Second level
  const d = { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
  const e = { x: b.x + (c.x - b.x) * t, y: b.y + (c.y - b.y) * t };
  // Split point
  const f = { x: d.x + (e.x - d.x) * t, y: d.y + (e.y - d.y) * t };

  return [
    { p0, p1: a, p2: d, p3: f },
    { p0: f, p1: e, p2: c, p3 },
  ];
}

// Evaluate a cubic bezier at parameter t
function evalBezier(curve: { p0: Point; p1: Point; p2: Point; p3: Point }, t: number): Point {
  const { p0, p1, p2, p3 } = curve;
  const mt = 1 - t;
  return {
    x: mt * mt * mt * p0.x + 3 * mt * mt * t * p1.x + 3 * mt * t * t * p2.x + t * t * t * p3.x,
    y: mt * mt * mt * p0.y + 3 * mt * mt * t * p1.y + 3 * mt * t * t * p2.y + t * t * t * p3.y,
  };
}

// Find closest point on a cubic bezier curve, returns { t, distSq }
function closestPointOnBezier(curve: { p0: Point; p1: Point; p2: Point; p3: Point }, pt: Point, steps = 50): { t: number; distSq: number } {
  let bestT = 0;
  let bestDistSq = Infinity;
  // Coarse search
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const bp = evalBezier(curve, t);
    const dx = bp.x - pt.x;
    const dy = bp.y - pt.y;
    const dSq = dx * dx + dy * dy;
    if (dSq < bestDistSq) { bestDistSq = dSq; bestT = t; }
  }
  // Refine with binary-style narrowing
  let lo = Math.max(0, bestT - 1 / steps);
  let hi = Math.min(1, bestT + 1 / steps);
  for (let iter = 0; iter < 20; iter++) {
    const t1 = lo + (hi - lo) / 3;
    const t2 = hi - (hi - lo) / 3;
    const p1 = evalBezier(curve, t1);
    const p2 = evalBezier(curve, t2);
    const d1 = (p1.x - pt.x) ** 2 + (p1.y - pt.y) ** 2;
    const d2 = (p2.x - pt.x) ** 2 + (p2.y - pt.y) ** 2;
    if (d1 < d2) { hi = t2; if (d1 < bestDistSq) { bestDistSq = d1; bestT = t1; } }
    else { lo = t1; if (d2 < bestDistSq) { bestDistSq = d2; bestT = t2; } }
  }
  return { t: bestT, distSq: bestDistSq };
}

// Simplify a bezier path by sampling, applying RDP, and fitting new smooth cubics
function simplifyBezierPath(path: VectorPath, epsilon: number): BezierSegment[] {
  const { segments } = path;
  if (segments.length <= 1) return segments;

  // Sample points along the path
  const samplesPerSeg = 10;
  const points: Point[] = [];
  for (const seg of segments) {
    for (let i = 0; i < samplesPerSeg; i++) {
      points.push(evalBezier(seg.curve, i / samplesPerSeg));
    }
  }
  // Add the final endpoint
  points.push(evalBezier(segments[segments.length - 1].curve, 1));

  // Apply RDP simplification
  const simplified = rdpSimplify(points, epsilon);
  if (simplified.length < 2) return segments;

  // Fit smooth cubic bezier segments through the simplified points
  // using Catmull-Rom tangent estimation for smooth curves
  const newSegs: BezierSegment[] = [];
  for (let i = 0; i < simplified.length - 1; i++) {
    const p0 = simplified[i];
    const p3 = simplified[i + 1];

    // Estimate tangent at p0 and p3 using neighboring points (Catmull-Rom style)
    const prev = i > 0 ? simplified[i - 1] : p0;
    const next = i + 2 < simplified.length ? simplified[i + 2] : p3;

    const segLen = Math.hypot(p3.x - p0.x, p3.y - p0.y);
    const tension = 1 / 3;

    const t0x = (p3.x - prev.x) * tension;
    const t0y = (p3.y - prev.y) * tension;
    const t1x = (next.x - p0.x) * tension;
    const t1y = (next.y - p0.y) * tension;

    // Scale tangent so control point distance is proportional to segment length
    const t0Len = Math.hypot(t0x, t0y);
    const t1Len = Math.hypot(t1x, t1y);
    const scale0 = t0Len > 0 ? (segLen * tension) / t0Len : 0;
    const scale1 = t1Len > 0 ? (segLen * tension) / t1Len : 0;

    const p1 = { x: p0.x + t0x * scale0, y: p0.y + t0y * scale0 };
    const p2 = { x: p3.x - t1x * scale1, y: p3.y - t1y * scale1 };

    newSegs.push({ curve: { p0, p1, p2, p3 }, is_corner_start: false });
  }
  return newSegs;
}

// Identifies an anchor point (p0 of a segment, or p3 of last segment for open paths)
interface AnchorRef {
  segIndex: number;
  isEnd: boolean; // true = p3 of this segment (only for the last anchor of open paths)
}

function hexToRgb(hex: string) {
  return {
    r: parseInt(hex.slice(1, 3), 16),
    g: parseInt(hex.slice(3, 5), 16),
    b: parseInt(hex.slice(5, 7), 16),
  };
}

function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("");
}

export function Canvas({ antiAlias = true, bgColor = null }: { antiAlias?: boolean; bgColor?: string | null }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const hitCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const { imageInfo, vectorResult } = useAppStore();

  const [zoom, setZoom] = useState(1);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [isPanning, setIsPanning] = useState(false);
  const [lastMouse, setLastMouse] = useState({ x: 0, y: 0 });
  const [spaceDown, setSpaceDown] = useState(false);
  const [mode, setMode] = useState<CanvasMode>("pan");
  const [selectedPaths, setSelectedPaths] = useState<Set<number>>(new Set());
  const [hoveredPath, setHoveredPath] = useState<number | null>(null);

  // Drawing / painting state
  const [drawColor, setDrawColor] = useState("#000000");
  const [brushSize, setBrushSize] = useState(8);
  const [isDrawing, setIsDrawing] = useState(false);
  const drawPointsRef = useRef<Point[]>([]);
  const overlayCanvasRef = useRef<HTMLCanvasElement>(null);

  // Reshape tool state
  const [reshapePath, setReshapePath] = useState<number | null>(null);
  const [dragCP, setDragCP] = useState<ControlPointRef | null>(null);
  const [showControlHandles, setShowControlHandles] = useState(true);
  const [selectedAnchor, setSelectedAnchor] = useState<AnchorRef | null>(null);
  const [scalePercent, setScalePercent] = useState(10);
  const pendingDragRef = useRef<{ cp: ControlPointRef; startPt: Point } | null>(null);
  const hasDraggedRef = useRef(false);

  // Select-tool move drag state
  const [isDraggingSelection, setIsDraggingSelection] = useState(false);
  const dragStartImagePtRef = useRef<Point | null>(null);
  const pendingSelectDragRef = useRef<{ idx: number | null; startPt: Point; shiftKey: boolean } | null>(null);
  const isMovingSelectionRef = useRef(false);

  // Select-tool rotation handle state
  const [isRotatingSelection, setIsRotatingSelection] = useState(false);
  const rotationStartAngleRef = useRef<number>(0);
  const rotationCenterRef = useRef<Point>({ x: 0, y: 0 });

  // Shape tool state
  const [activeShape, setActiveShape] = useState<ShapeType>("ellipse");
  const [shapeStrokeColor, setShapeStrokeColor] = useState("#000000");
  const [shapeStrokeWidth, setShapeStrokeWidth] = useState(0);
  const [shapeCornerRadius, setShapeCornerRadius] = useState(0);
  const [shapeStarPoints, setShapeStarPoints] = useState(5);
  const [shapePolygonSides, setShapePolygonSides] = useState(6);
  const [shapeInnerRadiusRatio, setShapeInnerRadiusRatio] = useState(0.4);
  const [isShapeDragging, setIsShapeDragging] = useState(false);
  const shapeDragStartRef = useRef<Point | null>(null);
  const [shapeDragCurrent, setShapeDragCurrent] = useState<Point | null>(null);
  const [shapeShiftHeld, setShapeShiftHeld] = useState(false);
  const [shapeAltHeld, setShapeAltHeld] = useState(false);

  // Track mouse position for brush cursor
  const [mousePos, setMousePos] = useState({ x: -200, y: -200 });
  const [overControls, setOverControls] = useState(false);

  // Recolor swatch bar state
  const [customSwatches, setCustomSwatches] = useState<string[]>([]);
  const [showSwatchPicker, setShowSwatchPicker] = useState(false);
  const swatchPickerRef = useRef<HTMLDivElement>(null);

  // Extract unique colors from current paths for the recolor swatch bar
  const paletteSwatches = useMemo(() => {
    if (!vectorResult) return [];
    const seen = new Set<string>();
    const bucket = new Set<string>();
    const colors: string[] = [];
    for (const path of vectorResult.paths) {
      const { r, g, b } = path.fill_color;
      const hex = rgbToHex(r, g, b).toLowerCase();
      const snap = (v: number) => Math.round(v / 8) * 8;
      const bucketKey = `${snap(r)},${snap(g)},${snap(b)}`;
      if (!bucket.has(bucketKey) && !seen.has(hex)) {
        seen.add(hex);
        bucket.add(bucketKey);
        colors.push(hex);
      }
    }
    return colors;
  }, [vectorResult]);

  // Close swatch picker on outside click
  useEffect(() => {
    if (!showSwatchPicker) return;
    const handler = (e: MouseEvent) => {
      if (swatchPickerRef.current && !swatchPickerRef.current.contains(e.target as Node)) {
        setShowSwatchPicker(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showSwatchPicker]);

  // Undo/redo history
  const undoStackRef = useRef<VectorPath[][]>([]);
  const redoStackRef = useRef<VectorPath[][]>([]);
  const [undoCount, setUndoCount] = useState(0);
  const [redoCount, setRedoCount] = useState(0);

  // Reset history only when a new image is loaded (dimensions change)
  const lastDimsForHistoryRef = useRef<string>("");
  useEffect(() => {
    if (vectorResult) {
      const dimsKey = vectorResult.dimensions.join(",");
      if (dimsKey !== lastDimsForHistoryRef.current) {
        lastDimsForHistoryRef.current = dimsKey;
        undoStackRef.current = [];
        redoStackRef.current = [];
        setUndoCount(0);
        setRedoCount(0);
        useAppStore.setState({ hasCanvasEdits: false });
      }
    }
  }, [vectorResult]);

  // Clear selection when vector result changes (but not during move/reorder operations)
  const preserveSelectionRef = useRef(false);
  useEffect(() => {
    if (isMovingSelectionRef.current || preserveSelectionRef.current) {
      preserveSelectionRef.current = false;
      return;
    }
    setSelectedPaths(new Set());
    setHoveredPath(null);
  }, [vectorResult]);

  // Build hit-test canvas (offscreen)
  useEffect(() => {
    if (!vectorResult) return;
    const [width, height] = vectorResult.dimensions;
    const hitCanvas = document.createElement("canvas");
    hitCanvas.width = width;
    hitCanvas.height = height;
    const ctx = hitCanvas.getContext("2d", { willReadFrequently: true });
    if (!ctx) return;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, width, height);

    vectorResult.paths.forEach((path, i) => {
      if (path.segments.length === 0) return;
      const id = i + 1;
      const r = (id >> 16) & 0xff;
      const g = (id >> 8) & 0xff;
      const b = id & 0xff;
      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.beginPath();
      const first = path.segments[0].curve;
      ctx.moveTo(first.p0.x, first.p0.y);
      for (const seg of path.segments) {
        const { p1, p2, p3 } = seg.curve;
        ctx.bezierCurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
      }
      if (path.is_closed) ctx.closePath();
      ctx.fill();
    });

    hitCanvasRef.current = hitCanvas;
  }, [vectorResult]);

  // Convert mouse event to image coordinates
  const toImageCoords = useCallback(
    (e: React.MouseEvent): Point | null => {
      const container = containerRef.current;
      if (!container || !vectorResult) return null;
      const rect = container.getBoundingClientRect();
      const x = (e.clientX - rect.left - panX) / zoom;
      const y = (e.clientY - rect.top - panY) / zoom;
      const [w, h] = vectorResult.dimensions;
      if (x < 0 || y < 0 || x >= w || y >= h) return null;
      return { x, y };
    },
    [panX, panY, zoom, vectorResult]
  );

  // Hit-test: mouse event → path index
  const hitTest = useCallback(
    (e: React.MouseEvent): number | null => {
      const hitCanvas = hitCanvasRef.current;
      const pt = toImageCoords(e);
      if (!hitCanvas || !pt) return null;
      const ctx = hitCanvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) return null;
      const pixel = ctx.getImageData(Math.floor(pt.x), Math.floor(pt.y), 1, 1).data;
      const id = (pixel[0] << 16) | (pixel[1] << 8) | pixel[2];
      if (id === 0) return null;
      return id - 1;
    },
    [toImageCoords]
  );

  // Push current paths onto undo stack
  const pushUndo = useCallback(() => {
    if (!vectorResult) return;
    undoStackRef.current.push([...vectorResult.paths]);
    redoStackRef.current = [];
    setUndoCount(undoStackRef.current.length);
    setRedoCount(0);
    useAppStore.setState({ hasCanvasEdits: true });
  }, [vectorResult]);

  // Erase paths touching a brush circle at image coords
  const eraseAt = useCallback(
    (imgPt: Point) => {
      if (!vectorResult) return;
      const hitCanvas = hitCanvasRef.current;
      if (!hitCanvas) return;
      const ctx = hitCanvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) return;

      const radius = brushSize / 2;
      const toRemove = new Set<number>();
      for (let dy = -radius; dy <= radius; dy += 2) {
        for (let dx = -radius; dx <= radius; dx += 2) {
          if (dx * dx + dy * dy > radius * radius) continue;
          const sx = Math.floor(imgPt.x + dx);
          const sy = Math.floor(imgPt.y + dy);
          if (sx < 0 || sy < 0 || sx >= hitCanvas.width || sy >= hitCanvas.height) continue;
          const pixel = ctx.getImageData(sx, sy, 1, 1).data;
          const id = (pixel[0] << 16) | (pixel[1] << 8) | pixel[2];
          if (id > 0) toRemove.add(id - 1);
        }
      }
      if (toRemove.size > 0) {
        pushUndo();
        const newPaths = vectorResult.paths.filter((_, i) => !toRemove.has(i));
        useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
      }
    },
    [vectorResult, brushSize, pushUndo]
  );

  // Draw overlay canvas (preview stroke while drawing)
  useEffect(() => {
    if (isShapeDragging) return; // Shape preview handles overlay separately
    const overlay = overlayCanvasRef.current;
    if (!overlay || !vectorResult) return;
    const [width, height] = vectorResult.dimensions;
    const dpr = window.devicePixelRatio || 1;
    const renderScale = dpr * zoom;
    overlay.width = width * renderScale;
    overlay.height = height * renderScale;
    overlay.style.width = `${width}px`;
    overlay.style.height = `${height}px`;
    const ctx = overlay.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, overlay.width, overlay.height);

    if (!isDrawing || drawPointsRef.current.length < 2) return;

    ctx.scale(renderScale, renderScale);
    ctx.strokeStyle = drawColor;
    ctx.lineWidth = brushSize;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    const pts = drawPointsRef.current;
    ctx.moveTo(pts[0].x, pts[0].y);
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(pts[i].x, pts[i].y);
    }
    ctx.stroke();
  });

  // Shape preview overlay
  useEffect(() => {
    if (!isShapeDragging) return;
    const overlay = overlayCanvasRef.current;
    if (!overlay || !vectorResult) return;
    const [width, height] = vectorResult.dimensions;
    const dpr = window.devicePixelRatio || 1;
    const renderScale = dpr * zoom;
    overlay.width = width * renderScale;
    overlay.height = height * renderScale;
    overlay.style.width = `${width}px`;
    overlay.style.height = `${height}px`;
    const ctx = overlay.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, overlay.width, overlay.height);

    const start = shapeDragStartRef.current;
    const end = shapeDragCurrent;
    if (!start || !end) return;

    const { effectiveStart, effectiveEnd } = applyShapeConstraints(start, end, shapeShiftHeld, shapeAltHeld);
    const segments = buildShapeSegments(activeShape, effectiveStart, effectiveEnd, {
      cornerRadius: shapeCornerRadius,
      starPoints: shapeStarPoints,
      polygonSides: shapePolygonSides,
      innerRadiusRatio: shapeInnerRadiusRatio,
    });
    if (!segments || segments.length === 0) return;

    ctx.scale(renderScale, renderScale);
    ctx.beginPath();
    const first = segments[0].curve;
    ctx.moveTo(first.p0.x, first.p0.y);
    for (const seg of segments) {
      const { p1, p2, p3 } = seg.curve;
      ctx.bezierCurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
    }
    ctx.closePath();

    ctx.fillStyle = drawColor + "44";
    ctx.fill();
    ctx.strokeStyle = drawColor;
    ctx.lineWidth = 1.5 / zoom;
    ctx.setLineDash([4 / zoom, 4 / zoom]);
    ctx.stroke();
  });

  // Main canvas: draw all vector paths
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !vectorResult) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const [width, height] = vectorResult.dimensions;
    const dpr = window.devicePixelRatio || 1;
    const renderScale = dpr * zoom;
    canvas.width = width * renderScale;
    canvas.height = height * renderScale;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    ctx.scale(renderScale, renderScale);
    ctx.imageSmoothingEnabled = antiAlias;

    ctx.clearRect(0, 0, width, height);

    // Draw background color (transparent if null)
    if (bgColor) {
      ctx.fillStyle = bgColor;
      ctx.fillRect(0, 0, width, height);
    }

    const container = containerRef.current;
    const vw = container ? container.clientWidth / zoom : width;
    const vh = container ? container.clientHeight / zoom : height;
    const vx = container ? -panX / zoom : 0;
    const vy = container ? -panY / zoom : 0;

    vectorResult.paths.forEach((path, i) => {
      if (path.segments.length === 0) return;
      if (container) {
        const bounds = getPathBounds(path);
        if (bounds.maxX < vx || bounds.minX > vx + vw || bounds.maxY < vy || bounds.minY > vy + vh) return;
      }
      drawVectorPath(ctx, path);

      // Highlight selected/hovered paths
      const isSelected = selectedPaths.has(i);
      const isHovered = hoveredPath === i && (mode === "select" || mode === "recolor");
      if (isSelected || isHovered) {
        ctx.save();
        ctx.beginPath();
        const first = path.segments[0].curve;
        ctx.moveTo(first.p0.x, first.p0.y);
        for (const seg of path.segments) {
          const { p1, p2, p3 } = seg.curve;
          ctx.bezierCurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
        }
        if (path.is_closed) ctx.closePath();

        if (isSelected) {
          ctx.strokeStyle = "#3b82f6";
          ctx.lineWidth = 2 / zoom;
          ctx.stroke();
          ctx.fillStyle = "rgba(59, 130, 246, 0.15)";
          ctx.fill();
        } else {
          ctx.strokeStyle = "rgba(59, 130, 246, 0.5)";
          ctx.lineWidth = 1.5 / zoom;
          ctx.stroke();
        }
        ctx.restore();
      }
    });

    // Draw rotation handle when a single path is selected in select mode
    if (mode === "select" && selectedPaths.size === 1) {
      const pathIdx = Array.from(selectedPaths)[0];
      const path = vectorResult.paths[pathIdx];
      if (path && path.segments.length > 0) {
        const bounds = getPathBounds(path);
        const cx = (bounds.minX + bounds.maxX) / 2;
        const topY = bounds.minY;
        const handleOffset = 24 / zoom;
        const handleRadius = 6 / zoom;
        const stemLength = handleOffset - handleRadius;

        // Draw stem line from top-center of bounding box to handle
        ctx.save();
        ctx.beginPath();
        ctx.moveTo(cx, topY);
        ctx.lineTo(cx, topY - stemLength);
        ctx.strokeStyle = "#3b82f6";
        ctx.lineWidth = 1.5 / zoom;
        ctx.stroke();
        ctx.restore();

        // Draw circular rotation handle
        ctx.save();
        ctx.beginPath();
        ctx.arc(cx, topY - handleOffset, handleRadius, 0, Math.PI * 2);
        ctx.fillStyle = "#ffffff";
        ctx.fill();
        ctx.strokeStyle = "#3b82f6";
        ctx.lineWidth = 1.5 / zoom;
        ctx.stroke();
        ctx.restore();

        // Draw rotation arrow icon inside the handle
        ctx.save();
        ctx.beginPath();
        const arrowR = handleRadius * 0.55;
        const arrowCx = cx;
        const arrowCy = topY - handleOffset;
        ctx.arc(arrowCx, arrowCy, arrowR, -Math.PI * 0.8, Math.PI * 0.4, false);
        ctx.strokeStyle = "#3b82f6";
        ctx.lineWidth = 1.2 / zoom;
        ctx.lineCap = "round";
        ctx.stroke();
        // Arrowhead
        const arrowTipAngle = Math.PI * 0.4;
        const tipX = arrowCx + arrowR * Math.cos(arrowTipAngle);
        const tipY = arrowCy + arrowR * Math.sin(arrowTipAngle);
        const arrowSize = handleRadius * 0.35;
        ctx.beginPath();
        ctx.moveTo(tipX + arrowSize * Math.cos(arrowTipAngle - 0.3), tipY + arrowSize * Math.sin(arrowTipAngle - 0.3));
        ctx.lineTo(tipX, tipY);
        ctx.lineTo(tipX + arrowSize * Math.cos(arrowTipAngle + 1.8), tipY + arrowSize * Math.sin(arrowTipAngle + 1.8));
        ctx.stroke();
        ctx.restore();
      }
    }

    // Draw reshape control points
    if (mode === "reshape" && reshapePath !== null) {
      const path = vectorResult.paths[reshapePath];
      if (path) {
        const dotRadius = 4 / zoom;
        const handleRadius = 3 / zoom;

        // Draw path outline
        ctx.save();
        ctx.beginPath();
        const first = path.segments[0].curve;
        ctx.moveTo(first.p0.x, first.p0.y);
        for (const seg of path.segments) {
          const { p1, p2, p3 } = seg.curve;
          ctx.bezierCurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
        }
        if (path.is_closed) ctx.closePath();
        ctx.strokeStyle = "#3b82f6";
        ctx.lineWidth = 1.5 / zoom;
        ctx.stroke();
        ctx.restore();

        for (let si = 0; si < path.segments.length; si++) {
          const seg = path.segments[si];

          if (showControlHandles) {
            // Draw handle lines (p0→p1 and p2→p3)
            ctx.save();
            ctx.strokeStyle = "rgba(139, 92, 246, 0.5)";
            ctx.lineWidth = 1 / zoom;
            ctx.setLineDash([3 / zoom, 3 / zoom]);
            ctx.beginPath();
            ctx.moveTo(seg.curve.p0.x, seg.curve.p0.y);
            ctx.lineTo(seg.curve.p1.x, seg.curve.p1.y);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(seg.curve.p3.x, seg.curve.p3.y);
            ctx.lineTo(seg.curve.p2.x, seg.curve.p2.y);
            ctx.stroke();
            ctx.restore();

            // Draw control handle dots (p1, p2) — smaller circles
            for (const pKey of ["p1", "p2"] as const) {
              const cp = seg.curve[pKey];
              ctx.save();
              ctx.beginPath();
              ctx.arc(cp.x, cp.y, handleRadius, 0, Math.PI * 2);
              ctx.fillStyle = "#8b5cf6";
              ctx.fill();
              ctx.strokeStyle = "#ffffff";
              ctx.lineWidth = 1 / zoom;
              ctx.stroke();
              ctx.restore();
            }
          }

          // Draw anchor dots (p0) — larger filled circles, highlighted if selected
          const p0 = seg.curve.p0;
          const isSelected = selectedAnchor && !selectedAnchor.isEnd && selectedAnchor.segIndex === si;
          ctx.save();
          ctx.beginPath();
          ctx.arc(p0.x, p0.y, isSelected ? dotRadius * 1.3 : dotRadius, 0, Math.PI * 2);
          ctx.fillStyle = isSelected ? "#3b82f6" : "#ffffff";
          ctx.fill();
          ctx.strokeStyle = isSelected ? "#1d4ed8" : "#3b82f6";
          ctx.lineWidth = 1.5 / zoom;
          ctx.stroke();
          ctx.restore();
        }

        // Also draw the last p3 anchor (end of last segment)
        if (path.segments.length > 0) {
          const lastP3 = path.segments[path.segments.length - 1].curve.p3;
          // Only draw if path is not closed (for closed, last p3 === first p0)
          if (!path.is_closed) {
            const isLastSelected = selectedAnchor && selectedAnchor.isEnd && selectedAnchor.segIndex === path.segments.length - 1;
            ctx.save();
            ctx.beginPath();
            ctx.arc(lastP3.x, lastP3.y, isLastSelected ? dotRadius * 1.3 : dotRadius, 0, Math.PI * 2);
            ctx.fillStyle = isLastSelected ? "#3b82f6" : "#ffffff";
            ctx.fill();
            ctx.strokeStyle = isLastSelected ? "#1d4ed8" : "#3b82f6";
            ctx.lineWidth = 1.5 / zoom;
            ctx.stroke();
            ctx.restore();
          }
        }
      }
    }
  }, [vectorResult, zoom, panX, panY, antiAlias, bgColor, selectedPaths, hoveredPath, mode, reshapePath, showControlHandles, selectedAnchor]);

  // Mouse wheel zoom
  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (overControls) return;
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      setZoom((z) => Math.min(50, Math.max(0.25, z * delta)));
    },
    [overControls]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      // Space+drag or middle-click always pans
      if (spaceDown || e.button === 1) {
        e.preventDefault();
        setIsPanning(true);
        setLastMouse({ x: e.clientX, y: e.clientY });
        return;
      }
      if (e.button !== 0) return;

      if (mode === "select") {
        const pt = toImageCoords(e);

        // Check if clicking on the rotation handle (single path selected)
        if (pt && selectedPaths.size === 1 && vectorResult) {
          const pathIdx = Array.from(selectedPaths)[0];
          const path = vectorResult.paths[pathIdx];
          if (path && path.segments.length > 0) {
            const bounds = getPathBounds(path);
            const cx = (bounds.minX + bounds.maxX) / 2;
            const topY = bounds.minY;
            const handleOffset = 24 / zoom;
            const handleHitRadius = 8 / zoom;
            const handleX = cx;
            const handleY = topY - handleOffset;
            const dx = pt.x - handleX;
            const dy = pt.y - handleY;
            if (dx * dx + dy * dy <= handleHitRadius * handleHitRadius) {
              // Start rotation
              const center: Point = { x: (bounds.minX + bounds.maxX) / 2, y: (bounds.minY + bounds.maxY) / 2 };
              rotationCenterRef.current = center;
              rotationStartAngleRef.current = Math.atan2(pt.y - center.y, pt.x - center.x);
              setIsRotatingSelection(true);
              pushUndo();
              return;
            }
          }
        }

        const idx = hitTest(e);
        if (idx !== null && selectedPaths.has(idx) && !e.shiftKey && pt) {
          // Clicked on an already-selected path: prepare for drag-move
          pendingSelectDragRef.current = { idx, startPt: pt, shiftKey: false };
        } else if (pt) {
          // Normal selection logic, but defer final commit in case of drag
          pendingSelectDragRef.current = { idx, startPt: pt, shiftKey: e.shiftKey };
          if (e.shiftKey) {
            setSelectedPaths((prev) => {
              const next = new Set(prev);
              if (idx !== null) {
                if (next.has(idx)) next.delete(idx);
                else next.add(idx);
              }
              return next;
            });
          } else {
            setSelectedPaths(idx !== null ? new Set([idx]) : new Set());
          }
        } else {
          if (!e.shiftKey) setSelectedPaths(new Set());
        }
        return;
      }

      if (mode === "draw") {
        const pt = toImageCoords(e);
        if (!pt) return;
        setIsDrawing(true);
        drawPointsRef.current = [pt];
        return;
      }

      if (mode === "erase") {
        const pt = toImageCoords(e);
        if (!pt) return;
        setIsDrawing(true);
        eraseAt(pt);
        return;
      }

      if (mode === "recolor") {
        const idx = hitTest(e);
        if (idx !== null && vectorResult) {
          pushUndo();
          const newPaths = [...vectorResult.paths];
          newPaths[idx] = { ...newPaths[idx], fill_color: hexToRgb(drawColor) };
          useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
        } else if (vectorResult) {
          const pt = toImageCoords(e);
          const hitCanvas = hitCanvasRef.current;
          if (!pt || !hitCanvas) return;

          const [width, height] = vectorResult.dimensions;
          const startX = Math.floor(pt.x);
          const startY = Math.floor(pt.y);
          if (startX < 0 || startY < 0 || startX >= width || startY >= height) return;

          const ctx = hitCanvas.getContext("2d", { willReadFrequently: true });
          if (!ctx) return;

          const imageData = ctx.getImageData(0, 0, width, height);
          const pixels = floodFillEmpty(imageData, startX, startY);
          if (pixels.size === 0) return;

          const boundary = traceBoundary(pixels, width, height);
          const segments = polygonToBezierSegments(boundary);
          if (segments.length === 0) return;

          const newPath: VectorPath = {
            segments,
            fill_color: hexToRgb(drawColor),
            is_closed: true,
          };

          pushUndo();
          useAppStore.setState({
            vectorResult: {
              ...vectorResult,
              paths: [newPath, ...vectorResult.paths],
            },
          });
        }
        return;
      }

      if (mode === "reshape") {
        const pt = toImageCoords(e);
        if (!pt) return;

        // If a path is already selected for reshaping, check if clicking a control point
        if (reshapePath !== null && vectorResult) {
          const path = vectorResult.paths[reshapePath];
          if (path) {
            const hitRadius = 6 / zoom;

            // Check last p3 anchor for open paths first
            if (!path.is_closed && path.segments.length > 0) {
              const lastP3 = path.segments[path.segments.length - 1].curve.p3;
              const dx = pt.x - lastP3.x;
              const dy = pt.y - lastP3.y;
              if (dx * dx + dy * dy <= hitRadius * hitRadius) {
                const cp: ControlPointRef = { segIndex: path.segments.length - 1, point: "p3" };
                pendingDragRef.current = { cp, startPt: pt };
                hasDraggedRef.current = false;
                setSelectedAnchor({ segIndex: path.segments.length - 1, isEnd: true });
                return;
              }
            }

            // Pass 1: check anchor points (p0) first — these are selectable nodes
            for (let si = 0; si < path.segments.length; si++) {
              const p0 = path.segments[si].curve.p0;
              const dx = pt.x - p0.x;
              const dy = pt.y - p0.y;
              if (dx * dx + dy * dy <= hitRadius * hitRadius) {
                pendingDragRef.current = { cp: { segIndex: si, point: "p0" }, startPt: pt };
                hasDraggedRef.current = false;
                setSelectedAnchor({ segIndex: si, isEnd: false });
                return;
              }
            }

            // Pass 2: check control handles (p1, p2) and trailing p3
            if (showControlHandles) {
              for (let si = 0; si < path.segments.length; si++) {
                const seg = path.segments[si];
                for (const pKey of ["p1", "p2", "p3"] as const) {
                  const cp = seg.curve[pKey];
                  const dx = pt.x - cp.x;
                  const dy = pt.y - cp.y;
                  if (dx * dx + dy * dy <= hitRadius * hitRadius) {
                    pendingDragRef.current = { cp: { segIndex: si, point: pKey }, startPt: pt };
                    hasDraggedRef.current = false;
                    setSelectedAnchor(null);
                    return;
                  }
                }
              }
            }
          }
        }

        // Click on a path to select it for reshaping
        const idx = hitTest(e);
        if (idx !== null) {
          setReshapePath(idx);
        } else {
          setReshapePath(null);
        }
        setDragCP(null);
        pendingDragRef.current = null;
        setSelectedAnchor(null);
        return;
      }

      if (mode === "shape") {
        const pt = toImageCoords(e);
        if (!pt) return;
        setIsShapeDragging(true);
        shapeDragStartRef.current = pt;
        setShapeDragCurrent(pt);
        return;
      }

      // Pan mode
      e.preventDefault();
      setIsPanning(true);
      setLastMouse({ x: e.clientX, y: e.clientY });
    },
    [spaceDown, mode, hitTest, toImageCoords, eraseAt, vectorResult, drawColor, pushUndo, reshapePath, zoom, showControlHandles, selectedPaths]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      // Track mouse for brush cursor
      if (mode === "draw" || mode === "erase") {
        setMousePos({ x: e.clientX, y: e.clientY });
      }

      if (isPanning) {
        const dx = e.clientX - lastMouse.x;
        const dy = e.clientY - lastMouse.y;
        setPanX((x) => x + dx);
        setPanY((y) => y + dy);
        setLastMouse({ x: e.clientX, y: e.clientY });
        return;
      }
      if (mode === "select" || mode === "recolor") {
        if (!isDraggingSelection && !isRotatingSelection) setHoveredPath(hitTest(e));
      }
      // Select-mode rotation
      if (mode === "select" && isRotatingSelection && vectorResult && selectedPaths.size === 1) {
        const pt = toImageCoords(e);
        if (pt) {
          const center = rotationCenterRef.current;
          const currentAngle = Math.atan2(pt.y - center.y, pt.x - center.x);
          const deltaAngle = currentAngle - rotationStartAngleRef.current;
          rotationStartAngleRef.current = currentAngle;

          const cos = Math.cos(deltaAngle);
          const sin = Math.sin(deltaAngle);
          const rotatePoint = (p: Point): Point => {
            const dx = p.x - center.x;
            const dy = p.y - center.y;
            return { x: center.x + dx * cos - dy * sin, y: center.y + dx * sin + dy * cos };
          };

          const pathIdx = Array.from(selectedPaths)[0];
          const newPaths = [...vectorResult.paths];
          const path = newPaths[pathIdx];
          if (path) {
            newPaths[pathIdx] = {
              ...path,
              segments: path.segments.map((seg) => ({
                ...seg,
                curve: {
                  p0: rotatePoint(seg.curve.p0),
                  p1: rotatePoint(seg.curve.p1),
                  p2: rotatePoint(seg.curve.p2),
                  p3: rotatePoint(seg.curve.p3),
                },
              })),
            };
            preserveSelectionRef.current = true;
            useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
          }
        }
        return;
      }
      // Select-mode drag-to-move
      if (mode === "select" && pendingSelectDragRef.current && !isDraggingSelection && !isMovingSelectionRef.current) {
        const pt = toImageCoords(e);
        if (pt) {
          const dx = pt.x - pendingSelectDragRef.current.startPt.x;
          const dy = pt.y - pendingSelectDragRef.current.startPt.y;
          const moveThreshold = 3 / zoom;
          if (dx * dx + dy * dy > moveThreshold * moveThreshold) {
            // If the clicked path wasn't already selected, select it now for dragging
            const clickedIdx = pendingSelectDragRef.current.idx;
            if (clickedIdx !== null && !selectedPaths.has(clickedIdx)) {
              setSelectedPaths(new Set([clickedIdx]));
            }
            if (selectedPaths.size > 0 || (clickedIdx !== null)) {
              pushUndo();
              isMovingSelectionRef.current = true;
              setIsDraggingSelection(true);
              dragStartImagePtRef.current = pendingSelectDragRef.current.startPt;
            }
          }
        }
      }
      if (mode === "select" && (isDraggingSelection || isMovingSelectionRef.current) && vectorResult) {
        const pt = toImageCoords(e);
        if (pt && dragStartImagePtRef.current) {
          const dx = pt.x - dragStartImagePtRef.current.x;
          const dy = pt.y - dragStartImagePtRef.current.y;
          dragStartImagePtRef.current = pt;
          const newPaths = [...vectorResult.paths];
          for (const pathIdx of selectedPaths) {
            const path = newPaths[pathIdx];
            if (!path) continue;
            newPaths[pathIdx] = {
              ...path,
              segments: path.segments.map((seg) => ({
                ...seg,
                curve: {
                  p0: { x: seg.curve.p0.x + dx, y: seg.curve.p0.y + dy },
                  p1: { x: seg.curve.p1.x + dx, y: seg.curve.p1.y + dy },
                  p2: { x: seg.curve.p2.x + dx, y: seg.curve.p2.y + dy },
                  p3: { x: seg.curve.p3.x + dx, y: seg.curve.p3.y + dy },
                },
              })),
            };
          }
          useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
        }
      }
      if (mode === "reshape" && reshapePath !== null && vectorResult) {
        // Promote pending drag to real drag on first move
        if (pendingDragRef.current && !dragCP) {
          const pt = toImageCoords(e);
          if (pt) {
            const dx = pt.x - pendingDragRef.current.startPt.x;
            const dy = pt.y - pendingDragRef.current.startPt.y;
            const moveThreshold = 2 / zoom;
            if (dx * dx + dy * dy > moveThreshold * moveThreshold) {
              pushUndo();
              setDragCP(pendingDragRef.current.cp);
              hasDraggedRef.current = true;
            }
          }
        }
        if (dragCP) {
          const pt = toImageCoords(e);
          if (pt) {
            const newPaths = [...vectorResult.paths];
            const path = { ...newPaths[reshapePath] };
            const newSegs = [...path.segments];
            const seg = { ...newSegs[dragCP.segIndex] };
            seg.curve = { ...seg.curve, [dragCP.point]: pt };

            // Keep adjacent segments connected
            if (dragCP.point === "p0" && dragCP.segIndex > 0) {
              const prevSeg = { ...newSegs[dragCP.segIndex - 1] };
              prevSeg.curve = { ...prevSeg.curve, p3: pt };
              newSegs[dragCP.segIndex - 1] = prevSeg;
            }
            if (dragCP.point === "p3" && dragCP.segIndex < newSegs.length - 1) {
              const nextSeg = { ...newSegs[dragCP.segIndex + 1] };
              nextSeg.curve = { ...nextSeg.curve, p0: pt };
              newSegs[dragCP.segIndex + 1] = nextSeg;
            }
            // For closed paths, wrap around
            if (path.is_closed && newSegs.length > 1) {
              if (dragCP.point === "p0" && dragCP.segIndex === 0) {
                const lastSeg = { ...newSegs[newSegs.length - 1] };
                lastSeg.curve = { ...lastSeg.curve, p3: pt };
                newSegs[newSegs.length - 1] = lastSeg;
              }
              if (dragCP.point === "p3" && dragCP.segIndex === newSegs.length - 1) {
                const firstSeg = { ...newSegs[0] };
                firstSeg.curve = { ...firstSeg.curve, p0: pt };
                newSegs[0] = firstSeg;
              }
            }

            newSegs[dragCP.segIndex] = seg;
            path.segments = newSegs;
            newPaths[reshapePath] = path;
            useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
          }
        }
      }
      if (mode === "draw" && isDrawing) {
        const pt = toImageCoords(e);
        if (pt) drawPointsRef.current.push(pt);
      }
      if (mode === "erase" && isDrawing) {
        const pt = toImageCoords(e);
        if (pt) eraseAt(pt);
      }
      if (mode === "shape" && isShapeDragging) {
        const pt = toImageCoords(e);
        if (pt) setShapeDragCurrent(pt);
        setShapeShiftHeld(e.shiftKey);
        setShapeAltHeld(e.altKey);
      }
    },
    [isPanning, lastMouse, mode, hitTest, isDrawing, toImageCoords, eraseAt, dragCP, reshapePath, vectorResult, pushUndo, zoom, isDraggingSelection, isRotatingSelection, selectedPaths, isShapeDragging]
  );

  const handleMouseUp = useCallback(() => {
    setIsPanning(false);
    if (dragCP) setDragCP(null);
    pendingDragRef.current = null;
    hasDraggedRef.current = false;

    // End select-mode rotation
    if (isRotatingSelection) {
      setIsRotatingSelection(false);
      return;
    }

    // End select-mode drag
    if (isDraggingSelection) {
      setIsDraggingSelection(false);
      isMovingSelectionRef.current = false;
      dragStartImagePtRef.current = null;
      pendingSelectDragRef.current = null;
      return;
    }
    pendingSelectDragRef.current = null;

    if (mode === "draw" && isDrawing && vectorResult) {
      setIsDrawing(false);
      const raw = drawPointsRef.current;
      if (raw.length >= 2) {
        const simplified = rdpSimplify(raw, 1.5);
        const segments = pointsToBezierSegments(simplified);
        if (segments.length > 0) {
          pushUndo();
          const newPath: VectorPath = {
            segments,
            fill_color: hexToRgb(drawColor),
            is_closed: false,
          };
          const newPaths = [...vectorResult.paths, newPath];
          preserveSelectionRef.current = true;
          useAppStore.setState({
            vectorResult: { ...vectorResult, paths: newPaths },
          });
          // Auto-select the newly drawn path and switch to Select tool
          setSelectedPaths(new Set([newPaths.length - 1]));
          setMode("select");
        }
      }
      drawPointsRef.current = [];
      return;
    }

    if (mode === "erase" && isDrawing) {
      setIsDrawing(false);
    }

    if (mode === "shape" && isShapeDragging && vectorResult) {
      setIsShapeDragging(false);
      const start = shapeDragStartRef.current;
      const end = shapeDragCurrent;
      shapeDragStartRef.current = null;
      setShapeDragCurrent(null);

      if (!start || !end) return;

      const { effectiveStart, effectiveEnd } = applyShapeConstraints(start, end, shapeShiftHeld, shapeAltHeld);
      const segments = buildShapeSegments(activeShape, effectiveStart, effectiveEnd, {
        cornerRadius: shapeCornerRadius,
        starPoints: shapeStarPoints,
        polygonSides: shapePolygonSides,
        innerRadiusRatio: shapeInnerRadiusRatio,
      });
      if (segments && segments.length > 0) {
        pushUndo();
        const newPath: VectorPath = {
          segments,
          fill_color: hexToRgb(drawColor),
          is_closed: true,
          ...(shapeStrokeWidth > 0 ? {
            stroke_color: hexToRgb(shapeStrokeColor),
            stroke_width: shapeStrokeWidth,
          } : {}),
        };
        const newPaths = [...vectorResult.paths, newPath];
        preserveSelectionRef.current = true;
        useAppStore.setState({
          vectorResult: { ...vectorResult, paths: newPaths },
        });
        // Auto-select the newly drawn shape and switch to Select tool
        setSelectedPaths(new Set([newPaths.length - 1]));
        setMode("select");
      }
    }
  }, [mode, isDrawing, vectorResult, drawColor, pushUndo, isDraggingSelection, isRotatingSelection, isShapeDragging, shapeDragCurrent, shapeShiftHeld, shapeAltHeld, activeShape, shapeStrokeColor, shapeStrokeWidth, shapeCornerRadius, shapeStarPoints, shapePolygonSides, shapeInnerRadiusRatio]);

  // Double-click: add a node by splitting a bezier segment
  const handleDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      if (mode !== "reshape" || reshapePath === null || !vectorResult) return;
      const pt = toImageCoords(e);
      if (!pt) return;

      const path = vectorResult.paths[reshapePath];
      if (!path) return;

      // Find the closest segment to the click point
      let bestSeg = -1;
      let bestT = 0;
      let bestDistSq = Infinity;
      const hitRadius = 8 / zoom;
      const hitRadiusSq = hitRadius * hitRadius;

      for (let si = 0; si < path.segments.length; si++) {
        const result = closestPointOnBezier(path.segments[si].curve, pt);
        if (result.distSq < bestDistSq) {
          bestDistSq = result.distSq;
          bestT = result.t;
          bestSeg = si;
        }
      }

      if (bestSeg < 0 || bestDistSq > hitRadiusSq) return;

      // Don't split at the very endpoints
      if (bestT < 0.01 || bestT > 0.99) return;

      pushUndo();
      const seg = path.segments[bestSeg];
      const [left, right] = splitBezierAt(seg.curve, bestT);

      const newSegs = [...path.segments];
      newSegs.splice(bestSeg, 1,
        { curve: left, is_corner_start: seg.is_corner_start },
        { curve: right, is_corner_start: false },
      );

      const newPaths = [...vectorResult.paths];
      newPaths[reshapePath] = { ...path, segments: newSegs };
      useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
      setSelectedAnchor({ segIndex: bestSeg + 1, isEnd: false });
    },
    [mode, reshapePath, vectorResult, toImageCoords, zoom, pushUndo]
  );

  // Remove the selected anchor node (merge two adjacent segments)
  const deleteAnchor = useCallback(() => {
    if (reshapePath === null || !vectorResult || !selectedAnchor) return;
    const path = vectorResult.paths[reshapePath];
    if (!path || path.segments.length < 2) return;

    const { segIndex, isEnd } = selectedAnchor;

    let newSegs = [...path.segments];

    if (isEnd && !path.is_closed) {
      // Deleting the last p3 anchor of an open path — just remove last segment
      newSegs.pop();
    } else if (segIndex === 0 && !path.is_closed) {
      // Deleting the first p0 anchor of an open path — just remove first segment
      newSegs.shift();
    } else {
      // Merge segment[segIndex-1] and segment[segIndex] into one
      // For closed paths, segIndex 0 wraps to the last segment
      const prevIdx = segIndex === 0 ? newSegs.length - 1 : segIndex - 1;
      const currIdx = segIndex;

      const prev = newSegs[prevIdx];
      const curr = newSegs[currIdx];

      // Create a merged segment keeping the outer control points
      const merged: BezierSegment = {
        curve: {
          p0: prev.curve.p0,
          p1: prev.curve.p1,
          p2: curr.curve.p2,
          p3: curr.curve.p3,
        },
        is_corner_start: prev.is_corner_start,
      };

      if (prevIdx < currIdx) {
        newSegs.splice(prevIdx, 2, merged);
      } else {
        // Closed path wrapping: prev is last, curr is first
        newSegs.splice(prevIdx, 1); // remove last
        newSegs.splice(0, 1, merged); // replace first with merged
      }
    }

    pushUndo();
    const newPaths = [...vectorResult.paths];
    newPaths[reshapePath] = { ...path, segments: newSegs };
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedAnchor(null);
  }, [reshapePath, vectorResult, selectedAnchor, pushUndo]);

  // Simplify the selected reshape path by reducing nodes
  const simplifyPath = useCallback(() => {
    if (reshapePath === null || !vectorResult) return;
    const path = vectorResult.paths[reshapePath];
    if (!path || path.segments.length < 3) return;

    // Scale epsilon based on path size and current segment density
    const bounds = getPathBounds(path);
    const size = Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY);
    // Higher density = more aggressive simplification; each click reduces further
    const density = path.segments.length / Math.max(size, 1);
    const epsilon = Math.max(size * 0.015, density * 0.5);

    const newSegs = simplifyBezierPath(path, epsilon);
    if (newSegs.length >= path.segments.length) {
      // RDP didn't help, fallback: remove every other interior segment
      const fallbackSegs: BezierSegment[] = [];
      for (let i = 0; i < path.segments.length; i++) {
        if (i === 0 || i === path.segments.length - 1 || i % 2 === 0) {
          fallbackSegs.push(path.segments[i]);
        }
      }
      // Re-stitch: ensure p0 of each segment matches p3 of the previous
      for (let i = 1; i < fallbackSegs.length; i++) {
        const prev = fallbackSegs[i - 1];
        const curr = fallbackSegs[i];
        if (prev.curve.p3.x !== curr.curve.p0.x || prev.curve.p3.y !== curr.curve.p0.y) {
          fallbackSegs[i] = {
            ...curr,
            curve: { ...curr.curve, p0: { ...prev.curve.p3 } },
          };
        }
      }
      if (fallbackSegs.length < path.segments.length) {
        pushUndo();
        const newPaths = [...vectorResult.paths];
        newPaths[reshapePath] = { ...path, segments: fallbackSegs };
        useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
        setSelectedAnchor(null);
      }
      return;
    }

    pushUndo();
    const newPaths = [...vectorResult.paths];
    newPaths[reshapePath] = { ...path, segments: newSegs };
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedAnchor(null);
  }, [reshapePath, vectorResult, pushUndo]);

  // Rotate the selected reshape path by a given number of degrees
  // Scale (shrink/enlarge) the selected reshape path by a percentage
  const scalePath = useCallback((percent: number) => {
    if (reshapePath === null || !vectorResult) return;
    const path = vectorResult.paths[reshapePath];
    if (!path || path.segments.length === 0) return;

    const bounds = getPathBounds(path);
    const cx = (bounds.minX + bounds.maxX) / 2;
    const cy = (bounds.minY + bounds.maxY) / 2;
    const factor = 1 + percent / 100;

    const scalePoint = (p: Point): Point => ({
      x: cx + (p.x - cx) * factor,
      y: cy + (p.y - cy) * factor,
    });

    const newSegs: BezierSegment[] = path.segments.map((seg) => ({
      ...seg,
      curve: {
        p0: scalePoint(seg.curve.p0),
        p1: scalePoint(seg.curve.p1),
        p2: scalePoint(seg.curve.p2),
        p3: scalePoint(seg.curve.p3),
      },
    }));

    pushUndo();
    const newPaths = [...vectorResult.paths];
    newPaths[reshapePath] = { ...path, segments: newSegs };
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedAnchor(null);
  }, [reshapePath, vectorResult, pushUndo]);

  // Delete selected paths
  const deleteSelected = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    pushUndo();
    const newPaths = vectorResult.paths.filter((_, i) => !selectedPaths.has(i));
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedPaths(new Set());
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Duplicate selected paths
  const duplicateSelected = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    pushUndo();
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    const duplicated = indices.map((i) => JSON.parse(JSON.stringify(vectorResult.paths[i])) as VectorPath);
    const newPaths = [...vectorResult.paths, ...duplicated];
    preserveSelectionRef.current = true;
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    // Select the newly duplicated paths
    const newSelection = new Set<number>();
    for (let i = 0; i < duplicated.length; i++) {
      newSelection.add(vectorResult.paths.length + i);
    }
    setSelectedPaths(newSelection);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Merge selected paths into one
  const mergeSelected = useCallback(() => {
    if (selectedPaths.size < 2 || !vectorResult) return;
    pushUndo();
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    const selectedPathObjs = indices.map((i) => vectorResult.paths[i]);

    // Determine fill color: majority color among selected paths
    const colorCounts = new Map<string, { color: RgbColor; count: number }>();
    for (const p of selectedPathObjs) {
      const key = `${p.fill_color.r},${p.fill_color.g},${p.fill_color.b}`;
      const entry = colorCounts.get(key);
      if (entry) entry.count++;
      else colorCounts.set(key, { color: p.fill_color, count: 1 });
    }
    let majorityColor = selectedPathObjs[0].fill_color;
    let maxCount = 0;
    for (const { color, count } of colorCounts.values()) {
      if (count > maxCount) { maxCount = count; majorityColor = color; }
    }

    // Combine all segments into one path
    const mergedSegments: BezierSegment[] = [];
    for (const p of selectedPathObjs) {
      mergedSegments.push(...p.segments);
    }

    const mergedPath: VectorPath = {
      segments: mergedSegments,
      fill_color: majorityColor,
      is_closed: selectedPathObjs.every((p) => p.is_closed),
    };

    // Remove old paths, insert merged at the position of the first selected
    const newPaths = vectorResult.paths.filter((_, i) => !selectedPaths.has(i));
    newPaths.splice(indices[0], 0, mergedPath);
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedPaths(new Set([indices[0]]));
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Move selected paths forward (toward front) in z-order
  const moveForward = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    // Can't move forward if any selected path is already at the front
    if (indices[indices.length - 1] >= vectorResult.paths.length - 1) return;
    pushUndo();
    const newPaths = [...vectorResult.paths];
    // Process from back to front to avoid conflicts
    for (let i = indices.length - 1; i >= 0; i--) {
      const idx = indices[i];
      [newPaths[idx], newPaths[idx + 1]] = [newPaths[idx + 1], newPaths[idx]];
    }
    preserveSelectionRef.current = true;
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedPaths(new Set(indices.map((i) => i + 1)));
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Move selected paths backward (toward back) in z-order
  const moveBackward = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    if (indices[0] <= 0) return;
    pushUndo();
    const newPaths = [...vectorResult.paths];
    for (let i = 0; i < indices.length; i++) {
      const idx = indices[i];
      [newPaths[idx - 1], newPaths[idx]] = [newPaths[idx], newPaths[idx - 1]];
    }
    preserveSelectionRef.current = true;
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedPaths(new Set(indices.map((i) => i - 1)));
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Bring selected paths to the very front
  const bringToFront = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    if (indices[indices.length - 1] >= vectorResult.paths.length - 1 && indices.length === 1) return;
    pushUndo();
    const selected = indices.map((i) => vectorResult.paths[i]);
    const rest = vectorResult.paths.filter((_, i) => !selectedPaths.has(i));
    const newPaths = [...rest, ...selected];
    preserveSelectionRef.current = true;
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    const newStart = rest.length;
    setSelectedPaths(new Set(selected.map((_, i) => newStart + i)));
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Send selected paths to the very back
  const sendToBack = useCallback(() => {
    if (selectedPaths.size === 0 || !vectorResult) return;
    const indices = Array.from(selectedPaths).sort((a, b) => a - b);
    if (indices[0] <= 0 && indices.length === 1) return;
    pushUndo();
    const selected = indices.map((i) => vectorResult.paths[i]);
    const rest = vectorResult.paths.filter((_, i) => !selectedPaths.has(i));
    const newPaths = [...selected, ...rest];
    preserveSelectionRef.current = true;
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: newPaths } });
    setSelectedPaths(new Set(selected.map((_, i) => i)));
    setHoveredPath(null);
  }, [selectedPaths, vectorResult, pushUndo]);

  // Flatten: rasterize & re-vectorize to remove hidden geometry
  const [isFlattening, setIsFlattening] = useState(false);
  const flattenSvg = useCallback(async () => {
    if (!vectorResult || isFlattening) return;
    pushUndo();
    setIsFlattening(true);
    try {
      // Count unique colors to use as color_count hint
      const uniqueColors = new Set(
        vectorResult.paths.map((p) => `${p.fill_color.r},${p.fill_color.g},${p.fill_color.b}`)
      );
      const colorCount = Math.max(2, Math.min(32, uniqueColors.size));
      const flattened = await invoke<VectorizationResult>("flatten_svg", {
        result: vectorResult,
        colorCount,
      });
      useAppStore.setState({ vectorResult: flattened });
      setSelectedPaths(new Set());
      setHoveredPath(null);
    } catch (e) {
      console.error("Flatten failed:", e);
    } finally {
      setIsFlattening(false);
    }
  }, [vectorResult, isFlattening, pushUndo]);

  const undo = useCallback(() => {
    if (undoStackRef.current.length === 0 || !vectorResult) return;
    redoStackRef.current.push([...vectorResult.paths]);
    const prevPaths = undoStackRef.current.pop()!;
    setUndoCount(undoStackRef.current.length);
    setRedoCount(redoStackRef.current.length);
    useAppStore.setState({
      vectorResult: { ...vectorResult, paths: prevPaths },
      hasCanvasEdits: undoStackRef.current.length > 0,
    });
    setSelectedPaths(new Set());
    setHoveredPath(null);
  }, [vectorResult]);

  const redo = useCallback(() => {
    if (redoStackRef.current.length === 0 || !vectorResult) return;
    undoStackRef.current.push([...vectorResult.paths]);
    const nextPaths = redoStackRef.current.pop()!;
    setUndoCount(undoStackRef.current.length);
    setRedoCount(redoStackRef.current.length);
    useAppStore.setState({ vectorResult: { ...vectorResult, paths: nextPaths } });
    setSelectedPaths(new Set());
    setHoveredPath(null);
  }, [vectorResult]);

  // Keyboard events
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === "Space") { e.preventDefault(); setSpaceDown(true); }
      if ((e.code === "Delete" || e.code === "Backspace") && mode === "reshape" && selectedAnchor) {
        e.preventDefault();
        deleteAnchor();
      } else if ((e.code === "Delete" || e.code === "Backspace") && selectedPaths.size > 0) {
        e.preventDefault();
        deleteSelected();
      }
      if (e.code === "Escape") { setSelectedPaths(new Set()); setHoveredPath(null); setReshapePath(null); setDragCP(null); setSelectedAnchor(null); }
      if (e.code === "KeyA" && (e.ctrlKey || e.metaKey) && mode === "select" && vectorResult) {
        e.preventDefault();
        setSelectedPaths(new Set(vectorResult.paths.map((_, i) => i)));
      }
      if (e.code === "KeyZ" && (e.ctrlKey || e.metaKey) && !e.shiftKey) { e.preventDefault(); undo(); }
      if (((e.code === "KeyZ" && e.shiftKey) || e.code === "KeyY") && (e.ctrlKey || e.metaKey)) { e.preventDefault(); redo(); }
      // Z-order shortcuts: ] forward, [ backward, Ctrl+] front, Ctrl+[ back
      if (e.code === "BracketRight" && mode === "select" && selectedPaths.size > 0) {
        e.preventDefault();
        if (e.ctrlKey || e.metaKey) bringToFront(); else moveForward();
      }
      if (e.code === "BracketLeft" && mode === "select" && selectedPaths.size > 0) {
        e.preventDefault();
        if (e.ctrlKey || e.metaKey) sendToBack(); else moveBackward();
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === "Space") { setSpaceDown(false); setIsPanning(false); }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [selectedPaths, deleteSelected, deleteAnchor, selectedAnchor, undo, redo, mode, vectorResult, moveForward, moveBackward, bringToFront, sendToBack]);

  const fitToWindow = useCallback(() => {
    const container = containerRef.current;
    if (!container || !vectorResult) return;
    const [w, h] = vectorResult.dimensions;
    const scaleX = (container.clientWidth - 32) / w;
    const scaleY = (container.clientHeight - 32) / h;
    const newZoom = Math.min(scaleX, scaleY);
    setZoom(newZoom);
    setPanX((container.clientWidth - w * newZoom) / 2);
    setPanY((container.clientHeight - h * newZoom) / 2);
  }, [vectorResult]);

  const resetZoom = useCallback(() => { setZoom(1); setPanX(0); setPanY(0); }, []);

  // Auto-fit only when dimensions change (new image), not on path edits
  const lastDimsRef = useRef<string>("");
  useEffect(() => {
    if (vectorResult) {
      const dimsKey = vectorResult.dimensions.join(",");
      if (dimsKey !== lastDimsRef.current) {
        lastDimsRef.current = dimsKey;
        requestAnimationFrame(() => fitToWindow());
      }
    }
  }, [vectorResult, fitToWindow]);

  if (!imageInfo || !vectorResult) return null;

  const cursorStyle =
    isPanning ? "grabbing"
    : spaceDown ? "grab"
    : isRotatingSelection ? "grabbing"
    : isDraggingSelection ? "move"
    : mode === "select" && hoveredPath !== null && selectedPaths.has(hoveredPath) ? "move"
    : mode === "select" || mode === "recolor" ? "crosshair"
    : mode === "reshape" ? (dragCP ? "grabbing" : "crosshair")
    : mode === "draw" || mode === "erase" ? "none"
    : mode === "shape" ? "crosshair"
    : "grab";

  const modeBtn = (m: CanvasMode, icon: string, label: string, title: string) => (
    <button
      onClick={() => {
        setMode(m);
        if (m !== "select") { setSelectedPaths(new Set()); setHoveredPath(null); }
        if (m !== "reshape") { setReshapePath(null); setDragCP(null); setSelectedAnchor(null); }
      }}
      className={`px-2 py-1 text-xs rounded-md font-medium transition-colors ${
        mode === m ? "bg-blue-600 text-white" : "text-gray-600 hover:bg-gray-100"
      }`}
      title={title}
    >
      {icon} {label}
    </button>
  );

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-hidden relative bg-gray-100"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onDoubleClick={handleDoubleClick}
      onMouseLeave={() => { handleMouseUp(); setHoveredPath(null); setMousePos({ x: -200, y: -200 }); }}
      style={{ cursor: cursorStyle }}
    >
      <div
        style={{
          transform: `translate(${panX}px, ${panY}px) scale(${zoom})`,
          transformOrigin: "0 0",
          transition: isPanning ? "none" : "transform 0.15s ease-out",
          position: "relative",
        }}
      >
        <canvas ref={canvasRef} className="shadow-lg" style={!bgColor ? {
          backgroundImage: "repeating-conic-gradient(#e0e0e0 0% 25%, #ffffff 0% 50%)",
          backgroundSize: "16px 16px",
        } : undefined} />
        <canvas ref={overlayCanvasRef} className="absolute top-0 left-0 pointer-events-none" />
      </div>

      {/* Brush cursor for draw/erase modes — hidden over controls */}
      {(mode === "draw" || mode === "erase") && mousePos.x > 0 && !isPanning && !overControls && (
        <div
          className="pointer-events-none fixed rounded-full border-2 z-40"
          style={{
            width: brushSize * zoom,
            height: brushSize * zoom,
            borderColor: mode === "erase" ? "#ef4444" : drawColor,
            backgroundColor: mode === "erase" ? "rgba(239,68,68,0.1)" : `${drawColor}22`,
            transform: "translate(-50%, -50%)",
            left: mousePos.x,
            top: mousePos.y,
          }}
        />
      )}

      {/* Top-left: mode toolbar */}
      <div className="absolute top-3 left-3 flex flex-col gap-2" style={{ cursor: "default" }} onMouseDown={(e) => e.stopPropagation()} onMouseEnter={() => setOverControls(true)} onMouseLeave={() => setOverControls(false)}>
        <div className="flex items-center gap-0.5 bg-white/90 backdrop-blur rounded-lg shadow p-1 flex-wrap">
          {modeBtn("pan", "✋", "Pan", "Pan — drag to move canvas")}
          {modeBtn("select", "🔍", "Select", "Select — click fills, Shift+click for multi-select")}
          {modeBtn("draw", "✏️", "Draw", "Draw — freehand vector strokes")}
          {modeBtn("shape", "⬠", "Shape", "Shape — click-drag to place geometric shapes")}
          {modeBtn("erase", "🧹", "Erase", "Erase — paint over fills to remove them")}
          {modeBtn("recolor", "🎨", "Fill", "Fill — click a fill to change its color")}
          {modeBtn("reshape", "◇", "Reshape", "Reshape — drag control points to reshape paths")}
          <div className="h-4 w-px bg-gray-300 mx-0.5" />
          <button
            onClick={undo} disabled={undoCount === 0}
            className="px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 disabled:opacity-30 disabled:cursor-default rounded-md transition-colors"
            title="Undo (Ctrl+Z)"
          >↩</button>
          <button
            onClick={redo} disabled={redoCount === 0}
            className="px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 disabled:opacity-30 disabled:cursor-default rounded-md transition-colors"
            title="Redo (Ctrl+Shift+Z)"
          >↪</button>
          <div className="h-4 w-px bg-gray-300 mx-0.5" />
          <button
            onClick={flattenSvg} disabled={isFlattening || !vectorResult}
            className="px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 disabled:opacity-30 disabled:cursor-default rounded-md transition-colors"
            title="Flatten — rasterize & re-vectorize to remove hidden geometry and simplify paths"
          >{isFlattening ? "⏳" : "🫓"} Flatten</button>
        </div>

        {/* Tool options */}
        {(mode === "draw" || mode === "erase") && (
          <div className="flex items-center gap-2 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-1.5">
            {mode === "draw" && (
              <>
                <label className="text-xs text-gray-500">Color</label>
                <InlineColorPicker value={drawColor} onChange={setDrawColor} />
              </>
            )}
            <>
              <label className="text-xs text-gray-500">Size</label>
              <input
                type="range" min={1} max={100} value={brushSize}
                onChange={(e) => setBrushSize(Number(e.target.value))}
                className="w-40 accent-blue-600"
              />
              <span className="text-xs text-gray-500 w-6">{brushSize}</span>
              <div className="flex flex-col -gap-px">
                <button
                  onClick={() => setBrushSize((s) => Math.min(100, s + 1))}
                  className="px-1 py-0 text-[10px] text-gray-500 hover:bg-gray-100 rounded leading-tight"
                  title="Increase size"
                >▲</button>
                <button
                  onClick={() => setBrushSize((s) => Math.max(1, s - 1))}
                  className="px-1 py-0 text-[10px] text-gray-500 hover:bg-gray-100 rounded leading-tight"
                  title="Decrease size"
                >▼</button>
              </div>
            </>
          </div>
        )}
        {mode === "recolor" && (
          <div className="flex items-center gap-0 bg-white/90 backdrop-blur rounded-lg shadow px-2 py-3 max-w-full">
            <div className="flex items-center gap-1.5 overflow-x-auto overflow-y-hidden pr-2" style={{ maxWidth: "500px" }}>
              {/* Palette swatches from image */}
              {paletteSwatches.map((color) => (
                <button
                  key={color}
                  onClick={() => setDrawColor(color)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    // Right-click on palette colors does nothing (they're auto-extracted)
                  }}
                  className={`w-7 h-7 rounded border-2 flex-shrink-0 transition-all ${
                    drawColor.toLowerCase() === color.toLowerCase()
                      ? "border-blue-500 ring-2 ring-blue-300 scale-110"
                      : "border-gray-200 hover:border-gray-400 hover:scale-105"
                  }`}
                  style={{ backgroundColor: color }}
                  title={`${color} (from image)`}
                />
              ))}
              {/* Custom swatches (deduped against palette) */}
              {customSwatches.filter((c) => !paletteSwatches.includes(c)).map((color) => (
                <button
                  key={`custom-${color}`}
                  onClick={() => setDrawColor(color)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setCustomSwatches((prev) => prev.filter((c) => c !== color));
                    if (drawColor.toLowerCase() === color.toLowerCase() && paletteSwatches.length > 0) {
                      setDrawColor(paletteSwatches[0]);
                    }
                  }}
                  className={`w-7 h-7 rounded border-2 flex-shrink-0 transition-all relative ${
                    drawColor.toLowerCase() === color.toLowerCase()
                      ? "border-blue-500 ring-2 ring-blue-300 scale-110"
                      : "border-dashed border-gray-300 hover:border-gray-400 hover:scale-105"
                  }`}
                  style={{ backgroundColor: color }}
                  title={`${color} (custom — right-click to remove)`}
                />
              ))}
            </div>
            {/* Sticky "+" button */}
            <div className="relative flex-shrink-0 ml-1" ref={swatchPickerRef}>
              <button
                onClick={() => setShowSwatchPicker(!showSwatchPicker)}
                className="w-7 h-7 rounded border-2 border-dashed border-gray-300 hover:border-blue-400 flex items-center justify-center text-gray-400 hover:text-blue-500 transition-colors text-lg font-light"
                title="Add custom color"
              >+</button>
              {showSwatchPicker && (
                <div className="absolute top-full left-0 mt-2 bg-white rounded-lg shadow-lg border border-gray-200 p-2 z-50 w-[220px]">
                  <div className="text-[10px] text-gray-400 uppercase tracking-wide mb-1.5">Add a color</div>
                  <div className="flex gap-1 mb-2">
                    <input
                      type="color"
                      value={drawColor}
                      onChange={(e) => setDrawColor(e.target.value)}
                      className="w-8 h-8 rounded border border-gray-200 cursor-pointer"
                    />
                    <input
                      type="text"
                      defaultValue={drawColor}
                      placeholder="#000000"
                      maxLength={7}
                      className="flex-1 text-xs px-2 py-1 border border-gray-200 rounded font-mono"
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          const val = (e.target as HTMLInputElement).value;
                          const cleaned = val.startsWith("#") ? val : `#${val}`;
                          if (/^#[0-9a-fA-F]{6}$/.test(cleaned)) {
                            setDrawColor(cleaned);
                            if (!customSwatches.includes(cleaned.toLowerCase()) && !paletteSwatches.includes(cleaned.toLowerCase())) {
                              setCustomSwatches((prev) => [...prev, cleaned.toLowerCase()]);
                            }
                            setShowSwatchPicker(false);
                          }
                        }
                      }}
                    />
                  </div>
                  <button
                    onClick={() => {
                      const color = drawColor.toLowerCase();
                      if (!customSwatches.includes(color) && !paletteSwatches.includes(color)) {
                        setCustomSwatches((prev) => [...prev, color]);
                      }
                      setShowSwatchPicker(false);
                    }}
                    className="w-full px-2 py-1.5 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 font-medium"
                  >Add to palette</button>
                </div>
              )}
            </div>
          </div>
        )}
        {mode === "shape" && (
          <ShapePicker
            activeShape={activeShape}
            onShapeChange={setActiveShape}
            fillColor={drawColor}
            onFillColorChange={setDrawColor}
            strokeColor={shapeStrokeColor}
            onStrokeColorChange={setShapeStrokeColor}
            strokeWidth={shapeStrokeWidth}
            onStrokeWidthChange={setShapeStrokeWidth}
            cornerRadius={shapeCornerRadius}
            onCornerRadiusChange={setShapeCornerRadius}
            starPoints={shapeStarPoints}
            onStarPointsChange={setShapeStarPoints}
            polygonSides={shapePolygonSides}
            onPolygonSidesChange={setShapePolygonSides}
            innerRadiusRatio={shapeInnerRadiusRatio}
            onInnerRadiusRatioChange={setShapeInnerRadiusRatio}
          />
        )}
        {mode === "reshape" && (
          <div className="flex items-center gap-2 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-1.5">
            <label className="flex items-center gap-1.5 text-xs text-gray-500 cursor-pointer">
              <input
                type="checkbox"
                checked={showControlHandles}
                onChange={(e) => setShowControlHandles(e.target.checked)}
                className="accent-blue-600"
              />
              Handles
            </label>
            {reshapePath !== null && (
              <>
                <div className="h-4 w-px bg-gray-300" />
                <span className="text-xs text-gray-500">
                  {selectedAnchor
                    ? "Anchor selected"
                    : "Drag dots to reshape · double-click curve to add node"}
                </span>
              </>
            )}
          </div>
        )}
      </div>

      {/* Top-right: reshape anchor actions */}
      {mode === "reshape" && selectedAnchor && reshapePath !== null && vectorResult && (
        <div className="absolute top-3 right-3 flex items-center gap-2 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-1.5" style={{ cursor: "default" }} onMouseDown={(e) => e.stopPropagation()} onMouseEnter={() => setOverControls(true)} onMouseLeave={() => setOverControls(false)}>
          <span className="text-xs text-gray-500">
            Node {selectedAnchor.isEnd ? vectorResult.paths[reshapePath].segments.length : selectedAnchor.segIndex + 1} selected
          </span>
          <div className="h-4 w-px bg-gray-300" />
          <button onClick={deleteAnchor}
            disabled={vectorResult.paths[reshapePath].segments.length < 2}
            className="px-2.5 py-1 text-xs text-red-600 hover:bg-red-50 rounded-md font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            title="Delete node (Delete key)"
          >🗑 Delete Node</button>
          <button onClick={() => setSelectedAnchor(null)}
            className="px-2.5 py-1 text-xs text-gray-500 hover:bg-gray-100 rounded-md font-medium transition-colors"
            title="Deselect (Escape)"
          >✕</button>
        </div>
      )}

      {/* Top-right: reshape path actions (path selected, no node selected) */}
      {mode === "reshape" && !selectedAnchor && reshapePath !== null && vectorResult && (
        <div className="absolute top-3 right-3 flex flex-col gap-1 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-1.5" style={{ cursor: "default" }} onMouseDown={(e) => e.stopPropagation()} onMouseEnter={() => setOverControls(true)} onMouseLeave={() => setOverControls(false)}>
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">
              Path selected · {vectorResult.paths[reshapePath].segments.length} nodes
            </span>
            <div className="h-4 w-px bg-gray-300" />
            <button onClick={simplifyPath}
              disabled={vectorResult.paths[reshapePath].segments.length < 3}
              className="px-2.5 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded-md font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              title="Reduce nodes to simplify path shape"
            >✨ Simplify</button>
            <button onClick={() => setReshapePath(null)}
              className="px-2.5 py-1 text-xs text-gray-500 hover:bg-gray-100 rounded-md font-medium transition-colors"
              title="Deselect path (Escape)"
            >✕</button>
          </div>
          <div className="flex items-center gap-2">
            <input type="range" min={1} max={100} value={scalePercent}
              onChange={(e) => setScalePercent(Number(e.target.value))}
              className="w-20 h-1 accent-blue-600"
              title={`Scale: ${scalePercent}%`}
            />
            <span className="text-xs text-gray-500 w-7 text-right">{scalePercent}%</span>
            <div className="h-4 w-px bg-gray-300" />
            <button onClick={() => scalePath(-scalePercent)}
              className="px-2.5 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded-md font-medium transition-colors"
              title={`Shrink by ${scalePercent}%`}
            >⊖ Shrink</button>
            <button onClick={() => scalePath(scalePercent)}
              className="px-2.5 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded-md font-medium transition-colors"
              title={`Enlarge by ${scalePercent}%`}
            >⊕ Enlarge</button>
          </div>
        </div>
      )}

      {/* Top-right: selection actions */}
      {mode === "select" && selectedPaths.size > 0 && (
        <div className="absolute top-3 right-3 flex flex-col gap-1.5 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-2" style={{ cursor: "default" }} onMouseDown={(e) => e.stopPropagation()} onMouseEnter={() => setOverControls(true)} onMouseLeave={() => setOverControls(false)}>
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">
              {selectedPaths.size} path{selectedPaths.size !== 1 ? "s" : ""} selected
            </span>
            <button onClick={() => setSelectedPaths(new Set())}
             className="px-1.5 py-0.5 text-xs text-gray-500 hover:bg-gray-100 rounded-md font-medium transition-colors ml-auto"
             title="Deselect all (Escape)"
            >✕</button>
          </div>
          <div className="flex items-center gap-2">
            <button onClick={deleteSelected}
              className="px-2.5 py-1 text-xs text-red-600 hover:bg-red-50 rounded-md font-medium transition-colors"
              title="Delete selected (Delete key)"
            >🗑 Delete</button>
            <button onClick={duplicateSelected}
              className="px-2.5 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded-md font-medium transition-colors"
              title="Duplicate selected"
            >📋 Duplicate</button>
            {selectedPaths.size >= 2 && (
              <button onClick={mergeSelected}
                className="px-2.5 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded-md font-medium transition-colors"
                title="Merge selected paths into one"
              >🔗 Merge</button>
            )}
          </div>
          <div className="flex items-center gap-2">
            <button onClick={sendToBack}
             className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors"
             title="Send to back"
            >⏮️</button>
            <button onClick={moveBackward}
             className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors"
             title="Move backward"
            >⏪</button>
            <button onClick={moveForward}
             className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors"
             title="Move forward"
            >⏩</button>
            <button onClick={bringToFront}
             className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md font-medium transition-colors"
             title="Bring to front"
            >⏭️</button>
          </div>
        </div>
      )}

      {/* Bottom-right: zoom controls */}
      <div className="absolute bottom-3 right-3 flex items-center gap-1 bg-white/90 backdrop-blur rounded-lg shadow px-2 py-1" style={{ cursor: "default" }} onMouseDown={(e) => e.stopPropagation()} onMouseEnter={() => setOverControls(true)} onMouseLeave={() => setOverControls(false)}>
        <button onClick={fitToWindow} className="px-2 py-1 text-xs text-gray-600 hover:text-gray-900" title="Fit to window">Fit</button>
        <button onClick={resetZoom} className="px-2 py-1 text-xs text-gray-600 hover:text-gray-900" title="100% zoom">1:1</button>
        <span className="text-xs text-gray-500 ml-1 w-10 text-right">{Math.round(zoom * 100)}%</span>
      </div>
    </div>
  );
}

function getPathBounds(path: VectorPath) {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const seg of path.segments) {
    for (const p of [seg.curve.p0, seg.curve.p1, seg.curve.p2, seg.curve.p3]) {
      if (p.x < minX) minX = p.x;
      if (p.y < minY) minY = p.y;
      if (p.x > maxX) maxX = p.x;
      if (p.y > maxY) maxY = p.y;
    }
  }
  return { minX, minY, maxX, maxY };
}

/**
 * Apply Shift (constrain aspect) and Alt (draw from center) modifiers
 * to the shape drag start/end points.
 */
function applyShapeConstraints(
  start: Point,
  end: Point,
  shift: boolean,
  alt: boolean
): { effectiveStart: Point; effectiveEnd: Point } {
  let dx = end.x - start.x;
  let dy = end.y - start.y;

  // Shift: constrain to square / circle (equal width & height)
  if (shift) {
    const maxDim = Math.max(Math.abs(dx), Math.abs(dy));
    dx = maxDim * Math.sign(dx || 1);
    dy = maxDim * Math.sign(dy || 1);
  }

  if (alt) {
    // Alt: draw from center — start is center, expand equally in all directions
    return {
      effectiveStart: { x: start.x - dx, y: start.y - dy },
      effectiveEnd: { x: start.x + dx, y: start.y + dy },
    };
  }

  return {
    effectiveStart: start,
    effectiveEnd: { x: start.x + dx, y: start.y + dy },
  };
}

function buildShapeSegments(
  shape: ShapeType,
  start: Point,
  end: Point,
  opts: { cornerRadius: number; starPoints: number; polygonSides: number; innerRadiusRatio: number }
): BezierSegment[] | null {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const cx = (start.x + end.x) / 2;
  const cy = (start.y + end.y) / 2;
  const width = Math.abs(dx);
  const height = Math.abs(dy);

  if (width < 2 && height < 2) return null;

  switch (shape) {
    case "ellipse":
      return generateEllipse({ cx, cy, rx: width / 2, ry: height / 2 });
    case "rectangle":
      return generateRect({
        x: Math.min(start.x, end.x),
        y: Math.min(start.y, end.y),
        width,
        height,
        cornerRadius: opts.cornerRadius,
      });
    case "triangle":
      return generateTriangle({ cx, cy, width, height });
    case "star":
      return generateStar({
        cx, cy,
        outerRadius: Math.max(width, height) / 2,
        innerRadiusRatio: opts.innerRadiusRatio,
        points: opts.starPoints,
      });
    case "polygon":
      return generatePolygon({ cx, cy, radius: Math.max(width, height) / 2, sides: opts.polygonSides });
    case "heart":
      return generateHeart({ cx, cy, size: Math.max(width, height) });
    default:
      return null;
  }
}

function drawVectorPath(ctx: CanvasRenderingContext2D, path: VectorPath) {
  if (path.segments.length === 0) return;
  const { r, g, b } = path.fill_color;
  ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;
  ctx.beginPath();
  const first = path.segments[0].curve;
  ctx.moveTo(first.p0.x, first.p0.y);
  for (const seg of path.segments) {
    const { p1, p2, p3 } = seg.curve;
    ctx.bezierCurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
  }
  if (path.is_closed) ctx.closePath();
  ctx.fill();

  // Optional stroke
  if (path.stroke_color && path.stroke_width && path.stroke_width > 0) {
    const sc = path.stroke_color;
    ctx.strokeStyle = `rgb(${sc.r}, ${sc.g}, ${sc.b})`;
    ctx.lineWidth = path.stroke_width;
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.stroke();
  }
}
