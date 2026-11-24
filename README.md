# CHIP-8 Emulator in Rust

A complete, cycle-accurate CHIP-8 interpreter and emulator written in Rust with reproducible Nix development builds.

## Features
- **Full Opcode Support:** Implements all 35 standard CHIP-8 instructions (`0x0000`–`0xFFFF`), including bitwise operations, BCD conversion, subroutines, and overflow/carry flags (`VF`).
- **Graphics & Input:** 64x32 monochrome display rendered at 60 FPS via `minifb` with full 16-key hexadecimal keypad mapping.
- **Reproducible Environment:** Configured with Nix Flakes (`rust-overlay` + native X11/Wayland dependencies) and automated GitHub Actions CI (`cargo test` & `cargo clippy`).

## Keypad Mapping

| CHIP-8 | Keyboard |
| :--- | :--- |
| `1 2 3 C` | `1 2 3 4` |
| `4 5 6 D` | `Q W E R` |
| `7 8 9 E` | `A S D F` |
| `A 0 B F` | `Z X C V` |

## Build & Run

Using Cargo:
```bash
cargo run --release -- path/to/rom.ch8
```

Using Nix Flakes:
```bash
nix develop
cargo run --release -- path/to/rom.ch8
```

## Run Tests

```bash
cargo test
```
