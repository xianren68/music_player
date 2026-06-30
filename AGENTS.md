# AGENTS.md

## Build & Run

```bash
cargo build
cargo run
```

## Project Structure

Single Rust crate using [GPUI](https://docs.rs/gpui) (Zed's GPU UI framework).

- `src/main.rs` — sole source file, contains the `HelloWorld` app

## Conventions

- **中文注释**: 每次添加/更新代码都必须添加/更新对应的中文注释。注释使用 `//` 行注释，紧贴在被注释代码的上方或同行。
- Rust edition 2024.
