//! Application state — Rust port of `StudioContext.tsx`.
//! One `AtelierState` owns the full studio: three persona workspaces, the
//! layer stack, non-destructive history, palettes, plugins and preferences.

use crate::model::*;
use crate::presets;
use crate::raster::Selection;
use crate::shortcuts;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modal {
    Shortcuts,
    Export,
    NewDoc,
    RustArch,
    Plugins,
    CloudSync,
    LinuxNative,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyncStatus {
    Synced,
    Syncing,
    Offline,
    SavedLocal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlignOp {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathfinderOp {
    Unite,
    Subtract,
    Intersect,
    Exclude,
    Divide,
}

#[derive(Clone, Debug)]
pub struct ToolOptions {
    pub brush_size: f32,
    pub brush_hardness: f32,
    pub brush_opacity: f32,
    pub brush_flow: f32,
    pub stroke_color: String,
    pub stroke_width: f32,
    pub fill_color: String,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub text_color: String,
    pub text_align: TextAlign,
    pub column_count: u32,
    pub column_gutter: f32,
}

impl Default for ToolOptions {
    fn default() -> Self {
        Self {
            brush_size: 32.0,
            brush_hardness: 80.0,
            brush_opacity: 100.0,
            brush_flow: 100.0,
            stroke_color: "#0d99ff".into(),
            stroke_width: 3.0,
            fill_color: "#38bdf8".into(),
            font_family: "Plus Jakarta Sans".into(),
            font_size: 48.0,
            font_weight: "700".into(),
            text_color: "#ffffff".into(),
            text_align: TextAlign::Left,
            column_count: 2,
            column_gutter: 24.0,
        }
    }
}

/// Export Studio dialog state (port of `ExportModal` useState set).
#[derive(Clone, Debug)]
pub struct ExportSettings {
    pub format: String,
    pub scale: f32,
    pub quality: u32,
    pub dpi: u32,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: "png".into(),
            scale: 2.0,
            quality: 95,
            dpi: 300,
        }
    }
}

/// New-document custom size scratch fields.
#[derive(Clone, Debug)]
pub struct NewDocSettings {
    pub width: f32,
    pub height: f32,
    pub dpi: u32,
}

impl Default for NewDocSettings {
    fn default() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
            dpi: 300,
        }
    }
}

pub struct AtelierState {
    // Persona & project
    pub persona: Persona,
    pub project: DocumentProject,
    persona_stash: std::collections::HashMap<Persona, DocumentProject>,

    // Tools
    pub active_tool: ToolId,
    pub tool_options: ToolOptions,

    // Colors
    pub foreground_color: String,
    pub background_color: String,

    // Viewport
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub show_rulers: bool,
    pub show_guides: bool,
    pub show_grid: bool,
    pub guides: Vec<Guide>,
    pub snapping: SnappingConfig,
    pub lock_guides: bool,

    // History
    pub history: Vec<HistoryStep>,
    pub history_index: usize,

    // Swatches
    pub palettes: Vec<SwatchPalette>,

    // Modals & dock
    pub active_modal: Option<Modal>,
    pub dock_collapsed: bool,
    pub active_dock_tab: DockTab,

    // Vector persona
    pub vector_shapes: Vec<VectorShape>,
    pub selected_shape_ids: Vec<String>,
    pub vector_fill: String,
    pub vector_stroke: String,
    pub vector_stroke_width: f32,
    pub vector_stroke_style: StrokeStyle,
    pub vector_corner_radius: f32,

    // Publish persona
    pub publish_frames: Vec<PublishFrame>,
    pub selected_frame_ids: Vec<String>,
    pub publish_spread_index: u32,
    pub publish_master_page: String,
    pub publish_column_count: u32,
    pub publish_column_gutter: f32,
    pub publish_text_wrap_mode: WrapMode,
    pub publish_paragraph_style: String,

    // Photo multi-selection
    pub selected_layer_ids: Vec<String>,

    // Preferences
    pub theme_id: ThemeId,
    pub os_mode: OsMode,
    pub shortcuts: Vec<KeyboardShortcut>,

    // Shortcut HUD
    pub shortcut_hint: Option<(String, Instant)>,

    // Sync / autosave
    pub sync_status: SyncStatus,
    pub last_saved_at: u64,

    // AI
    pub ai_opt_in: bool,

    // Plugins
    pub plugins: Vec<PluginExtension>,

    // Dialog scratch state
    pub export: ExportSettings,
    pub new_doc: NewDocSettings,

    // Canvas interaction state
    /// Active pixel selection (marquee / lasso), in doc coords.
    pub selection: Option<Selection>,
    /// Internal clipboard for cut/copy/paste.
    pub clipboard: Clipboard,
    /// In-progress Bézier pen path (vector persona).
    pub pen_path: Vec<VectorPoint>,
    /// Healing/clone-stamp source point (doc coords), set by Alt+click.
    pub clone_source: Option<(f32, f32)>,
    /// Selected swatch palette for the dock panel.
    pub selected_palette_id: String,
    /// Last known canvas viewport size (for fit-to-screen).
    pub viewport_size: [f32; 2],
    /// Bumped whenever raster buffers change → forces recomposite.
    pub raster_version: u64,
    /// Node being dragged by Direct Select: (shape_id, point_index).
    pub editing_node: Option<(String, usize)>,
    /// Live paint stroke in progress (suppresses mid-stroke history).
    pub painting: bool,
    /// Guide currently being dragged out of a ruler: (kind).
    pub pending_guide: Option<GuideKind>,
}

/// Clipboard payload for cut/copy/paste.
#[derive(Clone, Debug, Default)]
pub enum Clipboard {
    #[default]
    Empty,
    Layers(Vec<Layer>),
    Shapes(Vec<VectorShape>),
    Frames(Vec<PublishFrame>),
}

impl AtelierState {
    pub fn new() -> Self {
        let mut stash = std::collections::HashMap::new();
        let project = create_initial_project(Persona::Photo);
        stash.insert(Persona::Vector, create_initial_project(Persona::Vector));
        stash.insert(Persona::Publish, create_initial_project(Persona::Publish));

        let initial_step = HistoryStep {
            id: new_id("step"),
            title: "Initial Document Creation".into(),
            timestamp: now_millis(),
            persona: Some(Persona::Photo),
            layers_snapshot: project.layers.clone(),
            vector_shapes_snapshot: Vec::new(),
            publish_frames_snapshot: Vec::new(),
            guides_snapshot: Vec::new(),
        };

        Self {
            persona: Persona::Photo,
            project,
            persona_stash: stash,
            active_tool: ToolId::Brush,
            tool_options: ToolOptions::default(),
            foreground_color: "#0d99ff".into(),
            background_color: "#ffffff".into(),
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            show_rulers: true,
            show_guides: true,
            show_grid: false,
            guides: vec![
                Guide {
                    id: "guide-default-h1".into(),
                    kind: GuideKind::Horizontal,
                    position: 150.0,
                    locked: false,
                    color: None,
                    name: None,
                },
                Guide {
                    id: "guide-default-v1".into(),
                    kind: GuideKind::Vertical,
                    position: 200.0,
                    locked: false,
                    color: None,
                    name: None,
                },
            ],
            snapping: SnappingConfig::default(),
            lock_guides: false,
            history: vec![initial_step],
            history_index: 0,
            palettes: presets::default_palettes(),
            active_modal: None,
            dock_collapsed: false,
            active_dock_tab: DockTab::Layers,
            vector_shapes: initial_vector_shapes(),
            selected_shape_ids: vec!["v-shape-rect-1".into()],
            vector_fill: "#0d99ff".into(),
            vector_stroke: "#0f172a".into(),
            vector_stroke_width: 2.0,
            vector_stroke_style: StrokeStyle::Solid,
            vector_corner_radius: 0.0,
            publish_frames: initial_publish_frames(),
            selected_frame_ids: vec!["pub-headline".into()],
            publish_spread_index: 0,
            publish_master_page: "A-Master".into(),
            publish_column_count: 3,
            publish_column_gutter: 12.0,
            publish_text_wrap_mode: WrapMode::Box,
            publish_paragraph_style: "Body Text".into(),
            selected_layer_ids: Vec::new(),
            theme_id: ThemeId::DarkCc,
            os_mode: if cfg!(target_os = "macos") {
                OsMode::MacOs
            } else if cfg!(target_os = "windows") {
                OsMode::Windows
            } else {
                OsMode::Linux
            },
            shortcuts: shortcuts::default_shortcuts(),
            shortcut_hint: None,
            sync_status: SyncStatus::Synced,
            last_saved_at: now_millis(),
            ai_opt_in: false,
            plugins: presets::sample_plugins(),
            export: ExportSettings::default(),
            new_doc: NewDocSettings::default(),
            selection: None,
            clipboard: Clipboard::Empty,
            pen_path: Vec::new(),
            clone_source: None,
            selected_palette_id: "palette-core".into(),
            viewport_size: [1200.0, 800.0],
            raster_version: 0,
            editing_node: None,
            painting: false,
            pending_guide: None,
        }
    }

    // ---- Shortcut HUD ----------------------------------------------------
    pub fn hint(&mut self, label: impl Into<String>) {
        self.shortcut_hint = Some((label.into(), Instant::now()));
    }

    /// True while the hint should still be on screen (web used 1200 ms).
    pub fn hint_active(&self) -> Option<&str> {
        match &self.shortcut_hint {
            Some((label, at)) if at.elapsed().as_millis() < 1200 => Some(label),
            _ => None,
        }
    }

    // ---- Colors ----------------------------------------------------------
    pub fn cmyk(&self) -> CmykColor {
        rgb_to_cmyk(&self.foreground_color)
    }

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground_color, &mut self.background_color);
    }

    pub fn reset_colors(&mut self) {
        self.foreground_color = "#000000".into();
        self.background_color = "#ffffff".into();
    }

    // ---- Persona ----------------------------------------------------------
    pub fn set_persona(&mut self, next: Persona) {
        if next == self.persona {
            return;
        }
        // Stash outgoing project, load (or create) the target persona's document.
        let outgoing = std::mem::replace(
            &mut self.project,
            self.persona_stash
                .remove(&next)
                .unwrap_or_else(|| create_initial_project(next)),
        );
        self.persona_stash.insert(self.persona, outgoing);
        self.persona = next;

        match next {
            Persona::Photo => {
                self.active_tool = ToolId::Brush;
                self.active_dock_tab = DockTab::Layers;
                self.hint("Switched to Suiza-Photo (Raster & Retouching)");
            }
            Persona::Vector => {
                self.active_tool = ToolId::Move;
                self.active_dock_tab = DockTab::Pathfinder;
                self.hint("Switched to Suiza-Vector (Bezier & Pathfinder)");
            }
            Persona::Publish => {
                self.active_tool = ToolId::FrameText;
                self.active_dock_tab = DockTab::Pages;
                self.hint("Switched to Suiza-Publish (Pages & Typography)");
            }
        }
    }

    // ---- Viewport ----------------------------------------------------------
    pub fn reset_viewport(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    pub fn fit(&mut self) {
        let [viewport_w, viewport_h] = self.viewport_size;
        self.fit_to_screen(viewport_w, viewport_h);
    }

    pub fn fit_to_screen(&mut self, viewport_w: f32, viewport_h: f32) {
        let margin = 80.0;
        let zx = (viewport_w - margin) / self.project.width.max(1.0);
        let zy = (viewport_h - margin) / self.project.height.max(1.0);
        self.zoom = zx.min(zy).clamp(0.02, 8.0);
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    // ---- History ------------------------------------------------------------
    pub fn push_history(&mut self, title: impl Into<String>) {
        let step = HistoryStep {
            id: new_id("step"),
            title: title.into(),
            timestamp: now_millis(),
            persona: Some(self.persona),
            layers_snapshot: self.project.layers.clone(),
            vector_shapes_snapshot: self.vector_shapes.clone(),
            publish_frames_snapshot: self.publish_frames.clone(),
            guides_snapshot: self.guides.clone(),
        };
        self.history.truncate(self.history_index + 1);
        self.history.push(step);
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        self.history_index = self.history.len() - 1;
    }

    pub fn restore_history(&mut self, index: usize) {
        if index >= self.history.len() {
            return;
        }
        self.history_index = index;
        let step = self.history[index].clone();
        if !step.layers_snapshot.is_empty() {
            self.project.layers = step.layers_snapshot;
            self.project.updated_at = now_millis();
        }
        self.vector_shapes = step.vector_shapes_snapshot;
        self.publish_frames = step.publish_frames_snapshot;
        self.guides = step.guides_snapshot;
        let title = step.title.clone();
        self.hint(format!("Jumped to: {title}"));
    }

    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.restore_history(self.history_index - 1);
        }
    }

    pub fn redo(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.restore_history(self.history_index + 1);
        }
    }

    pub fn clear_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let current = self.history[self.history_index.min(self.history.len() - 1)].clone();
        self.history = vec![current];
        self.history_index = 0;
        self.hint("Cleared History Stack");
    }

    pub fn create_snapshot(&mut self, title: Option<String>) {
        let t = title.unwrap_or_else(|| "Checkpoint Snapshot".into());
        self.push_history(t);
        self.hint("Created History Snapshot");
    }

    // ---- Guides --------------------------------------------------------------
    pub fn add_guide(&mut self, kind: GuideKind, position: f32) {
        let position = position.round();
        self.guides.push(Guide {
            id: new_id("guide"),
            kind,
            position,
            locked: false,
            color: None,
            name: None,
        });
        let k = if kind == GuideKind::Horizontal {
            "horizontal"
        } else {
            "vertical"
        };
        self.hint(format!("Added {k} guide at {position}px"));
    }

    pub fn remove_guide(&mut self, id: &str) {
        self.guides.retain(|g| g.id != id);
        self.hint("Removed Guide");
    }

    pub fn clear_guides(&mut self) {
        self.guides.clear();
        self.hint("Cleared all guides");
    }

    // ---- Layers --------------------------------------------------------------
    pub fn selected_layer(&self) -> Option<&Layer> {
        self.project
            .layers
            .iter()
            .find(|l| l.id == self.project.selected_layer_id)
    }

    pub fn selected_layer_mut(&mut self) -> Option<&mut Layer> {
        let id = self.project.selected_layer_id.clone();
        self.project.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn add_layer(&mut self, layer_type: LayerType, name: Option<String>) {
        let id = new_id("layer");
        let n = self.project.layers.len() + 1;
        let default_name = match layer_type {
            LayerType::Raster => format!("Pixel Layer {n}"),
            LayerType::Vector => format!("Vector Path {n}"),
            LayerType::Text => format!("Text Layer {n}"),
            _ => format!("Layer {n}"),
        };
        let mut layer = Layer::new(&id, name.unwrap_or(default_name), layer_type);
        if layer_type == LayerType::Vector {
            layer.vector_paths = Some(Vec::new());
        }
        if layer_type == LayerType::Text {
            layer.text_content = Some("New Text".into());
        }
        self.project.layers.push(layer);
        self.project.selected_layer_id = id;
        self.project.updated_at = now_millis();
        let lt = format!("{layer_type:?}").to_uppercase();
        self.push_history(format!("New {lt} Layer"));
    }

    pub fn create_layer_group(&mut self, name: Option<String>) {
        let id = new_id("group");
        let n = self
            .project
            .layers
            .iter()
            .filter(|l| l.layer_type == LayerType::Group)
            .count()
            + 1;
        let mut group = Layer::new(
            &id,
            name.unwrap_or_else(|| format!("Group {n}")),
            LayerType::Group,
        );
        group.collapsed = Some(false);
        let gname = group.name.clone();
        self.project.layers.push(group);
        self.project.selected_layer_id = id;
        self.project.updated_at = now_millis();
        self.push_history(format!("New Layer Group: {gname}"));
    }

    pub fn toggle_group_collapse(&mut self, group_id: &str) {
        if let Some(l) = self.project.layers.iter_mut().find(|l| l.id == group_id) {
            l.collapsed = Some(!l.collapsed.unwrap_or(false));
            self.project.updated_at = now_millis();
        }
    }

    pub fn add_adjustment_layer(&mut self, adj: AdjustmentType) {
        let id = new_id("adj");
        let mut layer = Layer::new(&id, adj.label(), LayerType::Adjustment);
        layer.adjustment_type = Some(adj);
        layer.adjustment_values = Some(match adj {
            AdjustmentType::BrightnessContrast => {
                BTreeMap::from([("brightness".into(), 10.0), ("contrast".into(), 15.0)])
            }
            AdjustmentType::Hsl => BTreeMap::from([
                ("hue".into(), 0.0),
                ("saturation".into(), 10.0),
                ("lightness".into(), 0.0),
            ]),
            AdjustmentType::Levels => BTreeMap::from([
                ("black point".into(), 0.0),
                ("midtones".into(), 0.0),
                ("white point".into(), 0.0),
            ]),
            AdjustmentType::Curves => BTreeMap::from([
                ("shadows".into(), 0.0),
                ("midtones".into(), 0.0),
                ("highlights".into(), 0.0),
            ]),
            AdjustmentType::ColorBalance => BTreeMap::from([
                ("cyan-red".into(), 0.0),
                ("magenta-green".into(), 0.0),
                ("yellow-blue".into(), 0.0),
            ]),
            AdjustmentType::Invert => BTreeMap::from([("strength".into(), 100.0)]),
        });
        let label = adj.label().to_string();
        self.project.layers.push(layer);
        self.project.selected_layer_id = id;
        self.project.updated_at = now_millis();
        self.push_history(format!("Add {label}"));
    }

    pub fn delete_layer(&mut self, id: &str) {
        if self.project.layers.len() <= 1 {
            return;
        }
        self.project.layers.retain(|l| l.id != id);
        if let Some(last) = self.project.layers.last() {
            self.project.selected_layer_id = last.id.clone();
        }
        self.project.updated_at = now_millis();
        self.push_history("Delete Layer");
    }

    pub fn duplicate_layer(&mut self, id: &str) {
        let Some(idx) = self.project.layers.iter().position(|l| l.id == id) else {
            return;
        };
        let mut clone = self.project.layers[idx].clone();
        clone.id = new_id("layer");
        clone.name = format!("{} Copy", clone.name);
        let clone_id = clone.id.clone();
        self.project.layers.insert(idx + 1, clone);
        self.project.selected_layer_id = clone_id;
        self.project.updated_at = now_millis();
        self.push_history("Duplicate Layer");
    }

    pub fn toggle_layer_visibility(&mut self, id: &str) {
        if let Some(l) = self.project.layers.iter_mut().find(|l| l.id == id) {
            l.visible = !l.visible;
            self.project.updated_at = now_millis();
        }
    }

    pub fn toggle_layer_lock(&mut self, id: &str) {
        if let Some(l) = self.project.layers.iter_mut().find(|l| l.id == id) {
            l.locked = !l.locked;
            self.project.updated_at = now_millis();
        }
    }

    pub fn set_layer_opacity(&mut self, id: &str, opacity: f32) {
        if let Some(l) = self.project.layers.iter_mut().find(|l| l.id == id) {
            l.opacity = opacity.clamp(0.0, 100.0);
            self.project.updated_at = now_millis();
        }
    }

    pub fn set_layer_blend_mode(&mut self, id: &str, mode: BlendMode) {
        if let Some(l) = self.project.layers.iter_mut().find(|l| l.id == id) {
            l.blend_mode = mode;
            self.project.updated_at = now_millis();
        }
        self.push_history(format!("Change Blend Mode: {}", mode.label()));
    }

    pub fn move_layer(&mut self, from: usize, to: usize) {
        let len = self.project.layers.len();
        if from >= len || to >= len {
            return;
        }
        let layer = self.project.layers.remove(from);
        self.project.layers.insert(to, layer);
        self.project.updated_at = now_millis();
        self.push_history("Reorder Layer Stack");
    }

    pub fn merge_layer_down(&mut self, id: &str) {
        let Some(idx) = self.project.layers.iter().position(|l| l.id == id) else {
            return;
        };
        if idx == 0 {
            return;
        }
        let upper = self.project.layers.remove(idx);
        let lower = &mut self.project.layers[idx - 1];
        lower.name = format!("{} + {}", lower.name, upper.name);
        let merged_id = lower.id.clone();
        self.project.selected_layer_id = merged_id;
        self.project.updated_at = now_millis();
        self.push_history("Merge Layers Down");
    }

    // ---- Project props ---------------------------------------------------------
    pub fn update_project_name(&mut self, name: String) {
        self.project.name = name;
        self.project.updated_at = now_millis();
    }

    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.project.color_mode = mode;
        self.project.updated_at = now_millis();
        self.push_history(format!("Convert Color Mode to {mode:?}"));
    }

    // ---- Vector ops -------------------------------------------------------------
    pub fn update_selected_vector_shapes(&mut self, f: impl Fn(&mut VectorShape)) {
        for s in &mut self.vector_shapes {
            if self.selected_shape_ids.contains(&s.id) {
                f(s);
            }
        }
    }

    pub fn apply_pathfinder(&mut self, op: PathfinderOp) {
        let mut targets: Vec<VectorShape> = self
            .vector_shapes
            .iter()
            .filter(|s| self.selected_shape_ids.contains(&s.id))
            .cloned()
            .collect();
        if targets.len() < 2 && self.vector_shapes.len() >= 2 {
            targets = self.vector_shapes[..2].to_vec();
        }
        if targets.len() < 2 {
            self.hint("Pathfinder requires at least 2 overlapping shapes");
            return;
        }
        let (s1, s2) = (targets[0].clone(), targets[1].clone());
        let remove = |shapes: &mut Vec<VectorShape>| {
            shapes.retain(|s| s.id != s1.id && s.id != s2.id);
        };

        let result: Option<(VectorShape, &'static str)> = match op {
            PathfinderOp::Unite => {
                let min_x = s1.x.min(s2.x);
                let min_y = s1.y.min(s2.y);
                let max_x = (s1.x + s1.width).max(s2.x + s2.width);
                let max_y = (s1.y + s1.height).max(s2.y + s2.height);
                let mut sh =
                    VectorShape::new(ShapeKind::Rect, min_x, min_y, max_x - min_x, max_y - min_y);
                sh.fill = if s2.fill.is_empty() {
                    s1.fill.clone()
                } else {
                    s2.fill.clone()
                };
                sh.stroke = if s2.stroke.is_empty() {
                    s1.stroke.clone()
                } else {
                    s2.stroke.clone()
                };
                sh.stroke_width = if s2.stroke_width > 0.0 {
                    s2.stroke_width
                } else {
                    s1.stroke_width
                };
                sh.corner_radius = Some(6.0);
                Some((sh, "Pathfinder: Unite (Merged Shapes)"))
            }
            PathfinderOp::Subtract => {
                let w = (s1.width - (s1.x + s1.width - s2.x).max(0.0) * 0.5).max(20.0);
                let mut sh = VectorShape::new(ShapeKind::Rect, s1.x, s1.y, w, s1.height);
                sh.fill = s1.fill.clone();
                sh.stroke = s1.stroke.clone();
                sh.stroke_width = s1.stroke_width;
                sh.corner_radius = s1.corner_radius;
                Some((sh, "Pathfinder: Minus Front"))
            }
            PathfinderOp::Intersect => {
                let ix = s1.x.max(s2.x);
                let iy = s1.y.max(s2.y);
                let iw = (s1.x + s1.width).min(s2.x + s2.width) - ix;
                let ih = (s1.y + s1.height).min(s2.y + s2.height) - iy;
                if iw > 0.0 && ih > 0.0 {
                    let mut sh = VectorShape::new(ShapeKind::Ellipse, ix, iy, iw, ih);
                    sh.fill = s2.fill.clone();
                    sh.stroke = s2.stroke.clone();
                    sh.stroke_width = s2.stroke_width;
                    Some((sh, "Pathfinder: Intersect Overlap"))
                } else {
                    self.hint("No geometric intersection between shapes");
                    None
                }
            }
            PathfinderOp::Exclude => {
                let min_x = s1.x.min(s2.x);
                let min_y = s1.y.min(s2.y);
                let w = (s1.x + s1.width).max(s2.x + s2.width) - min_x;
                let h = (s1.y + s1.height).max(s2.y + s2.height) - min_y;
                let mut sh = VectorShape::new(ShapeKind::Polygon, min_x, min_y, w, h);
                sh.fill = s1.fill.clone();
                sh.stroke = s2.stroke.clone();
                sh.stroke_width = 2.0;
                Some((sh, "Pathfinder: Exclude Overlap"))
            }
            PathfinderOp::Divide => {
                let mut p1 = s1.clone();
                p1.id = new_id("v-div-1");
                p1.width *= 0.6;
                let mut p2 = s2.clone();
                p2.id = new_id("v-div-2");
                p2.width *= 0.6;
                remove(&mut self.vector_shapes);
                let ids = vec![p1.id.clone(), p2.id.clone()];
                self.vector_shapes.push(p1);
                self.vector_shapes.push(p2);
                self.selected_shape_ids = ids;
                self.push_history("Pathfinder: Divide");
                self.hint("Pathfinder: Divide Subpaths");
                return;
            }
        };

        if let Some((shape, label)) = result {
            let id = shape.id.clone();
            remove(&mut self.vector_shapes);
            self.vector_shapes.push(shape);
            self.selected_shape_ids = vec![id];
            self.push_history(label);
            self.hint(label);
        }
    }

    // ---- Selection / grouping -------------------------------------------------
    pub fn group_selected(&mut self) {
        match self.persona {
            Persona::Vector => {
                if self.selected_shape_ids.len() < 2 {
                    self.hint("Select at least 2 shapes to group (Shift+Click)");
                    return;
                }
                let gid = new_id("grp");
                self.update_selected_vector_shapes(|s| s.group_id = Some(gid.clone()));
                let n = self.selected_shape_ids.len();
                self.push_history(format!("Group {n} Vector Shapes"));
                self.hint(format!("Grouped {n} Objects"));
            }
            Persona::Publish => {
                if self.selected_frame_ids.len() < 2 {
                    self.hint("Select at least 2 frames to group (Shift+Click)");
                    return;
                }
                let gid = new_id("grp");
                for f in &mut self.publish_frames {
                    if self.selected_frame_ids.contains(&f.id) {
                        f.group_id = Some(gid.clone());
                    }
                }
                let n = self.selected_frame_ids.len();
                self.push_history(format!("Group {n} Publication Frames"));
                self.hint(format!("Grouped {n} Frames"));
            }
            Persona::Photo => {
                self.create_layer_group(None);
                self.hint("Grouped Layers");
            }
        }
    }

    pub fn ungroup_selected(&mut self) {
        match self.persona {
            Persona::Vector => {
                self.update_selected_vector_shapes(|s| s.group_id = None);
                self.push_history("Ungroup Vector Objects");
            }
            Persona::Publish => {
                for f in &mut self.publish_frames {
                    if self.selected_frame_ids.contains(&f.id) {
                        f.group_id = None;
                    }
                }
                self.push_history("Ungroup Publication Frames");
            }
            Persona::Photo => {}
        }
        self.hint("Ungrouped Objects");
    }

    pub fn select_all(&mut self) {
        match self.persona {
            Persona::Vector => {
                self.selected_shape_ids = self.vector_shapes.iter().map(|s| s.id.clone()).collect();
            }
            Persona::Publish => {
                self.selected_frame_ids =
                    self.publish_frames.iter().map(|f| f.id.clone()).collect();
            }
            Persona::Photo => {
                self.selected_layer_ids =
                    self.project.layers.iter().map(|l| l.id.clone()).collect();
                self.selection = Some(crate::raster::Selection::Rect(
                    0.0,
                    0.0,
                    self.project.width,
                    self.project.height,
                ));
            }
        }
        self.hint("Selected All");
    }

    pub fn deselect_all(&mut self) {
        self.selected_shape_ids.clear();
        self.selected_frame_ids.clear();
        self.selected_layer_ids.clear();
        self.selection = None;
        self.hint("Deselected");
    }

    pub fn delete_selected(&mut self) {
        match self.persona {
            Persona::Vector => {
                if self.selected_shape_ids.is_empty() {
                    return;
                }
                self.vector_shapes
                    .retain(|s| !self.selected_shape_ids.contains(&s.id));
                self.selected_shape_ids.clear();
                self.push_history("Delete Selected Shapes");
                self.hint("Deleted Selected Shape(s)");
            }
            Persona::Publish => {
                if self.selected_frame_ids.is_empty() {
                    return;
                }
                self.publish_frames
                    .retain(|f| !self.selected_frame_ids.contains(&f.id));
                self.selected_frame_ids.clear();
                self.push_history("Delete Selected Frames");
                self.hint("Deleted Selected Frame(s)");
            }
            Persona::Photo => {
                let (id, locked) = match self.selected_layer() {
                    Some(l) => (l.id.clone(), l.locked),
                    None => return,
                };
                if !locked {
                    self.delete_layer(&id);
                    self.hint("Deleted Layer");
                }
            }
        }
    }

    pub fn nudge_selected(&mut self, dx: f32, dy: f32) {
        match self.persona {
            Persona::Vector => self.update_selected_vector_shapes(|s| {
                s.x += dx;
                s.y += dy;
            }),
            Persona::Publish => {
                for f in &mut self.publish_frames {
                    if self.selected_frame_ids.contains(&f.id) {
                        f.x += dx;
                        f.y += dy;
                    }
                }
            }
            Persona::Photo => {
                if let Some(l) = self.selected_layer_mut() {
                    l.x = Some(l.x.unwrap_or(0.0) + dx);
                    l.y = Some(l.y.unwrap_or(0.0) + dy);
                }
            }
        }
    }

    /// Move selection one step toward the viewer (top of z-order).
    pub fn bring_forward(&mut self) {
        fn fwd<T: HasId>(items: &mut Vec<T>, sel: &[String]) {
            for i in (0..items.len().saturating_sub(1)).rev() {
                if sel.contains(items[i].id()) && !sel.contains(items[i + 1].id()) {
                    items.swap(i, i + 1);
                    break;
                }
            }
        }
        match self.persona {
            Persona::Vector => fwd(&mut self.vector_shapes, &self.selected_shape_ids),
            Persona::Publish => fwd(&mut self.publish_frames, &self.selected_frame_ids),
            Persona::Photo => {
                if let Some(idx) = self
                    .project
                    .layers
                    .iter()
                    .position(|l| l.id == self.project.selected_layer_id)
                {
                    if idx + 1 < self.project.layers.len() {
                        self.move_layer(idx, idx + 1);
                    }
                }
            }
        }
        self.hint("Bring Forward");
    }

    pub fn send_backward(&mut self) {
        fn bwd<T: HasId>(items: &mut Vec<T>, sel: &[String]) {
            for i in 1..items.len() {
                if sel.contains(items[i].id()) && !sel.contains(items[i - 1].id()) {
                    items.swap(i, i - 1);
                    break;
                }
            }
        }
        match self.persona {
            Persona::Vector => bwd(&mut self.vector_shapes, &self.selected_shape_ids),
            Persona::Publish => bwd(&mut self.publish_frames, &self.selected_frame_ids),
            Persona::Photo => {
                if let Some(idx) = self
                    .project
                    .layers
                    .iter()
                    .position(|l| l.id == self.project.selected_layer_id)
                {
                    if idx > 0 {
                        self.move_layer(idx, idx - 1);
                    }
                }
            }
        }
        self.hint("Send Backward");
    }

    pub fn bring_to_front(&mut self) {
        fn front<T: HasId + Clone>(items: &mut Vec<T>, sel: &[String]) {
            let (a, b): (Vec<T>, Vec<T>) =
                items.iter().cloned().partition(|i| sel.contains(i.id()));
            *items = b.into_iter().chain(a).collect();
        }
        match self.persona {
            Persona::Vector => front(&mut self.vector_shapes, &self.selected_shape_ids),
            Persona::Publish => front(&mut self.publish_frames, &self.selected_frame_ids),
            Persona::Photo => {}
        }
        self.hint("Bring to Front");
    }

    pub fn send_to_back(&mut self) {
        fn back<T: HasId + Clone>(items: &mut Vec<T>, sel: &[String]) {
            let (a, b): (Vec<T>, Vec<T>) =
                items.iter().cloned().partition(|i| sel.contains(i.id()));
            *items = a.into_iter().chain(b).collect();
        }
        match self.persona {
            Persona::Vector => back(&mut self.vector_shapes, &self.selected_shape_ids),
            Persona::Publish => back(&mut self.publish_frames, &self.selected_frame_ids),
            Persona::Photo => {}
        }
        self.hint("Send to Back");
    }

    pub fn align_selected(&mut self, align: AlignOp) {
        fn align_items<T: Alignable>(items: &mut Vec<T>, sel: &[String], align: AlignOp) -> bool {
            let selected: Vec<&T> = items.iter().filter(|i| sel.contains(i.id())).collect();
            if selected.len() < 2 {
                return false;
            }
            let min_x = selected.iter().map(|i| i.x()).fold(f32::INFINITY, f32::min);
            let max_x = selected
                .iter()
                .map(|i| i.x() + i.w())
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = selected.iter().map(|i| i.y()).fold(f32::INFINITY, f32::min);
            let max_y = selected
                .iter()
                .map(|i| i.y() + i.h())
                .fold(f32::NEG_INFINITY, f32::max);
            let mid_x = (min_x + max_x) / 2.0;
            let mid_y = (min_y + max_y) / 2.0;
            for i in items.iter_mut() {
                if !sel.contains(i.id()) {
                    continue;
                }
                match align {
                    AlignOp::Left => i.set_x(min_x),
                    AlignOp::Center => i.set_x(mid_x - i.w() / 2.0),
                    AlignOp::Right => i.set_x(max_x - i.w()),
                    AlignOp::Top => i.set_y(min_y),
                    AlignOp::Middle => i.set_y(mid_y - i.h() / 2.0),
                    AlignOp::Bottom => i.set_y(max_y - i.h()),
                }
            }
            true
        }
        let done = match self.persona {
            Persona::Vector => {
                align_items(&mut self.vector_shapes, &self.selected_shape_ids, align)
            }
            Persona::Publish => {
                align_items(&mut self.publish_frames, &self.selected_frame_ids, align)
            }
            Persona::Photo => false,
        };
        if done {
            self.push_history(format!("Align Objects ({align:?})"));
            self.hint(format!("Aligned {align:?}").to_uppercase());
        }
    }

    // ---- Swatches ----------------------------------------------------------------
    pub fn apply_swatch(&mut self, color: &str, target: &str) {
        match self.persona {
            Persona::Vector => {
                match target {
                    "fill" => {
                        self.vector_fill = color.into();
                        let c = color.to_string();
                        self.update_selected_vector_shapes(|s| s.fill = c.clone());
                    }
                    "stroke" => {
                        self.vector_stroke = color.into();
                        let c = color.to_string();
                        self.update_selected_vector_shapes(|s| s.stroke = c.clone());
                    }
                    _ => {
                        let c = color.to_string();
                        self.update_selected_vector_shapes(|s| s.fill = c.clone());
                    }
                }
                self.push_history(format!("Apply {target} Swatch: {color}"));
            }
            Persona::Publish => {
                let c = color.to_string();
                for f in &mut self.publish_frames {
                    if self.selected_frame_ids.contains(&f.id) {
                        f.fill = Some(c.clone());
                    }
                }
                self.push_history(format!("Apply {target} Swatch: {color}"));
            }
            Persona::Photo => {
                self.foreground_color = color.into();
                let is_text = self
                    .selected_layer()
                    .map(|l| l.layer_type == LayerType::Text)
                    .unwrap_or(false);
                if is_text {
                    if let Some(l) = self.selected_layer_mut() {
                        l.text_color = Some(color.into());
                    }
                    self.project.updated_at = now_millis();
                }
                self.push_history(format!("Apply Color Swatch: {color}"));
            }
        }
        self.hint(format!("Applied {target}: {color}"));
    }

    // ---- Plugins / sync -----------------------------------------------------------
    pub fn toggle_plugin(&mut self, id: &str) {
        if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
            p.enabled = !p.enabled;
        }
    }

    pub fn mark_saved(&mut self) {
        self.sync_status = SyncStatus::Synced;
        self.last_saved_at = now_millis();
    }

    // ---- Clipboard ---------------------------------------------------------------

    pub fn copy_selection(&mut self, cut: bool) {
        match self.persona {
            Persona::Vector => {
                let items: Vec<VectorShape> = self
                    .vector_shapes
                    .iter()
                    .filter(|s| self.selected_shape_ids.contains(&s.id))
                    .cloned()
                    .collect();
                if items.is_empty() {
                    self.hint("Nothing selected to copy");
                    return;
                }
                if cut {
                    self.vector_shapes
                        .retain(|s| !self.selected_shape_ids.contains(&s.id));
                    self.selected_shape_ids.clear();
                }
                self.clipboard = Clipboard::Shapes(items);
            }
            Persona::Publish => {
                let items: Vec<PublishFrame> = self
                    .publish_frames
                    .iter()
                    .filter(|f| self.selected_frame_ids.contains(&f.id))
                    .cloned()
                    .collect();
                if items.is_empty() {
                    self.hint("Nothing selected to copy");
                    return;
                }
                if cut {
                    self.publish_frames
                        .retain(|f| !self.selected_frame_ids.contains(&f.id));
                    self.selected_frame_ids.clear();
                }
                self.clipboard = Clipboard::Frames(items);
            }
            Persona::Photo => {
                let layer = match self.selected_layer() {
                    Some(l) => l.clone(),
                    None => {
                        self.hint("Nothing selected to copy");
                        return;
                    }
                };
                if cut {
                    if self.project.layers.len() <= 1 {
                        self.hint("Cannot cut the last layer");
                        return;
                    }
                    self.project.layers.retain(|l| l.id != layer.id);
                    if let Some(last) = self.project.layers.last() {
                        self.project.selected_layer_id = last.id.clone();
                    }
                }
                self.clipboard = Clipboard::Layers(vec![layer]);
            }
        }
        self.hint(if cut {
            "Cut to clipboard"
        } else {
            "Copied to clipboard"
        });
        if cut {
            self.push_history("Cut Selection");
        }
    }

    pub fn paste_clipboard(&mut self) {
        match self.clipboard.clone() {
            Clipboard::Layers(layers) => {
                for mut l in layers {
                    l.id = new_id("layer");
                    l.name = format!("{} Paste", l.name);
                    self.project.selected_layer_id = l.id.clone();
                    self.project.layers.push(l);
                }
                self.project.updated_at = now_millis();
                self.push_history("Paste Layer");
            }
            Clipboard::Shapes(shapes) => {
                let mut ids = Vec::new();
                for mut s in shapes {
                    s.id = new_id("shape");
                    s.x += 16.0;
                    s.y += 16.0;
                    ids.push(s.id.clone());
                    self.vector_shapes.push(s);
                }
                self.selected_shape_ids = ids;
                self.push_history("Paste Shapes");
            }
            Clipboard::Frames(frames) => {
                let mut ids = Vec::new();
                for mut f in frames {
                    f.id = new_id("frame");
                    f.x += 16.0;
                    f.y += 16.0;
                    ids.push(f.id.clone());
                    self.publish_frames.push(f);
                }
                self.selected_frame_ids = ids;
                self.push_history("Paste Frames");
            }
            Clipboard::Empty => self.hint("Clipboard is empty"),
        }
        if !matches!(self.clipboard, Clipboard::Empty) {
            self.hint("Pasted");
        }
    }

    // ---- Pen tool (vector) ----------------------------------------------------------

    /// Commit the in-progress pen path as a Path shape.
    pub fn commit_pen_path(&mut self, close: bool) {
        if self.pen_path.len() < 2 {
            self.pen_path.clear();
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        );
        for p in &self.pen_path {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        let mut sh = VectorShape::new(ShapeKind::Path, x0, y0, x1 - x0, y1 - y0);
        sh.points = Some(self.pen_path.clone());
        sh.fill = "none".into();
        sh.stroke = self.vector_stroke.clone();
        sh.stroke_width = self.vector_stroke_width.max(1.0);
        if !close {
            sh.name = Some("Open Path".into());
        }
        let id = sh.id.clone();
        self.vector_shapes.push(sh);
        self.selected_shape_ids = vec![id];
        self.pen_path.clear();
        self.push_history(if close {
            "Pen: Close Path"
        } else {
            "Pen: Draw Path"
        });
        self.hint("Path committed — Enter closes, Esc cancels");
    }

    // ---- Crop ------------------------------------------------------------------------

    /// Crop the document to a rect (doc coords). Resizes the canvas and
    /// re-anchors all object coordinates.
    pub fn apply_crop(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if w < 8.0 || h < 8.0 {
            self.hint("Crop region too small");
            return;
        }
        self.project.width = w;
        self.project.height = h;
        for l in &mut self.project.layers {
            if let Some(lx) = l.x.as_mut() {
                *lx -= x;
            }
            if let Some(ly) = l.y.as_mut() {
                *ly -= y;
            }
            if l.layer_type == LayerType::Raster || l.layer_type == LayerType::Vector {
                l.width = Some(w);
                l.height = Some(h);
            }
        }
        for s in &mut self.vector_shapes {
            s.x -= x;
            s.y -= y;
            if let Some(pts) = &mut s.points {
                for p in pts.iter_mut() {
                    p.x -= x;
                    p.y -= y;
                }
            }
        }
        for f in &mut self.publish_frames {
            f.x -= x;
            f.y -= y;
        }
        for g in &mut self.guides {
            g.position = match g.kind {
                GuideKind::Horizontal => (g.position - y).max(0.0),
                GuideKind::Vertical => (g.position - x).max(0.0),
            };
        }
        self.selection = None;
        self.project.updated_at = now_millis();
        self.raster_version += 1;
        self.push_history("Crop Canvas");
        self.hint(format!("Cropped to {}×{}", w as i32, h as i32));
    }

    /// The layer painting operates on: selected raster layer, else the topmost
    /// raster layer, else None (caller auto-creates one).
    pub fn paint_target_layer(&self) -> Option<String> {
        if let Some(l) = self.selected_layer() {
            if l.layer_type == LayerType::Raster && !l.locked {
                return Some(l.id.clone());
            }
        }
        self.project
            .layers
            .iter()
            .rev()
            .find(|l| l.layer_type == LayerType::Raster && !l.locked)
            .map(|l| l.id.clone())
    }
}

// Helpers for generic z-order / align ops -------------------------------------

pub trait HasId {
    fn id(&self) -> &String;
}

pub trait Alignable: HasId {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
    fn w(&self) -> f32;
    fn h(&self) -> f32;
    fn set_x(&mut self, v: f32);
    fn set_y(&mut self, v: f32);
}

impl HasId for VectorShape {
    fn id(&self) -> &String {
        &self.id
    }
}
impl Alignable for VectorShape {
    fn x(&self) -> f32 {
        self.x
    }
    fn y(&self) -> f32 {
        self.y
    }
    fn w(&self) -> f32 {
        self.width
    }
    fn h(&self) -> f32 {
        self.height
    }
    fn set_x(&mut self, v: f32) {
        self.x = v;
    }
    fn set_y(&mut self, v: f32) {
        self.y = v;
    }
}

impl HasId for PublishFrame {
    fn id(&self) -> &String {
        &self.id
    }
}
impl Alignable for PublishFrame {
    fn x(&self) -> f32 {
        self.x
    }
    fn y(&self) -> f32 {
        self.y
    }
    fn w(&self) -> f32 {
        self.width
    }
    fn h(&self) -> f32 {
        self.height
    }
    fn set_x(&mut self, v: f32) {
        self.x = v;
    }
    fn set_y(&mut self, v: f32) {
        self.y = v;
    }
}

// Initial document/sample data --------------------------------------------------

pub fn create_initial_project(persona: Persona) -> DocumentProject {
    let is_publish = persona == Persona::Publish;
    let is_vector = persona == Persona::Vector;
    let width = if is_publish {
        2480.0
    } else if is_vector {
        1920.0
    } else {
        2560.0
    };
    let height = if is_publish {
        1754.0
    } else if is_vector {
        1080.0
    } else {
        1440.0
    };

    let mut base = Layer::new(
        "layer-base",
        if is_publish {
            "Master Spread Background"
        } else if is_vector {
            "Vector Composition"
        } else {
            "Background Layer"
        },
        if is_vector {
            LayerType::Vector
        } else {
            LayerType::Raster
        },
    );
    base.width = Some(width);
    base.height = Some(height);

    let mut layers = vec![base];
    if persona == Persona::Photo {
        let mut t = Layer::new("layer-photo-title", "Typography Title", LayerType::Text);
        t.text_content = Some("Suiza Studio".into());
        t.text_font_size = Some(72.0);
        t.text_font_family = Some("Plus Jakarta Sans".into());
        t.text_font_weight = Some("700".into());
        t.text_color = Some("#0f172a".into());
        t.x = Some(140.0);
        t.y = Some(160.0);
        layers.push(t);
    } else if is_publish {
        let mut h = Layer::new(
            "layer-pub-header",
            "Editorial Headline Frame",
            LayerType::Text,
        );
        h.text_content = Some("ARCHITECTURAL HORIZONS: ISSUE 42".into());
        h.text_font_size = Some(64.0);
        h.text_font_family = Some("Plus Jakarta Sans".into());
        h.text_color = Some("#0f172a".into());
        h.x = Some(180.0);
        h.y = Some(220.0);
        layers.push(h);

        let mut b = Layer::new("layer-pub-body", "3-Column Body Text Grid", LayerType::Text);
        b.text_content = Some("Exploring the nexus of memory safety and computational design. In the modern publishing landscape, layout rigor meets real-time vector compositing.".into());
        b.text_font_size = Some(24.0);
        b.text_font_family = Some("Plus Jakarta Sans".into());
        b.text_color = Some("#334155".into());
        b.x = Some(180.0);
        b.y = Some(340.0);
        layers.push(b);
    }

    DocumentProject {
        id: new_id("project"),
        name: match persona {
            Persona::Photo => "Untitled-Photo-1.szt",
            Persona::Vector => "Untitled-Vector-1.szt",
            Persona::Publish => "Untitled-Publication-1.szt",
        }
        .into(),
        width,
        height,
        dpi: 300,
        color_mode: if is_publish {
            ColorMode::Cmyk
        } else {
            ColorMode::Rgb
        },
        cmyk_profile: "FOGRA39 (ISO 12647-2)".into(),
        persona,
        selected_layer_id: layers.last().map(|l| l.id.clone()).unwrap_or_default(),
        layers,
        page_count: if is_publish { 4 } else { 1 },
        active_page_index: 0,
        facing_pages: is_publish,
        bleed: 3.0,
        margins: Margins {
            top: 36.0,
            bottom: 36.0,
            inside: 48.0,
            outside: 36.0,
        },
        guides: Vec::new(),
        created_at: now_millis(),
        updated_at: now_millis(),
    }
}

fn initial_vector_shapes() -> Vec<VectorShape> {
    let mut rect = VectorShape::new(ShapeKind::Rect, 100.0, 120.0, 220.0, 160.0);
    rect.id = "v-shape-rect-1".into();
    rect.fill = "#ef4444".into();
    rect.stroke = "#991b1b".into();
    rect.stroke_width = 3.0;
    rect.corner_radius = Some(8.0);

    let mut ellipse = VectorShape::new(ShapeKind::Ellipse, 240.0, 180.0, 200.0, 200.0);
    ellipse.id = "v-shape-ellipse-2".into();
    ellipse.fill = "#0d99ff".into();
    ellipse.stroke = "#0369a1".into();
    ellipse.stroke_width = 3.0;

    let mut poly = VectorShape::new(ShapeKind::Polygon, 480.0, 140.0, 180.0, 180.0);
    poly.id = "v-shape-poly-3".into();
    poly.fill = "#10b981".into();
    poly.stroke = "#065f46".into();
    poly.stroke_width = 2.0;

    let mut text = VectorShape::new(ShapeKind::Text, 100.0, 60.0, 420.0, 50.0);
    text.id = "v-shape-text-4".into();
    text.name = Some("Suiza Vector Typo".into());
    text.fill = "#0f172a".into();
    text.stroke = "none".into();
    text.stroke_width = 0.0;
    text.text = Some("Suiza Vector Artboard".into());
    text.font_size = Some(32.0);
    text.font_family = Some("Plus Jakarta Sans".into());

    vec![rect, ellipse, poly, text]
}

fn initial_publish_frames() -> Vec<PublishFrame> {
    let mut headline = PublishFrame::new(FrameKind::Text, 70.0, 110.0, 580.0, 85.0);
    headline.id = "pub-headline".into();
    headline.title = Some("SWISS TYPOGRAPHY & ALPINE FORM".into());
    headline.content = Some("A modernist review of modular architecture, raw granite construction, and grid discipline.".into());

    let mut col1 = PublishFrame::new(FrameKind::Text, 70.0, 220.0, 275.0, 420.0);
    col1.id = "pub-col-1".into();
    col1.content = Some("When designing within pure spatial limits, negative space dictates rhythm as decisively as ink. The monolithic granite structures of the Swiss Alps demonstrate that structural integrity arises not from ornamentation, but from the uncompromising precision of joints and tolerances.\n\nIn typography as in stonework, adherence to the baseline grid gives weight to ideas and clarity to communication.".into());

    let mut col2 = PublishFrame::new(FrameKind::Text, 375.0, 220.0, 275.0, 420.0);
    col2.id = "pub-col-2".into();
    col2.content = Some("Early modernist publications understood that legibility is a moral imperative. By aligning column gutters to golden ratios, the eye glides across prose with effortless velocity.\n\nThis editorial layout demonstrates publication-grade multi-column text flow with automated optical kerning, proportional leading, and non-destructive bleed geometry.".into());

    let mut pic = PublishFrame::new(FrameKind::Picture, 740.0, 110.0, 590.0, 430.0);
    pic.id = "pub-pic-1".into();
    pic.title = Some("EDITORIAL GRAPHIC BOX · 300 DPI CMYK ASSET".into());

    let mut caption = PublishFrame::new(FrameKind::Text, 740.0, 560.0, 590.0, 50.0);
    caption.id = "pub-caption".into();
    caption.content = Some("Fig 1.1 — Monolithic Cantilever Pavilion overlooking the Rhine Gorge. Photographed on 4×5 Large Format Technical Camera.".into());

    vec![headline, col1, col2, pic, caption]
}

// Color math --------------------------------------------------------------------

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        (
            u8::from_str_radix(&s[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&s[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&s[4..6], 16).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    }
}

/// Naive RGB→CMYK conversion matching `rgbToCmyk` in canvasRenderer.ts.
pub fn rgb_to_cmyk(hex: &str) -> CmykColor {
    let (r, g, b) = hex_to_rgb(hex);
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return CmykColor {
            c: 0,
            m: 0,
            y: 0,
            k: 100,
        };
    }
    let c = (1.0 - r - k) / (1.0 - k);
    let m = (1.0 - g - k) / (1.0 - k);
    let y = (1.0 - b - k) / (1.0 - k);
    CmykColor {
        c: (c * 100.0).round() as u8,
        m: (m * 100.0).round() as u8,
        y: (y * 100.0).round() as u8,
        k: (k * 100.0).round() as u8,
    }
}
