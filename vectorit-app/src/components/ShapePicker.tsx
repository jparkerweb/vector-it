import { SHAPE_LABELS, type ShapeType } from "../utils/shapeGenerators";
import { InlineColorPicker } from "./InlineColorPicker";

interface ShapePickerProps {
  activeShape: ShapeType;
  onShapeChange: (shape: ShapeType) => void;
  fillColor: string;
  onFillColorChange: (color: string) => void;
  strokeColor: string;
  onStrokeColorChange: (color: string) => void;
  strokeWidth: number;
  onStrokeWidthChange: (w: number) => void;
  cornerRadius: number;
  onCornerRadiusChange: (r: number) => void;
  starPoints: number;
  onStarPointsChange: (p: number) => void;
  polygonSides: number;
  onPolygonSidesChange: (s: number) => void;
  innerRadiusRatio: number;
  onInnerRadiusRatioChange: (r: number) => void;
}

const SHAPES = Object.keys(SHAPE_LABELS) as ShapeType[];

export function ShapePicker({
  activeShape,
  onShapeChange,
  fillColor,
  onFillColorChange,
  strokeColor,
  onStrokeColorChange,
  strokeWidth,
  onStrokeWidthChange,
  cornerRadius,
  onCornerRadiusChange,
  starPoints,
  onStarPointsChange,
  polygonSides,
  onPolygonSidesChange,
  innerRadiusRatio,
  onInnerRadiusRatioChange,
}: ShapePickerProps) {
  return (
    <div className="flex flex-col gap-2 bg-white/90 backdrop-blur rounded-lg shadow px-3 py-2">
      {/* Shape selector grid */}
      <div className="flex items-center gap-1 flex-wrap">
        {SHAPES.map((shape) => (
          <button
            key={shape}
            onClick={() => onShapeChange(shape)}
            className={`w-7 h-7 flex items-center justify-center rounded text-sm transition-colors ${
              activeShape === shape
                ? "bg-blue-600 text-white"
                : "text-gray-600 hover:bg-gray-100"
            }`}
            title={SHAPE_LABELS[shape].label}
          >
            {SHAPE_LABELS[shape].icon}
          </button>
        ))}
      </div>

      {/* Color options */}
      <div className="flex items-center gap-2 flex-wrap">
        <label className="text-xs text-gray-500">Fill</label>
        <InlineColorPicker value={fillColor} onChange={onFillColorChange} />

        <div className="h-4 w-px bg-gray-200" />

        <label className="text-xs text-gray-500">Stroke</label>
        <InlineColorPicker value={strokeColor} onChange={onStrokeColorChange} />

        <label className="text-xs text-gray-500">W</label>
        <input
          type="range"
          min={0}
          max={20}
          step={0.5}
          value={strokeWidth}
          onChange={(e) => onStrokeWidthChange(Number(e.target.value))}
          className="w-20 accent-blue-600"
        />
        <span className="text-xs text-gray-500 w-6">{strokeWidth}</span>
      </div>

      {/* Shape-specific options */}
      {activeShape === "rectangle" && (
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500">Radius</label>
          <input
            type="range"
            min={0}
            max={50}
            value={cornerRadius}
            onChange={(e) => onCornerRadiusChange(Number(e.target.value))}
            className="w-24 accent-blue-600"
          />
          <span className="text-xs text-gray-500 w-6">{cornerRadius}</span>
        </div>
      )}

      {activeShape === "star" && (
        <div className="flex items-center gap-2 flex-wrap">
          <label className="text-xs text-gray-500">Points</label>
          <input
            type="range"
            min={3}
            max={12}
            value={starPoints}
            onChange={(e) => onStarPointsChange(Number(e.target.value))}
            className="w-20 accent-blue-600"
          />
          <span className="text-xs text-gray-500 w-4">{starPoints}</span>
          <label className="text-xs text-gray-500">Inner</label>
          <input
            type="range"
            min={20}
            max={80}
            value={Math.round(innerRadiusRatio * 100)}
            onChange={(e) => onInnerRadiusRatioChange(Number(e.target.value) / 100)}
            className="w-20 accent-blue-600"
          />
          <span className="text-xs text-gray-500 w-8">{Math.round(innerRadiusRatio * 100)}%</span>
        </div>
      )}

      {activeShape === "polygon" && (
        <div className="flex items-center gap-2">
          <label className="text-xs text-gray-500">Sides</label>
          <input
            type="range"
            min={3}
            max={12}
            value={polygonSides}
            onChange={(e) => onPolygonSidesChange(Number(e.target.value))}
            className="w-24 accent-blue-600"
          />
          <span className="text-xs text-gray-500 w-4">{polygonSides}</span>
        </div>
      )}

      <div className="text-[10px] text-gray-400">
        Shift = constrain • Alt = from center
      </div>
    </div>
  );
}
