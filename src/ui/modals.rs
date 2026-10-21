//! Modal windows — port of the `components/modals/*` set.

use crate::model::{now_millis, ColorMode};
use crate::presets::{self, DocPreset};
use crate::state::{AtelierState, Modal};
use crate::theme::{hex32, theme_by_id};
use egui::RichText;

pub fn show_modals(ctx: &egui::Context, st: &mut AtelierState) {
    match st.active_modal {
        Some(Modal::Export) => export_modal(ctx, st),
        Some(Modal::NewDoc) => new_doc_modal(ctx, st),
        Some(Modal::Shortcuts) => shortcuts_modal(ctx, st),
        Some(Modal::Plugins) => plugins_modal(ctx, st),
        Some(Modal::RustArch) => rust_arch_modal(ctx, st),
        Some(Modal::CloudSync) => cloud_sync_modal(ctx, st),
        Some(Modal::LinuxNative) => linux_native_modal(ctx, st),
        None => {}
    }
}

// ---- Export ----------------------------------------------------------------------

fn export_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let accent = hex32(theme.accent);
    let mut open = true;

    egui::Window::new("Export Studio — High-Resolution Renderer")
        .open(&mut open)
        .collapsible(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            ui.label(RichText::new("Target file format").size(11.0));
            ui.horizontal_wrapped(|ui| {
                for fmt in ["png", "jpg", "webp", "svg", "pdf", "szt", "tiff"] {
                    if ui.selectable_label(st.export.format == fmt, fmt.to_uppercase()).clicked() {
                        st.export.format = fmt.into();
                    }
                }
            });
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Resolution multiplier");
                for s in [1.0f32, 2.0, 4.0, 8.0] {
                    if ui.selectable_label(st.export.scale == s, format!("{s}×")).clicked() {
                        st.export.scale = s;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Target print DPI");
                egui::ComboBox::from_id_salt("dpi")
                    .selected_text(format!("{} DPI", st.export.dpi))
                    .show_ui(ui, |ui| {
                        for (d, l) in [(72u32, "72 — Web"), (150, "150 — Proof"), (300, "300 — Offset"), (600, "600 — Archival")] {
                            ui.selectable_value(&mut st.export.dpi, d, format!("{l}"));
                        }
                    });
            });
            if st.export.format == "jpg" || st.export.format == "webp" {
                ui.horizontal(|ui| {
                    ui.label("Compression quality");
                    ui.add(egui::Slider::new(&mut st.export.quality, 30..=100).suffix("%"));
                });
            }

            ui.separator();
            let ew = (st.project.width * st.export.scale) as u32;
            let eh = (st.project.height * st.export.scale) as u32;
            ui.monospace(format!("Raster: {ew} × {eh} px"));
            ui.monospace(format!(
                "Print: {:.1}\" × {:.1}\" @ {} dpi",
                ew as f32 / st.export.dpi as f32,
                eh as f32 / st.export.dpi as f32,
                st.export.dpi
            ));
            ui.monospace(format!("Profile: {}", st.project.cmyk_profile));
            ui.add_space(4.0);

            match st.export.format.as_str() {
                "png" | "jpg" => {}
                "szt" => {}
                other => {
                    ui.label(
                        RichText::new(format!("{other} export is not implemented yet — PNG/JPEG raster and .szt document export are live."))
                            .size(10.0)
                            .color(hex32("#f59e0b")),
                    );
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let supported = matches!(st.export.format.as_str(), "png" | "jpg" | "szt");
                if ui
                    .add_enabled(supported, egui::Button::new(RichText::new("Export File").color(egui::Color32::WHITE)).fill(accent))
                    .clicked()
                {
                    if st.export.format == "szt" {
                        crate::io::save_project_dialog(st);
                    } else {
                        let fmt = st.export.format.clone();
                        let scale = st.export.scale;
                        crate::io::export_image(st, &fmt, scale);
                    }
                    st.active_modal = None;
                }
                if ui.button("Cancel").clicked() {
                    st.active_modal = None;
                }
            });
        });
    if !open {
        st.active_modal = None;
    }
}

// ---- New Document -----------------------------------------------------------------

fn new_doc_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("New Document")
        .open(&mut open)
        .collapsible(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            for cat in presets::DOCUMENT_PRESETS {
                ui.label(RichText::new(cat.category).strong());
                ui.horizontal_wrapped(|ui| {
                    for preset in cat.items {
                        if ui.button(preset.name).clicked() {
                            create_from_preset(st, preset);
                            st.active_modal = None;
                        }
                    }
                });
                ui.add_space(6.0);
            }
            ui.separator();
            ui.label(RichText::new("Custom").strong());
            ui.horizontal(|ui| {
                ui.label("Size");
                ui.add(
                    egui::DragValue::new(&mut st.new_doc.width)
                        .range(1.0..=20000.0)
                        .suffix(" px"),
                );
                ui.label("×");
                ui.add(
                    egui::DragValue::new(&mut st.new_doc.height)
                        .range(1.0..=20000.0)
                        .suffix(" px"),
                );
                ui.label("DPI");
                ui.add(egui::DragValue::new(&mut st.new_doc.dpi).range(72..=1200));
            });
            if ui.button("Create Custom Document").clicked() {
                let p = DocPreset {
                    name: "Custom",
                    width: st.new_doc.width,
                    height: st.new_doc.height,
                    dpi: st.new_doc.dpi,
                    color_mode: ColorMode::Rgb,
                    persona: st.persona,
                    facing_pages: st.persona == crate::model::Persona::Publish,
                    page_count: if st.persona == crate::model::Persona::Publish {
                        4
                    } else {
                        1
                    },
                };
                create_from_preset(st, &p);
                st.active_modal = None;
            }
        });
    if !open {
        st.active_modal = None;
    }
}

fn create_from_preset(st: &mut AtelierState, p: &DocPreset) {
    let mut project = crate::state::create_initial_project(p.persona);
    project.width = p.width;
    project.height = p.height;
    project.dpi = p.dpi;
    project.color_mode = p.color_mode;
    project.facing_pages = p.facing_pages;
    project.page_count = p.page_count;
    project.name = format!("{}.szt", p.name.replace(' ', "-"));
    project.created_at = now_millis();
    project.updated_at = now_millis();
    // Resize the base layer to the new canvas.
    if let Some(base) = project.layers.first_mut() {
        base.width = Some(p.width);
        base.height = Some(p.height);
    }
    st.project = project;
    st.persona = p.persona;
    st.push_history(format!("New Document: {}", p.name));
    st.hint(format!("Created {}", p.name));
}

// ---- Shortcuts ---------------------------------------------------------------------

fn shortcuts_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("Keyboard Shortcuts")
        .open(&mut open)
        .default_width(520.0)
        .show(ctx, |ui| {
            let theme = *theme_by_id(st.theme_id);
            let muted = hex32(theme.text_muted);
            let mut last_cat: Option<String> = None;
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    for sc in &st.shortcuts {
                        let cat = sc.category.label().to_string();
                        if last_cat.as_deref() != Some(&cat) {
                            ui.label(RichText::new(&cat).strong());
                            last_cat = Some(cat);
                        }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&sc.name).size(11.0));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut combo = String::new();
                                    if sc.ctrl {
                                        combo.push_str("Ctrl+");
                                    }
                                    if sc.shift {
                                        combo.push_str("Shift+");
                                    }
                                    if sc.alt {
                                        combo.push_str("Alt+");
                                    }
                                    combo.push_str(&sc.key.to_uppercase());
                                    ui.label(
                                        RichText::new(combo).size(11.0).monospace().color(muted),
                                    );
                                },
                            );
                        });
                    }
                });
            ui.separator();
            ui.label(
                RichText::new("Remapping editor is on the roadmap — defaults are active.")
                    .size(10.0)
                    .color(muted),
            );
        });
    if !open {
        st.active_modal = None;
    }
}

// ---- Plugins ------------------------------------------------------------------------

fn plugins_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("Plugin Registry — Memory-Safe Extensions")
        .open(&mut open)
        .default_width(560.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    let ids: Vec<String> = st.plugins.iter().map(|p| p.id.clone()).collect();
                    for id in ids {
                        let i = st.plugins.iter().position(|p| p.id == id).unwrap();
                        let p = st.plugins[i].clone();
                        ui.horizontal(|ui| {
                            let mut enabled = p.enabled;
                            if ui.checkbox(&mut enabled, "").changed() {
                                st.toggle_plugin(&p.id);
                            }
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&p.name).strong().size(11.0));
                                    ui.label(
                                        RichText::new(format!("v{}", p.version))
                                            .size(10.0)
                                            .monospace(),
                                    );
                                    if p.memory_safety_verified {
                                        ui.label(
                                            RichText::new("SAFE")
                                                .size(9.0)
                                                .monospace()
                                                .color(hex32("#10b981")),
                                        );
                                    }
                                });
                                ui.label(
                                    RichText::new(format!("{} · {:?}", p.author, p.plugin_type))
                                        .size(10.0),
                                );
                                ui.label(RichText::new(&p.description).size(10.0));
                            });
                        });
                        ui.separator();
                    }
                });
        });
    if !open {
        st.active_modal = None;
    }
}

// ---- Info modals ------------------------------------------------------------------

fn rust_arch_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("Rust Core Architecture")
        .open(&mut open)
        .default_width(440.0)
        .show(ctx, |ui| {
            ui.label("SUIZA Atelier Suite runs on a native Rust core — no browser, no GC:");
            ui.add_space(6.0);
            for line in [
                "egui/eframe immediate-mode GPU UI (wgpu/glow backends)",
                "Ownership & borrow-checker memory safety, zero leaks",
                "Serde-typed .szt document model with non-destructive history",
                "Cross-platform: macOS (AppKit), Linux (GTK/Wayland), Windows (WinUI)",
                "Compositor + raster engine port in progress (tiny-skia planned)",
            ] {
                ui.label(format!("· {line}"));
            }
            ui.add_space(6.0);
            ui.monospace(format!("suiza-atelier-suite {}", env!("CARGO_PKG_VERSION")));
        });
    if !open {
        st.active_modal = None;
    }
}

fn cloud_sync_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("Cloud Sync & Backup")
        .open(&mut open)
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.label("Local-first storage — documents save to .szt files on disk.");
            ui.label("Cloud drive sync and collaborative sessions are on the roadmap.");
            ui.separator();
            ui.monospace(format!("Last saved: {}", st.last_saved_at));
        });
    if !open {
        st.active_modal = None;
    }
}

fn linux_native_modal(ctx: &egui::Context, st: &mut AtelierState) {
    let mut open = true;
    egui::Window::new("Linux Native Integration")
        .open(&mut open)
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.label("GTK4/Wayland bridge parity notes — this Rust build already runs natively on X11 and Wayland via winit.");
        });
    if !open {
        st.active_modal = None;
    }
}
