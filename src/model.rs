//! Data model — Rust port of `src/types/index.ts` from the SUIZA-Suite web app.
//! Field names serialize as camelCase to stay compatible with `.szt` documents
//! exported by the web prototype.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn new_id(prefix: &str) -> String {
    format!("{}-{}", prefix, now_millis())
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Persona {
    Photo,
    Vector,
    Publish,
}

impl Persona {
    pub const ALL: [Persona; 3] = [Persona::Photo, Persona::Vector, Persona::Publish];

    pub fn label(self) -> &'static str {
        match self {
            Persona::Photo => "SUIZA Photo",
            Persona::Vector => "SUIZA Vector",
            Persona::Publish => "SUIZA Publish",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolId {
    // Photo tools
    Move,
    Marquee,
    Lasso,
    Crop,
    Eyedropper,
    Healing,
    Brush,
    Eraser,
    Bucket,
    Gradient,
    Dodge,
    Burn,
    Pen,
    Text,
    Hand,
    Zoom,
    // Vector tools
    DirectSelect,
    NodeEdit,
    AnchorConvert,
    ShapeRect,
    ShapeEllipse,
    ShapePolygon,
    ShapeBuilder,
    Knife,
    // Publish tools
    FrameText,
    FramePicture,
    FrameShape,
    LineTool,
    PageTool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockTab {
    Layers,
    Typography,
    Swatches,
    Adjustments,
    Properties,
    History,
    Ai,
    Stroke,
    Pathfinder,
    Pages,
    Wrap,
    Preflight,
}

impl DockTab {
    pub fn for_persona(p: Persona) -> &'static [DockTab] {
        match p {
            Persona::Photo => &[
                DockTab::Layers,
                DockTab::Typography,
                DockTab::Swatches,
                DockTab::Adjustments,
                DockTab::Properties,
                DockTab::History,
                DockTab::Ai,
            ],
            Persona::Vector => &[
                DockTab::Layers,
                DockTab::Typography,
                DockTab::Swatches,
                DockTab::Stroke,
                DockTab::Pathfinder,
                DockTab::Properties,
                DockTab::History,
            ],
            Persona::Publish => &[
                DockTab::Pages,
                DockTab::Typography,
                DockTab::Swatches,
                DockTab::Wrap,
                DockTab::Properties,
                DockTab::Preflight,
                DockTab::Layers,
                DockTab::History,
            ],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DockTab::Layers => "Layers",
            DockTab::Typography => "Type",
            DockTab::Swatches => "Swatches",
            DockTab::Adjustments => "Adjust",
            DockTab::Properties => "Props",
            DockTab::History => "History",
            DockTab::Ai => "AI",
            DockTab::Stroke => "Stroke",
            DockTab::Pathfinder => "Pathfinder",
            DockTab::Pages => "Pages",
            DockTab::Wrap => "Wrap",
            DockTab::Preflight => "Preflight",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

impl BlendMode {
    pub const ALL: [BlendMode; 12] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerType {
    Raster,
    Vector,
    Text,
    Adjustment,
    Frame,
    Group,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdjustmentType {
    BrightnessContrast,
    Curves,
    Levels,
    Hsl,
    ColorBalance,
    Invert,
}

impl AdjustmentType {
    pub const ALL: [AdjustmentType; 6] = [
        AdjustmentType::BrightnessContrast,
        AdjustmentType::Curves,
        AdjustmentType::Levels,
        AdjustmentType::Hsl,
        AdjustmentType::ColorBalance,
        AdjustmentType::Invert,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AdjustmentType::BrightnessContrast => "Brightness / Contrast",
            AdjustmentType::Curves => "Curves",
            AdjustmentType::Levels => "Levels",
            AdjustmentType::Hsl => "HSL / Saturation",
            AdjustmentType::ColorBalance => "Color Balance",
            AdjustmentType::Invert => "Invert",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub layer_type: LayerType,
    pub visible: bool,
    pub locked: bool,
    /// 0–100
    pub opacity: f32,
    pub blend_mode: BlendMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment_type: Option<AdjustmentType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment_values: Option<BTreeMap<String, f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    /// PNG/JPEG bytes of the raster contents (replaces web `canvasDataUrl`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raster_png: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_paths: Option<Vec<VectorPath>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_font_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_font_weight: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_letter_spacing: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_line_height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<TextAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_transform: Option<TextTransform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<TextDecoration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_enabled: Option<bool>,
}

impl Layer {
    pub fn new(id: impl Into<String>, name: impl Into<String>, layer_type: LayerType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            layer_type,
            visible: true,
            locked: false,
            opacity: 100.0,
            blend_mode: BlendMode::Normal,
            adjustment_type: None,
            adjustment_values: None,
            group_id: None,
            collapsed: None,
            raster_png: None,
            vector_paths: None,
            text_content: None,
            text_font_size: None,
            text_font_family: None,
            text_color: None,
            text_font_weight: None,
            text_letter_spacing: None,
            text_line_height: None,
            text_align: None,
            text_transform: None,
            font_style: None,
            text_decoration: None,
            x: None,
            y: None,
            width: None,
            height: None,
            rotation: None,
            mask_enabled: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecoration {
    None,
    Underline,
    LineThrough,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct VectorPoint {
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp1x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp1y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp2x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cp2y: Option<f32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Polygon,
    Path,
    Text,
    Group,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorShape {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: ShapeKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_style: Option<StrokeStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<LineCap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<LineJoin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<VectorPoint>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<VectorShape>,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

impl VectorShape {
    pub fn new(kind: ShapeKind, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id: new_id("shape"),
            name: None,
            kind,
            x,
            y,
            width: w,
            height: h,
            fill: "#0d99ff".into(),
            stroke: "#000000".into(),
            stroke_width: 0.0,
            stroke_style: None,
            cap: None,
            join: None,
            points: None,
            corner_radius: None,
            text: None,
            font_size: None,
            font_family: None,
            rotation: None,
            opacity: None,
            group_id: None,
            children: Vec::new(),
            visible: true,
            locked: false,
        }
    }

    /// Bounding-box center, used by select/align ops.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameKind {
    Text,
    Picture,
    Rule,
    Shape,
    Group,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WrapMode {
    None,
    Box,
    Contour,
    Jump,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct WrapMargin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishFrame {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: FrameKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<TextAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_mode: Option<WrapMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_margin: Option<WrapMargin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<PublishFrame>,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

impl PublishFrame {
    pub fn new(kind: FrameKind, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id: new_id("frame"),
            name: None,
            kind,
            x,
            y,
            width: w,
            height: h,
            title: None,
            content: None,
            image_path: None,
            stroke_width: None,
            fill: None,
            font_family: None,
            font_size: None,
            leading: None,
            tracking: None,
            align: None,
            wrap_mode: None,
            wrap_margin: None,
            group_id: None,
            children: Vec::new(),
            visible: true,
            locked: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuideKind {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Guide {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: GuideKind,
    /// Document coordinate (x for vertical, y for horizontal).
    pub position: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnappingConfig {
    pub enabled: bool,
    pub snap_to_guides: bool,
    pub snap_to_grid: bool,
    pub snap_to_pixels: bool,
    pub snap_to_objects: bool,
    pub snap_to_doc_bounds: bool,
    pub tolerance: f32,
    pub grid_size: f32,
}

impl Default for SnappingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            snap_to_guides: true,
            snap_to_grid: false,
            snap_to_pixels: true,
            snap_to_objects: true,
            snap_to_doc_bounds: true,
            tolerance: 8.0,
            grid_size: 20.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorPath {
    pub id: String,
    pub points: Vec<VectorPoint>,
    pub closed: bool,
    pub stroke_color: String,
    pub stroke_width: f32,
    pub fill_color: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ColorMode {
    #[serde(rename = "RGB")]
    Rgb,
    #[serde(rename = "CMYK")]
    Cmyk,
    #[serde(rename = "GRAYSCALE")]
    Grayscale,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CmykColor {
    pub c: u8,
    pub m: u8,
    pub y: u8,
    pub k: u8,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub inside: f32,
    pub outside: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProject {
    pub id: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub dpi: u32,
    pub color_mode: ColorMode,
    pub cmyk_profile: String,
    pub persona: Persona,
    pub layers: Vec<Layer>,
    pub selected_layer_id: String,
    pub page_count: u32,
    pub active_page_index: u32,
    pub facing_pages: bool,
    pub bleed: f32,
    pub margins: Margins,
    #[serde(default)]
    pub guides: Vec<Guide>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShortcutCategory {
    Tools,
    File,
    Edit,
    View,
    Layer,
    Studio,
}

impl ShortcutCategory {
    pub fn label(self) -> &'static str {
        match self {
            ShortcutCategory::Tools => "Tools",
            ShortcutCategory::File => "File",
            ShortcutCategory::Edit => "Edit",
            ShortcutCategory::View => "View",
            ShortcutCategory::Layer => "Layer",
            ShortcutCategory::Studio => "Studio",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    pub id: String,
    pub name: String,
    pub category: ShortcutCategory,
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub description: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    DarkCc,
    Charcoal,
    Midnight,
    LightStudio,
    LightContrast,
}

#[derive(Clone, Copy, Debug)]
pub struct StudioTheme {
    pub id: ThemeId,
    pub name: &'static str,
    pub is_light: bool,
    pub bg_main: &'static str,
    pub bg_panel: &'static str,
    pub bg_canvas: &'static str,
    pub bg_subtle: &'static str,
    pub border: &'static str,
    pub border_strong: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub text_muted: &'static str,
    pub chip_bg: &'static str,
    pub input_bg: &'static str,
    pub hover_bg: &'static str,
    pub accent: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OsMode {
    MacOs,
    Linux,
    Windows,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginType {
    RustWasm,
    GeglOp,
    PythonGimp,
    ScribusScript,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PluginCategory {
    Retouch,
    Color,
    Typography,
    Export,
    Automation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginExtension {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub description: String,
    pub enabled: bool,
    pub category: PluginCategory,
    pub memory_safety_verified: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GradientType {
    Linear,
    Radial,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientStop {
    pub offset: f32,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorSwatch {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swatch_type: Option<SwatchKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_type: Option<GradientType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_angle: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gradient_stops: Vec<GradientStop>,
    #[serde(default)]
    pub is_none: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub is_favorite: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwatchKind {
    Solid,
    Gradient,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwatchPalette {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub swatches: Vec<ColorSwatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStep {
    pub id: String,
    pub title: String,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona: Option<Persona>,
    #[serde(default)]
    pub layers_snapshot: Vec<Layer>,
    #[serde(default)]
    pub vector_shapes_snapshot: Vec<VectorShape>,
    #[serde(default)]
    pub publish_frames_snapshot: Vec<PublishFrame>,
    #[serde(default)]
    pub guides_snapshot: Vec<Guide>,
}
