//! Left tool palette — port of `Toolbox.tsx`.
//! Buttons are labeled with the tool's keyboard key until an icon font ships.

use crate::model::{Persona, ToolId};
use crate::state::AtelierState;
use crate::theme::hex32;
use egui::{Color32, Pos2, Rect, RichText, Vec2};

pub struct ToolDef {
    pub id: ToolId,
    pub name: &'static str,
    pub key: &'static str,
}

fn photo_tools() -> &'static [ToolDef] {
    &[
        ToolDef { id: ToolId::Move, name: "Move / Selection", key: "V" },
        ToolDef { id: ToolId::Marquee, name: "Rectangular Marquee", key: "M" },
        ToolDef { id: ToolId::Crop, name: "Crop Canvas", key: "C" },
        ToolDef { id: ToolId::Eyedropper, name: "Eyedropper Sampler", key: "I" },
        ToolDef { id: ToolId::Healing, name: "Spot Healing Brush", key: "J" },
        ToolDef { id: ToolId::Brush, name: "Paint Brush Tool", key: "B" },
        ToolDef { id: ToolId::Eraser, name: "Eraser Tool", key: "E" },
        ToolDef { id: ToolId::Bucket, name: "Paint Bucket Tool", key: "G" },
        ToolDef { id: ToolId::Dodge, name: "Dodge / Burn Exposure", key: "O" },
        ToolDef { id: ToolId::Pen, name: "Pen Tool (Bezier Paths)", key: "P" },
        ToolDef { id: ToolId::Text, name: "Horizontal Type Tool", key: "T" },
        ToolDef { id: ToolId::Hand, name: "Hand Pan Tool", key: "H" },
        ToolDef { id: ToolId::Zoom, name: "Zoom Tool", key: "Z" },
    ]
}

fn vector_tools() -> &'static [ToolDef] {
    &[
        ToolDef { id: ToolId::Move, name: "Selection Tool", key: "V" },
        ToolDef { id: ToolId::DirectSelect, name: "Direct Selection (Nodes)", key: "A" },
        ToolDef { id: ToolId::Pen, name: "Bezier Pen Spline", key: "P" },
        ToolDef { id: ToolId::ShapeRect, name: "Rectangle Tool", key: "M" },
        ToolDef { id: ToolId::ShapeEllipse, name: "Ellipse Tool", key: "L" },
        ToolDef { id: ToolId::ShapePolygon, name: "Polygon / Star Tool", key: "Y" },
        ToolDef { id: ToolId::ShapeBuilder, name: "Shape Builder (Pathfinder)", key: "S+M" },
        ToolDef { id: ToolId::Knife, name: "Knife / Cut Paths", key: "C" },
        ToolDef { id: ToolId::Text, name: "Vector Type Tool", key: "T" },
        ToolDef { id: ToolId::Eyedropper, name: "Eyedropper Sampler", key: "I" },
        ToolDef { id: ToolId::Hand, name: "Hand Pan Tool", key: "H" },
        ToolDef { id: ToolId::Zoom, name: "Zoom Tool", key: "Z" },
    ]
}

fn publish_tools() -> &'static [ToolDef] {
    &[
        ToolDef { id: ToolId::Move, name: "Selection Tool", key: "V" },
        ToolDef { id: ToolId::DirectSelect, name: "Direct Selection", key: "A" },
        ToolDef { id: ToolId::FrameText, name: "Type Text Frame Tool", key: "T" },
        ToolDef { id: ToolId::FramePicture, name: "Graphic Picture Frame", key: "F" },
        ToolDef { id: ToolId::FrameShape, name: "Rectangle Frame", key: "M" },
        ToolDef { id: ToolId::LineTool, name: "Editorial Rule / Line Tool", key: "\\" },
        ToolDef { id: ToolId::PageTool, name: "Page Layout Tool", key: "S+P" },
        ToolDef { id: ToolId::Eyedropper, name: "Eyedropper Sampler", key: "I" },
        ToolDef { id: ToolId::Hand, name: "Hand Pan Tool", key: "H" },
        ToolDef { id: ToolId::Zoom, name: "Zoom Tool", key: "Z" },
    ]
}

pub fn tools_for(p: Persona) -> &'static [ToolDef] {
    match p {
        Persona::Photo => photo_tools(),
        Persona::Vector => vector_tools(),
        Persona::Publish => publish_tools(),
    }
}

pub fn toolbox(ui: &mut egui::Ui, st: &mut AtelierState) {
    let theme = crate::theme::theme_by_id(st.theme_id);
    let accent = hex32(theme.accent);
    let text_dim = hex32(theme.text_secondary);

    ui.vertical_centered(|ui| {
        ui.add_space(6.0);
        for tool in tools_for(st.persona) {
            let active = st.active_tool == tool.id;
            let label = RichText::new(tool.key).size(11.0).strong();
            let btn = egui::Button::new(if active {
                label.color(Color32::WHITE)
            } else {
                label.color(text_dim)
            })
            .min_size(Vec2::new(34.0, 34.0));
            let btn = if active { btn.fill(accent) } else { btn };
            if ui
                .add(btn)
                .on_hover_text(format!("{} ({})", tool.name, tool.key))
                .clicked()
            {
                st.active_tool = tool.id;
                st.hint(format!("{} ({})", tool.name, tool.key));
            }
        }

        ui.add_space(8.0);
        ui.separator();

        // Foreground / background color chips
        let mut fg = hex32(&st.foreground_color);
        let mut bg = hex32(&st.background_color);

        ui.horizontal(|ui| {
            if ui.small_button("D").on_hover_text("Default Colors: Black & White (D)").clicked() {
                st.reset_colors();
                st.hint("Reset Black & White (D)");
            }
            if ui.small_button("X").on_hover_text("Swap Foreground/Background (X)").clicked() {
                st.swap_colors();
                st.hint("Swapped Colors (X)");
            }
        });

        // Overlapping chips drawn manually
        let (rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::hover());
        let bg_rect = Rect::from_min_size(rect.min + Vec2::new(9.0, 9.0), Vec2::new(20.0, 20.0));
        let fg_rect = Rect::from_min_size(rect.min, Vec2::new(20.0, 20.0));
        ui.painter().rect_filled(bg_rect, 2.0, bg);
        ui.painter().rect_stroke(bg_rect, 2.0, egui::Stroke::new(1.0_f32, hex32(theme.border_strong)), egui::StrokeKind::Inside);
        ui.painter().rect_filled(fg_rect, 2.0, fg);
        ui.painter().rect_stroke(fg_rect, 2.0, egui::Stroke::new(1.5_f32, Color32::WHITE), egui::StrokeKind::Inside);
        let _ = Pos2::ZERO; // silence unused import in some cfgs

        ui.label(RichText::new("FG").size(8.0).color(text_dim));
        if ui.color_edit_button_srgba(&mut fg).changed() {
            st.foreground_color = to_hex(fg);
        }
        ui.label(RichText::new("BG").size(8.0).color(text_dim));
        if ui.color_edit_button_srgba(&mut bg).changed() {
            st.background_color = to_hex(bg);
        }

        // CMYK gamut indicator
        let cmyk = st.cmyk();
        ui.label(
            RichText::new(format!("{}% K", cmyk.k))
                .size(9.0)
                .monospace()
                .color(text_dim),
        )
        .on_hover_text(format!(
            "Active CMYK Ink: C:{}% M:{}% Y:{}% K:{}%",
            cmyk.c, cmyk.m, cmyk.y, cmyk.k
        ));
        ui.add_space(4.0);
    });
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}
