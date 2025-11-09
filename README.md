# Freminal

> **A modern, Catppuccin-themed terminal emulator written in Rust — built for precision, performance, and beauty.**

[![CI](https://github.com/fredclausen/freminal/actions/workflows/ci.yml/badge.svg)](https://github.com/fredclausen/freminal/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/codecov/c/github/fredclausen/freminal?label=code%20coverage)](https://app.codecov.io/gh/fredclausen/freminal)
![Escape Sequence Coverage](https://img.shields.io/badge/escape--sequence--coverage-72%25-green?logo=gnometerminal)
![License](https://img.shields.io/badge/license-MIT-blue)
![Theme](https://img.shields.io/badge/theme-Catppuccin_Mocha-8aadf4?logo=palette)

---

Freminal is a fully Rust-based terminal emulator built to prioritize **accuracy**, **speed**, and **aesthetic coherence**.
It aims to be _deeply standards-compliant_ while also embracing modern design ideas — smooth rendering, clear typography,
and a cohesive Catppuccin-inspired visual style.

> “A terminal emulator that feels like it actually understands what you meant.”

---

## ✨ Features

- **Comprehensive ANSI/DEC/xterm support**
  Full SGR (256 + TrueColor) and most CSI, OSC, and DEC sequences implemented.
  See [Escape Sequence Coverage](./docs/ESCAPE_SEQUENCE_COVERAGE.md) for full details.

- **Modern Rendering Pipeline**
  Built on [`egui`](https://github.com/emilk/egui), tuned for pixel-perfect glyph alignment and efficient draw batching.

- **Mouse & Input Handling**
  Supports mouse tracking modes (?1000–1006) and full keyboard interaction.

- **Alt Screen & Scrollback Buffers**
  True alternate screen behavior, smooth scrolling, and instant context switching.

- **Reproducible Nix Development Environment**
  Deterministic devshells and CI via flakes. One command brings up the full toolchain.

- **Beautiful Catppuccin Theme**
  Default palette matches Catppuccin Mocha. Full theming system planned.

---

## 🚀 Getting Started

### 1. **Preferred: Nix / Flake Environment**

If you use [Nix](https://nixos.org) or [direnv](https://direnv.net):

```bash
git clone https://github.com/fredclausen/freminal.git
cd freminal
direnv allow     # or: nix develop
```

This enters a reproducible dev shell with:

- Rust toolchain (stable) via `rust-overlay`
- `cargo-llvm-cov`, `cargo-machete`, and benchmarking tools
- All required system libraries (libGL, wayland, xkbcommon, etc.)

### 2. **Run the Emulator**

```bash
cargo run --release
```

or, for testing and benchmarking:

```bash
cargo test
cargo bench
```

---

## 🧱 Architecture Overview

Freminal is composed of three primary layers:

| Layer                | Crate                        | Role                                                              |
| -------------------- | ---------------------------- | ----------------------------------------------------------------- |
| **Parser / Core**    | `freminal-terminal-emulator` | Handles ANSI/DEC/OSC parsing, cursor state, and buffer logic.     |
| **Common Utilities** | `freminal-common`            | Shared color, geometry, and data types.                           |
| **GUI / Frontend**   | `freminal`                   | Renders terminal grid using `egui`; manages PTY ↔ renderer loop. |

### Data Flow

```text
PTY Input  →  AnsiParser  →  Terminal State  →  Renderer (egui)
                  ↑                ↓
            Mode handling      Output actions
```

---

## 📘 Documentation

| Document                                                        | Description                              |
| --------------------------------------------------------------- | ---------------------------------------- |
| [Escape Sequence Coverage](./docs/ESCAPE_SEQUENCE_COVERAGE.md)  | Detailed per-sequence coverage table.    |
| [Escape Sequence Gaps](./docs/ESCAPE_SEQUENCE_GAPS.md)          | Roadmap of missing or partial sequences. |
| [SGR.md](./docs/SGR.md)                                         | Attribute-level SGR breakdown.           |
| [SUPPORTED_CONTROL_CODES.md](./docs/SUPPORTED_CONTROL_CODES.md) | Low-level control code reference.        |

---

## 🧪 Development Notes

- Uses `cargo xtask` for CI and build orchestration.
- Test coverage targets **100 %** across crates (`cargo llvm-cov`).
- Profiling and benchmarking via `cargo bench` and `samply`.
- CI runs inside Nix with full caching through [Cachix](https://cachix.org).

---

## 🖌️ Theming

The default color palette is **Catppuccin Mocha**, chosen for its readability and aesthetic warmth.
Theme customization will become user-configurable in a future release.

| Example         | Catppuccin Mocha |
| --------------- | ---------------- |
| Background      | `#1E1E2E`        |
| Foreground      | `#CDD6F4`        |
| Accent (Cursor) | `#89B4FA`        |

---

## 🧩 Project Goals

- Match or exceed **xterm** escape sequence compatibility.
- Achieve sub-millisecond average frame times during scrollback rendering.
- Provide full Nix-based build reproducibility.
- Serve as a reference-grade open terminal emulator written in idiomatic Rust.

---

## 💬 Contributing

Contributions, feedback, and bug reports are welcome!
If you use Nix, your environment is already set up to run formatting and tests:

```bash
cargo fmt
cargo clippy
cargo test
```

Please see `.github/CONTRIBUTING.md` for contribution guidelines.

---

## 🪪 License

Licensed under the [MIT License](LICENSE).

---

## 🏗️ Project Status

Freminal is **actively developed** and serves as both a personal project and a demonstration of
high-fidelity terminal emulation written in pure Rust.

Escape Sequence Coverage: SGR ✅ CSI ✅ OSC 🚧 DEC ✅ FTCS ⬜

---

© 2024–2025 Fred Clausen — MIT License.
