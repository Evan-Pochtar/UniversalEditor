# Universal Editor

Universal Editor is an all-in-one native desktop application built with Rust. The goal is to create a single environment where you can handle text, images, video, and data files without the overhead of web-based frameworks like Electron. It focuses on using systems-level programming to stay fast and light, even when dealing with files that are several gigabytes in size. This is extremely early in production and only an idea/prototype at the moment.

## Core Approach

The project is built around a "Kernel and Module" architecture. The main shell handles the windowing, GPU rendering, and global styling, while specific editors are plugged in as modules. By using egui, the interface stays responsive because it’s rendered directly on your graphics card. For text (so far), it uses a Rope data structure, which allows you to edit massive files by treating the text as a tree of chunks rather than one giant block of memory.

## Project Goals

1. **Performance:** It uses manual memory management strategies and hardware acceleration to ensure that large files don't cause lag or high CPU usage.
2. **Modularity:** The codebase is designed so that functions and UI components are reusable. You can swap between a text editor and an image viewer within the same window without loading a new application instance.
3. **Modern Design:** While this is a native app, it avoids the "dated" look of old software. It uses a custom-tuned styling engine to provide a clean, dark-mode interface with plenty of spacing and rounded geometry.

## How to Run

To try it out, make sure you have the Rust toolchain installed. Since this is a performance-heavy application, running it in release mode is necessary to see the actual speed.

```bash
cargo run --release
```
