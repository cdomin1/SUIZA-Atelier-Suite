//! Factory palettes, document presets and sample plugins —
//! port of `src/constants/presets.ts` + `DEFAULT_PALETTES` from StudioContext.

use crate::model::*;

pub struct DocPreset {
    pub name: &'static str,
    pub width: f32,
    pub height: f32,
    pub dpi: u32,
    pub color_mode: ColorMode,
    pub persona: Persona,
    pub facing_pages: bool,
    pub page_count: u32,
}

pub struct PresetCategory {
    pub category: &'static str,
    pub items: &'static [DocPreset],
}

pub const DOCUMENT_PRESETS: &[PresetCategory] = &[
    PresetCategory {
        category: "Photo & Retouching",
        items: &[
            DocPreset { name: "4K Ultra High-Res", width: 3840.0, height: 2160.0, dpi: 300, color_mode: ColorMode::Rgb, persona: Persona::Photo, facing_pages: false, page_count: 1 },
            DocPreset { name: "Studio Editorial 16:9", width: 2560.0, height: 1440.0, dpi: 300, color_mode: ColorMode::Rgb, persona: Persona::Photo, facing_pages: false, page_count: 1 },
            DocPreset { name: "Square Retouch Portrait", width: 2048.0, height: 2048.0, dpi: 300, color_mode: ColorMode::Rgb, persona: Persona::Photo, facing_pages: false, page_count: 1 },
            DocPreset { name: "Print Fine Art 8x10 in", width: 2400.0, height: 3000.0, dpi: 300, color_mode: ColorMode::Rgb, persona: Persona::Photo, facing_pages: false, page_count: 1 },
        ],
    },
    PresetCategory {
        category: "Vector Illustration",
        items: &[
            DocPreset { name: "App Icon & Vector Grid", width: 1024.0, height: 1024.0, dpi: 72, color_mode: ColorMode::Rgb, persona: Persona::Vector, facing_pages: false, page_count: 1 },
            DocPreset { name: "Branding Artboard CMYK", width: 1920.0, height: 1080.0, dpi: 300, color_mode: ColorMode::Cmyk, persona: Persona::Vector, facing_pages: false, page_count: 1 },
            DocPreset { name: "Scalable SVG Canvas", width: 1200.0, height: 800.0, dpi: 72, color_mode: ColorMode::Rgb, persona: Persona::Vector, facing_pages: false, page_count: 1 },
        ],
    },
    PresetCategory {
        category: "Editorial & Publishing",
        items: &[
            DocPreset { name: "A4 Magazine Spread", width: 2480.0, height: 3508.0, dpi: 300, color_mode: ColorMode::Cmyk, persona: Persona::Publish, facing_pages: true, page_count: 4 },
            DocPreset { name: "US Letter Brochure", width: 2550.0, height: 3300.0, dpi: 300, color_mode: ColorMode::Cmyk, persona: Persona::Publish, facing_pages: true, page_count: 4 },
            DocPreset { name: "Modern Book Layout 6x9 in", width: 1800.0, height: 2700.0, dpi: 300, color_mode: ColorMode::Cmyk, persona: Persona::Publish, facing_pages: true, page_count: 8 },
        ],
    },
];

fn sw(id: &str, name: &str, color: &str) -> ColorSwatch {
    ColorSwatch {
        id: id.into(),
        name: name.into(),
        color: color.into(),
        swatch_type: None,
        gradient_type: None,
        gradient_angle: None,
        gradient_stops: Vec::new(),
        is_none: false,
        category: None,
        is_favorite: false,
    }
}

pub fn default_palettes() -> Vec<SwatchPalette> {
    vec![
        SwatchPalette {
            id: "palette-core".into(),
            name: "Brand & Process CMYK".into(),
            description: Some("Suiza identity, standard process plates, and high-contrast core tones".into()),
            swatches: vec![
                ColorSwatch { is_none: true, ..sw("sw-none", "None / Transparent", "transparent") },
                sw("sw-white", "Paper White", "#ffffff"),
                sw("sw-rich-black", "Rich Black (60/40/40/100)", "#090a0f"),
                sw("sw-cyan", "Process Cyan", "#00a3e0"),
                sw("sw-magenta", "Process Magenta", "#e4007c"),
                sw("sw-yellow", "Process Yellow", "#fed100"),
                sw("sw-black", "Process Black", "#000000"),
                ColorSwatch { is_favorite: true, ..sw("sw-suiza-blue", "Suiza Electric Blue", "#0d99ff") },
                sw("sw-amber", "Vibrant Amber", "#f59e0b"),
                sw("sw-emerald", "Alpine Pine", "#10b981"),
                ColorSwatch { is_favorite: true, ..sw("sw-rose", "Editorial Rose", "#ec4899") },
                sw("sw-indigo", "Deep Indigo", "#6366f1"),
            ],
        },
        SwatchPalette {
            id: "palette-pantone".into(),
            name: "Pantone PMS Solid".into(),
            description: Some("Pre-press spot color reproductions for editorial & print design".into()),
            swatches: vec![
                sw("sw-p286", "PANTONE 286 C (Deep Cobalt)", "#0033a0"),
                sw("sw-p186", "PANTONE 186 C (Vivid Crimson)", "#c8102e"),
                sw("sw-p368", "PANTONE 368 C (Spring Green)", "#78be20"),
                sw("sw-p021", "PANTONE 021 C (Bright Orange)", "#ff5100"),
                sw("sw-p2685", "PANTONE 2685 C (Royal Violet)", "#330072"),
                sw("sw-p877", "PANTONE 877 C (Metallic Silver)", "#8a8d8f"),
                sw("sw-pwg1", "PANTONE Warm Gray 1 C", "#d7d2cb"),
                sw("sw-pcg7", "PANTONE Cool Gray 7 C", "#97999b"),
            ],
        },
        SwatchPalette {
            id: "palette-modern-ui".into(),
            name: "Modern UI & Editorial".into(),
            description: Some("Curated modern hues for layout cards, shapes, and typography".into()),
            swatches: vec![
                sw("sw-slate-900", "Slate Deep 900", "#0f172a"),
                sw("sw-slate-500", "Slate Mid 500", "#64748b"),
                sw("sw-sky-400", "Sky Glow 400", "#38bdf8"),
                sw("sw-teal-500", "Pacific Teal 500", "#14b8a6"),
                sw("sw-violet-600", "Electric Violet 600", "#7c3aed"),
                sw("sw-fuchsia-500", "Neon Fuchsia 500", "#d946ef"),
                sw("sw-coral-500", "Living Coral 500", "#f97316"),
                sw("sw-warm-sand", "Nordic Sand", "#f3ede2"),
            ],
        },
        SwatchPalette {
            id: "palette-gradients".into(),
            name: "Gradients & Blends".into(),
            description: Some("Multi-stop linear and radial blends for backgrounds and fills".into()),
            swatches: vec![
                ColorSwatch { swatch_type: Some(SwatchKind::Gradient), gradient_type: Some(GradientType::Linear), gradient_angle: Some(135.0), gradient_stops: vec![GradientStop { offset: 0.0, color: "#0d99ff".into() }, GradientStop { offset: 1.0, color: "#10b981".into() }], ..sw("sw-grad-aurora", "Aurora Borealis", "#0d99ff") },
                ColorSwatch { swatch_type: Some(SwatchKind::Gradient), gradient_type: Some(GradientType::Linear), gradient_angle: Some(135.0), gradient_stops: vec![GradientStop { offset: 0.0, color: "#ec4899".into() }, GradientStop { offset: 1.0, color: "#f59e0b".into() }], ..sw("sw-grad-sunset", "Cosmic Sunset", "#ec4899") },
                ColorSwatch { swatch_type: Some(SwatchKind::Gradient), gradient_type: Some(GradientType::Linear), gradient_angle: Some(135.0), gradient_stops: vec![GradientStop { offset: 0.0, color: "#6366f1".into() }, GradientStop { offset: 1.0, color: "#ec4899".into() }], ..sw("sw-grad-neon", "Cyberpunk Neon", "#6366f1") },
                ColorSwatch { swatch_type: Some(SwatchKind::Gradient), gradient_type: Some(GradientType::Linear), gradient_angle: Some(135.0), gradient_stops: vec![GradientStop { offset: 0.0, color: "#0f172a".into() }, GradientStop { offset: 1.0, color: "#1e1b4b".into() }], ..sw("sw-grad-dark", "Deep Space Mesh", "#0f172a") },
            ],
        },
    ]
}

pub fn sample_plugins() -> Vec<PluginExtension> {
    let p = |id: &str, name: &str, author: &str, version: &str, t: PluginType, cat: PluginCategory, desc: &str| PluginExtension {
        id: id.into(),
        name: name.into(),
        author: author.into(),
        version: version.into(),
        plugin_type: t,
        category: cat,
        description: desc.into(),
        enabled: true,
        memory_safety_verified: true,
    };
    vec![
        p("plug-gegl-retouch", "GEGL High-Pass Frequency Separation", "GNOME / GIMP Porting Team", "3.1.2", PluginType::GeglOp, PluginCategory::Retouch, "Hardware-accelerated texture split for skin blemish retouching preserving high frequencies."),
        p("plug-rust-denoise", "Rust SIMD Bilateral Denoise Engine", "Suiza Core Research", "1.4.0", PluginType::RustWasm, PluginCategory::Retouch, "Zero-cost SIMD vectorized noise reduction with zero GC pauses and edge-preserving filtering."),
        p("plug-inkscape-bezier", "Inkscape PowerStroke & Spiro Paths", "Inkscape Vector Lab", "2.0.4", PluginType::RustWasm, PluginCategory::Typography, "Non-linear Bezier spline interpolation and variable pressure vector calligraphic ribbons."),
        p("plug-scribus-pdfx4", "Scribus PDF/X-4 CMYK Engine", "Scribus Publishing Group", "1.6.0", PluginType::ScribusScript, PluginCategory::Export, "ISO 15930-7 compliant professional print preflight with FOGRA39 / GRACoL profile embedding."),
        p("plug-python-resynth", "GIMP Resynthesizer Content-Aware Bridge", "Paul Harrison / Suiza Bridge", "2.2.0", PluginType::PythonGimp, PluginCategory::Automation, "Texture synthesis engine for intelligent object erasure and seamless tile generation."),
        p("plug-cmyk-separation", "Spectral CMYK Ink Density Analyzer", "Suiza Colorimetric Lab", "1.1.0", PluginType::RustWasm, PluginCategory::Color, "Total Area Coverage (TAC) monitor ensuring ink densities do not exceed 300% on offset presses."),
    ]
}
