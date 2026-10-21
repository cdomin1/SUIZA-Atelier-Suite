//! Central canvas viewport — doc→screen transform, per-persona compositing,
//! pan/zoom, tool interactions (paint, marquee, lasso, crop, pen, nodes,
//! eyedropper, text, frames) and ruler-drag guides.

use crate::model::*;
use crate::raster::{self, RasterStore, Selection};
use crate::state::AtelierState;
use crate::theme::{hex32, theme_by_id};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::io::Cursor;
use std::time::Instant;

/// Canvas-side rendering caches: raster buffers + composite texture.
pub struct CanvasEngine {
    pub store: RasterStore,
    composite_tex: Option<egui::TextureHandle>,
    composite_buf: Option<image::RgbaImage>,
    composite_ver: u64,
    stroke_target: Option<String>,
    stroke_last: Option<(f32, f32)>,
    lasso_pts: Vec<Pos2>,
    ants_start: Instant,
}

impl CanvasEngine {
    pub fn new() -> Self {
        Self {
            store: RasterStore::new(),
            composite_tex: None,
            composite_buf: None,
            composite_ver: u64::MAX,
            stroke_target: None,
            stroke_last: None,
            lasso_pts: Vec::new(),
            ants_start: Instant::now(),
        }
    }

    /// Rebuild composite if raster contents changed.
    fn composite<'a>(
        &'a mut self,
        ctx: &egui::Context,
        st: &AtelierState,
    ) -> Option<(&'a egui::TextureHandle, &'a image::RgbaImage)> {
        if self.composite_ver != st.raster_version || self.composite_tex.is_none() {
            let (w, h) = (
                st.project.width.max(1.0) as u32,
                st.project.height.max(1.0) as u32,
            );
            let buf = raster::composite(&st.project.layers, &mut self.store, w, h);
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [buf.width() as usize, buf.height() as usize],
                &buf,
            );
            let tex = ctx.load_texture("composite", img, egui::TextureOptions::LINEAR);
            self.composite_tex = Some(tex);
            self.composite_buf = Some(buf);
            self.composite_ver = st.raster_version;
        }
        match (&self.composite_tex, &self.composite_buf) {
            (Some(t), Some(b)) => Some((t, b)),
            _ => None,
        }
    }
}

/// Map document coordinates to screen coordinates.
struct View {
    origin: Pos2,
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

fn temp_id(ui: &egui::Ui, key: &str) -> egui::Id {
    ui.id().with(("canvas-temp", key))
}

fn get_drag_start(ui: &egui::Ui) -> Option<Pos2> {
    ui.ctx()
        .data_mut(|d| d.get_temp::<Option<Pos2>>(temp_id(ui, "drag_start")))
        .flatten()
}

fn set_drag_start(ui: &egui::Ui, v: Option<Pos2>) {
    ui.ctx()
        .data_mut(|d| d.insert_temp(temp_id(ui, "drag_start"), v));
}

pub fn canvas(ui: &mut egui::Ui, st: &mut AtelierState, eng: &mut CanvasEngine) {
    let theme = *theme_by_id(st.theme_id);
    let accent = hex32(theme.accent);

    let available = ui.available_rect_before_wrap();
    ui.painter()
        .rect_filled(available, 0.0, hex32(theme.bg_canvas));
    st.viewport_size = [available.width(), available.height()];

    let ruler = if st.show_rulers { 20.0 } else { 0.0 };
    let canvas_rect = Rect::from_min_max(available.min + vec2(ruler, ruler), available.max);

    let doc_w = st.project.width * st.zoom;
    let doc_h = st.project.height * st.zoom;
    let origin = canvas_rect.center() - vec2(doc_w / 2.0, doc_h / 2.0) + vec2(st.pan_x, st.pan_y);
    let view = View {
        origin,
        zoom: st.zoom,
    };
    let doc_rect = view.doc_rect(0.0, 0.0, st.project.width, st.project.height);

    // Full-area region (incl. ruler strips) for ruler drags.
    let full_resp = ui.interact(available, ui.id().with("canvas-full"), Sense::drag());
    let response = ui.interact(canvas_rect, ui.id().with("canvas"), Sense::click_and_drag());

    // ---- Zoom: wheel anchored at pointer -------------------------------------
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let old_zoom = st.zoom;
            let new_zoom = (old_zoom * (scroll * 0.002).exp()).clamp(0.02, 32.0);
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let doc_pt = (pointer - view.origin) / old_zoom;
                let new_origin = pointer - doc_pt * new_zoom;
                st.pan_x =
                    new_origin.x - canvas_rect.center().x + st.project.width * new_zoom / 2.0;
                st.pan_y =
                    new_origin.y - canvas_rect.center().y + st.project.height * new_zoom / 2.0;
            }
            st.zoom = new_zoom;
        }
    }

    // ---- Ruler-drag guide creation ---------------------------------------------
    ruler_guides(ui, st, &full_resp, &view, available, ruler);

    // ---- Pan ---------------------------------------------------------------------
    let panning = st.active_tool == ToolId::Hand
        || ui.input(|i| i.pointer.middle_down())
        || ui.input(|i| i.key_down(egui::Key::Space));
    if panning && response.dragged() {
        let d = response.drag_delta();
        st.pan_x += d.x;
        st.pan_y += d.y;
    }

    // ---- Paper ---------------------------------------------------------------------
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

    // ---- Content ----------------------------------------------------------------------
    match st.persona {
        Persona::Photo => draw_photo(ui, st, eng, &view, doc_rect),
        Persona::Vector => draw_vector(ui, st, &view),
        Persona::Publish => {
            draw_photo(ui, st, eng, &view, doc_rect);
            draw_publish(ui, st, &view, doc_rect);
        }
    }

    if st.show_grid {
        draw_grid(ui, st, &view, doc_rect);
    }
    draw_guides(ui, st, &view, doc_rect);
    if st.show_rulers {
        draw_rulers(
            ui,
            st,
            &view,
            available,
            ruler,
            theme.border,
            theme.text_muted,
        );
    }

    // ---- Tool input -----------------------------------------------------------------------
    handle_tools(ui, st, eng, &response, &view, canvas_rect, accent);

    // Right-click context menu.
    response.context_menu(|ui| {
        if ui.button("Copy  Ctrl+C").clicked() {
            st.copy_selection(false);
            ui.close_menu();
        }
        if ui.button("Cut  Ctrl+X").clicked() {
            st.copy_selection(true);
            ui.close_menu();
        }
        if ui.button("Paste  Ctrl+V").clicked() {
            st.paste_clipboard();
            ui.close_menu();
        }
        ui.separator();
        if st.selection.is_some() && ui.button("Deselect").clicked() {
            st.selection = None;
            ui.close_menu();
        }
        match st.persona {
            Persona::Photo => {
                if ui.button("Select All  Ctrl+A").clicked() {
                    st.select_all();
                    ui.close_menu();
                }
                if ui.button("Delete Selected Layer").clicked() {
                    st.delete_selected();
                    eng.composite_ver = u64::MAX;
                    ui.close_menu();
                }
            }
            Persona::Vector => {
                if ui.button("Group  Ctrl+G").clicked() {
                    st.group_selected();
                    ui.close_menu();
                }
                if ui.button("Ungroup  Ctrl+Shift+G").clicked() {
                    st.ungroup_selected();
                    ui.close_menu();
                }
                if ui.button("Delete Shape(s)").clicked() {
                    st.delete_selected();
                    ui.close_menu();
                }
            }
            Persona::Publish => {
                if ui.button("Delete Frame(s)").clicked() {
                    st.delete_selected();
                    ui.close_menu();
                }
            }
        }
        ui.separator();
        if ui.button("Fit to Screen  Ctrl+0").clicked() {
            st.fit();
            ui.close_menu();
        }
        if ui.button("Actual Pixels  Ctrl+1").clicked() {
            st.reset_viewport();
            ui.close_menu();
        }
    });

    // ---- Overlays ------------------------------------------------------------------------------
    draw_selection(ui, st, &view, eng.ants_start);
    draw_pen_preview(ui, st, &view, accent);
    draw_node_handles(ui, st, &view, accent);

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

    // Marquee animation tick.
    if st.selection.is_some() || !st.pen_path.is_empty() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(66));
    }
}

// ---- Guides -------------------------------------------------------------------------

fn ruler_guides(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    full_resp: &egui::Response,
    view: &View,
    available: Rect,
    ruler: f32,
) {
    if ruler <= 0.0 || st.lock_guides {
        return;
    }
    let h_strip = Rect::from_min_max(
        available.min,
        pos2(available.right(), available.top() + ruler),
    );
    let v_strip = Rect::from_min_max(
        available.min,
        pos2(available.left() + ruler, available.bottom()),
    );

    if full_resp.drag_started() {
        if let Some(p) = full_resp.interact_pointer_pos() {
            if h_strip.contains(p) && !v_strip.contains(p) {
                st.pending_guide = Some(GuideKind::Vertical);
            } else if v_strip.contains(p) && !h_strip.contains(p) {
                st.pending_guide = Some(GuideKind::Horizontal);
            }
        }
    }
    if let Some(kind) = st.pending_guide {
        if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
            let d = view.screen_to_doc(p);
            let c = hex32("#0d99ff");
            match kind {
                GuideKind::Vertical => ui.painter().line_segment(
                    [pos2(p.x, available.top()), pos2(p.x, available.bottom())],
                    Stroke::new(1.0_f32, c),
                ),
                GuideKind::Horizontal => ui.painter().line_segment(
                    [pos2(available.left(), p.y), pos2(available.right(), p.y)],
                    Stroke::new(1.0_f32, c),
                ),
            };
            if full_resp.drag_stopped() {
                let doc_w = st.project.width;
                let doc_h = st.project.height;
                let in_doc = d.x >= 0.0 && d.x <= doc_w && d.y >= 0.0 && d.y <= doc_h;
                if in_doc {
                    let pos = if kind == GuideKind::Vertical {
                        d.x
                    } else {
                        d.y
                    };
                    st.add_guide(kind, pos);
                }
                st.pending_guide = None;
            }
        } else if full_resp.drag_stopped() {
            st.pending_guide = None;
        }
    }
}

fn draw_guides(ui: &mut egui::Ui, st: &AtelierState, view: &View, doc_rect: Rect) {
    if !st.show_guides {
        return;
    }
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

// ---- Photo/publish layer compositing ------------------------------------------------

fn draw_photo(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    eng: &mut CanvasEngine,
    view: &View,
    doc_rect: Rect,
) {
    let ctx = ui.ctx().clone();
    let (w, h) = (st.project.width, st.project.height);

    // Checker under transparency.
    let has_any_raster = st
        .project
        .layers
        .iter()
        .any(|l| l.visible && matches!(l.layer_type, LayerType::Raster | LayerType::Frame));
    if has_any_raster {
        draw_checker(ui, doc_rect, 24.0 * view.zoom);
    }

    if let Some((tex, _buf)) = eng.composite(&ctx, st) {
        ui.painter().image(
            tex.id(),
            view.doc_rect(0.0, 0.0, w, h),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    // Text + vector-path layers render over the composite in stack order.
    for layer in &st.project.layers {
        if !layer.visible {
            continue;
        }
        match layer.layer_type {
            LayerType::Text => {
                let pos = view.doc_to_screen(layer.x.unwrap_or(0.0), layer.y.unwrap_or(0.0));
                let size = (layer.text_font_size.unwrap_or(32.0) * view.zoom).max(4.0);
                let color = hex32(layer.text_color.as_deref().unwrap_or("#0f172a"));
                ui.painter().text(
                    pos,
                    Align2::LEFT_TOP,
                    layer.text_content.clone().unwrap_or_default(),
                    FontId::proportional(size),
                    color,
                );
            }
            LayerType::Vector => {
                if let Some(paths) = &layer.vector_paths {
                    for p in paths {
                        draw_vector_path(ui, view, p);
                    }
                }
            }
            _ => {}
        }
    }

    // Selected-layer outline hint.
    if let Some(l) = st.selected_layer() {
        if l.layer_type == LayerType::Text {
            if let (Some(x), Some(y)) = (l.x, l.y) {
                let size = l.text_font_size.unwrap_or(32.0);
                let approx_w = l.text_content.as_deref().unwrap_or("").len() as f32 * size * 0.55;
                let r = view.doc_rect(x, y, approx_w, size * 1.3);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0_f32, hex32(theme_by_id(st.theme_id).accent)),
                    egui::StrokeKind::Inside,
                );
            }
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

fn draw_vector_path(ui: &mut egui::Ui, view: &View, path: &VectorPath) {
    if path.points.len() < 2 {
        return;
    }
    let pts: Vec<Pos2> = path
        .points
        .iter()
        .map(|p| view.doc_to_screen(p.x, p.y))
        .collect();
    let stroke = Stroke::new(
        (path.stroke_width * view.zoom).max(0.5),
        hex32(&path.stroke_color),
    );
    if path.closed {
        ui.painter().add(egui::Shape::closed_line(pts, stroke));
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
    let stroke_col = if shape.stroke == "none" {
        Color32::TRANSPARENT
    } else {
        hex32(&shape.stroke)
    };
    let sw = (shape.stroke_width * view.zoom).max(0.0);
    let stroke = Stroke::new(sw, stroke_col);
    let r = view.doc_rect(shape.x, shape.y, shape.width, shape.height);

    match shape.kind {
        ShapeKind::Rect | ShapeKind::Group => {
            let rad = shape.corner_radius.unwrap_or(0.0) * view.zoom;
            ui.painter().rect_filled(r, rad, fill);
            if sw > 0.0 {
                ui.painter()
                    .rect_stroke(r, rad, stroke, egui::StrokeKind::Inside);
            }
        }
        ShapeKind::Ellipse => {
            let center = r.center();
            let n = 48;
            let pts: Vec<Pos2> = (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                    center + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                })
                .collect();
            ui.painter()
                .add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        ShapeKind::Polygon => {
            let center = r.center();
            let n = 6;
            let pts: Vec<Pos2> = (0..n)
                .map(|i| {
                    let a =
                        i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                    center + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
                })
                .collect();
            ui.painter()
                .add(egui::Shape::convex_polygon(pts, fill, stroke));
        }
        ShapeKind::Path => {
            if let Some(points) = &shape.points {
                let pts: Vec<Pos2> = points
                    .iter()
                    .map(|p| view.doc_to_screen(p.x, p.y))
                    .collect();
                if pts.len() >= 2 {
                    ui.painter().add(egui::Shape::line(pts, stroke));
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
        let frame_fill = frame
            .fill
            .as_deref()
            .filter(|f| *f != "none")
            .map(|f| hex32(f))
            .unwrap_or(fill);
        ui.painter().rect_filled(r, 0.0, frame_fill);
        ui.painter()
            .rect_stroke(r, 0.0, Stroke::new(1.0_f32, edge), egui::StrokeKind::Inside);

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
        ui.painter().line_segment(
            [pos2(x, doc_rect.top()), pos2(x, doc_rect.bottom())],
            Stroke::new(0.5_f32, c),
        );
        x += step;
    }
    let mut y = doc_rect.top();
    while y <= doc_rect.bottom() {
        ui.painter().line_segment(
            [pos2(doc_rect.left(), y), pos2(doc_rect.right(), y)],
            Stroke::new(0.5_f32, c),
        );
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
    let hr = Rect::from_min_max(
        available.min + vec2(ruler, 0.0),
        pos2(available.right(), available.top() + ruler),
    );
    ui.painter().rect_filled(hr, 0.0, bg);
    ui.painter().line_segment(
        [hr.left_bottom(), hr.right_bottom()],
        Stroke::new(1.0_f32, hex32(border)),
    );
    let step_px = 50.0 * view.zoom;
    if step_px > 24.0 {
        let start_doc = ((hr.left() - view.origin.x) / view.zoom / 50.0).floor() * 50.0;
        let mut doc_x = start_doc;
        while view.doc_to_screen(doc_x, 0.0).x < hr.right() {
            let sx = view.doc_to_screen(doc_x, 0.0).x;
            if sx >= hr.left() {
                ui.painter().line_segment(
                    [pos2(sx, hr.bottom() - 5.0), pos2(sx, hr.bottom())],
                    Stroke::new(1.0_f32, tick),
                );
                ui.painter().text(
                    pos2(sx + 2.0, hr.top() + 2.0),
                    Align2::LEFT_TOP,
                    format!("{}", doc_x as i32),
                    FontId::monospace(8.0),
                    tick,
                );
            }
            doc_x += 50.0;
        }
    }
    let vr = Rect::from_min_max(
        available.min + vec2(0.0, ruler),
        pos2(available.left() + ruler, available.bottom()),
    );
    ui.painter().rect_filled(vr, 0.0, bg);
    ui.painter().line_segment(
        [vr.right_top(), vr.right_bottom()],
        Stroke::new(1.0_f32, hex32(border)),
    );
    if step_px > 24.0 {
        let start_doc = ((vr.top() - view.origin.y) / view.zoom / 50.0).floor() * 50.0;
        let mut doc_y = start_doc;
        while view.doc_to_screen(0.0, doc_y).y < vr.bottom() {
            let sy = view.doc_to_screen(0.0, doc_y).y;
            if sy >= vr.top() {
                ui.painter().line_segment(
                    [pos2(vr.right() - 5.0, sy), pos2(vr.right(), sy)],
                    Stroke::new(1.0_f32, tick),
                );
                ui.painter().text(
                    pos2(vr.left() + 2.0, sy + 2.0),
                    Align2::LEFT_TOP,
                    format!("{}", doc_y as i32),
                    FontId::monospace(8.0),
                    tick,
                );
            }
            doc_y += 50.0;
        }
    }
    let corner = Rect::from_min_size(available.min, vec2(ruler, ruler));
    ui.painter().rect_filled(corner, 0.0, bg);
}

// ---- Selection overlay -------------------------------------------------------------------------

fn draw_selection(ui: &mut egui::Ui, st: &AtelierState, view: &View, ants_start: Instant) {
    let Some(sel) = &st.selection else { return };
    let phase = (ants_start.elapsed().as_secs_f32() * 8.0) % 8.0; // marching ants
    let ants = Stroke::new(1.0_f32, Color32::WHITE);
    let ants2 = Stroke::new(1.0_f32, Color32::BLACK);
    match sel {
        Selection::Rect(x, y, w, h) => {
            let r = view.doc_rect(*x, *y, *w, *h);
            dashed_rect(ui, r, phase, ants);
            dashed_rect(ui, r, phase + 4.0, ants2);
        }
        Selection::Poly(pts) => {
            let screen: Vec<Pos2> = pts.iter().map(|p| view.doc_to_screen(p.x, p.y)).collect();
            for w in screen.windows(2) {
                ui.painter().line_segment([w[0], w[1]], ants2);
            }
            if screen.len() > 2 {
                ui.painter()
                    .line_segment([*screen.last().unwrap(), screen[0]], ants2);
            }
        }
    }
}

fn dashed_rect(ui: &mut egui::Ui, r: Rect, offset: f32, stroke: Stroke) {
    let edges = [
        (r.left_top(), r.right_top()),
        (r.right_top(), r.right_bottom()),
        (r.right_bottom(), r.left_bottom()),
        (r.left_bottom(), r.left_top()),
    ];
    for (a, b) in edges {
        dashed_line(ui, a, b, offset, stroke);
    }
}

fn dashed_line(ui: &mut egui::Ui, a: Pos2, b: Pos2, offset: f32, stroke: Stroke) {
    const DASH: f32 = 4.0;
    const GAP: f32 = 4.0;
    let len = a.distance(b);
    if len < 1.0 {
        return;
    }
    let dir = (b - a) / len;
    let mut t = -(offset % (DASH + GAP));
    while t < len {
        let start = t.max(0.0);
        let end = (t + DASH).min(len);
        if end > start {
            ui.painter()
                .line_segment([a + dir * start, a + dir * end], stroke);
        }
        t += DASH + GAP;
    }
}

// ---- Pen preview + node handles -------------------------------------------------------------------

fn draw_pen_preview(ui: &mut egui::Ui, st: &AtelierState, view: &View, accent: Color32) {
    if st.pen_path.is_empty() || st.persona != Persona::Vector {
        return;
    }
    let pts: Vec<Pos2> = st
        .pen_path
        .iter()
        .map(|p| view.doc_to_screen(p.x, p.y))
        .collect();
    if pts.len() >= 2 {
        ui.painter()
            .add(egui::Shape::line(pts.clone(), Stroke::new(1.5_f32, accent)));
    }
    if let Some(hover) = ui.input(|i| i.pointer.hover_pos()) {
        ui.painter().line_segment(
            [*pts.last().unwrap(), hover],
            Stroke::new(1.0_f32, accent.gamma_multiply(0.5)),
        );
    }
    for p in &pts {
        ui.painter()
            .rect_filled(Rect::from_center_size(*p, vec2(6.0, 6.0)), 1.0, accent);
    }
}

fn draw_node_handles(ui: &mut egui::Ui, st: &AtelierState, view: &View, accent: Color32) {
    if st.active_tool != ToolId::DirectSelect && st.active_tool != ToolId::NodeEdit {
        return;
    }
    for shape in &st.vector_shapes {
        if !st.selected_shape_ids.contains(&shape.id) {
            continue;
        }
        let Some(points) = &shape.points else {
            continue;
        };
        for (i, p) in points.iter().enumerate() {
            let s = view.doc_to_screen(p.x, p.y);
            let r = Rect::from_center_size(s, vec2(8.0, 8.0));
            let active = st
                .editing_node
                .as_ref()
                .map(|(id, n)| id == &shape.id && *n == i)
                .unwrap_or(false);
            ui.painter()
                .rect_filled(r, 1.0, if active { accent } else { Color32::WHITE });
            ui.painter().rect_stroke(
                r,
                1.0,
                Stroke::new(1.0_f32, accent),
                egui::StrokeKind::Inside,
            );
        }
    }
}

// ---- Tool interactions -----------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn handle_tools(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    eng: &mut CanvasEngine,
    response: &egui::Response,
    view: &View,
    canvas_rect: Rect,
    accent: Color32,
) {
    let press_pos = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos());
    let doc_pt = press_pos.map(|p| view.screen_to_doc(p));

    match st.persona {
        Persona::Photo => photo_tools(ui, st, eng, response, view, doc_pt, accent),
        Persona::Vector => vector_tools(ui, st, response, view, doc_pt, accent),
        Persona::Publish => publish_tools(ui, st, response, view, doc_pt, accent),
    }

    // Zoom tool: click zooms in, Alt+click zooms out (anchored).
    if st.active_tool == ToolId::Zoom && response.clicked() {
        if let (Some(p), Some(d)) = (press_pos, doc_pt) {
            let out = ui.input(|i| i.modifiers.alt);
            let factor = if out { 1.0 / 1.5 } else { 1.5 };
            let new_zoom = (st.zoom * factor).clamp(0.02, 32.0);
            let new_origin = p - d * new_zoom;
            st.pan_x = new_origin.x - canvas_rect.center().x + st.project.width * new_zoom / 2.0;
            st.pan_y = new_origin.y - canvas_rect.center().y + st.project.height * new_zoom / 2.0;
            st.zoom = new_zoom;
            st.hint(format!("Zoom {:.0}%", st.zoom * 100.0));
        }
    }

    // Double-click resets viewport.
    if response.double_clicked() {
        st.reset_viewport();
    }
}

// ---- Photo tools -----------------------------------------------------------------------------------

fn photo_tools(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    eng: &mut CanvasEngine,
    response: &egui::Response,
    view: &View,
    doc_pt: Option<Vec2>,
    _accent: Color32,
) {
    let tool = st.active_tool;
    let (doc_w, doc_h) = (st.project.width, st.project.height);

    // -- Painting tools (brush / eraser / healing-clone / dodge-burn) --
    let is_paint = matches!(
        tool,
        ToolId::Brush | ToolId::Eraser | ToolId::Healing | ToolId::Dodge | ToolId::Burn
    );

    if is_paint {
        if tool == ToolId::Healing && response.clicked() && ui.input(|i| i.modifiers.alt) {
            if let Some(d) = doc_pt {
                st.clone_source = Some((d.x, d.y));
                st.hint("Clone source set — drag to heal");
            }
            return;
        }
        if response.drag_started() {
            let target = st.paint_target_layer().or_else(|| {
                st.add_layer(LayerType::Raster, Some("Paint Layer".into()));
                st.project.layers.last().map(|l| l.id.clone())
            });
            eng.stroke_target = target;
            eng.stroke_last = doc_pt.map(|d| (d.x, d.y));
            st.painting = true;
        }
        if response.dragged() {
            if let (Some(target), Some(cur)) = (eng.stroke_target.clone(), doc_pt) {
                let (w, h) = (doc_w.max(1.0) as u32, doc_h.max(1.0) as u32);
                let o = &st.tool_options;
                let fg = hex32(&st.foreground_color);
                let color = [fg.r(), fg.g(), fg.b()];
                let alpha = (o.brush_opacity / 100.0) * (o.brush_flow / 100.0);
                let radius = (o.brush_size / 2.0).max(0.5);
                let prev = eng.stroke_last.unwrap_or((cur.x, cur.y));
                let from = (cur.x, cur.y);
                let sel = st.selection.as_ref();
                let layer_idx = st.project.layers.iter().position(|l| l.id == target);
                if let Some(idx) = layer_idx {
                    let layer = st.project.layers[idx].clone();
                    let buf = eng.store.ensure(&layer, w, h);
                    match tool {
                        ToolId::Eraser => raster::paint_stroke(
                            buf,
                            prev,
                            from,
                            radius,
                            o.brush_hardness,
                            [0, 0, 0],
                            alpha,
                            sel,
                            true,
                        ),
                        ToolId::Healing => {
                            let src = st.clone_source.unwrap_or(prev);
                            let off = (src.0 - prev.0, src.1 - prev.1);
                            raster::clone_stroke(
                                buf,
                                off,
                                prev,
                                from,
                                radius,
                                o.brush_hardness,
                                sel,
                            );
                        }
                        ToolId::Dodge | ToolId::Burn => {
                            raster::dodge_burn_stroke(
                                buf,
                                prev,
                                from,
                                radius,
                                alpha,
                                tool == ToolId::Dodge,
                                sel,
                            );
                        }
                        _ => raster::paint_stroke(
                            buf,
                            prev,
                            from,
                            radius,
                            o.brush_hardness,
                            color,
                            alpha,
                            sel,
                            false,
                        ),
                    }
                    eng.stroke_last = Some(from);
                    st.raster_version += 1;
                }
            }
        }
        if response.drag_stopped() && eng.stroke_target.is_some() {
            finish_stroke(st, eng);
        }
        return;
    }

    match tool {
        ToolId::Marquee => {
            if response.drag_started() {
                set_drag_start(ui, response.interact_pointer_pos());
            }
            if response.dragged() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), press_pos(response)) {
                    ui.painter().rect_stroke(
                        Rect::from_two_pos(a, b),
                        0.0,
                        Stroke::new(1.0_f32, Color32::WHITE),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            if response.drag_stopped() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), response.interact_pointer_pos()) {
                    let da = view.screen_to_doc(a);
                    let db = view.screen_to_doc(b);
                    let (x, y) = (da.x.min(db.x), da.y.min(db.y));
                    let (w, h) = ((da.x - db.x).abs(), (da.y - db.y).abs());
                    st.selection = if w > 2.0 && h > 2.0 {
                        Some(Selection::Rect(x, y, w, h))
                    } else {
                        None
                    };
                    if st.selection.is_some() {
                        st.hint(format!("Marquee {}×{}", w as i32, h as i32));
                    }
                }
                set_drag_start(ui, None);
            }
        }
        ToolId::Lasso => {
            if response.drag_started() {
                eng.lasso_pts.clear();
            }
            if response.dragged() {
                if let Some(d) = doc_pt {
                    eng.lasso_pts.push(pos2(d.x, d.y));
                }
                if eng.lasso_pts.len() > 1 {
                    let pts: Vec<Pos2> = eng
                        .lasso_pts
                        .iter()
                        .map(|p| view.doc_to_screen(p.x, p.y))
                        .collect();
                    ui.painter()
                        .add(egui::Shape::line(pts, Stroke::new(1.0_f32, Color32::WHITE)));
                }
            }
            if response.drag_stopped() && eng.lasso_pts.len() > 2 {
                st.selection = Some(Selection::Poly(eng.lasso_pts.clone()));
                st.hint(format!("Lasso selection — {} pts", eng.lasso_pts.len()));
            }
        }
        ToolId::Crop => {
            if response.drag_started() {
                set_drag_start(ui, response.interact_pointer_pos());
            }
            if response.dragged() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), press_pos(response)) {
                    ui.painter().rect_stroke(
                        Rect::from_two_pos(a, b),
                        0.0,
                        Stroke::new(1.5_f32, hex32("#f59e0b")),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            if response.drag_stopped() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), response.interact_pointer_pos()) {
                    let da = view.screen_to_doc(a);
                    let db = view.screen_to_doc(b);
                    let (x, y) = (da.x.min(db.x).max(0.0), da.y.min(db.y).max(0.0));
                    let (x2, y2) = (da.x.max(db.x).min(doc_w), da.y.max(db.y).min(doc_h));
                    if x2 - x > 8.0 && y2 - y > 8.0 {
                        st.apply_crop(x, y, x2 - x, y2 - y);
                        eng.store.gc(&st
                            .project
                            .layers
                            .iter()
                            .map(|l| l.id.clone())
                            .collect::<Vec<_>>());
                    }
                }
                set_drag_start(ui, None);
            }
        }
        ToolId::Bucket => {
            if response.clicked() {
                if let Some(d) = doc_pt {
                    let target = st.paint_target_layer().or_else(|| {
                        st.add_layer(LayerType::Raster, Some("Fill Layer".into()));
                        st.project.layers.last().map(|l| l.id.clone())
                    });
                    if let Some(tid) = target {
                        let idx = st.project.layers.iter().position(|l| l.id == tid).unwrap();
                        let layer = st.project.layers[idx].clone();
                        let (w, h) = (doc_w.max(1.0) as u32, doc_h.max(1.0) as u32);
                        let buf = eng.store.ensure(&layer, w, h);
                        let fg = hex32(&st.foreground_color);
                        let alt = ui.input(|i| i.modifiers.alt);
                        if alt {
                            raster::fill_all(
                                buf,
                                [fg.r(), fg.g(), fg.b(), 255],
                                st.selection.as_ref(),
                            );
                        } else {
                            raster::flood_fill(
                                buf,
                                d.x as i32,
                                d.y as i32,
                                [fg.r(), fg.g(), fg.b(), 255],
                                24,
                                st.selection.as_ref(),
                            );
                        }
                        st.raster_version += 1;
                        eng.stroke_target = Some(tid.clone());
                        finish_stroke_named(st, eng, "Bucket Fill");
                    }
                }
            }
        }
        ToolId::Gradient => {
            if response.drag_started() {
                set_drag_start(ui, response.interact_pointer_pos());
            }
            if response.dragged() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), press_pos(response)) {
                    ui.painter()
                        .line_segment([a, b], Stroke::new(1.5_f32, hex32("#0d99ff")));
                }
            }
            if response.drag_stopped() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), response.interact_pointer_pos()) {
                    let target = st.paint_target_layer().or_else(|| {
                        st.add_layer(LayerType::Raster, Some("Gradient Layer".into()));
                        st.project.layers.last().map(|l| l.id.clone())
                    });
                    if let Some(tid) = target {
                        let da = view.screen_to_doc(a);
                        let db = view.screen_to_doc(b);
                        let idx = st.project.layers.iter().position(|l| l.id == tid).unwrap();
                        let layer = st.project.layers[idx].clone();
                        let (w, h) = (doc_w.max(1.0) as u32, doc_h.max(1.0) as u32);
                        let buf = eng.store.ensure(&layer, w, h);
                        let fg = hex32(&st.foreground_color);
                        let bg = hex32(&st.background_color);
                        raster::linear_gradient(
                            buf,
                            (da.x, da.y),
                            (db.x, db.y),
                            [fg.r(), fg.g(), fg.b(), 255],
                            [bg.r(), bg.g(), bg.b(), 255],
                            st.selection.as_ref(),
                        );
                        st.raster_version += 1;
                        eng.stroke_target = Some(tid.clone());
                        finish_stroke_named(st, eng, "Gradient Fill");
                    }
                }
                set_drag_start(ui, None);
            }
        }
        ToolId::Eyedropper => {
            if response.clicked() {
                if let Some(d) = doc_pt {
                    if let Some((_, buf)) = eng.composite(&ui.ctx().clone(), st) {
                        let (x, y) = (
                            d.x.clamp(0.0, doc_w - 1.0) as u32,
                            d.y.clamp(0.0, doc_h - 1.0) as u32,
                        );
                        let p = *buf.get_pixel(x, y);
                        // Composite over white paper for transparent areas.
                        let a = p[3] as f32 / 255.0;
                        let r = (p[0] as f32 * a + 255.0 * (1.0 - a)) as u8;
                        let g = (p[1] as f32 * a + 255.0 * (1.0 - a)) as u8;
                        let b = (p[2] as f32 * a + 255.0 * (1.0 - a)) as u8;
                        st.foreground_color = format!("#{:02x}{:02x}{:02x}", r, g, b);
                        st.hint(format!("Sampled {}", st.foreground_color));
                    }
                }
            }
        }
        ToolId::Text => {
            if response.clicked() {
                if let Some(d) = doc_pt {
                    let (font_size, font_family, font_weight, text_color) = (
                        st.tool_options.font_size,
                        st.tool_options.font_family.clone(),
                        st.tool_options.font_weight.clone(),
                        st.tool_options.text_color.clone(),
                    );
                    st.add_layer(LayerType::Text, None);
                    if let Some(l) = st.project.layers.last_mut() {
                        l.text_content = Some("New Text".into());
                        l.text_font_size = Some(font_size);
                        l.text_font_family = Some(font_family);
                        l.text_font_weight = Some(font_weight);
                        l.text_color = Some(text_color);
                        l.x = Some(d.x);
                        l.y = Some(d.y);
                    }
                    st.active_dock_tab = DockTab::Typography;
                    st.hint("Type layer created — edit content in the Type panel");
                }
            }
        }
        _ => {}
    }
}

fn press_pos(response: &egui::Response) -> Option<Pos2> {
    response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos())
}

/// Encode the edited buffer back into the layer (for .szt + history) and
/// push one history step for the whole stroke.
fn finish_stroke(st: &mut AtelierState, eng: &mut CanvasEngine) {
    finish_stroke_named(st, eng, "Paint Stroke");
}

fn finish_stroke_named(st: &mut AtelierState, eng: &mut CanvasEngine, title: &str) {
    if let Some(tid) = eng.stroke_target.take().or_else(|| st.paint_target_layer()) {
        if let Some(idx) = st.project.layers.iter().position(|l| l.id == tid) {
            if let Some(buf) = eng.store.buffers.get(&tid) {
                let mut png = Cursor::new(Vec::new());
                if image::DynamicImage::ImageRgba8(buf.clone())
                    .write_to(&mut png, image::ImageFormat::Png)
                    .is_ok()
                {
                    st.project.layers[idx].raster_png = Some(png.into_inner());
                }
            }
        }
    }
    st.painting = false;
    eng.stroke_last = None;
    st.project.updated_at = now_millis();
    st.push_history(title);
}

// ---- Vector tools -------------------------------------------------------------------------------------

fn vector_tools(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    response: &egui::Response,
    view: &View,
    doc_pt: Option<Vec2>,
    accent: Color32,
) {
    match st.active_tool {
        ToolId::Pen => {
            if response.clicked() {
                if let Some(d) = doc_pt {
                    st.pen_path.push(VectorPoint {
                        x: d.x,
                        y: d.y,
                        ..Default::default()
                    });
                }
            }
            if response.double_clicked() {
                st.commit_pen_path(false);
            }
        }
        ToolId::Move => {
            // Click select + drag move with snapping.
            if response.clicked() {
                if let Some(d) = doc_pt {
                    let hit = st
                        .vector_shapes
                        .iter()
                        .rev()
                        .find(|s| {
                            d.x >= s.x
                                && d.x <= s.x + s.width
                                && d.y >= s.y
                                && d.y <= s.y + s.height
                        })
                        .map(|s| s.id.clone());
                    match hit {
                        Some(id) => {
                            if ui.input(|i| i.modifiers.shift) {
                                if !st.selected_shape_ids.contains(&id) {
                                    st.selected_shape_ids.push(id);
                                }
                            } else if !st.selected_shape_ids.contains(&id) {
                                st.selected_shape_ids = vec![id];
                            }
                        }
                        None => st.selected_shape_ids.clear(),
                    }
                }
            }
            if response.dragged() {
                let mut d = response.drag_delta() / view.zoom;
                if st.snapping.enabled {
                    d += snap_delta(st, d);
                }
                st.nudge_selected(d.x, d.y);
            }
            if response.drag_stopped() && !st.selected_shape_ids.is_empty() {
                st.push_history("Move Shape(s)");
            }
        }
        ToolId::DirectSelect | ToolId::NodeEdit => {
            // Click empty space selects the path shape; drag on a node moves it.
            if response.drag_started() {
                st.editing_node = None;
                if doc_pt.is_some() {
                    'outer: for shape in &st.vector_shapes {
                        if let Some(points) = &shape.points {
                            for (i, p) in points.iter().enumerate() {
                                let s = view.doc_to_screen(p.x, p.y);
                                if let Some(pp) = press_pos(response) {
                                    if s.distance(pp) < 10.0 {
                                        st.editing_node = Some((shape.id.clone(), i));
                                        if !st.selected_shape_ids.contains(&shape.id) {
                                            st.selected_shape_ids = vec![shape.id.clone()];
                                        }
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if response.dragged() {
                if let (Some((sid, ni)), Some(d)) = (st.editing_node.clone(), doc_pt) {
                    if let Some(shape) = st.vector_shapes.iter_mut().find(|s| s.id == sid) {
                        if let Some(points) = &mut shape.points {
                            if let Some(p) = points.get_mut(ni) {
                                p.x = d.x;
                                p.y = d.y;
                            }
                            // Recompute bbox.
                            let (mut x0, mut y0, mut x1, mut y1) = (
                                f32::INFINITY,
                                f32::INFINITY,
                                f32::NEG_INFINITY,
                                f32::NEG_INFINITY,
                            );
                            for p in points.iter() {
                                x0 = x0.min(p.x);
                                y0 = y0.min(p.y);
                                x1 = x1.max(p.x);
                                y1 = y1.max(p.y);
                            }
                            shape.x = x0;
                            shape.y = y0;
                            shape.width = x1 - x0;
                            shape.height = y1 - y0;
                        }
                    }
                }
            }
            if response.drag_stopped() && st.editing_node.is_some() {
                st.editing_node = None;
                st.push_history("Edit Path Node");
            }
        }
        _ => {
            // Shape creation tools (rect/ellipse/polygon) — shared drag-create.
            let creation = matches!(
                st.active_tool,
                ToolId::ShapeRect | ToolId::ShapeEllipse | ToolId::ShapePolygon
            );
            if creation {
                if response.drag_started() {
                    set_drag_start(ui, response.interact_pointer_pos());
                }
                if response.dragged() {
                    if let (Some(a), Some(b)) = (get_drag_start(ui), press_pos(response)) {
                        ui.painter().rect_stroke(
                            Rect::from_two_pos(a, b),
                            0.0,
                            Stroke::new(1.0_f32, accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                if response.drag_stopped() {
                    if let (Some(a), Some(b)) =
                        (get_drag_start(ui), response.interact_pointer_pos())
                    {
                        let da = view.screen_to_doc(a);
                        let db = view.screen_to_doc(b);
                        let (x, y) = (da.x.min(db.x), da.y.min(db.y));
                        let (w, h) = ((da.x - db.x).abs().max(4.0), (da.y - db.y).abs().max(4.0));
                        let kind = match st.active_tool {
                            ToolId::ShapeEllipse => ShapeKind::Ellipse,
                            ToolId::ShapePolygon => ShapeKind::Polygon,
                            _ => ShapeKind::Rect,
                        };
                        let mut sh = VectorShape::new(kind, x, y, w, h);
                        sh.fill = st.vector_fill.clone();
                        sh.stroke = st.vector_stroke.clone();
                        sh.stroke_width = st.vector_stroke_width;
                        sh.stroke_style = Some(st.vector_stroke_style);
                        sh.corner_radius = Some(st.vector_corner_radius);
                        let id = sh.id.clone();
                        st.vector_shapes.push(sh);
                        st.selected_shape_ids = vec![id];
                        st.push_history("Draw Vector Shape");
                    }
                    set_drag_start(ui, None);
                }
            }
        }
    }
}

/// Snap the drag delta so the selection bbox edges land on guides/doc bounds.
fn snap_delta(st: &AtelierState, delta: Vec2) -> Vec2 {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    let mut any = false;
    for s in &st.vector_shapes {
        if st.selected_shape_ids.contains(&s.id) {
            min_x = min_x.min(s.x);
            min_y = min_y.min(s.y);
            max_x = max_x.max(s.x + s.width);
            max_y = max_y.max(s.y + s.height);
            any = true;
        }
    }
    if !any {
        return Vec2::ZERO;
    }
    let (min_x, min_y, max_x, max_y) = (
        min_x + delta.x,
        min_y + delta.y,
        max_x + delta.x,
        max_y + delta.y,
    );
    let mid_x = (min_x + max_x) / 2.0;
    let mid_y = (min_y + max_y) / 2.0;
    let tol = st.snapping.tolerance;
    let (mut cx, mut cy) = (0.0f32, 0.0f32);

    let snap = |v: f32, targets: &mut dyn Iterator<Item = f32>, tol: f32| -> Option<f32> {
        targets
            .map(|t| (t - v, (t - v).abs()))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(d, _)| d)
    };

    if st.snapping.snap_to_doc_bounds {
        let mut targets = [0.0, st.project.width / 2.0, st.project.width].into_iter();
        cx = snap(min_x, &mut targets, tol)
            .or_else(|| {
                snap(
                    max_x,
                    &mut [0.0, st.project.width / 2.0, st.project.width].into_iter(),
                    tol,
                )
            })
            .or_else(|| snap(mid_x, &mut [st.project.width / 2.0].into_iter(), tol))
            .unwrap_or(0.0);
        cy = snap(
            min_y,
            &mut [0.0, st.project.height / 2.0, st.project.height].into_iter(),
            tol,
        )
        .or_else(|| {
            snap(
                max_y,
                &mut [0.0, st.project.height / 2.0, st.project.height].into_iter(),
                tol,
            )
        })
        .or_else(|| snap(mid_y, &mut [st.project.height / 2.0].into_iter(), tol))
        .unwrap_or(0.0);
    }
    if st.snapping.snap_to_guides {
        let vert: Vec<f32> = st
            .guides
            .iter()
            .filter(|g| g.kind == GuideKind::Vertical)
            .map(|g| g.position)
            .collect();
        let horiz: Vec<f32> = st
            .guides
            .iter()
            .filter(|g| g.kind == GuideKind::Horizontal)
            .map(|g| g.position)
            .collect();
        if cx == 0.0 {
            cx = snap(min_x, &mut vert.iter().copied(), tol)
                .or_else(|| snap(max_x, &mut vert.iter().copied(), tol))
                .or_else(|| snap(mid_x, &mut vert.iter().copied(), tol))
                .unwrap_or(0.0);
        }
        if cy == 0.0 {
            cy = snap(min_y, &mut horiz.iter().copied(), tol)
                .or_else(|| snap(max_y, &mut horiz.iter().copied(), tol))
                .or_else(|| snap(mid_y, &mut horiz.iter().copied(), tol))
                .unwrap_or(0.0);
        }
    }
    vec2(cx, cy)
}

// ---- Publish tools --------------------------------------------------------------------------------------

fn publish_tools(
    ui: &mut egui::Ui,
    st: &mut AtelierState,
    response: &egui::Response,
    view: &View,
    doc_pt: Option<Vec2>,
    accent: Color32,
) {
    let creation = matches!(
        st.active_tool,
        ToolId::FrameText | ToolId::FramePicture | ToolId::FrameShape | ToolId::LineTool
    );

    match st.active_tool {
        ToolId::Move => {
            if response.clicked() {
                if let Some(d) = doc_pt {
                    let hit = st
                        .publish_frames
                        .iter()
                        .rev()
                        .find(|f| {
                            d.x >= f.x
                                && d.x <= f.x + f.width
                                && d.y >= f.y
                                && d.y <= f.y + f.height
                        })
                        .map(|f| f.id.clone());
                    match hit {
                        Some(id) => st.selected_frame_ids = vec![id],
                        None => st.selected_frame_ids.clear(),
                    }
                }
            }
            if response.dragged() {
                let d = response.drag_delta() / view.zoom;
                for f in &mut st.publish_frames {
                    if st.selected_frame_ids.contains(&f.id) {
                        f.x += d.x;
                        f.y += d.y;
                    }
                }
            }
            if response.drag_stopped() && !st.selected_frame_ids.is_empty() {
                st.push_history("Move Frame(s)");
            }
        }
        _ if creation => {
            if response.drag_started() {
                set_drag_start(ui, response.interact_pointer_pos());
            }
            if response.dragged() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), press_pos(response)) {
                    ui.painter().rect_stroke(
                        Rect::from_two_pos(a, b),
                        0.0,
                        Stroke::new(1.0_f32, accent),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            if response.drag_stopped() {
                if let (Some(a), Some(b)) = (get_drag_start(ui), response.interact_pointer_pos()) {
                    let da = view.screen_to_doc(a);
                    let db = view.screen_to_doc(b);
                    let (x, y) = (da.x.min(db.x), da.y.min(db.y));
                    let (w, h) = ((da.x - db.x).abs().max(4.0), (da.y - db.y).abs().max(4.0));
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
                        f.content = Some("Text frame — edit content in the Type panel".into());
                    }
                    if kind == FrameKind::Rule {
                        f.height = 2.0;
                    }
                    let id = f.id.clone();
                    st.publish_frames.push(f);
                    st.selected_frame_ids = vec![id];
                    st.push_history("Draw Publish Frame");
                }
                set_drag_start(ui, None);
            }
        }
        _ => {}
    }
}
