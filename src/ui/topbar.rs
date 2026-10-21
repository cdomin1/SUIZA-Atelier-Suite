//! Top window bar + contextual option bar — port of `TopNav.tsx`/`OptionBar.tsx`.

use crate::model::{ColorMode, Persona, ToolId};
use crate::state::{AtelierState, Modal, SyncStatus};
use crate::theme::{hex32, theme_by_id, THEMES};
use crate::ui::toolbox::to_hex;
use egui::{RichText, Vec2};

/// Menu row: native-style menus, persona switcher, sync + theme controls.
pub fn top_nav(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let accent = hex32(theme.accent);
    let muted = hex32(theme.text_muted);

    ui.horizontal(|ui| {
        ui.label(RichText::new("SUIZA Atelier").strong().color(accent));
        ui.separator();

        // Menus
        ui.menu_button("File", |ui| {
            if ui.button("New Document…  Ctrl+N").clicked() {
                st.active_modal = Some(Modal::NewDoc);
                ui.close_menu();
            }
            if ui.button("Open…").clicked() {
                crate::io::open_project_dialog(st);
                ui.close_menu();
            }
            if ui.button("Save .szt  Ctrl+S").clicked() {
                crate::io::save_project_dialog(st);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Export Studio…  Ctrl+Shift+E").clicked() {
                st.active_modal = Some(Modal::Export);
                ui.close_menu();
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui.button("Undo  Ctrl+Z").clicked() {
                st.undo();
                ui.close_menu();
            }
            if ui.button("Redo  Ctrl+Shift+Z").clicked() {
                st.redo();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Select All  Ctrl+A").clicked() {
                st.select_all();
                ui.close_menu();
            }
            if ui.button("Deselect  Ctrl+D").clicked() {
                st.deselect_all();
                ui.close_menu();
            }
            if ui.button("Delete Selection").clicked() {
                st.delete_selected();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Keyboard Shortcuts…  Ctrl+K").clicked() {
                st.active_modal = Some(Modal::Shortcuts);
                ui.close_menu();
            }
        });
        ui.menu_button("Object", |ui| {
            if ui.button("Group  Ctrl+G").clicked() {
                st.group_selected();
                ui.close_menu();
            }
            if ui.button("Ungroup  Ctrl+Shift+G").clicked() {
                st.ungroup_selected();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Bring Forward  Ctrl+]").clicked() {
                st.bring_forward();
                ui.close_menu();
            }
            if ui.button("Send Backward  Ctrl+[").clicked() {
                st.send_backward();
                ui.close_menu();
            }
            if ui.button("Bring to Front  Ctrl+Shift+]").clicked() {
                st.bring_to_front();
                ui.close_menu();
            }
            if ui.button("Send to Back  Ctrl+Shift+[").clicked() {
                st.send_to_back();
                ui.close_menu();
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Fit to Screen  Ctrl+0").clicked() {
                st.hint("Fit to Screen (pending canvas size)");
                ui.close_menu();
            }
            if ui.button("Actual Pixels  Ctrl+1").clicked() {
                st.reset_viewport();
                ui.close_menu();
            }
            ui.separator();
            ui.checkbox(&mut st.show_rulers, "Rulers  Ctrl+R");
            ui.checkbox(&mut st.show_guides, "Guides  Ctrl+;");
            ui.checkbox(&mut st.show_grid, "Pixel Grid  Ctrl+'");
            ui.checkbox(&mut st.snapping.enabled, "Snapping  Ctrl+Shift+;");
            ui.checkbox(&mut st.lock_guides, "Lock Guides");
            ui.separator();
            if ui.button("Toggle Studio Dock  Ctrl+\\").clicked() {
                st.dock_collapsed = !st.dock_collapsed;
                ui.close_menu();
            }
        });
        ui.menu_button("Studio", |ui| {
            for p in Persona::ALL {
                if ui
                    .selectable_label(st.persona == p, p.label())
                    .clicked()
                {
                    st.set_persona(p);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("Rust Architecture…").clicked() {
                st.active_modal = Some(Modal::RustArch);
                ui.close_menu();
            }
            if ui.button("Plugins…").clicked() {
                st.active_modal = Some(Modal::Plugins);
                ui.close_menu();
            }
            if ui.button("Cloud Sync…").clicked() {
                st.active_modal = Some(Modal::CloudSync);
                ui.close_menu();
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Dock collapse
            if ui
                .button(if st.dock_collapsed { "◀" } else { "▶" })
                .on_hover_text("Toggle Studio Dock (Ctrl+\\)")
                .clicked()
            {
                st.dock_collapsed = !st.dock_collapsed;
            }

            // Theme picker
            let current = theme_by_id(st.theme_id).name;
            egui::ComboBox::from_id_salt("theme-picker")
                .selected_text(RichText::new(current).size(11.0))
                .show_ui(ui, |ui| {
                    for t in THEMES {
                        if ui.selectable_label(st.theme_id == t.id, t.name).clicked() {
                            st.theme_id = t.id;
                        }
                    }
                });

            // Sync status
            let (label, color) = match st.sync_status {
                SyncStatus::Synced => ("Synced", hex32("#10b981")),
                SyncStatus::Syncing => ("Syncing…", hex32("#f59e0b")),
                SyncStatus::Offline => ("Offline", hex32("#ef4444")),
                SyncStatus::SavedLocal => ("Saved Local", hex32("#38bdf8")),
            };
            ui.label(RichText::new(label).size(11.0).color(color));

            // Persona switcher
            for p in Persona::ALL {
                let short = match p {
                    Persona::Photo => "PHOTO",
                    Persona::Vector => "VECTOR",
                    Persona::Publish => "PUBLISH",
                };
                let active = st.persona == p;
                let text = if active {
                    RichText::new(short).color(egui::Color32::WHITE).strong().size(11.0)
                } else {
                    RichText::new(short).color(muted).size(11.0)
                };
                let btn = egui::Button::new(text).min_size(Vec2::new(56.0, 20.0));
                let btn = if active { btn.fill(accent) } else { btn };
                if ui.add(btn).on_hover_text(p.label()).clicked() {
                    st.set_persona(p);
                }
            }
        });
    });
}

/// Option bar — contextual controls for the active tool.
pub fn option_bar(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);
    let accent = hex32(theme.accent);

    ui.horizontal(|ui| {
        let o = &mut st.tool_options;
        match st.active_tool {
            ToolId::Brush | ToolId::Eraser | ToolId::Healing | ToolId::Dodge | ToolId::Burn => {
                ui.label(RichText::new("Size").size(11.0).color(muted));
                ui.add_sized([100.0, 18.0], egui::Slider::new(&mut o.brush_size, 1.0..=500.0).logarithmic(true));
                ui.label(RichText::new("Hardness").size(11.0).color(muted));
                ui.add_sized([80.0, 18.0], egui::Slider::new(&mut o.brush_hardness, 0.0..=100.0).suffix("%"));
                ui.label(RichText::new("Opacity").size(11.0).color(muted));
                ui.add_sized([80.0, 18.0], egui::Slider::new(&mut o.brush_opacity, 0.0..=100.0).suffix("%"));
                ui.label(RichText::new("Flow").size(11.0).color(muted));
                ui.add_sized([80.0, 18.0], egui::Slider::new(&mut o.brush_flow, 0.0..=100.0).suffix("%"));
            }
            ToolId::Pen | ToolId::ShapeRect | ToolId::ShapeEllipse | ToolId::ShapePolygon
            | ToolId::FrameShape | ToolId::LineTool => {
                ui.label(RichText::new("Stroke").size(11.0).color(muted));
                let mut sc = hex32(&o.stroke_color);
                if ui.color_edit_button_srgba(&mut sc).changed() {
                    o.stroke_color = to_hex(sc);
                }
                ui.label(RichText::new("Width").size(11.0).color(muted));
                ui.add_sized([80.0, 18.0], egui::Slider::new(&mut o.stroke_width, 0.0..=48.0).suffix("px"));
                ui.label(RichText::new("Fill").size(11.0).color(muted));
                let mut fc = hex32(&o.fill_color);
                if ui.color_edit_button_srgba(&mut fc).changed() {
                    o.fill_color = to_hex(fc);
                }
                if st.persona == Persona::Vector {
                    ui.label(RichText::new("Corner").size(11.0).color(muted));
                    ui.add_sized([70.0, 18.0], egui::Slider::new(&mut st.vector_corner_radius, 0.0..=100.0).suffix("px"));
                }
            }
            ToolId::Text | ToolId::FrameText => {
                ui.label(RichText::new("Font").size(11.0).color(muted));
                ui.add(egui::TextEdit::singleline(&mut o.font_family).desired_width(140.0));
                ui.label(RichText::new("Size").size(11.0).color(muted));
                ui.add_sized([90.0, 18.0], egui::Slider::new(&mut o.font_size, 6.0..=400.0).logarithmic(true).suffix("pt"));
                ui.label(RichText::new("Color").size(11.0).color(muted));
                let mut tc = hex32(&o.text_color);
                if ui.color_edit_button_srgba(&mut tc).changed() {
                    o.text_color = to_hex(tc);
                }
                if st.persona == Persona::Publish {
                    ui.label(RichText::new("Cols").size(11.0).color(muted));
                    ui.add(egui::DragValue::new(&mut st.publish_column_count).range(1..=12));
                    ui.label(RichText::new("Gutter").size(11.0).color(muted));
                    ui.add(egui::DragValue::new(&mut st.publish_column_gutter).range(0.0..=120.0).suffix("px"));
                }
            }
            ToolId::Zoom => {
                ui.label(RichText::new("Zoom").size(11.0).color(muted));
                ui.add_sized([120.0, 18.0], egui::Slider::new(&mut st.zoom, 0.05..=16.0).logarithmic(true));
                if ui.button("Fit").clicked() {
                    st.hint("Fit to Screen");
                }
                if ui.button("100%").clicked() {
                    st.zoom = 1.0;
                }
            }
            _ => {
                ui.label(
                    RichText::new(format!("{:?} — canvas options", st.active_tool))
                        .size(11.0)
                        .color(muted),
                );
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Snapping indicator + doc color mode
            let mode = match st.project.color_mode {
                ColorMode::Rgb => "RGB/8",
                ColorMode::Cmyk => "CMYK/8",
                ColorMode::Grayscale => "GRAY/8",
            };
            ui.label(RichText::new(format!("{} · {}dpi", mode, st.project.dpi)).size(11.0).color(muted));
            if st.snapping.enabled {
                ui.label(RichText::new("SNAP").size(10.0).monospace().color(accent));
            }
        });
    });
}
