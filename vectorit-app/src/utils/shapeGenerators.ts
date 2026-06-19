/**
 * Pure shape generator functions.
 * Each returns BezierSegment[] ready to wrap in a VectorPath.
 */
import type { BezierSegment, Point } from "../stores/appStore";

// Bézier circle constant: κ = 4(√2 - 1) / 3 ≈ 0.5522847
const KAPPA = 0.5522847498;

function seg(p0: Point, p1: Point, p2: Point, p3: Point, isCorner = false): BezierSegment {
  return { curve: { p0, p1, p2, p3 }, is_corner_start: isCorner };
}

/** Degenerate cubic for a straight line segment */
function lineSeg(a: Point, b: Point, isCorner = true): BezierSegment {
  return seg(a, a, b, b, isCorner);
}

// ─── Ellipse / Circle ────────────────────────────────────────────────────────

export interface EllipseParams {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
}

/** 4-arc Bézier approximation of an ellipse */
export function generateEllipse({ cx, cy, rx, ry }: EllipseParams): BezierSegment[] {
  const kx = rx * KAPPA;
  const ky = ry * KAPPA;

  // Start at top, go clockwise
  const top: Point = { x: cx, y: cy - ry };
  const right: Point = { x: cx + rx, y: cy };
  const bottom: Point = { x: cx, y: cy + ry };
  const left: Point = { x: cx - rx, y: cy };

  return [
    seg(top, { x: cx + kx, y: cy - ry }, { x: cx + rx, y: cy - ky }, right),
    seg(right, { x: cx + rx, y: cy + ky }, { x: cx + kx, y: cy + ry }, bottom),
    seg(bottom, { x: cx - kx, y: cy + ry }, { x: cx - rx, y: cy + ky }, left),
    seg(left, { x: cx - rx, y: cy - ky }, { x: cx - kx, y: cy - ry }, top),
  ];
}

// ─── Rectangle ───────────────────────────────────────────────────────────────

export interface RectParams {
  x: number;
  y: number;
  width: number;
  height: number;
  cornerRadius?: number;
}

export function generateRect({ x, y, width, height, cornerRadius = 0 }: RectParams): BezierSegment[] {
  const r = Math.min(cornerRadius, Math.min(Math.abs(width), Math.abs(height)) / 2);

  if (r <= 0) {
    // Simple rectangle — 4 line segments
    const tl: Point = { x, y };
    const tr: Point = { x: x + width, y };
    const br: Point = { x: x + width, y: y + height };
    const bl: Point = { x, y: y + height };
    return [lineSeg(tl, tr), lineSeg(tr, br), lineSeg(br, bl), lineSeg(bl, tl)];
  }

  // Rounded rectangle — 4 lines + 4 quarter-circle arcs
  const k = r * KAPPA;
  const segments: BezierSegment[] = [];

  // Top edge (left-to-right, excluding corners)
  const topLeft: Point = { x: x + r, y };
  const topRight: Point = { x: x + width - r, y };
  segments.push(lineSeg(topLeft, topRight));

  // Top-right corner arc
  segments.push(seg(
    topRight,
    { x: x + width - r + k, y },
    { x: x + width, y: y + r - k },
    { x: x + width, y: y + r }
  ));

  // Right edge
  segments.push(lineSeg({ x: x + width, y: y + r }, { x: x + width, y: y + height - r }));

  // Bottom-right corner arc
  segments.push(seg(
    { x: x + width, y: y + height - r },
    { x: x + width, y: y + height - r + k },
    { x: x + width - r + k, y: y + height },
    { x: x + width - r, y: y + height }
  ));

  // Bottom edge (right-to-left)
  segments.push(lineSeg({ x: x + width - r, y: y + height }, { x: x + r, y: y + height }));

  // Bottom-left corner arc
  segments.push(seg(
    { x: x + r, y: y + height },
    { x: x + r - k, y: y + height },
    { x, y: y + height - r + k },
    { x, y: y + height - r }
  ));

  // Left edge
  segments.push(lineSeg({ x, y: y + height - r }, { x, y: y + r }));

  // Top-left corner arc
  segments.push(seg(
    { x, y: y + r },
    { x, y: y + r - k },
    { x: x + r - k, y },
    topLeft
  ));

  return segments;
}

// ─── Triangle ────────────────────────────────────────────────────────────────

export interface TriangleParams {
  cx: number;
  cy: number;
  width: number;
  height: number;
}

export function generateTriangle({ cx, cy, width, height }: TriangleParams): BezierSegment[] {
  const top: Point = { x: cx, y: cy - height / 2 };
  const bottomRight: Point = { x: cx + width / 2, y: cy + height / 2 };
  const bottomLeft: Point = { x: cx - width / 2, y: cy + height / 2 };
  return [lineSeg(top, bottomRight), lineSeg(bottomRight, bottomLeft), lineSeg(bottomLeft, top)];
}

// ─── Star ────────────────────────────────────────────────────────────────────

export interface StarParams {
  cx: number;
  cy: number;
  outerRadius: number;
  innerRadiusRatio: number; // 0-1, typically 0.4
  points: number;
  rotation?: number; // degrees
}

export function generateStar({ cx, cy, outerRadius, innerRadiusRatio, points, rotation = -90 }: StarParams): BezierSegment[] {
  const innerRadius = outerRadius * innerRadiusRatio;
  const angleStep = Math.PI / points;
  const rotRad = (rotation * Math.PI) / 180;
  const vertices: Point[] = [];

  for (let i = 0; i < points * 2; i++) {
    const angle = rotRad + i * angleStep;
    const r = i % 2 === 0 ? outerRadius : innerRadius;
    vertices.push({ x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) });
  }

  const segments: BezierSegment[] = [];
  for (let i = 0; i < vertices.length; i++) {
    const next = vertices[(i + 1) % vertices.length];
    segments.push(lineSeg(vertices[i], next));
  }
  return segments;
}

// ─── Regular Polygon ─────────────────────────────────────────────────────────

export interface PolygonParams {
  cx: number;
  cy: number;
  radius: number;
  sides: number;
  rotation?: number; // degrees
}

export function generatePolygon({ cx, cy, radius, sides, rotation = -90 }: PolygonParams): BezierSegment[] {
  const rotRad = (rotation * Math.PI) / 180;
  const angleStep = (2 * Math.PI) / sides;
  const vertices: Point[] = [];

  for (let i = 0; i < sides; i++) {
    const angle = rotRad + i * angleStep;
    vertices.push({ x: cx + radius * Math.cos(angle), y: cy + radius * Math.sin(angle) });
  }

  const segments: BezierSegment[] = [];
  for (let i = 0; i < vertices.length; i++) {
    const next = vertices[(i + 1) % vertices.length];
    segments.push(lineSeg(vertices[i], next));
  }
  return segments;
}

// ─── Line ────────────────────────────────────────────────────────────────────

export interface LineParams {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export function generateLine({ x1, y1, x2, y2 }: LineParams): BezierSegment[] {
  return [lineSeg({ x: x1, y: y1 }, { x: x2, y: y2 })];
}

// ─── Arrow ───────────────────────────────────────────────────────────────────

export interface ArrowParams {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  headSize?: number;
}

export function generateArrow({ x1, y1, x2, y2, headSize = 12 }: ArrowParams): BezierSegment[] {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const headAngle = Math.PI / 6; // 30 degrees

  const left: Point = {
    x: x2 - headSize * Math.cos(angle - headAngle),
    y: y2 - headSize * Math.sin(angle - headAngle),
  };
  const right: Point = {
    x: x2 - headSize * Math.cos(angle + headAngle),
    y: y2 - headSize * Math.sin(angle + headAngle),
  };

  const start: Point = { x: x1, y: y1 };
  const end: Point = { x: x2, y: y2 };

  return [lineSeg(start, end), lineSeg(end, left), lineSeg(end, right)];
}

// ─── Heart ───────────────────────────────────────────────────────────────────

export interface HeartParams {
  cx: number;
  cy: number;
  size: number;
}

export function generateHeart({ cx, cy, size }: HeartParams): BezierSegment[] {
  // Heart shape scaled to fit within size x size bounding box
  const s = size / 2;

  // Bottom point
  const bottom: Point = { x: cx, y: cy + s * 0.8 };
  // Top center dip
  const topCenter: Point = { x: cx, y: cy - s * 0.2 };
  // Left and right bumps
  const leftPeak: Point = { x: cx - s * 0.5, y: cy - s * 0.8 };
  const rightPeak: Point = { x: cx + s * 0.5, y: cy - s * 0.8 };

  return [
    // Left half: bottom → topCenter (left curve)
    seg(
      bottom,
      { x: cx - s * 1.0, y: cy + s * 0.2 },
      { x: cx - s * 1.0, y: cy - s * 0.6 },
      leftPeak
    ),
    // Top-left arc: leftPeak → topCenter
    seg(
      leftPeak,
      { x: cx - s * 0.1, y: cy - s * 1.0 },
      { x: cx - s * 0.05, y: cy - s * 0.4 },
      topCenter
    ),
    // Top-right arc: topCenter → rightPeak
    seg(
      topCenter,
      { x: cx + s * 0.05, y: cy - s * 0.4 },
      { x: cx + s * 0.1, y: cy - s * 1.0 },
      rightPeak
    ),
    // Right half: rightPeak → bottom
    seg(
      rightPeak,
      { x: cx + s * 1.0, y: cy - s * 0.6 },
      { x: cx + s * 1.0, y: cy + s * 0.2 },
      bottom
    ),
  ];
}

// ─── Shape Type Enum ─────────────────────────────────────────────────────────

export type ShapeType =
  | "ellipse"
  | "rectangle"
  | "triangle"
  | "star"
  | "polygon"
  | "heart";

export const SHAPE_LABELS: Record<ShapeType, { icon: string; label: string }> = {
  ellipse: { icon: "⬭", label: "Ellipse" },
  rectangle: { icon: "▬", label: "Rectangle" },
  triangle: { icon: "△", label: "Triangle" },
  star: { icon: "☆", label: "Star" },
  polygon: { icon: "⬡", label: "Polygon" },
  heart: { icon: "♡", label: "Heart" },
};
