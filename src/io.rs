//! File IO — `.szt` document save/load and raster export.

use crate::model::DocumentProject;
use crate::state::AtelierState;
use std::path::PathBuf;

/// Serialize the active project to `.szt` (JSON envelope, mirrors the web
/// exporter's shape).
pub fn save_project_dialog(st: &mut AtelierState) {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&st.project.name)
        .add_filter("SUIZA Atelier Document", &["szt"])
        .save_file()
    else {
        return;
    };
    if let Err(e) = save_project(st, &path) {
        st.hint(format!("Save failed: {e}"));
    } else {
        st.mark_saved();
        st.hint(format!("Saved {}", path.display()));
    }
}

pub fn save_project(st: &AtelierState, path: &PathBuf) -> std::io::Result<()> {
    let envelope = serde_json::json!({
        "app": "SUIZA Atelier",
        "version": env!("CARGO_PKG_VERSION"),
        "format": "szt",
        "project": st.project,
        "vectorShapes": st.vector_shapes,
        "publishFrames": st.publish_frames,
        "guides": st.guides,
        "timestamp": crate::model::now_millis(),
    });
    let mut p = path.clone();
    if p.extension().is_none() {
        p.set_extension("szt");
    }
    std::fs::write(&p, serde_json::to_string_pretty(&envelope)?)
}

pub fn open_project_dialog(st: &mut AtelierState) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("SUIZA Atelier Document", &["szt"])
        .pick_file()
    else {
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => {
                // Accept both the raw project and the exported envelope shape.
                let proj_val = v.get("project").cloned().unwrap_or(v.clone());
                match serde_json::from_value::<DocumentProject>(proj_val) {
                    Ok(project) => {
                        if let Some(vs) = v.get("vectorShapes").cloned() {
                            if let Ok(shapes) = serde_json::from_value(vs) {
                                st.vector_shapes = shapes;
                            }
                        }
                        if let Some(pf) = v.get("publishFrames").cloned() {
                            if let Ok(frames) = serde_json::from_value(pf) {
                                st.publish_frames = frames;
                            }
                        }
                        if let Some(g) = v.get("guides").cloned() {
                            if let Ok(guides) = serde_json::from_value(g) {
                                st.guides = guides;
                            }
                        }
                        let persona = project.persona;
                        st.project = project;
                        st.persona = persona;
                        st.push_history("Open Document");
                        st.hint(format!("Opened {}", path.display()));
                    }
                    Err(e) => st.hint(format!("Invalid .szt project: {e}")),
                }
            }
            Err(e) => st.hint(format!("Could not parse file: {e}")),
        },
        Err(e) => st.hint(format!("Could not read file: {e}")),
    }
}

/// Flatten visible raster/text layers into an RGBA buffer for export.
/// Vector compositing arrives with the render engine; text is skipped for now.
pub fn composite_raster(st: &AtelierState, scale: f32) -> image::RgbaImage {
    let w = (st.project.width * scale).round().max(1.0) as u32;
    let h = (st.project.height * scale).round().max(1.0) as u32;
    let mut buf = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]));
    for layer in &st.project.layers {
        if !layer.visible {
            continue;
        }
        let Some(png) = &layer.raster_png else { continue };
        if let Ok(img) = image::load_from_memory(png) {
            let mut rgba = img.to_rgba8();
            if scale != 1.0 {
                rgba = image::imageops::resize(
                    &rgba,
                    (rgba.width() as f32 * scale) as u32,
                    (rgba.height() as f32 * scale) as u32,
                    image::imageops::FilterType::Lanczos3,
                );
            }
            let alpha = (layer.opacity.clamp(0.0, 100.0) / 100.0 * 255.0) as u8;
            for px in rgba.pixels_mut() {
                px[3] = px[3].saturating_mul(alpha) / 255;
            }
            image::imageops::overlay(&mut buf, &rgba, 0, 0);
        }
    }
    buf
}

pub fn export_image(st: &mut AtelierState, format: &str, scale: f32) {
    let stem = st
        .project
        .name
        .trim_end_matches(".szt")
        .to_string();
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{stem}@{scale}x.{format}"))
        .add_filter(format.to_uppercase(), &[format])
        .save_file()
    else {
        return;
    };
    let img = composite_raster(st, scale);
    let dynimg = image::DynamicImage::ImageRgba8(img);
    let res = match format {
        "jpg" | "jpeg" => dynimg.save_with_format(&path, image::ImageFormat::Jpeg),
        _ => dynimg.save_with_format(&path, image::ImageFormat::Png),
    };
    match res {
        Ok(()) => {
            st.mark_saved();
            st.hint(format!("Exported {}", path.display()));
        }
        Err(e) => st.hint(format!("Export failed: {e}")),
    }
}
