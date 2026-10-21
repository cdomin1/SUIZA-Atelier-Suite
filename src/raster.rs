//! Raster engine — per-layer RGBA buffers, brush stamps, fills and the
//! layer compositor with blend modes + adjustment layers.

use crate::model::{AdjustmentType, BlendMode, Layer};
use egui::Pos2;
use image::RgbaImage;
use std::collections::HashMap;

pub struct RasterStore {
    pub buffers: HashMap<String, RgbaImage>,
}

impl RasterStore {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Get (or lazily create) a layer's pixel buffer.
    pub fn ensure(&mut self, layer: &Layer, w: u32, h: u32) -> &mut RgbaImage {
        self.buffers.entry(layer.id.clone()).or_insert_with(|| {
            layer
                .raster_png
                .as_ref()
                .and_then(|png| image::load_from_memory(png).ok())
                .map(|img| img.to_rgba8())
                .unwrap_or_else(|| RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0])))
        })
    }

    pub fn invalidate(&mut self, layer_id: &str) {
        self.buffers.remove(layer_id);
    }

    /// Buffers whose layer no longer exists are dropped.
    pub fn gc(&mut self, live_ids: &[String]) {
        self.buffers.retain(|k, _| live_ids.contains(k));
    }
}

// ---- Selection ----------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Selection {
    Rect(f32, f32, f32, f32), // x, y, w, h in doc coords
    Poly(Vec<Pos2>),
}

impl Selection {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Selection::Rect(rx, ry, rw, rh) => x >= *rx && x <= rx + rw && y >= *ry && y <= ry + rh,
            Selection::Poly(pts) => point_in_poly(pts, x, y),
        }
    }

    /// Fast reject for stroke segments.
    pub fn bbox(&self) -> (f32, f32, f32, f32) {
        match self {
            Selection::Rect(x, y, w, h) => (*x, *y, *x + w, *y + h),
            Selection::Poly(pts) => {
                let (mut x0, mut y0, mut x1, mut y1) = (
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                );
                for p in pts {
                    x0 = x0.min(p.x);
                    y0 = y0.min(p.y);
                    x1 = x1.max(p.x);
                    y1 = y1.max(p.y);
                }
                (x0, y0, x1, y1)
            }
        }
    }
}

fn point_in_poly(pts: &[Pos2], x: f32, y: f32) -> bool {
    // Ray-cast even/odd test.
    let mut inside = false;
    let mut j = pts.len().saturating_sub(1);
    for i in 0..pts.len() {
        let (xi, yi) = (pts[i].x, pts[i].y);
        let (xj, yj) = (pts[j].x, pts[j].y);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi + f32::EPSILON) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

// ---- Brush stamps ----------------------------------------------------------------

/// Soft-edged circular stamp at (cx, cy) in buffer coords.
/// `alpha` 0..1 already includes opacity*flow.
fn stamp(
    buf: &mut RgbaImage,
    cx: f32,
    cy: f32,
    radius: f32,
    hardness: f32,
    color: [u8; 3],
    alpha: f32,
    sel: Option<&Selection>,
    erase: bool,
) {
    let r = radius.max(0.5);
    let (x0, y0, x1, y1) = (
        (cx - r).floor().max(0.0) as i64,
        (cy - r).floor().max(0.0) as i64,
        (cx + r).ceil().min(buf.width() as f32) as i64,
        (cy + r).ceil().min(buf.height() as f32) as i64,
    );
    let hard = (hardness / 100.0).clamp(0.0, 1.0);
    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / r;
            if dist > 1.0 {
                continue;
            }
            if let Some(s) = sel {
                if !s.contains(px as f32 + 0.5, py as f32 + 0.5) {
                    continue;
                }
            }
            // Softness ramp: solid core out to `hard`, then linear falloff.
            let fall = if dist <= hard {
                1.0
            } else {
                (1.0 - (dist - hard) / (1.0 - hard).max(0.001)).clamp(0.0, 1.0)
            };
            let a = (alpha * fall).clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            let p = buf.get_pixel_mut(px as u32, py as u32);
            if erase {
                p[3] = ((p[3] as f32) * (1.0 - a)) as u8;
            } else {
                blend_normal(p, color, a);
            }
        }
    }
}

fn blend_normal(dst: &mut image::Rgba<u8>, src: [u8; 3], src_alpha: f32) {
    let da = dst[3] as f32 / 255.0;
    let a = src_alpha + da * (1.0 - src_alpha);
    if a <= 0.0 {
        *dst = image::Rgba([0, 0, 0, 0]);
        return;
    }
    for c in 0..3 {
        dst[c] = ((src[c] as f32 / 255.0 * src_alpha
            + dst[c] as f32 / 255.0 * da * (1.0 - src_alpha))
            / a
            * 255.0)
            .clamp(0.0, 255.0) as u8;
    }
    dst[3] = (a * 255.0).clamp(0.0, 255.0) as u8;
}

/// Stroke stamps interpolated along a segment (spacing = radius/4).
#[allow(clippy::too_many_arguments)]
pub fn paint_stroke(
    buf: &mut RgbaImage,
    from: (f32, f32),
    to: (f32, f32),
    radius: f32,
    hardness: f32,
    color: [u8; 3],
    alpha: f32,
    sel: Option<&Selection>,
    erase: bool,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let spacing = (radius / 4.0).max(0.5);
    let steps = (dist / spacing).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        stamp(
            buf,
            from.0 + dx * t,
            from.1 + dy * t,
            radius,
            hardness,
            color,
            alpha,
            sel,
            erase,
        );
    }
}

/// Clone-stamp: copy pixels from `src` offset region — used for Healing/Clone.
pub fn clone_stroke(
    buf: &mut RgbaImage,
    src_off: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
    radius: f32,
    hardness: f32,
    sel: Option<&Selection>,
) {
    let snapshot = buf.clone();
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let spacing = (radius / 4.0).max(0.5);
    let steps = (dist / spacing).ceil().max(1.0) as i32;
    let hard = (hardness / 100.0).clamp(0.0, 1.0);
    let r = radius.max(0.5);
    for i in 0..=steps {
        let cx = from.0 + dx * t_(i, steps);
        let cy = from.1 + dy * t_(i, steps);
        let (x0, y0, x1, y1) = (
            (cx - r).floor().max(0.0) as i64,
            (cy - r).floor().max(0.0) as i64,
            (cx + r).ceil().min(buf.width() as f32) as i64,
            (cy + r).ceil().min(buf.height() as f32) as i64,
        );
        for py in y0..y1 {
            for px in x0..x1 {
                let ddx = px as f32 + 0.5 - cx;
                let ddy = py as f32 + 0.5 - cy;
                let dist2 = (ddx * ddx + ddy * ddy).sqrt() / r;
                if dist2 > 1.0 {
                    continue;
                }
                if let Some(s) = sel {
                    if !s.contains(px as f32 + 0.5, py as f32 + 0.5) {
                        continue;
                    }
                }
                let (sx, sy) = (px as f32 + src_off.0, py as f32 + src_off.1);
                if sx < 0.0 || sy < 0.0 || sx >= buf.width() as f32 || sy >= buf.height() as f32 {
                    continue;
                }
                let fall = if dist2 <= hard {
                    1.0
                } else {
                    (1.0 - (dist2 - hard) / (1.0 - hard).max(0.001)).clamp(0.0, 1.0)
                };
                let src = snapshot.get_pixel(sx as u32, sy as u32);
                let dst = buf.get_pixel_mut(px as u32, py as u32);
                let sa = src[3] as f32 / 255.0 * fall;
                blend_normal(dst, [src[0], src[1], src[2]], sa);
            }
        }
    }
}

fn t_(i: i32, steps: i32) -> f32 {
    i as f32 / steps.max(1) as f32
}

/// Dodge (lighten) or burn (darken) along a stroke.
pub fn dodge_burn_stroke(
    buf: &mut RgbaImage,
    from: (f32, f32),
    to: (f32, f32),
    radius: f32,
    amount: f32,
    dodge: bool,
    sel: Option<&Selection>,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let spacing = (radius / 4.0).max(0.5);
    let steps = (dist / spacing).ceil().max(1.0) as i32;
    let r = radius.max(0.5);
    for i in 0..=steps {
        let cx = from.0 + dx * t_(i, steps);
        let cy = from.1 + dy * t_(i, steps);
        let (x0, y0, x1, y1) = (
            (cx - r).floor().max(0.0) as i64,
            (cy - r).floor().max(0.0) as i64,
            (cx + r).ceil().min(buf.width() as f32) as i64,
            (cy + r).ceil().min(buf.height() as f32) as i64,
        );
        for py in y0..y1 {
            for px in x0..x1 {
                let ddx = px as f32 + 0.5 - cx;
                let ddy = py as f32 + 0.5 - cy;
                let dist2 = (ddx * ddx + ddy * ddy).sqrt() / r;
                if dist2 > 1.0 {
                    continue;
                }
                if let Some(s) = sel {
                    if !s.contains(px as f32 + 0.5, py as f32 + 0.5) {
                        continue;
                    }
                }
                let p = buf.get_pixel_mut(px as u32, py as u32);
                let fall = 1.0 - dist2;
                let f = amount * fall * 0.15;
                for c in 0..3 {
                    let v = p[c] as f32;
                    p[c] = if dodge {
                        (v + (255.0 - v) * f).min(255.0) as u8
                    } else {
                        (v * (1.0 - f)).max(0.0) as u8
                    };
                }
            }
        }
    }
}

// ---- Fills ------------------------------------------------------------------------

/// 4-way flood fill from a seed point, tolerance-band matching.
pub fn flood_fill(
    buf: &mut RgbaImage,
    x: i32,
    y: i32,
    color: [u8; 4],
    tolerance: u8,
    sel: Option<&Selection>,
) {
    let (w, h) = (buf.width() as i32, buf.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return;
    }
    let target = *buf.get_pixel(x as u32, y as u32);
    if color_diff(target.0, color) <= tolerance as i32 {
        return; // already the target color
    }
    let tol = tolerance as i32;
    let mut stack = vec![(x, y)];
    let mut seen = vec![false; (w * h) as usize];
    while let Some((cx, cy)) = stack.pop() {
        if cx < 0 || cy < 0 || cx >= w || cy >= h {
            continue;
        }
        let idx = (cy * w + cx) as usize;
        if seen[idx] {
            continue;
        }
        seen[idx] = true;
        let p = *buf.get_pixel(cx as u32, cy as u32);
        if color_diff(p.0, target.0) > tol {
            continue;
        }
        if let Some(s) = sel {
            if !s.contains(cx as f32 + 0.5, cy as f32 + 0.5) {
                continue;
            }
        }
        buf.put_pixel(cx as u32, cy as u32, image::Rgba(color));
        stack.push((cx + 1, cy));
        stack.push((cx - 1, cy));
        stack.push((cx, cy + 1));
        stack.push((cx, cy - 1));
    }
}

fn color_diff(a: [u8; 4], b: [u8; 4]) -> i32 {
    (0..3)
        .map(|i| (a[i] as i32 - b[i] as i32).abs())
        .sum::<i32>()
        + (a[3] as i32 - b[3] as i32).abs()
}

/// Fill the whole buffer (or selection) with a color.
pub fn fill_all(buf: &mut RgbaImage, color: [u8; 4], sel: Option<&Selection>) {
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            if let Some(s) = sel {
                if !s.contains(x as f32 + 0.5, y as f32 + 0.5) {
                    continue;
                }
            }
            buf.put_pixel(x, y, image::Rgba(color));
        }
    }
}

/// Linear gradient between two doc-space points.
pub fn linear_gradient(
    buf: &mut RgbaImage,
    from: (f32, f32),
    to: (f32, f32),
    c1: [u8; 4],
    c2: [u8; 4],
    sel: Option<&Selection>,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len2 = (dx * dx + dy * dy).max(1.0);
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            if let Some(s) = sel {
                if !s.contains(x as f32 + 0.5, y as f32 + 0.5) {
                    continue;
                }
            }
            let t = (((x as f32 - from.0) * dx + (y as f32 - from.1) * dy) / len2).clamp(0.0, 1.0);
            let px = [
                (c1[0] as f32 + (c2[0] as f32 - c1[0] as f32) * t) as u8,
                (c1[1] as f32 + (c2[1] as f32 - c1[1] as f32) * t) as u8,
                (c1[2] as f32 + (c2[2] as f32 - c1[2] as f32) * t) as u8,
                (c1[3] as f32 + (c2[3] as f32 - c1[3] as f32) * t) as u8,
            ];
            buf.put_pixel(x, y, image::Rgba(px));
        }
    }
}

// ---- Compositor ---------------------------------------------------------------------

fn blend_channel(mode: BlendMode, s: f32, d: f32) -> f32 {
    match mode {
        BlendMode::Normal => s,
        BlendMode::Multiply => s * d,
        BlendMode::Screen => s + d - s * d,
        BlendMode::Overlay => {
            if d < 0.5 {
                2.0 * s * d
            } else {
                1.0 - 2.0 * (1.0 - s) * (1.0 - d)
            }
        }
        BlendMode::Darken => s.min(d),
        BlendMode::Lighten => s.max(d),
        BlendMode::ColorDodge => {
            if s >= 1.0 {
                1.0
            } else {
                (d / (1.0 - s)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - d) / s).min(1.0)
            }
        }
        BlendMode::HardLight => {
            if s < 0.5 {
                2.0 * s * d
            } else {
                1.0 - 2.0 * (1.0 - s) * (1.0 - d)
            }
        }
        BlendMode::SoftLight => (1.0 - 2.0 * s) * d * d + 2.0 * s * d,
        BlendMode::Difference => (d - s).abs(),
        BlendMode::Exclusion => d + s - 2.0 * d * s,
    }
}

fn composite_layer(dst: &mut RgbaImage, src: &RgbaImage, opacity: f32, mode: BlendMode) {
    for (d, s) in dst.pixels_mut().zip(src.pixels()) {
        let sa = s[3] as f32 / 255.0 * opacity;
        if sa <= 0.0 {
            continue;
        }
        let da = d[3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            continue;
        }
        for c in 0..3 {
            let sc = s[c] as f32 / 255.0;
            let dc = d[c] as f32 / 255.0;
            let blended = blend_channel(mode, sc, dc);
            // Porter-Duff with blend applied to the source contribution.
            let out = blended * sa + dc * da * (1.0 - sa);
            d[c] = (out / out_a * 255.0).clamp(0.0, 255.0) as u8;
        }
        d[3] = (out_a * 255.0) as u8;
    }
}

/// Apply an adjustment layer's values to the accumulated buffer.
fn apply_adjustment(buf: &mut RgbaImage, layer: &Layer) {
    let vals = layer.adjustment_values.clone().unwrap_or_default();
    let v = |k: &str, def: f32| vals.get(k).copied().unwrap_or(def);
    match layer.adjustment_type {
        Some(AdjustmentType::BrightnessContrast) => {
            let b = v("brightness", 0.0) * 2.55;
            let c = 1.0 + v("contrast", 0.0) / 100.0;
            for p in buf.pixels_mut() {
                for ch in 0..3 {
                    p[ch] = ((p[ch] as f32 - 128.0) * c + 128.0 + b).clamp(0.0, 255.0) as u8;
                }
            }
        }
        Some(AdjustmentType::Invert) => {
            for p in buf.pixels_mut() {
                for ch in 0..3 {
                    p[ch] = 255 - p[ch];
                }
            }
        }
        Some(AdjustmentType::Levels) => {
            let black = v("black", 0.0) * 2.55;
            let white = 255.0 - v("white", 0.0) * 2.55;
            let range = (white - black).max(1.0);
            for p in buf.pixels_mut() {
                for ch in 0..3 {
                    p[ch] = (((p[ch] as f32 - black) / range) * 255.0).clamp(0.0, 255.0) as u8;
                }
            }
        }
        Some(AdjustmentType::Hsl) => {
            let sat = 1.0 + v("saturation", 0.0) / 100.0;
            let light = v("lightness", 0.0) * 2.55;
            let hue = v("hue", 0.0) * std::f32::consts::PI / 180.0;
            let (cos_h, sin_h) = (hue.cos(), hue.sin());
            for p in buf.pixels_mut() {
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                let l = 0.299 * r + 0.587 * g + 0.114 * b;
                // Saturation first
                let mut nr = l + (r - l) * sat;
                let mut ng = l + (g - l) * sat;
                let mut nb = l + (b - l) * sat;
                // Cheap hue rotate in RGB space (approximate, luminance-preserving).
                nr += (ng - nb) * sin_h * 0.5;
                ng += (nb - nr) * sin_h * 0.5;
                nb += (nr - ng) * sin_h * 0.5;
                let _ = cos_h;
                p[0] = (nr + light).clamp(0.0, 255.0) as u8;
                p[1] = (ng + light).clamp(0.0, 255.0) as u8;
                p[2] = (nb + light).clamp(0.0, 255.0) as u8;
            }
        }
        Some(AdjustmentType::ColorBalance) => {
            let (dr, dg, db) = (v("red", 0.0), v("green", 0.0), v("blue", 0.0));
            for p in buf.pixels_mut() {
                p[0] = (p[0] as f32 + dr).clamp(0.0, 255.0) as u8;
                p[1] = (p[1] as f32 + dg).clamp(0.0, 255.0) as u8;
                p[2] = (p[2] as f32 + db).clamp(0.0, 255.0) as u8;
            }
        }
        _ => {}
    }
}

/// Composite all visible layers bottom-to-top into a display buffer.
/// Text layers are skipped (rendered by the UI on top).
pub fn composite(layers: &[Layer], store: &mut RasterStore, w: u32, h: u32) -> RgbaImage {
    let mut acc = RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]));
    for layer in layers {
        if !layer.visible {
            continue;
        }
        match layer.layer_type {
            crate::model::LayerType::Raster | crate::model::LayerType::Frame => {
                let buf = store.ensure(layer, w, h).clone();
                composite_layer(&mut acc, &buf, layer.opacity / 100.0, layer.blend_mode);
            }
            crate::model::LayerType::Adjustment => {
                apply_adjustment(&mut acc, layer);
            }
            _ => {}
        }
    }
    acc
}
