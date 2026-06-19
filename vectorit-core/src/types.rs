use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core error type for VectorIt operations.
#[derive(Debug, Error)]
pub enum VectorItError {
    #[error("Failed to decode image: {0}")]
    ImageDecode(String),

    #[error("Quantization failed: {0}")]
    QuantizationFailed(String),

    #[error("Image too large: {actual_mp:.1} MP exceeds limit of {max_mp:.1} MP")]
    ImageTooLarge { actual_mp: f64, max_mp: f64 },

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error("Pipeline cancelled")]
    Cancelled,

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VectorItError>;

/// A 2D point with sub-pixel precision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// Raw decoded image in RGBA format.
#[derive(Debug, Clone)]
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
    pub has_alpha: bool,
}

/// A color in CIE Lab color space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LabColor {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

/// An RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// A color palette extracted from the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub colors: Vec<LabColor>,
}

impl Palette {
    /// Convert palette colors to RGB for export.
    pub fn to_rgb(&self) -> Vec<RgbColor> {
        use palette::{FromColor, Lab, Srgb};
        self.colors
            .iter()
            .map(|lab| {
                let lab_color: Lab = Lab::new(lab.l, lab.a, lab.b);
                let rgb: Srgb<f32> = Srgb::from_color(lab_color);
                let rgb = rgb.into_format::<u8>();
                RgbColor {
                    r: rgb.red,
                    g: rgb.green,
                    b: rgb.blue,
                }
            })
            .collect()
    }
}

/// Image after color quantization.
#[derive(Debug, Clone)]
pub struct QuantizedImage {
    pub width: u32,
    pub height: u32,
    pub labels: Vec<u16>,
    pub palette: Palette,
}

/// A connected region of same-colored pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: u32,
    pub color_index: u16,
    pub pixel_count: u32,
}

/// Segmentation result: regions + per-pixel label map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segmentation {
    pub regions: Vec<Region>,
    pub label_map: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

/// A traced boundary of a region.
#[derive(Debug, Clone)]
pub struct Boundary {
    pub region_id: u32,
    pub points: Vec<Point>,
    pub is_closed: bool,
}

/// A cubic Bézier curve segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CubicBezier {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl CubicBezier {
    /// Evaluate the curve at parameter t ∈ [0, 1].
    pub fn eval(&self, t: f64) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;
        Point::new(
            mt3 * self.p0.x + 3.0 * mt2 * t * self.p1.x + 3.0 * mt * t2 * self.p2.x + t3 * self.p3.x,
            mt3 * self.p0.y + 3.0 * mt2 * t * self.p1.y + 3.0 * mt * t2 * self.p2.y + t3 * self.p3.y,
        )
    }
}

/// A segment in a vector path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierSegment {
    pub curve: CubicBezier,
    pub is_corner_start: bool,
}

/// A complete vector path with fill color and optional stroke.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPath {
    pub segments: Vec<BezierSegment>,
    pub fill_color: RgbColor,
    pub is_closed: bool,
    /// Optional stroke color. If None, no stroke is drawn (or a hairline anti-seam stroke is used).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<RgbColor>,
    /// Stroke width in SVG units. Only meaningful when stroke_color is Some.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
}

/// Configuration for the vectorization pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationConfig {
    /// Number of colors to quantize to (default: 12).
    #[serde(default = "default_color_count")]
    pub color_count: u16,
    /// Smoothness factor 0.0–1.0 (default: 0.5).
    #[serde(default = "default_smoothness")]
    pub smoothness: f64,
    /// Corner angle threshold in degrees (default: 60.0).
    #[serde(default = "default_corner_threshold")]
    pub corner_threshold: f64,
    /// Simplification tolerance (default: 1.0). Overridden by quality preset.
    #[serde(default = "default_simplify_tolerance")]
    pub simplify_tolerance: f64,
    /// Quality preset. If not Custom, overrides smoothness/corner_threshold/simplify_tolerance.
    #[serde(default = "default_quality")]
    pub quality: Quality,
    /// How to handle image transparency.
    #[serde(default)]
    pub transparency_mode: TransparencyMode,
    /// Path simplification mode: "polygon" for sharp edges, "spline" for smooth curves.
    #[serde(default = "default_path_mode")]
    pub path_mode: String,
    /// Speckle filter: discard patches smaller than N×N pixels (default: 4).
    #[serde(default = "default_speckle_filter")]
    pub speckle_filter: u32,
    /// Color precision: significant bits per RGB channel 1-8 (default: 6).
    #[serde(default = "default_color_precision")]
    pub color_precision: u32,
    /// Automatically resize large images before vectorization (default: true).
    #[serde(default = "default_auto_resize")]
    pub auto_resize: bool,
}

fn default_color_count() -> u16 { 12 }
fn default_smoothness() -> f64 { 0.5 }
fn default_corner_threshold() -> f64 { 100.0 }
fn default_simplify_tolerance() -> f64 { 1.0 }
fn default_quality() -> Quality { Quality::Custom }
fn default_path_mode() -> String { "polygon".to_string() }
fn default_speckle_filter() -> u32 { 4 }
fn default_color_precision() -> u32 { 6 }
fn default_auto_resize() -> bool { true }

impl VectorizationConfig {
    /// Resolve effective parameters, applying quality preset overrides.
    pub fn effective_smoothness(&self) -> f64 {
        match self.quality {
            Quality::Low => 0.8,
            Quality::Medium => 0.5,
            Quality::High => 0.2,
            Quality::Custom => self.smoothness,
        }
    }

    pub fn effective_corner_threshold(&self) -> f64 {
        match self.quality {
            Quality::Low => 90.0,
            Quality::Medium => 120.0,
            Quality::High => 140.0,
            Quality::Custom => self.corner_threshold,
        }
    }

    pub fn effective_simplify_tolerance(&self) -> f64 {
        match self.quality {
            Quality::Low => 2.0,
            Quality::Medium => 1.0,
            Quality::High => 0.5,
            Quality::Custom => self.simplify_tolerance,
        }
    }
}

impl Default for VectorizationConfig {
    fn default() -> Self {
        Self {
            color_count: default_color_count(),
            smoothness: default_smoothness(),
            corner_threshold: default_corner_threshold(),
            simplify_tolerance: default_simplify_tolerance(),
            quality: default_quality(),
            transparency_mode: TransparencyMode::default(),
            path_mode: default_path_mode(),
            speckle_filter: default_speckle_filter(),
            color_precision: default_color_precision(),
            auto_resize: default_auto_resize(),
        }
    }
}

/// Classification of image type for vectorization strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageType {
    Photo,
    AntiAliased,
    Aliased,
}

/// Quality level for vectorization presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Low,
    Medium,
    High,
    Custom,
}

/// Information about a detected anti-aliasing pixel.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AaPixelInfo {
    pub x: u32,
    pub y: u32,
    pub region_a: u32,
    pub region_b: u32,
    pub blend_ratio: f64,
}

/// An edit operation on a segmentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegEdit {
    PaintPixels {
        pixels: Vec<(u32, u32)>,
        target_region: u32,
    },
    SplitRegion {
        region_id: u32,
        split_line: (Point, Point),
    },
    MergeRegions {
        source: u32,
        target: u32,
    },
}

/// How to handle transparency in the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransparencyMode {
    Transparent,
    FlattenToColor(RgbColor),
}

impl Default for TransparencyMode {
    fn default() -> Self {
        TransparencyMode::Transparent
    }
}

/// The complete result of vectorization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationResult {
    pub paths: Vec<VectorPath>,
    pub palette: Palette,
    pub dimensions: (u32, u32),
    pub segmentation: Segmentation,
}


/// Export format selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Svg,
    Eps,
    Pdf,
    Dxf(DxfMode),
}

/// DXF export mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DxfMode {
    Spline,
    LineOnly(u16),
}

/// Bitmap export format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BitmapFormat {
    Png,
    Bmp,
    Jpg(u8),
}
