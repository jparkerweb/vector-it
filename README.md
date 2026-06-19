# VectorIt

<div align="center">

<img src="https://github.com/jparkerweb/vector-it/blob/main/banner.jpg?raw=true" alt="banner" style="max-height:300px;">

### ⚡ *Pixels in. Vectors out. No questions asked.* ⚡

**Open-source desktop app that converts raster images into vector graphics.**

VectorIt converts PNG, JPG, BMP, GIF, and TIFF images into clean SVG, EPS, PDF, and DXF vector output.  
Built with **Rust** for blazing speed and **Tauri v2** for a featherweight desktop shell (~5 MB),  
wrapped in a **React + TypeScript** frontend.

`────────────────────────── ◈ ──────────────────────────`

[![License](https://img.shields.io/badge/license-Apache--2.0-ff00ff?style=flat-square&logo=apache)](LICENSE)
[![Built With](https://img.shields.io/badge/built_with-Rust_🦀-e44d26?style=flat-square&logo=rust&logoColor=white)](#-tech-stack)
[![Desktop](https://img.shields.io/badge/desktop-Tauri_v2-00e5ff?style=flat-square&logo=tauri&logoColor=white)](#-tech-stack)
[![Frontend](https://img.shields.io/badge/frontend-React_⚛️-61dafb?style=flat-square&logo=react&logoColor=black)](#-tech-stack)

</div>

`═══════════════════════════════════════════════════════════════`

## 🌟 Features

> *Maximum power. Minimum friction.*

| | Feature | Description |
|---|---------|-------------|
| 🎨 | **Multi-format input** | PNG, JPG, BMP, GIF, TIFF |
| 📐 | **Multi-format output** | SVG, EPS, PDF, DXF (Spline & Line-only) |
| 🖼️ | **Bitmap export** | Re-rasterize vectors at any resolution (PNG, BMP, JPEG) |
| ⚙️ | **Quality presets** | Logo, Illustration, Photo, Pixel Art, Minimal, Detailed |
| 🎛️ | **Fine-tune controls** | Colors, smoothness, corner threshold, speckle filter, path mode |
| 📋 | **Clipboard paste** | `Ctrl+V` to paste and vectorize directly |
| 💾 | **Quick Save** | `Ctrl+S` for instant re-export with last settings |
| 🖱️ | **Drag-and-drop** | Drag exported files to other applications |
| ✏️ | **Segmentation editor** | Manually adjust color regions before vectorization |
| 📏 | **Auto-resize** | Handles large images (20MP+) by auto-downsampling for analysis |
| 🔒 | **Fully offline** | No internet required — your pixels stay yours |

`═══════════════════════════════════════════════════════════════`

## 📂 Project Structure

> *Two crates. One mission.*

```
vector-it/
├── Cargo.toml                  # Rust workspace root (defines both crates)
├── Cargo.lock                  # Locked dependency versions
├── vectorit-core/              # ⚡ Pure Rust library — vectorization engine
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # Library entry point
│   │   ├── types.rs            # All shared types/structs/enums
│   │   ├── pipeline.rs         # Orchestrates the vectorization stages
│   │   ├── decoder.rs          # Image loading (PNG/JPG/BMP/GIF/TIFF)
│   │   ├── resizer.rs          # Box-filter downsample for large images
│   │   ├── quantizer.rs        # K-means++ color quantization
│   │   ├── segmenter.rs        # Connected-component labeling
│   │   ├── tracer.rs           # Marching-squares boundary tracing
│   │   ├── simplifier.rs       # Path point reduction
│   │   ├── corner.rs           # Corner detection in traced paths
│   │   ├── fitter.rs           # Bézier curve fitting
│   │   ├── optimizer.rs        # Path optimization passes
│   │   ├── detector.rs         # Feature detection utilities
│   │   ├── aa_detector.rs      # Anti-aliasing detection
│   │   ├── subpixel.rs         # Sub-pixel precision helpers
│   │   ├── editor.rs           # Segmentation editor operations
│   │   └── export/             # Output format exporters
│   │       ├── svg.rs, eps.rs, pdf.rs, dxf.rs, bitmap.rs
│   ├── tests/                  # Integration & property tests
│   └── benches/                # Criterion performance benchmarks
│
├── vectorit-app/               # 🖥️ Tauri desktop application
│   ├── package.json            # Node.js / npm config (frontend deps)
│   ├── vite.config.ts          # Vite bundler config
│   ├── tsconfig.json           # TypeScript config
│   ├── index.html              # HTML entry point
│   ├── src/                    # React + TypeScript frontend
│   │   ├── App.tsx             # Main UI component
│   │   ├── main.tsx            # React entry point
│   │   ├── stores/             # Zustand state management
│   │   ├── components/         # UI components (Canvas, Wizard, Editor, etc.)
│   │   ├── hooks/              # Custom React hooks
│   │   └── utils/              # Shared utility functions
│   └── src-tauri/              # Tauri Rust backend (thin wrapper)
│       ├── Cargo.toml
│       ├── tauri.conf.json     # Tauri app configuration
│       └── src/
│           ├── main.rs         # Entry point
│           ├── lib.rs          # Command registration
│           ├── commands.rs     # IPC command handlers
│           ├── clipboard.rs    # Clipboard paste support
│           └── dragdrop.rs     # Drag-and-drop file handling
│
├── docs/                       # Architecture documentation
└── .github/workflows/          # CI/CD (GitHub Actions)
```

> 🔑 **Key concept:** This is a Rust *workspace* with two crates:
> - `vectorit-core` — the engine (pure library, no UI)
> - `vectorit-app/src-tauri` — the desktop shell (depends on `vectorit-core`)

`═══════════════════════════════════════════════════════════════`

## 🔌 Prerequisites (First-Time Setup)

> *Jack in before you ride the grid.*

### `01` 🦀 Install Rust

Go to <https://rustup.rs/> and run the installer. On Windows this downloads `rustup-init.exe`.

- Accept all defaults (it installs `rustc`, `cargo`, and the MSVC toolchain)
- **Requires:** Visual Studio Build Tools with "Desktop development with C++" workload
  - If you don't have it, the Rust installer will prompt you to install it
  - Or download from: <https://visualstudio.microsoft.com/visual-cpp-build-tools/>

After install, open a **new** terminal and verify:
```powershell
rustc --version    # Should show 1.78+ (e.g., rustc 1.82.0)
cargo --version    # Should show cargo 1.78+
```

### `02` 🟢 Install Node.js

Go to <https://nodejs.org/> and install the **LTS** version (v20+).

Verify:
```powershell
node --version     # Should show v20.x or v22.x
npm --version      # Should show 10.x+
```

### `03` 📦 Install Tauri CLI

```powershell
cargo install tauri-cli
```

This compiles from source and takes 2–5 minutes the first time. After it finishes:
```powershell
cargo tauri --version   # Should show tauri-cli 2.x
```

### `04` 🪟 (Windows) WebView2 Runtime

Tauri uses WebView2 (built into Windows 10/11). If you're on Windows 10 and it's not installed:  
<https://developer.microsoft.com/en-us/microsoft-edge/webview2/>

`═══════════════════════════════════════════════════════════════`

## 🚀 Building & Running

### ⚡ Development Mode (hot-reload)

```powershell
cd vectorit-app
npm install              # Install frontend dependencies (first time only)
cargo tauri dev          # Starts dev server + compiles Rust + opens window
```

**What happens:**
1. Vite starts a dev server at `http://localhost:1420` (frontend hot-reload)
2. Cargo compiles the Rust backend (first build takes 2–5 min, subsequent builds are fast)
3. A desktop window opens showing the app

> ⏳ **First build is slow** — Rust compiles ~200+ dependency crates. After that, incremental builds are 5–15 seconds.

### 📦 Production Build (installer)

```powershell
cd vectorit-app
npm install
cargo tauri build
```

Produces a Windows installer at:
```
vectorit-app/src-tauri/target/release/bundle/nsis/VectorIt_1.0.0_x64-setup.exe
```

### 🧪 Running just the core library tests (no UI)

```powershell
cd vectorit-core
cargo test
```

`═══════════════════════════════════════════════════════════════`

## 🔧 Troubleshooting

> *When the neon flickers…*

<details>
<summary>💥 <b>"Couldn't recognize the current folder as a Tauri project"</b></summary>

You must run `cargo tauri dev` from the `vectorit-app/` directory (where `package.json` lives), **NOT** from the workspace root.
</details>

<details>
<summary>💥 <b>Stale build cache errors (paths to old directories)</b></summary>

```powershell
cargo clean
cargo tauri dev
```
</details>

<details>
<summary>💥 <b>"linker `link.exe` not found"</b></summary>

You need Visual Studio Build Tools with C++ workload. Install from:  
<https://visualstudio.microsoft.com/visual-cpp-build-tools/>
</details>

<details>
<summary>💥 <b>npm install fails</b></summary>

Make sure you're in `vectorit-app/` (not the root). Delete `node_modules` and retry:
```powershell
cd vectorit-app
Remove-Item -Recurse node_modules
npm install
```
</details>

<details>
<summary>💥 <b>Vite port already in use</b></summary>

Another process is using port 1420. Kill it or change the port in `vite.config.ts`.
</details>

`═══════════════════════════════════════════════════════════════`

## 🕹️ Usage

### ⚡ Quick Start — 3 clicks to vector glory

```
 ┌─────────┐      ┌─────────┐      ┌─────────┐
 │  OPEN   │ ───▶ │  AUTO   │ ───▶ │ EXPORT  │
 │ 🖼️ Load │      │ ⚙️ Trace │      │ 💾 Save │
 └─────────┘      └─────────┘      └─────────┘
```

1. **Open** — Click "Open" or drag an image onto the window
2. **Auto** — VectorIt auto-vectorizes with the selected preset
3. **Export** — Click "Export" to save as SVG/EPS/PDF/DXF

### ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|:---------|:-------|
| `Ctrl+V` | Paste image from clipboard |
| `Ctrl+S` | Quick Save (re-export with last settings) |
| `Ctrl+Z` | Undo (in canvas/segmentation editor) |
| `Ctrl+Shift+Z` | Redo (in canvas editor) |

### 🎛️ Sidebar Controls

| Control | What it does |
|:--------|:------------|
| **Preset** | One-click configurations (Logo, Photo, Pixel Art, etc.) |
| **Quality** | Low (fast) / Medium / High (best fidelity) |
| **Colors** | How many colors to quantize to (2–32) |
| **Path Mode** | Polygon = sharp edges, Spline = smooth curves |
| **Speckle Filter** | Remove tiny noise regions (higher = cleaner) |
| **Color Precision** | How aggressively to merge similar colors |
| **Corner Threshold** | Angle to treat as a sharp corner vs smooth curve |
| **Auto-Render** | Re-process automatically when you change settings |

`═══════════════════════════════════════════════════════════════`

## 🧪 Running Tests

```powershell
# ⚡ All tests (from workspace root)
cargo test --workspace

# 🦀 Just the core engine tests
cd vectorit-core
cargo test

# 🔬 Integration tests only (format validation)
cargo test --test format_tests

# 📊 Performance benchmarks
cargo bench

# ✅ Frontend type-check
cd vectorit-app
npm run build
```

`═══════════════════════════════════════════════════════════════`

## 💾 Tech Stack

> *The machines under the hood.*

| | Layer | Technology | Why |
|---|-------|-----------|-----|
| 🦀 | Core Engine | **Rust** | Fast, memory-safe, compiles to native code |
| 🖥️ | Desktop Shell | **Tauri v2** | Lightweight (~5 MB), uses OS webview, Rust backend |
| ⚛️ | Frontend | **React 18 + TypeScript** | Component model, type safety |
| 🎨 | Styling | **Tailwind CSS v4** | Utility-first, fast iteration |
| 🧠 | State | **Zustand** | Lightweight React state management |
| ⚡ | Bundler | **Vite** | Fast HMR (hot module reload) for development |
| 🔍 | Vectorization | **vtracer** crate | Battle-tested image tracing engine |
| 🧬 | Color Science | **palette** crate | CIE Lab color space conversions |
| 📄 | PDF Output | **pdf-writer** crate | Low-level PDF generation |
| 📐 | DXF Output | **dxf** crate | AutoCAD DXF file format |
| 📊 | Testing | **criterion** | Statistical benchmarking for Rust |

`═══════════════════════════════════════════════════════════════`

## 🏗️ Architecture

> *The signal path.*

```
  ┌───────┐   ┌────────┐   ┌──────────┐   ┌─────────┐   ┌────────┐   ┌─────────┐   ┌────────┐
  │ IMAGE │──▶│ RESIZE │──▶│ QUANTIZE │──▶│ SEGMENT │──▶│ EDITOR │──▶│  TRACE  │──▶│ EXPORT │
  └───────┘   └────────┘   └──────────┘   └─────────┘   └────────┘   └─────────┘   └────────┘
     📷          📏            🎨             🧩           ✏️            ✒️            💾
```

| Stage | What it does |
|:------|:-------------|
| **Resize** | Auto-downsample images > 4MP for analysis speed |
| **Quantize** | K-means++ clustering in CIE Lab color space → fixed palette |
| **Segment** | Flood-fill connected components → regions |
| **Editor** | Optional manual region corrections (paint/split/merge) |
| **Trace** | vtracer converts pixel data → Bézier paths; custom modules (tracer, simplifier, corner, fitter, optimizer) provide additional boundary tracing and path refinement |
| **Export** | Serialize to SVG / EPS / PDF / DXF / Bitmap |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full details including IPC protocol and module dependency graph.

`═══════════════════════════════════════════════════════════════`

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR guidelines.

`═══════════════════════════════════════════════════════════════`

## 📜 License

Licensed under [Apache-2.0](LICENSE).

<div align="center">

```
╔══════════════════════════════════════════════════════════════╗
║          P I X E L S   I N  ·  V E C T O R S   O U T       ║
╚══════════════════════════════════════════════════════════════╝
```

*Made with 🦀 Rust, ⚡ Tauri, and mass quantities of mass quantities of mass quantities of neon.*

</div>

