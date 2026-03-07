# Universal Editor

Universal Editor is a native desktop application built entirely in Rust. It is a single environment for editing or converting text, images, JSON, and data files, with no web application and clean performance. The goal is to match the look and feel of a modern web application while remaining a lean, systems-level program that handles massive files without memory pressure or UI lag.

Currently tested and developed on Windows 11.

## Architecture

The application is built around a kernel and module model. A persistent shell handles windowing, GPU-accelerated rendering via [egui](https://github.com/emilk/egui), global theming, and the sidebar registry. Individual editors are registered as discrete modules in `registry.rs` and mounted into the shell on demand. This means switching from the text editor to the image editor does not spawn a new process or reload the application state.

The module registry holds two kinds of entries: **screens** (file editors that open when a matching file extension is detected) and **converters** (tools that operate on files without being tied to a single open document). Both share the same trait interface, so anything written for one module is available to any other.

The text editor uses [ropey](https://github.com/cessen/ropey), a Rope data structure that stores text as a balanced tree of chunks rather than a single contiguous string. This allows constant-time insertions and deletions anywhere in a file regardless of size. Image handling is done through the [image](https://github.com/image-rs/image) crate, compiled with only the format features actually used to keep the binary lean.

Styling is handled through a central `ColorPalette` and a `ThemeMode` enum, similar to a web apps CSS file. Buttons, modals, sidebars, and overlays all pull from the same palette, keeping the visual language consistent across every module. Light and dark mode are both supported. Typography uses embedded Ubuntu and Roboto font families compiled directly into the binary, so no system fonts are required.

## Project Goals

**Modularity** is the first priority. Code is written so that helpers, UI components, and styling primitives are shared across modules rather than duplicated. The registry pattern means adding a new editor requires defining a single struct and registering it, with no changes to the shell.

**Performance** is the second priority. The Rope structure, lazy image decoding, and GPU-direct rendering are the main way this is done. No operation should block the main thread, and memory usage should remain flat regardless of how large the open file is.

**Modern design** is the third priority. The application uses custom typography, a Tailwind CSS-like color system, consistent spacing, and smooth interactions that are on par with web-based tools, without the overhead of a browser engine.

## Running

Requires the Rust toolchain. Always build in release mode, debug builds disable compiler optimizations that are critical to egui rendering performance.

```bash
cargo run --release
```

## Current Modules

| Module          | Extensions                                                        | Description                                                       |
| --------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------- |
| Text Editor     | `.txt`, `.md`                                                     | Plaintext and Markdown editing backed by a Rope data structure    |
| Image Editor    | `.jpg`, `.jpeg`, `.png`, `.webp`, `.bmp`, `.tiff`, `.gif`, `.ico` | Non-destructive image editing, cropping, and transformation       |
| JSON Editor     | `.json`                                                           | Dual-view editor with a raw text mode and a collapsible tree view |
| Image Converter | -                                                                 | Batch conversion between any supported image formats              |
| Data Converter  | -                                                                 | Batch conversion between any supported data formats               |

## More Information

To view recent changes and match them with their versions, visit the `Patchnotes.md` file.

To view incoming changes and known bugs, visit the `todo.md` file.

To view changes that have already happened, view the `ArchivedTodo.md` file.
