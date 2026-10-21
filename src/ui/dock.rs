//! Right inspector dock — tab strip + persona-specific panels.
//! Port of `DockingPanel.tsx` and the panel components.

use crate::model::*;
use crate::state::{AlignOp, AtelierState, PathfinderOp};
use crate::theme::{hex32, theme_by_id};
use crate::ui::toolbox::to_hex;
use egui::{Color32, RichText};

pub fn dock(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    // Tab strip
    ui.horizontal_wrapped(|ui| {
        for tab in DockTab::for_persona(st.persona) {
            let selected = st.active_dock_tab == *tab;
            if ui
                .selectable_label(selected, RichText::new(tab.label()).size(11.0))
                .clicked()
            {
                st.active_dock_tab = *tab;
            }
        }
    });
    ui.separator();
    let _ = muted;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match st.active_dock_tab {
            DockTab::Layers => layers_panel(ui, st),
            DockTab::Typography => typography_panel(ui, st),
            DockTab::Swatches => swatches_panel(ui, st),
            DockTab::Adjustments => adjustments_panel(ui, st),
            DockTab::Properties => properties_panel(ui, st),
            DockTab::History => history_panel(ui, st),
            DockTab::Ai => ai_panel(ui, st),
            DockTab::Stroke => stroke_panel(ui, st),
            DockTab::Pathfinder => pathfinder_panel(ui, st),
            DockTab::Pages => pages_panel(ui, st),
            DockTab::Wrap => wrap_panel(ui, st),
            DockTab::Preflight => preflight_panel(ui, st),
        });
}

// ---- Layers -------------------------------------------------------------------

fn layers_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.horizontal(|ui| {
        if ui.button("+ Layer").clicked() {
            let t = match st.persona {
                Persona::Vector => LayerType::Vector,
                _ => LayerType::Raster,
            };
            st.add_layer(t, None);
        }
        if ui.button("+ Text").clicked() {
            st.add_layer(LayerType::Text, None);
        }
        if ui.button("+ Group").clicked() {
            st.create_layer_group(None);
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Duplicate").clicked() {
            let id = st.project.selected_layer_id.clone();
            st.duplicate_layer(&id);
        }
        if ui.button("Merge Down").clicked() {
            let id = st.project.selected_layer_id.clone();
            st.merge_layer_down(&id);
        }
        if ui.button("Delete").clicked() {
            let id = st.project.selected_layer_id.clone();
            st.delete_layer(&id);
        }
    });
    ui.separator();

    // Render top-down (last layer = top of stack, like the web panel).
    let count = st.project.layers.len();
    for i in (0..count).rev() {
        let layer = st.project.layers[i].clone();
        let selected = st.project.selected_layer_id == layer.id;

        ui.horizontal(|ui| {
            // visibility + lock
            let eye = if layer.visible { "●" } else { "○" };
            if ui
                .selectable_label(
                    false,
                    RichText::new(eye).size(11.0).color(if layer.visible {
                        hex32(theme.accent)
                    } else {
                        muted
                    }),
                )
                .on_hover_text("Toggle visibility")
                .clicked()
            {
                st.toggle_layer_visibility(&layer.id);
            }
            let lock = if layer.locked { "L" } else { "·" };
            if ui
                .selectable_label(false, RichText::new(lock).size(11.0).color(muted))
                .on_hover_text("Toggle lock")
                .clicked()
            {
                st.toggle_layer_lock(&layer.id);
            }

            let kind = match layer.layer_type {
                LayerType::Raster => "PX",
                LayerType::Vector => "VC",
                LayerType::Text => "TX",
                LayerType::Adjustment => "ADJ",
                LayerType::Frame => "FR",
                LayerType::Group => "GRP",
            };
            let label = format!("{kind}  {}", layer.name);
            let resp = ui.selectable_label(selected, RichText::new(label).size(11.0));
            if resp.clicked() {
                st.project.selected_layer_id = layer.id.clone();
            }
            resp.context_menu(|ui| {
                if ui.button("Duplicate").clicked() {
                    st.duplicate_layer(&layer.id);
                    ui.close_menu();
                }
                if ui.button("Merge Down").clicked() {
                    st.merge_layer_down(&layer.id);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete Layer").clicked() {
                    st.delete_layer(&layer.id);
                    ui.close_menu();
                }
            });
        });

        // Selected layer inline controls
        if selected {
            ui.indent("layer-ctl", |ui| {
                let mut opacity = layer.opacity;
                if ui
                    .add_sized(
                        [160.0, 18.0],
                        egui::Slider::new(&mut opacity, 0.0..=100.0).suffix("%"),
                    )
                    .changed()
                {
                    st.set_layer_opacity(&layer.id, opacity);
                }
                let mut mode = layer.blend_mode;
                egui::ComboBox::from_id_salt("blend-mode")
                    .selected_text(mode.label())
                    .show_ui(ui, |ui| {
                        for m in BlendMode::ALL {
                            ui.selectable_value(&mut mode, m, m.label());
                        }
                    });
                if mode != layer.blend_mode {
                    st.set_layer_blend_mode(&layer.id, mode);
                }
                ui.horizontal(|ui| {
                    if ui
                        .small_button("▲")
                        .on_hover_text("Bring Forward")
                        .clicked()
                        && i + 1 < count
                    {
                        st.move_layer(i, i + 1);
                    }
                    if ui
                        .small_button("▼")
                        .on_hover_text("Send Backward")
                        .clicked()
                        && i > 0
                    {
                        st.move_layer(i, i - 1);
                    }
                });
            });
        }
    }
}

// ---- Typography -----------------------------------------------------------------

fn typography_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    let is_text = st
        .selected_layer()
        .map(|l| l.layer_type == LayerType::Text)
        .unwrap_or(false);

    if !is_text {
        ui.label(
            RichText::new("Select a text layer to edit typography.")
                .size(11.0)
                .color(muted),
        );
        return;
    }

    // Clone the layer we edit, then write back — avoids borrow conflicts.
    let mut layer = st.selected_layer().unwrap().clone();
    let mut changed = false;

    ui.label(RichText::new("Content").size(11.0).color(muted));
    let mut content = layer.text_content.clone().unwrap_or_default();
    if ui
        .add(egui::TextEdit::multiline(&mut content).desired_rows(3))
        .changed()
    {
        layer.text_content = Some(content);
        changed = true;
    }

    ui.label(RichText::new("Family").size(11.0).color(muted));
    let mut fam = layer
        .text_font_family
        .clone()
        .unwrap_or_else(|| "Plus Jakarta Sans".into());
    if ui.add(egui::TextEdit::singleline(&mut fam)).changed() {
        layer.text_font_family = Some(fam);
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Size").size(11.0).color(muted));
        let mut size = layer.text_font_size.unwrap_or(32.0);
        if ui
            .add(
                egui::DragValue::new(&mut size)
                    .range(1.0..=800.0)
                    .suffix("pt"),
            )
            .changed()
        {
            layer.text_font_size = Some(size);
            changed = true;
        }
        ui.label(RichText::new("Tracking").size(11.0).color(muted));
        let mut ls = layer.text_letter_spacing.unwrap_or(0.0);
        if ui
            .add(
                egui::DragValue::new(&mut ls)
                    .range(-20.0..=100.0)
                    .speed(0.1),
            )
            .changed()
        {
            layer.text_letter_spacing = Some(ls);
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Leading").size(11.0).color(muted));
        let mut lh = layer.text_line_height.unwrap_or(1.2);
        if ui
            .add(egui::DragValue::new(&mut lh).range(0.5..=4.0).speed(0.01))
            .changed()
        {
            layer.text_line_height = Some(lh);
            changed = true;
        }
        ui.label(RichText::new("Color").size(11.0).color(muted));
        let mut c = hex32(layer.text_color.as_deref().unwrap_or("#ffffff"));
        if ui.color_edit_button_srgba(&mut c).changed() {
            layer.text_color = Some(to_hex(c));
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        let mut align = layer.text_align.unwrap_or(TextAlign::Left);
        for (a, label) in [
            (TextAlign::Left, "L"),
            (TextAlign::Center, "C"),
            (TextAlign::Right, "R"),
            (TextAlign::Justify, "J"),
        ] {
            if ui.selectable_label(align == a, label).clicked() {
                align = a;
                layer.text_align = Some(a);
                changed = true;
            }
        }
        ui.separator();
        let mut tt = layer.text_transform.unwrap_or(TextTransform::None);
        for (t, label) in [
            (TextTransform::None, "Aa"),
            (TextTransform::Uppercase, "AA"),
            (TextTransform::Lowercase, "aa"),
        ] {
            if ui.selectable_label(tt == t, label).clicked() {
                tt = t;
                layer.text_transform = Some(t);
                changed = true;
            }
        }
    });

    if changed {
        let id = layer.id.clone();
        if let Some(target) = st.project.layers.iter_mut().find(|l| l.id == id) {
            *target = layer;
            st.project.updated_at = now_millis();
        }
    }
}

// ---- Swatches -------------------------------------------------------------------

fn swatches_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    if !st.palettes.iter().any(|p| p.id == st.selected_palette_id) {
        st.selected_palette_id = st
            .palettes
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
    }

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("palette")
            .selected_text(
                st.palettes
                    .iter()
                    .find(|p| p.id == st.selected_palette_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
            )
            .show_ui(ui, |ui| {
                for p in &st.palettes {
                    ui.selectable_value(&mut st.selected_palette_id, p.id.clone(), &p.name);
                }
            });
        if ui
            .small_button("+FG")
            .on_hover_text("Save foreground color as a swatch")
            .clicked()
        {
            let fg = st.foreground_color.clone();
            if let Some(p) = st
                .palettes
                .iter_mut()
                .find(|p| p.id == st.selected_palette_id)
            {
                p.swatches.push(ColorSwatch {
                    id: new_id("sw"),
                    name: fg.clone(),
                    color: fg,
                    swatch_type: Some(SwatchKind::Solid),
                    gradient_type: None,
                    gradient_angle: None,
                    gradient_stops: Vec::new(),
                    is_none: false,
                    category: None,
                    is_favorite: false,
                });
            }
            st.hint("Swatch saved");
        }
        if ui
            .small_button("+BG")
            .on_hover_text("Save background color as a swatch")
            .clicked()
        {
            let bg = st.background_color.clone();
            if let Some(p) = st
                .palettes
                .iter_mut()
                .find(|p| p.id == st.selected_palette_id)
            {
                p.swatches.push(ColorSwatch {
                    id: new_id("sw"),
                    name: bg.clone(),
                    color: bg,
                    swatch_type: Some(SwatchKind::Solid),
                    gradient_type: None,
                    gradient_angle: None,
                    gradient_stops: Vec::new(),
                    is_none: false,
                    category: None,
                    is_favorite: false,
                });
            }
            st.hint("Swatch saved");
        }
    });
    ui.add_space(4.0);

    let palette = st
        .palettes
        .iter()
        .find(|p| p.id == st.selected_palette_id)
        .cloned();
    if let Some(palette) = palette {
        if let Some(desc) = &palette.description {
            ui.label(RichText::new(desc).size(10.0).color(muted));
        }
        ui.add_space(4.0);

        let mut remove_id: Option<String> = None;
        ui.horizontal_wrapped(|ui| {
            for sw in &palette.swatches {
                let color = if sw.is_none {
                    Color32::TRANSPARENT
                } else {
                    hex32(&sw.color)
                };
                let (rect, resp) =
                    ui.allocate_exact_size(egui::Vec2::new(26.0, 26.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 3.0, color);
                if sw.is_none {
                    ui.painter().line_segment(
                        [rect.left_bottom(), rect.right_top()],
                        egui::Stroke::new(1.5_f32, Color32::RED),
                    );
                }
                ui.painter().rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0_f32, hex32(theme.border_strong)),
                    egui::StrokeKind::Inside,
                );
                resp.clone().on_hover_text(format!(
                    "{} — click: fill · alt-click: stroke · right-click: remove",
                    sw.name
                ));
                if resp.clicked() {
                    let target = if ui.input(|i| i.modifiers.alt) {
                        "stroke"
                    } else {
                        "fill"
                    };
                    st.apply_swatch(&sw.color, target);
                }
                if resp.secondary_clicked() {
                    remove_id = Some(sw.id.clone());
                }
            }
        });
        if let Some(rid) = remove_id {
            if let Some(p) = st
                .palettes
                .iter_mut()
                .find(|p| p.id == st.selected_palette_id)
            {
                p.swatches.retain(|s| s.id != rid);
            }
            st.hint("Swatch removed");
        }
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new("Click a swatch to apply · Alt+Click applies to stroke")
            .size(10.0)
            .color(muted),
    );
}

// ---- Adjustments ------------------------------------------------------------------

fn adjustments_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.label(
        RichText::new("Non-destructive adjustment layers")
            .size(11.0)
            .color(muted),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for adj in AdjustmentType::ALL {
            if ui.button(adj.label()).clicked() {
                st.add_adjustment_layer(adj);
            }
        }
    });
    ui.separator();

    let selected_adj = st
        .selected_layer()
        .filter(|l| l.layer_type == LayerType::Adjustment)
        .cloned();

    if let Some(layer) = selected_adj {
        ui.label(RichText::new(&layer.name).strong());
        let mut vals = layer.adjustment_values.clone().unwrap_or_default();
        let keys: Vec<String> = vals.keys().cloned().collect();
        let mut changed = false;
        for key in keys {
            let mut v = vals.get(&key).copied().unwrap_or(0.0);
            if ui
                .add(egui::Slider::new(&mut v, -100.0..=100.0).text(&key))
                .changed()
            {
                vals.insert(key, v);
                changed = true;
            }
        }
        if changed {
            if let Some(l) = st.project.layers.iter_mut().find(|l| l.id == layer.id) {
                l.adjustment_values = Some(vals);
                st.project.updated_at = now_millis();
            }
        }
    } else {
        ui.label(
            RichText::new("Select an adjustment layer to tune its values.")
                .size(11.0)
                .color(muted),
        );
    }
}

// ---- Properties --------------------------------------------------------------------

fn properties_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.label(RichText::new("Document").strong());
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").size(11.0).color(muted));
        let mut name = st.project.name.clone();
        if ui.add(egui::TextEdit::singleline(&mut name)).changed() {
            st.update_project_name(name);
        }
    });
    ui.label(
        RichText::new(format!(
            "{} × {} px · {} dpi · {:?}",
            st.project.width as u32,
            st.project.height as u32,
            st.project.dpi,
            st.project.color_mode
        ))
        .size(11.0)
        .color(muted),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new("Color mode").size(11.0).color(muted));
        for (m, l) in [
            (ColorMode::Rgb, "RGB"),
            (ColorMode::Cmyk, "CMYK"),
            (ColorMode::Grayscale, "Gray"),
        ] {
            if ui.selectable_label(st.project.color_mode == m, l).clicked() {
                st.set_color_mode(m);
            }
        }
    });
    ui.separator();

    if let Some(layer) = st.selected_layer().cloned() {
        ui.label(RichText::new("Selected Layer").strong());
        ui.label(
            RichText::new(format!("{:?} · {}", layer.layer_type, layer.name))
                .size(11.0)
                .color(muted),
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new("X").size(11.0).color(muted));
            let mut x = layer.x.unwrap_or(0.0);
            if ui.add(egui::DragValue::new(&mut x).speed(1.0)).changed() {
                if let Some(l) = st.selected_layer_mut() {
                    l.x = Some(x);
                }
            }
            ui.label(RichText::new("Y").size(11.0).color(muted));
            let mut y = layer.y.unwrap_or(0.0);
            if ui.add(egui::DragValue::new(&mut y).speed(1.0)).changed() {
                if let Some(l) = st.selected_layer_mut() {
                    l.y = Some(y);
                }
            }
        });
    }

    // Vector shape properties.
    if st.persona == Persona::Vector && !st.selected_shape_ids.is_empty() {
        ui.separator();
        ui.label(RichText::new("Selected Shape(s)").strong());
        if let Some(shape) = st
            .vector_shapes
            .iter()
            .find(|s| st.selected_shape_ids.contains(&s.id))
            .cloned()
        {
            let sid = shape.id.clone();
            ui.label(
                RichText::new(format!(
                    "{:?} · {}",
                    shape.kind,
                    shape.name.unwrap_or_else(|| sid.clone())
                ))
                .size(11.0)
                .color(muted),
            );
            ui.horizontal(|ui| {
                for (label, field) in [("X", "x"), ("Y", "y"), ("W", "w"), ("H", "h")] {
                    ui.label(RichText::new(label).size(11.0).color(muted));
                    let mut v = match field {
                        "x" => shape.x,
                        "y" => shape.y,
                        "w" => shape.width,
                        _ => shape.height,
                    };
                    if ui.add(egui::DragValue::new(&mut v).speed(1.0)).changed() {
                        if let Some(s) = st.vector_shapes.iter_mut().find(|s| s.id == sid) {
                            match field {
                                "x" => s.x = v,
                                "y" => s.y = v,
                                "w" => s.width = v.max(1.0),
                                _ => s.height = v.max(1.0),
                            }
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fill").size(11.0).color(muted));
                let mut f = hex32(&shape.fill);
                if ui.color_edit_button_srgba(&mut f).changed() {
                    let c = to_hex(f);
                    if let Some(s) = st.vector_shapes.iter_mut().find(|s| s.id == sid) {
                        s.fill = c;
                    }
                }
                ui.label(RichText::new("Stroke").size(11.0).color(muted));
                let mut s = hex32(&shape.stroke);
                if ui.color_edit_button_srgba(&mut s).changed() {
                    let c = to_hex(s);
                    if let Some(sh) = st.vector_shapes.iter_mut().find(|s| s.id == sid) {
                        sh.stroke = c;
                    }
                }
            });
            if shape.kind == ShapeKind::Text {
                ui.label(RichText::new("Text").size(11.0).color(muted));
                let mut txt = shape.text.clone().unwrap_or_default();
                if ui.add(egui::TextEdit::singleline(&mut txt)).changed() {
                    if let Some(s) = st.vector_shapes.iter_mut().find(|s| s.id == sid) {
                        s.text = Some(txt);
                    }
                }
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Group").clicked() {
                st.group_selected();
            }
            if ui.button("Delete").clicked() {
                let ids = st.selected_shape_ids.clone();
                st.vector_shapes.retain(|s| !ids.contains(&s.id));
                st.selected_shape_ids.clear();
                st.push_history("Delete Shape(s)");
            }
        });
    }

    // Publish frame properties.
    if st.persona == Persona::Publish && !st.selected_frame_ids.is_empty() {
        ui.separator();
        ui.label(RichText::new("Selected Frame(s)").strong());
        if let Some(frame) = st
            .publish_frames
            .iter()
            .find(|f| st.selected_frame_ids.contains(&f.id))
            .cloned()
        {
            let fid = frame.id.clone();
            ui.label(
                RichText::new(format!("{:?} frame", frame.kind))
                    .size(11.0)
                    .color(muted),
            );
            ui.horizontal(|ui| {
                for (label, v0) in [
                    ("X", frame.x),
                    ("Y", frame.y),
                    ("W", frame.width),
                    ("H", frame.height),
                ] {
                    ui.label(RichText::new(label).size(11.0).color(muted));
                    let mut v = v0;
                    if ui.add(egui::DragValue::new(&mut v).speed(1.0)).changed() {
                        if let Some(f) = st.publish_frames.iter_mut().find(|f| f.id == fid) {
                            match label {
                                "X" => f.x = v,
                                "Y" => f.y = v,
                                "W" => f.width = v.max(1.0),
                                _ => f.height = v.max(1.0),
                            }
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Title").size(11.0).color(muted));
                let mut t = frame.title.clone().unwrap_or_default();
                if ui.add(egui::TextEdit::singleline(&mut t)).changed() {
                    if let Some(f) = st.publish_frames.iter_mut().find(|f| f.id == fid) {
                        f.title = Some(t);
                    }
                }
            });
            if frame.kind == FrameKind::Text {
                ui.label(RichText::new("Content").size(11.0).color(muted));
                let mut c = frame.content.clone().unwrap_or_default();
                if ui
                    .add(egui::TextEdit::multiline(&mut c).desired_rows(3))
                    .changed()
                {
                    if let Some(f) = st.publish_frames.iter_mut().find(|f| f.id == fid) {
                        f.content = Some(c);
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Size").size(11.0).color(muted));
                    let mut fs = frame.font_size.unwrap_or(14.0);
                    if ui
                        .add(egui::DragValue::new(&mut fs).range(1.0..=400.0))
                        .changed()
                    {
                        if let Some(f) = st.publish_frames.iter_mut().find(|f| f.id == fid) {
                            f.font_size = Some(fs);
                        }
                    }
                    ui.label(RichText::new("Fill").size(11.0).color(muted));
                    let mut fl = hex32(frame.fill.as_deref().unwrap_or("#ffffff"));
                    if ui.color_edit_button_srgba(&mut fl).changed() {
                        if let Some(f) = st.publish_frames.iter_mut().find(|f| f.id == fid) {
                            f.fill = Some(to_hex(fl));
                        }
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            if ui.button("Delete").clicked() {
                let ids = st.selected_frame_ids.clone();
                st.publish_frames.retain(|f| !ids.contains(&f.id));
                st.selected_frame_ids.clear();
                st.push_history("Delete Frame(s)");
            }
        });
    }
}

// ---- History ------------------------------------------------------------------------

fn history_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    ui.horizontal(|ui| {
        if ui.button("Snapshot").clicked() {
            st.create_snapshot(None);
        }
        if ui.button("Clear").clicked() {
            st.clear_history();
        }
    });
    ui.separator();
    let steps: Vec<String> = st.history.iter().map(|s| s.title.clone()).collect();
    for (i, title) in steps.iter().enumerate() {
        let current = i == st.history_index;
        if ui
            .selectable_label(current, RichText::new(title).size(11.0))
            .clicked()
        {
            st.restore_history(i);
        }
    }
}

// ---- AI ------------------------------------------------------------------------------

fn ai_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);
    let accent = hex32(theme.accent);

    ui.checkbox(&mut st.ai_opt_in, "Opt in to agentic AI automation");
    ui.separator();
    if !st.ai_opt_in {
        ui.label(
            RichText::new(
                "AI features are off. Nothing leaves your machine — enable opt-in to use them.",
            )
            .size(11.0)
            .color(muted),
        );
        return;
    }
    for (label, desc) in [
        (
            "Smart Background Removal",
            "Isolate subject onto a non-destructive layer",
        ),
        (
            "Neural Super-Resolution 2×",
            "High-frequency texture synthesis upscale",
        ),
        ("Neural Super-Resolution 4×", "Maximum-detail enlargement"),
        ("Content-Aware Fill", "Procedural texture inpainting"),
        ("Generate Plugin Code", "Rust/WASM plugin scaffolding"),
    ] {
        if ui.button(label).clicked() {
            st.hint(format!("{label}: requires AI backend (not wired yet)"));
        }
        ui.label(RichText::new(desc).size(10.0).color(muted));
        ui.add_space(2.0);
    }
    let _ = accent;
}

// ---- Vector stroke ---------------------------------------------------------------------

fn stroke_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.label(RichText::new("Vector Appearance").strong());
    ui.horizontal(|ui| {
        ui.label(RichText::new("Fill").size(11.0).color(muted));
        let mut f = hex32(&st.vector_fill);
        if ui.color_edit_button_srgba(&mut f).changed() {
            st.vector_fill = to_hex(f);
            let c = st.vector_fill.clone();
            st.update_selected_vector_shapes(|s| s.fill = c.clone());
        }
        ui.label(RichText::new("Stroke").size(11.0).color(muted));
        let mut s = hex32(&st.vector_stroke);
        if ui.color_edit_button_srgba(&mut s).changed() {
            st.vector_stroke = to_hex(s);
            let c = st.vector_stroke.clone();
            st.update_selected_vector_shapes(|sh| sh.stroke = c.clone());
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Width").size(11.0).color(muted));
        if ui
            .add(egui::Slider::new(&mut st.vector_stroke_width, 0.0..=48.0).suffix("px"))
            .changed()
        {
            let w = st.vector_stroke_width;
            st.update_selected_vector_shapes(|s| s.stroke_width = w);
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Style").size(11.0).color(muted));
        for (v, l) in [
            (StrokeStyle::Solid, "Solid"),
            (StrokeStyle::Dashed, "Dashed"),
            (StrokeStyle::Dotted, "Dotted"),
        ] {
            if ui
                .selectable_label(st.vector_stroke_style == v, l)
                .clicked()
            {
                st.vector_stroke_style = v;
                st.update_selected_vector_shapes(|s| s.stroke_style = Some(v));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Corner radius").size(11.0).color(muted));
        if ui
            .add(egui::Slider::new(&mut st.vector_corner_radius, 0.0..=100.0).suffix("px"))
            .changed()
        {
            let r = st.vector_corner_radius;
            st.update_selected_vector_shapes(|s| s.corner_radius = Some(r));
        }
    });

    ui.separator();
    ui.label(RichText::new("Align").size(11.0).color(muted));
    ui.horizontal(|ui| {
        for (op, l) in [
            (AlignOp::Left, "Left"),
            (AlignOp::Center, "H-Center"),
            (AlignOp::Right, "Right"),
            (AlignOp::Top, "Top"),
            (AlignOp::Middle, "V-Middle"),
            (AlignOp::Bottom, "Bottom"),
        ] {
            if ui.button(l).clicked() {
                st.align_selected(op);
            }
        }
    });
}

// ---- Pathfinder --------------------------------------------------------------------------

fn pathfinder_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.label(RichText::new("Boolean Path Operations").strong());
    ui.label(
        RichText::new("Select 2+ shapes (or first two are used)")
            .size(10.0)
            .color(muted),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for (op, l) in [
            (PathfinderOp::Unite, "Unite"),
            (PathfinderOp::Subtract, "Minus Front"),
            (PathfinderOp::Intersect, "Intersect"),
            (PathfinderOp::Exclude, "Exclude"),
            (PathfinderOp::Divide, "Divide"),
        ] {
            if ui.button(l).clicked() {
                st.apply_pathfinder(op);
            }
        }
    });
    ui.separator();
    ui.label(
        RichText::new(format!(
            "{} shapes · {} selected",
            st.vector_shapes.len(),
            st.selected_shape_ids.len()
        ))
        .size(10.0)
        .color(muted),
    );
}

// ---- Publish pages ---------------------------------------------------------------------------

fn pages_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.horizontal(|ui| {
        ui.checkbox(&mut st.project.facing_pages, "Facing pages");
        if ui.button("+ Page").clicked() {
            st.project.page_count += 1;
        }
        if ui.button("- Page").clicked() && st.project.page_count > 1 {
            st.project.page_count -= 1;
            st.project.active_page_index =
                st.project.active_page_index.min(st.project.page_count - 1);
        }
    });
    ui.label(
        RichText::new(format!("Master: {}", st.publish_master_page))
            .size(11.0)
            .color(muted),
    );
    ui.separator();

    for i in 0..st.project.page_count {
        let active = st.project.active_page_index == i;
        if ui
            .selectable_label(active, format!("Page {}", i + 1))
            .clicked()
        {
            st.project.active_page_index = i;
        }
    }
}

// ---- Publish text wrap ----------------------------------------------------------------------------

fn wrap_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let muted = hex32(theme.text_muted);

    ui.label(RichText::new("Text Wrap Mode").strong());
    for (m, l) in [
        (WrapMode::None, "None — ignore wrap"),
        (WrapMode::Box, "Bounding box wrap"),
        (WrapMode::Contour, "Contour / shape wrap"),
        (WrapMode::Jump, "Jump object"),
    ] {
        if ui
            .selectable_label(st.publish_text_wrap_mode == m, l)
            .clicked()
        {
            st.publish_text_wrap_mode = m;
            for f in &mut st.publish_frames {
                if st.selected_frame_ids.contains(&f.id) {
                    f.wrap_mode = Some(m);
                }
            }
        }
    }
    ui.separator();
    ui.label(RichText::new("Wrap margins (px)").size(11.0).color(muted));

    // Edit margins on all selected frames.
    let has_selection = !st.selected_frame_ids.is_empty();
    if !has_selection {
        ui.label(
            RichText::new("Select a frame to edit its wrap margins.")
                .size(10.0)
                .color(muted),
        );
        return;
    }
    let mut margin = st
        .publish_frames
        .iter()
        .find(|f| st.selected_frame_ids.contains(&f.id))
        .and_then(|f| f.wrap_margin)
        .unwrap_or_default();
    let mut changed = false;
    ui.horizontal(|ui| {
        for (label, v) in [
            ("T", &mut margin.top),
            ("R", &mut margin.right),
            ("B", &mut margin.bottom),
            ("L", &mut margin.left),
        ] {
            ui.label(RichText::new(label).size(10.0).color(muted));
            if ui
                .add(egui::DragValue::new(v).range(0.0..=200.0).speed(0.5))
                .changed()
            {
                changed = true;
            }
        }
    });
    if changed {
        for f in &mut st.publish_frames {
            if st.selected_frame_ids.contains(&f.id) {
                f.wrap_margin = Some(margin);
            }
        }
    }
}

// ---- Preflight ----------------------------------------------------------------------------------------

fn preflight_panel(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = *theme_by_id(st.theme_id);
    let ok = hex32("#10b981");
    let warn = hex32("#f59e0b");
    let muted = hex32(theme.text_muted);

    ui.label(RichText::new("Print Preflight — FOGRA39").strong());
    ui.add_space(4.0);

    let checks: Vec<(&str, bool, String)> = vec![
        (
            "Resolution",
            st.project.dpi >= 300,
            format!("{} dpi (300+ required for offset)", st.project.dpi),
        ),
        (
            "Color mode",
            st.project.color_mode == ColorMode::Cmyk,
            format!("{:?} (CMYK required)", st.project.color_mode),
        ),
        (
            "Bleed",
            st.project.bleed >= 3.0,
            format!("{} mm bleed (3mm ISO)", st.project.bleed),
        ),
        (
            "Facing pages",
            st.project.facing_pages,
            "Spread layout configured".into(),
        ),
        (
            "Text frames",
            !st.publish_frames.is_empty(),
            format!("{} frames on active page", st.publish_frames.len()),
        ),
    ];
    for (name, pass, detail) in checks {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if pass { "PASS" } else { "WARN" })
                    .size(10.0)
                    .monospace()
                    .color(if pass { ok } else { warn }),
            );
            ui.vertical(|ui| {
                ui.label(RichText::new(name).size(11.0));
                ui.label(RichText::new(&detail).size(10.0).color(muted));
            });
        });
    }
    ui.separator();
    ui.label(
        RichText::new(format!("Profile: {}", st.project.cmyk_profile))
            .size(10.0)
            .color(muted),
    );
}
