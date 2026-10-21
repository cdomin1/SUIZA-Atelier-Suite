# SUIZA-Atelier Suite ✦

> A memory-safe, cross-platform creative suite written in **Rust** — three studios in one app:
> raster photo retouching (**Photo**, in the spirit of GIMP), vector illustration (**Vector**, in the spirit of Inkscape), and desktop publishing (**Publish**, in the spirit of Scribus).

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)]()

---

## Overview

SUIZA-Atelier Suite is a native desktop application that unifies the three pillars of a
creative workflow into a single document model and a single window:

| Studio | Inspired by | What it does |
|---|---|---|
| **Photo** | GIMP | Layer-based raster editing — brush, eraser, healing/clone, dodge/burn, fills, gradients, selections, adjustment layers, blend modes |
| **Vector** | Inkscape | Bézier pen paths, shape tools, direct node editing, Pathfinder booleans, stroke/fill styling, snapping |
| **Publish** | Scribus | Text/picture frames, facing pages, margins & bleed, column guides, text wrap, print preflight |

A switchable **persona** system re-tools the toolbox, option bar, dock panels, and canvas
interactions per studio — the underlying document stays the same.

## Status

`0.1.0-alpha.1` — the application is functional, with real editing (not a UI mockup).
This is the native Rust port of the [SUIZA-Suite](https://github.com/cdomin1/SUIZA-Suite)
web prototype, rebuilt so it runs on macOS, Linux, and Windows.

### Working today

- **Photo**: brush/eraser/healing/dodge/burn painting into real pixel buffers,
  flood fill & FG→BG gradients, marquee/lasso selections with marching ants,
  crop, eyedropper (samples the flattened composite), text placement,
  zoom tool, 12 blend modes + per-layer opacity through the compositor
- **Vector**: pen plotting with live preview, Enter to commit / Esc to cancel,
  direct-select node dragging, move/select with guide & document-edge snapping,
  rect/ellipse/polygon generators, Pathfinder (unite/subtract/intersect/exclude/divide)
- **Publish**: text/picture/rule frame drawing, frame move & properties,
  margins + bleed + column overlays, wrap modes and wrap margins, preflight checks
- **Suite-wide**: per-persona cut/copy/paste, undo history with named steps,
  guides dragged from rulers, pixel grid, snapping toggles, swatch palettes
  (switch palettes, add/remove swatches), workspace themes, remappable shortcuts
  with a keystroke HUD, `.szt` document save/open, PNG/JPEG export

### Roadmap

- Free-transform handles and in-place canvas text editing
- Freehand vector drawing, Knife/path cutting
- SVG / PDF / TIFF export
- True Bézier handles (currently polyline paths)
- Opt-in AI features (background removal, super-resolution, content-aware fill)

## Download (Alpha)

Pre-built packages are published under
[Releases](https://github.com/cdomin1/SUIZA-Atelier-Suite/releases):

| Platform | Package |
|---|---|
| macOS (universal — Apple Silicon + Intel) | `SUIZA-Atelier-Suite-*-macOS-universal.dmg` |
| Linux x86_64 | `SUIZA-Atelier-Suite-*-x86_64.AppImage` |
| Windows x86_64 | `SUIZA-Atelier-Suite-*-Windows-x86_64.zip` |

Alpha builds are unsigned/unnotarized — macOS: right-click → Open (or
`xattr -dr com.apple.quarantine` on the app); Windows: SmartScreen →
More info → Run anyway.

## Building & running

Requires a stable Rust toolchain (`rustup`, 2021 edition) and platform-native
dependencies for `eframe`/`winit`:

```bash
cargo run          # debug build
cargo run --release
```

- **Linux**: `sudo apt install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev` (or equivalent) is typically required for winit.
- **macOS**: builds with the Command Line Tools. If your CLT is older than the
  default SDK, pin `SDKROOT` to an installed SDK (the repo's local
  `.cargo/config.toml` demonstrates this; it is gitignored).

### Packaging

```bash
scripts/bundle-macos.sh release [universal]   # dist/SUIZA Atelier Suite.app
scripts/bundle-dmg.sh universal               # signed DMG (ad-hoc)
scripts/bundle-linux.sh                       # dist/suiza-atelier-linux/
scripts/bundle-appimage.sh                    # dist/*.AppImage (Linux, needs LINUXDEPLOY+APPIMAGETOOL)
scripts/bundle-windows.sh                     # dist/*.zip (Windows)
```

Tagging `v*` runs `.github/workflows/release.yml`, which builds all three
packages in CI and publishes a prerelease with `SHA256SUMS`.

## Document format

`.szt` files are JSON envelopes containing the project model, vector shapes,
publish frames, and guides. Embedded raster layers are stored as PNG payloads,
so a document is a single portable file.

## Tech stack

| Area | Choice |
|---|---|
| UI | [egui](https://github.com/emilk/egui) / eframe (immediate-mode, GPU-backed) |
| Raster | per-layer RGBA buffers, custom blend-mode compositor |
| Serialization | serde + serde_json (`.szt`) |
| File dialogs | rfd (native OS dialogs) |
| Image IO | image crate (PNG/JPEG) |

## Related projects

- [`SUIZA-Canvas`](https://github.com/cdomin1/SUIZA-Canvas) — a separate
  Rust/egui UI/UX vector design editor

## License

Apache-2.0
