//! Central canvas viewport — port of `CanvasEngine.tsx`.
//! Handles the doc→screen transform, per-persona compositing, pan/zoom,
//! shape creation and selection.

use crate::model::*;
use crate::state::AtelierState;
use crate::theme::{hex32, theme_by_id};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

/// Cache of decoded raster layer textures, keyed by layer id.
#[derive(Default)]
pub struct RasterCache {
    textures: HashMap<String, (usize, egui::TextureHandle)>,
}

impl RasterCache {
    fn get(
        &mut self,
        ctx: &egui::Context,
        layer: &Layer,
    ) -> Option<&egui::TextureHandle> {
        let png = layer.raster_png.as_ref()?;
        let needs = match self.textures.get(&layer.id) {
            Some((len, _)) => *len != png.len(),
            None => true,
        };
        if needs {
            let img = image::load_from_memory(png).ok()?.to_rgba8();
            let (w, h) = (img.width() as usize, img.height() as usize);
            let color = egui::ColorImage::from_rgba_unmultiplied([w, h], &img);
            let tex = ctx.load_texture(&layer.id, color, egui::TextureOptions::LINEAR);
            self.textures.insert(layer.id.clone(), (png.len(), tex));
        }
        self.textures.get(&layer.id).map(|(_, t)| t)
    }
}

/// Map document coordinates to screen coordinates.
struct View {
    origin: Pos2, // screen position of doc (0,0)
    zoom: f32,
}

impl View {
    fn doc_to_screen(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x * self.zoom, y * self.zoom)
    }
    fn screen_to_doc(&self, p: Pos2) -> Vec2 {
        (p - self.origin) / self.zoom
    }
    fn doc_rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.doc_to_screen(x, y), vec2(w * self.zoom, h * self.zoom))
    }
}

pub fn canvas(ui: &mut egui::Ui, st: &mut AtelierState, cache: &mut RasterCache) {
    let theme = *theme_by_id(st.theme_id);
    let accent = hex32(theme.accent);

    let available = ui.available_rect_before_wrap();
    ui.painter().rect_filled(available, 0.0, hex32(theme.bg_canvas));

    // Rulers reserve a strip when enabled.
    let ruler = if st.show_rulers { 20.0 } else { 0.0 };
    let canvas_rect = Rect::from_min_max(
        available.min + vec2(ruler, ruler),
        available.max,
    );

    let doc_w = st.project.width * st.zoom;
    let doc_h = st.project.height * st.zoom;
    let origin = canvas_rect.center() - vec2(doc_w / 2.0, doc_h / 2.0) + vec2(st.pan_x, st.pan_y);
    let view = View { origin, zoom: st.zoom };
    let doc_rect = view.doc_rect(0.0, 0.0, st.project.width, st.project.height);

    // Interaction region (everything below the rulers).
    let response = ui.interact(canvas_rect, ui.id().with("canvas"), Sense::click_and_drag());

    // ---- Zoom: scroll wheel anchored at pointer ------------------------------
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let old_zoom = st.zoom;
            let new_zoom = (old_zoom * (scroll * 0.002).exp()).clamp(0.02, 32.0);
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                // Keep the doc point under the cursor stationary:
                //   origin = canvas_center - doc_size*zoom/2 + pan
                //   doc_pt = (pointer - origin) / zoom  (invariant across zoom)
                let doc_pt = (pointer - view.origin) / old_zoom;
                let new_origin = pointer - doc_pt * new_zoom;
                st.pan_x = new_origin.x - canvas_rect.center().x + st.project.width * new_zoom / 2.0;
                st.pan_y = new_origin.y - canvas_rect.center().y + st.project.height * new_zoom / 2.0;
            }
            st.zoom = new_zoom;
        }
    }

    // ---- Pan: hand tool, middle mouse, or space-drag --------------------------
    let panning = st.active_tool == ToolId::Hand
        || ui.input(|i| i.pointer.middle_down())
        || ui.input(|i| i.key_down(egui::Key::Space));
    if panning && response.dragged() {
        let d = response.drag_delta();
        st.pan_x += d.x;
        st.pan_y += d.y;
    }

    // ---- Document shadow + paper ---------------------------------------------
    let shadow = doc_rect.translate(vec2(6.0, 8.0));
    ui.painter()
        .rect_filled(shadow, 2.0, Color32::from_black_alpha(90));
    ui.painter().rect_filled(doc_rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        doc_rect,
        0.0,
        Stroke::new(1.0_f32, hex32(theme.border_strong)),
        egui::StrokeKind::Inside,
    );

    // ---- Persona content --------------------------------------------------------
    ui.painter().with_clip_rect(doc_rect).rect_filled(
        doc_rect,
        0.0,
        Color32::WHITE,
    );
    match st.persona {
        Persona::Photo | Persona::Publish => draw_layers(ui, st, cache, &view, doc_rect),
        Persona::Vector => draw_vector(ui, st, &view),
    }
    if st.persona == Persona::Publish {
        draw_publish(ui, st, &view, doc_rect);
    }

    // ---- Grid + guides ------------------------------------------------------------
    if st.show_grid {
        draw_grid(ui, st, &view, doc_rect);
    }
    if st.show_guides {
        let gc = hex32("#0d99ff").gamma_multiply(0.7);
        for g in &st.guides {
            match g.kind {
                GuideKind::Horizontal => {
                    let y = view.doc_to_screen(0.0, g.position).y;
                    ui.painter().line_segment(
                        [pos2(doc_rect.left(), y), pos2(doc_rect.right(), y)],
                        Stroke::new(1.0_f32, gc),
                    );
                }
                GuideKind::Vertical => {
                    let x = view.doc_to_screen(g.position, 0.0).x;
                    ui.painter().line_segment(
                        [pos2(x, doc_rect.top()), pos2(x, doc_rect.bottom())],
                        Stroke::new(1.0_f32, gc),
                    );
                }
            }
        }
    }

    // ---- Rulers ---------------------------------------------------------------------
    if st.show_rulers {
        draw_rulers(ui, st, &view, available, ruler, theme.border, theme.text_muted);
    }

    // ---- Tool interactions ------------------------------------------------------------
    handle_canvas_input(ui, st, &response, &view, accent);

    // ---- Status strip -------------------------------------------------------------------
    let status = format!(
        "{}  ·  {:.0}%  ·  {:.0}×{:.0}px",
        st.project.name,
        st.zoom * 100.0,
        st.project.width,
        st.project.height
    );
    ui.painter().text(
        canvas_rect.left_bottom() + vec2(8.0, -8.0),
        Align2::LEFT_BOTTOM,
        status,
        FontId::monospace(10.0),
        hex32(theme.text_muted),
    );
}

// ---- Raster/text layer compositing --------------------------------------------------

fn draw_layers(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    cache: &mut RasterCache,
    view: &View,
    doc_rect: Rect,
) {
    let ctx = ui.ctx().clone();
    for layer in &st.project.layers {
        if !layer.visible {
            continue;
        }
        let alpha = (layer.opacity.clamp(0.0, 100.0) / 100.0 * 255.0) as u8;
        match layer.layer_type {
            LayerType::Raster | LayerType::Frame | LayerType::Group => {
                if layer.raster_png.is_some() {
                    if let Some(tex) = cache.get(&ctx, layer) {
                        ui.painter().image(
                            tex.id(),
                            view.doc_rect(0.0, 0.0, st.project.width, st.project.height),
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE.gamma_multiply(alpha as f32 / 255.0),
                        );
                    }
                } else if layer.id == "layer-base" || layer.width.is_some() {
                    // Placeholder paper for empty raster layers: subtle checker.
                    draw_checker(ui, doc_rect, 24.0 * view.zoom);
                }
            }
            LayerType::Vector => {
                if let Some(paths) = &layer.vector_paths {
                    for p in paths {
                        draw_vector_path(ui, view, p, alpha);
                    }
                }
            }
            LayerType::Text => {
                let pos = view.doc_to_screen(layer.x.unwrap_or(0.0), layer.y.unwrap_or(0.0));
                let size = (layer.text_font_size.unwrap_or(32.0) * view.zoom).max(4.0);
                let color = hex32(layer.text_color.as_deref().unwrap_or("#ffffff"))
                    .gamma_multiply(alpha as f32 / 255.0);
                let text = layer.text_content.clone().unwrap_or_default();
                ui.painter().text(
                    pos,
                    Align2::LEFT_TOP,
                    text,
                    FontId::proportional(size),
                    color,
                );
            }
            LayerType::Adjustment => {}
        }
    }
}

fn draw_checker(ui: &mut egui::Ui, rect: Rect, cell: f32) {
    let cell = cell.clamp(4.0, 64.0);
    let a = Color32::from_gray(245);
    let b = Color32::from_gray(228);
    ui.painter().rect_filled(rect, 0.0, a);
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { cell } else { 0.0 };
        while x < rect.right() {
            let r = Rect::from_min_max(
                pos2(x, y),
                pos2((x + cell).min(rect.right()), (y + cell).min(rect.bottom())),
            );
            ui.painter().rect_filled(r, 0.0, b);
            x += cell * 2.0;
        }
        y += cell;
        row += 1;
    }
}

fn draw_vector_path(ui: &mut egui::Ui, view: &View, path: &VectorPath, alpha: u8) {
    if path.points.len() < 2 {
        return;
    }
    let pts: Vec<Pos2> = path.points.iter().map(|p| view.doc_to_screen(p.x, p.y)).collect();
    let stroke = Stroke::new(
        (path.stroke_width * view.zoom).max(0.5),
        hex32(&path.stroke_color).gamma_multiply(alpha as f32 / 255.0),
    );
    let fill = hex32(&path.fill_color).gamma_multiply(alpha as f32 / 255.0);
    if path.closed {
        ui.painter().add(egui::Shape::closed_line(pts, stroke));
        let _ = fill;
    } else {
        ui.painter().add(egui::Shape::line(pts, stroke));
    }
}

// ---- Vector persona -------------------------------------------------------------------

fn draw_vector(ui: &mut egui::Ui, st: &mut AtelierState, view: &View) {
    let accent = hex32(theme_by_id(st.theme_id).accent);
    let shapes = st.vector_shapes.clone();
    for shape in &shapes {
        if !shape.visible {
            continue;
        }
        draw_shape(ui, view, shape);
        if st.selected_shape_ids.contains(&shape.id) {
            let r = view.doc_rect(shape.x, shape.y, shape.width, shape.height);
            ui.painter().rect_stroke(
                r.expand(2.0),
                0.0,
                Stroke::new(1.5_f32, accent),
                egui::StrokeKind::Outside,
            );
        }
    }
}

fn draw_shape(ui: &mut egui::Ui, view: &View, shape: &VectorShape) {
    let fill = hex32(&shape.fill);
    let stroke_col = if shape.stroke == "none" { Color32::TRANSPARENT } else { hex32(&shape.stroke) };
    let sw = (shape.stroke_width * view.zoom).max(0.0);
    let stroke = Stroke::new(sw, stroke_col);
    let r = view.doc_rect(shape.x, shape.y, shape.width, shape.height);

    match shape.kind {
        ShapeKind::Rect | ShapeKind::Group => {
            let rad = shape.corner_radius.unwrap_or(0.0) * view.zoom;
            ui.painter().rect_filled(r, rad, fill);
            if sw > 0.0 {
                ui.painter().rect_stroke(r, rad, stroke, egui::StrokeKind::Inside);
            }
        }
        ShapeKind::Ellipse => {
            // Tessellate to avoid version-specific ellipse APIs.
            let center = r.center();
            let n = 48;
            let pts: Vec<Pos2> = (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                    center + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                })
                .collect();
            ui.painter().add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        ShapeKind::Polygon => {
            // Hexagon placeholder until star/polygon params land.
            let center = r.center();
            let n = 6;
            let pts: Vec<Pos2> = (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                    center + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                })
                .collect();
            ui.painter().add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        ShapeKind::Path => {
            if let Some(points) = &shape.points {
                let pts: Vec<Pos2> = points.iter().map(|p| view.doc_to_screen(p.x, p.y)).collect();
                if pts.len() >= 2 {
                    ui.painter().add(egui::Shape::closed_line(pts, stroke));
                }
            }
        }
        ShapeKind::Text => {
            let size = (shape.font_size.unwrap_or(32.0) * view.zoom).max(4.0);
            ui.painter().text(
                r.min,
                Align2::LEFT_TOP,
                shape.text.clone().unwrap_or_default(),
                FontId::proportional(size),
                fill,
            );
        }
    }
}

// ---- Publish persona ---------------------------------------------------------------------

fn draw_publish(ui: &mut egui::Ui, st: &AtelierState, view: &View, doc_rect: Rect) {
    // Bleed (red) + margins (blue) for the active page.
    let bleed = st.project.bleed;
    ui.painter().rect_stroke(
        doc_rect.expand(bleed * view.zoom),
        0.0,
        Stroke::new(1.0_f32, hex32("#ef4444")),
        egui::StrokeKind::Outside,
    );
    let m = &st.project.margins;
    let margin_rect = view.doc_rect(
        m.inside,
        m.top,
        st.project.width - m.inside - m.outside,
        st.project.height - m.top - m.bottom,
    );
    ui.painter().rect_stroke(
        margin_rect,
        0.0,
        Stroke::new(1.0_f32, hex32("#8b5cf6").gamma_multiply(0.8)),
        egui::StrokeKind::Inside,
    );

    // Column guides.
    let cols = st.publish_column_count.max(1);
    let gut = st.publish_column_gutter;
    for c in 1..cols {
        let col_w = (margin_rect.width() - gut * view.zoom * (cols - 1) as f32) / cols as f32;
        let x = margin_rect.left() + (col_w + gut * view.zoom) * c as f32 - gut * view.zoom / 2.0;
        ui.painter().line_segment(
            [pos2(x, margin_rect.top()), pos2(x, margin_rect.bottom())],
            Stroke::new(0.5_f32, hex32("#8b5cf6").gamma_multiply(0.4)),
        );
    }

    for frame in &st.publish_frames {
        if !frame.visible {
            continue;
        }
        let r = view.doc_rect(frame.x, frame.y, frame.width, frame.height);
        let (edge, fill) = match frame.kind {
            FrameKind::Text => (hex32("#ec4899"), Color32::TRANSPARENT),
            FrameKind::Picture => (hex32("#0ea5e9"), hex32("#0ea5e9").gamma_multiply(0.08)),
            FrameKind::Rule => (hex32("#64748b"), Color32::TRANSPARENT),
            FrameKind::Shape | FrameKind::Group => (hex32("#8b5cf6"), Color32::TRANSPARENT),
        };
        ui.painter().rect_filled(r, 0.0, fill);
        ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0_f32, edge), egui::StrokeKind::Inside);

        if let Some(title) = &frame.title {
            ui.painter().with_clip_rect(r).text(
                r.min + vec2(4.0, 4.0),
                Align2::LEFT_TOP,
                title,
                FontId::proportional((13.0 * view.zoom).max(8.0)),
                edge,
            );
        }
        if let Some(content) = &frame.content {
            ui.painter().with_clip_rect(r.shrink(4.0)).text(
                r.min + vec2(4.0, 16.0 * view.zoom),
                Align2::LEFT_TOP,
                content,
                FontId::proportional((11.0 * view.zoom).max(6.0)),
                hex32("#334155"),
            );
        }

        if st.selected_frame_ids.contains(&frame.id) {
            ui.painter().rect_stroke(
                r.expand(2.0),
                0.0,
                Stroke::new(1.5_f32, hex32("#0d99ff")),
                egui::StrokeKind::Outside,
            );
        }
    }
}

// ---- Grid + rulers ---------------------------------------------------------------------------

fn draw_grid(ui: &mut egui::Ui, st: &AtelierState, view: &View, doc_rect: Rect) {
    let step = st.snapping.grid_size.max(4.0) * view.zoom;
    if step < 4.0 {
        return;
    }
    let c = hex32("#888888").gamma_multiply(0.15);
    let mut x = doc_rect.left();
    while x <= doc_rect.right() {
        ui.painter().line_segment([pos2(x, doc_rect.top()), pos2(x, doc_rect.bottom())], Stroke::new(0.5_f32, c));
        x += step;
    }
    let mut y = doc_rect.top();
    while y <= doc_rect.bottom() {
        ui.painter().line_segment([pos2(doc_rect.left(), y), pos2(doc_rect.right(), y)], Stroke::new(0.5_f32, c));
        y += step;
    }
}

fn draw_rulers(
    ui: &mut egui::Ui,
    st: &AtelierState,
    view: &View,
    available: Rect,
    ruler: f32,
    border: &str,
    text_muted: &str,
) {
    let bg = hex32(theme_by_id(st.theme_id).bg_panel);
    let tick = hex32(text_muted);
    // Horizontal ruler
    let hr = Rect::from_min_max(available.min + vec2(ruler, 0.0), pos2(available.right(), available.top() + ruler));
    ui.painter().rect_filled(hr, 0.0, bg);
    ui.painter().line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(1.0_f32, hex32(border)));
    let step_px = 50.0 * view.zoom;
    if step_px > 24.0 {
        let start_doc = ((hr.left() - view.origin.x) / view.zoom / 50.0).floor() * 50.0;
        let mut doc_x = start_doc;
        while view.doc_to_screen(doc_x, 0.0).x < hr.right() {
            let sx = view.doc_to_screen(doc_x, 0.0).x;
            if sx >= hr.left() {
                ui.painter().line_segment([pos2(sx, hr.bottom() - 5.0), pos2(sx, hr.bottom())], Stroke::new(1.0_f32, tick));
                ui.painter().text(pos2(sx + 2.0, hr.top() + 2.0), Align2::LEFT_TOP, format!("{}", doc_x as i32), FontId::monospace(8.0), tick);
            }
            doc_x += 50.0;
        }
    }
    // Vertical ruler
    let vr = Rect::from_min_max(available.min + vec2(0.0, ruler), pos2(available.left() + ruler, available.bottom()));
    ui.painter().rect_filled(vr, 0.0, bg);
    ui.painter().line_segment([vr.right_top(), vr.right_bottom()], Stroke::new(1.0_f32, hex32(border)));
    if step_px > 24.0 {
        let start_doc = ((vr.top() - view.origin.y) / view.zoom / 50.0).floor() * 50.0;
        let mut doc_y = start_doc;
        while view.doc_to_screen(0.0, doc_y).y < vr.bottom() {
            let sy = view.doc_to_screen(0.0, doc_y).y;
            if sy >= vr.top() {
                ui.painter().line_segment([pos2(vr.right() - 5.0, sy), pos2(vr.right(), sy)], Stroke::new(1.0_f32, tick));
                ui.painter().text(pos2(vr.left() + 2.0, sy + 2.0), Align2::LEFT_TOP, format!("{}", doc_y as i32), FontId::monospace(8.0), tick);
            }
            doc_y += 50.0;
        }
    }
    // Corner box
    let corner = Rect::from_min_size(available.min, vec2(ruler, ruler));
    ui.painter().rect_filled(corner, 0.0, bg);
}

// ---- Canvas input -------------------------------------------------------------------------

fn handle_canvas_input(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    response: &egui::Response,
    view: &View,
    accent: Color32,
) {
    // Shape creation drag (vector + publish personas).
    let creation_tool = matches!(
        st.active_tool,
        ToolId::ShapeRect | ToolId::ShapeEllipse | ToolId::ShapePolygon
            | ToolId::FrameText | ToolId::FramePicture | ToolId::FrameShape | ToolId::LineTool
    );

    if creation_tool && response.drag_started() {
        ui.ctx().data_mut(|d| d.insert_temp(ui.id().with("drag_start"), response.interact_pointer_pos()));
    }
    if creation_tool && response.dragged() {
        if let (Some(start), Some(cur)) = (
            ui.ctx().data_mut(|d| d.get_temp::<Option<Pos2>>(ui.id().with("drag_start"))).flatten(),
            response.interact_pointer_pos(),
        ) {
            let r = Rect::from_two_pos(start, cur);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0_f32, accent), egui::StrokeKind::Inside);
        }
    }
    if creation_tool && response.drag_stopped() {
        if let (Some(start), Some(end)) = (
            ui.ctx().data_mut(|d| d.get_temp::<Option<Pos2>>(ui.id().with("drag_start"))).flatten(),
            response.interact_pointer_pos(),
        ) {
            let a = view.screen_to_doc(start);
            let b = view.screen_to_doc(end);
            let (x, y) = (a.x.min(b.x), a.y.min(b.y));
            let (w, h) = ((a.x - b.x).abs().max(4.0), (a.y - b.y).abs().max(4.0));

            match st.persona {
                Persona::Vector => {
                    let kind = match st.active_tool {
                        ToolId::ShapeEllipse => ShapeKind::Ellipse,
                        ToolId::ShapePolygon => ShapeKind::Polygon,
                        _ => ShapeKind::Rect,
                    };
                    let mut sh = VectorShape::new(kind, x, y, w, h);
                    sh.fill = st.vector_fill.clone();
                    sh.stroke = st.vector_stroke.clone();
                    sh.stroke_width = st.vector_stroke_width;
                    sh.corner_radius = Some(st.vector_corner_radius);
                    let id = sh.id.clone();
                    st.vector_shapes.push(sh);
                    st.selected_shape_ids = vec![id];
                    st.push_history("Draw Vector Shape");
                }
                Persona::Publish => {
                    let kind = match st.active_tool {
                        ToolId::FrameText => FrameKind::Text,
                        ToolId::FramePicture => FrameKind::Picture,
                        ToolId::LineTool => FrameKind::Rule,
                        _ => FrameKind::Shape,
                    };
                    let mut f = PublishFrame::new(kind, x, y, w, h);
                    if kind == FrameKind::Text {
                        f.font_size = Some(st.tool_options.font_size);
                        f.wrap_mode = Some(st.publish_text_wrap_mode);
                    }
                    let id = f.id.clone();
                    st.publish_frames.push(f);
                    st.selected_frame_ids = vec![id];
                    st.push_history("Draw Publish Frame");
                }
                Persona::Photo => {}
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(ui.id().with("drag_start"), Option::<Pos2>::None));
    }

    // Click-select for move tool.
    if st.active_tool == ToolId::Move && response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let doc = view.screen_to_doc(pos);
            match st.persona {
                Persona::Vector => {
                    // Topmost hit wins.
                    let hit = st
                        .vector_shapes
                        .iter()
                        .rev()
                        .find(|s| {
                            doc.x >= s.x && doc.x <= s.x + s.width && doc.y >= s.y && doc.y <= s.y + s.height
                        })
                        .map(|s| s.id.clone());
                    match hit {
                        Some(id) => {
                            if ui.input(|i| i.modifiers.shift) {
                                if !st.selected_shape_ids.contains(&id) {
                                    st.selected_shape_ids.push(id);
                                }
                            } else {
                                st.selected_shape_ids = vec![id];
                            }
                        }
                        None => st.selected_shape_ids.clear(),
                    }
                }
                Persona::Publish => {
                    let hit = st
                        .publish_frames
                        .iter()
                        .rev()
                        .find(|f| {
                            doc.x >= f.x && doc.x <= f.x + f.width && doc.y >= f.y && doc.y <= f.y + f.height
                        })
                        .map(|f| f.id.clone());
                    match hit {
                        Some(id) => st.selected_frame_ids = vec![id],
                        None => st.selected_frame_ids.clear(),
                    }
                }
                Persona::Photo => {}
            }
        }
    }

    // Move-drag selected vector shapes / publish frames.
    if st.active_tool == ToolId::Move && response.dragged() {
        let d = response.drag_delta() / view.zoom;
        if d.length_sq() > 0.0 {
            st.nudge_selected(d.x, d.y);
        }
    }

    // Eyedropper samples the *stroke* color target for now (true pixel sampling
    // lands with the raster engine).
    if st.active_tool == ToolId::Eyedropper && response.clicked() {
        st.hint("Eyedropper: pixel sampling pending raster engine");
    }
}
