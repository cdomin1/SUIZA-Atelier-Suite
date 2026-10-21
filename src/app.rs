//! eframe App — panels layout, shortcut dispatch, shortcut HUD.

use crate::model::{LayerType, Persona, ToolId};
use crate::shortcuts;
use crate::state::{AtelierState, Modal};
use crate::theme::{apply_theme, hex32, theme_by_id};
use crate::ui;
use egui::{Align2, Color32, RichText};

pub struct AtelierApp {
    pub st: AtelierState,
    raster: ui::canvas::RasterCache,
    last_canvas_size: egui::Vec2,
}

impl AtelierApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let st = AtelierState::new();
        apply_theme(&cc.egui_ctx, theme_by_id(st.theme_id));
        Self {
            st,
            raster: ui::canvas::RasterCache::default(),
            last_canvas_size: egui::vec2(1200.0, 800.0),
        }
    }

    fn dispatch_shortcut(&mut self, id: &str) {
        let st = &mut self.st;
        match id {
            // Tools
            "tool-move" => st.active_tool = ToolId::Move,
            "tool-direct" => st.active_tool = ToolId::DirectSelect,
            "tool-marquee" => st.active_tool = ToolId::Marquee,
            "tool-lasso" => st.active_tool = ToolId::Lasso,
            "tool-crop" => st.active_tool = ToolId::Crop,
            "tool-eyedropper" => st.active_tool = ToolId::Eyedropper,
            "tool-healing" => st.active_tool = ToolId::Healing,
            "tool-brush" => st.active_tool = ToolId::Brush,
            "tool-eraser" => st.active_tool = ToolId::Eraser,
            "tool-bucket" => st.active_tool = ToolId::Bucket,
            "tool-pen" => st.active_tool = ToolId::Pen,
            "tool-text" => st.active_tool = ToolId::Text,
            "tool-hand" => st.active_tool = ToolId::Hand,
            "tool-zoom" => st.active_tool = ToolId::Zoom,
            // File
            "file-new" => st.active_modal = Some(Modal::NewDoc),
            "file-save" => crate::io::save_project_dialog(st),
            "file-export" => st.active_modal = Some(Modal::Export),
            // Edit
            "edit-undo" => st.undo(),
            "edit-redo" => st.redo(),
            "edit-selectall" => st.select_all(),
            "edit-deselect" => st.deselect_all(),
            "edit-delete" => st.delete_selected(),
            "edit-shortcuts" => st.active_modal = Some(Modal::Shortcuts),
            "edit-cut" | "edit-copy" | "edit-paste" | "edit-transform" => {
                st.hint("Clipboard ops arrive with the compositor pass")
            }
            // Object / Layer
            "object-group" => st.group_selected(),
            "object-ungroup" => st.ungroup_selected(),
            "layer-bringforward" => st.bring_forward(),
            "layer-sendbackward" => st.send_backward(),
            "layer-bringtofront" => st.bring_to_front(),
            "layer-sendtoback" => st.send_to_back(),
            "layer-new" => {
                let t = if st.persona == Persona::Vector {
                    LayerType::Vector
                } else {
                    LayerType::Raster
                };
                st.add_layer(t, None);
            }
            "layer-duplicate" => {
                let id = st.project.selected_layer_id.clone();
                st.duplicate_layer(&id);
            }
            // View
            "view-zoomin" => st.zoom = (st.zoom * 1.25).min(32.0),
            "view-zoomout" => st.zoom = (st.zoom / 1.25).max(0.02),
            "view-fit" => {
                let (w, h) = (self.last_canvas_size.x, self.last_canvas_size.y);
                st.fit_to_screen(w, h);
            }
            "view-actual" => st.reset_viewport(),
            "view-rulers" => st.show_rulers = !st.show_rulers,
            "view-guides" => st.show_guides = !st.show_guides,
            "view-snap" => {
                st.snapping.enabled = !st.snapping.enabled;
                st.hint(if st.snapping.enabled { "Magnetic Snapping ON" } else { "Snapping OFF" });
            }
            "view-lockguides" => st.lock_guides = !st.lock_guides,
            "view-grid" => st.show_grid = !st.show_grid,
            "window-dock-toggle" => st.dock_collapsed = !st.dock_collapsed,
            // Personas
            "persona-photo" => st.set_persona(Persona::Photo),
            "persona-vector" => st.set_persona(Persona::Vector),
            "persona-publish" => st.set_persona(Persona::Publish),
            // Colors
            "color-reset" => {
                st.reset_colors();
                st.hint("Reset Black & White (D)");
            }
            "color-swap" => {
                st.swap_colors();
                st.hint("Swapped Colors (X)");
            }
            _ => {}
        }
    }
}

impl eframe::App for AtelierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Re-apply theme only when it changes (cheap guard).
        ctx.data_mut(|d| {
            let last = d.get_temp::<crate::model::ThemeId>(egui::Id::new("theme"));
            if last != Some(self.st.theme_id) {
                apply_theme(ctx, theme_by_id(self.st.theme_id));
                d.insert_temp(egui::Id::new("theme"), self.st.theme_id);
            }
        });

        // Keyboard shortcuts (skip while typing in a text field).
        if !ctx.wants_keyboard_input() {
            let hits = ctx.input(|i| shortcuts::triggered(i, &self.st.shortcuts));
            for id in hits {
                self.dispatch_shortcut(&id);
            }
        }

        // Top bars.
        egui::TopBottomPanel::top("top-nav")
            .exact_height(30.0)
            .show(ctx, |ui| ui::topbar::top_nav(ui, &mut self.st));
        egui::TopBottomPanel::top("option-bar")
            .exact_height(34.0)
            .show(ctx, |ui| ui::topbar::option_bar(ui, &mut self.st));

        // Tool palette.
        egui::SidePanel::left("toolbox")
            .exact_width(48.0)
            .show(ctx, |ui| ui::toolbox::toolbox(ui, &mut self.st));

        // Inspector dock.
        if !self.st.dock_collapsed {
            egui::SidePanel::right("dock")
                .resizable(true)
                .default_width(280.0)
                .width_range(220.0..=480.0)
                .show(ctx, |ui| ui::dock::dock(ui, &mut self.st));
        }

        // Canvas.
        egui::CentralPanel::default().show(ctx, |ui| {
            self.last_canvas_size = ui.available_size();
            ui::canvas::canvas(ui, &mut self.st, &mut self.raster);
        });

        // Modals.
        ui::modals::show_modals(ctx, &mut self.st);

        // Shortcut confirmation HUD (1200 ms, port of the web pill).
        if let Some(label) = self.st.hint_active() {
            let accent = hex32(theme_by_id(self.st.theme_id).accent);
            egui::Area::new(egui::Id::new("shortcut-hud"))
                .anchor(Align2::CENTER_BOTTOM, egui::vec2(0.0, -48.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(Color32::from_black_alpha(220))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("⌘ {label}"))
                                    .monospace()
                                    .size(12.0)
                                    .color(accent),
                            );
                        });
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        } else if self.st.shortcut_hint.is_some() {
            self.st.shortcut_hint = None;
        }
    }
}
