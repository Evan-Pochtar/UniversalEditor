use eframe::egui;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba};
use crate::modules::helpers::image_export::export_image;
use std::path::PathBuf;
use std::sync::{Arc};
use std::thread;
use ab_glyph::{Font as AbFont, FontRef, PxScale, ScaleFont};
use super::ie_helpers::{ rgb_to_hsv, hsv_to_rgb };
use super::ie_main::{
    ImageEditor, Tool, FilterPanel, CropState, TransformHandleSet,
    FONT_UB_REG, FONT_UB_BLD, FONT_UB_ITL, FONT_RB_REG, FONT_RB_BLD, FONT_RB_ITL,
};

impl ImageEditor {
    pub(super) fn apply_brush_stroke(&mut self) {
        if let Some(img) = self.image.as_mut() {
            if !matches!(img, DynamicImage::ImageRgba8(_)) {
                *img = DynamicImage::ImageRgba8(img.to_rgba8());
            }
        }
        let buf: &mut ImageBuffer<Rgba<u8>, Vec<u8>> = match self.image.as_mut() { Some(DynamicImage::ImageRgba8(b)) => b, _ => return };
        if self.stroke_points.len() < 2 { return; }

        let width: u32 = buf.width(); let height = buf.height();
        let (r, g, b, base_a) = if self.tool == Tool::Eraser {
            if self.eraser_transparent { (0u8, 0u8, 0u8, 0u8) } else { (255u8, 255u8, 255u8, 255u8) }
        } else {
            (self.color.r(), self.color.g(), self.color.b(), self.color.a())
        };
        let radius: f32  = if self.tool == Tool::Eraser { self.eraser_size / 2.0 } else { self.brush_size / 2.0 };
        let opacity: f32 = if self.tool == Tool::Eraser { 1.0 } else { self.brush_opacity };
        let radius_sq: f32 = radius * radius;

        for i in 0..self.stroke_points.len().saturating_sub(1) {
            let (x0, y0) = self.stroke_points[i];
            let (x1, y1) = self.stroke_points[i + 1];
            let dx: f32 = x1 - x0; let dy = y1 - y0;
            let dist: f32  = (dx * dx + dy * dy).sqrt();
            let steps: usize = (dist / (radius * 0.4).max(1.0)).ceil() as usize;

            for s in 0..=steps {
                let t: f32  = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let cx: f32 = x0 + dx * t; let cy = y0 + dy * t;
                let min_x: u32 = (cx - radius).max(0.0) as u32;
                let max_x: u32 = ((cx + radius).ceil() as u32).min(width);
                let min_y: u32 = (cy - radius).max(0.0) as u32;
                let max_y: u32 = ((cy + radius).ceil() as u32).min(height);

                for py in min_y..max_y {
                    let dy_sq: f32 = (py as f32 - cy).powi(2);
                    for px in min_x..max_x {
                        let dist_sq: f32 = (px as f32 - cx).powi(2) + dy_sq;
                        if dist_sq <= radius_sq {
                            let falloff: f32 = 1.0 - (dist_sq / radius_sq).sqrt();
                            let alpha: u8   = (falloff * opacity * 255.0) as u8;
                            unsafe {
                                let pixel: Rgba<u8> = buf.unsafe_get_pixel(px, py);
                                let [er, eg, eb, ea] = pixel.0;
                                let new_pixel: Rgba<u8> = if self.tool == Tool::Eraser && self.eraser_transparent {
                                    Rgba([er, eg, eb, ea.saturating_sub(alpha)])
                                } else {
                                    let fa: u16 = alpha as u16;
                                    let base_factor: u16 = (base_a as u16 * fa) / 255;
                                    let fb: u16 = 255 - base_factor;
                                    Rgba([
                                        ((r as u16 * base_factor + er as u16 * fb) / 255) as u8,
                                        ((g as u16 * base_factor + eg as u16 * fb) / 255) as u8,
                                        ((b as u16 * base_factor + eb as u16 * fb) / 255) as u8,
                                        ((base_factor + ea as u16 * fb / 255).min(255)) as u8,
                                    ])
                                };
                                buf.unsafe_put_pixel(px, py, new_pixel);
                            }
                        }
                    }
                }
            }
        }
        self.dirty = true; self.texture_dirty = true;
    }

    pub(super) fn flood_fill(&mut self, start_x: u32, start_y: u32) {
        let img: &mut DynamicImage = match self.image.as_mut() { Some(i) => i, None => return };
        let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
        let width: u32 = buf.width(); let height: u32 = buf.height();
        let target: [u8; 4] = buf.get_pixel(start_x, start_y).0;
        let fill: [u8; 4] = [self.color.r(), self.color.g(), self.color.b(), self.color.a()];

        if target == fill { return; }
        let mut visited: Vec<bool> = vec![false; (width * height) as usize];
        let mut stack: Vec<(u32, u32)>   = Vec::with_capacity(1024);
        stack.push((start_x, start_y));

        let tolerance: i32 = 30i32;
        while let Some((x, y)) = stack.pop() {
            let idx: usize = (y * width + x) as usize;
            if visited[idx] { continue; }
            visited[idx] = true;
            let cur: [u8; 4]  = buf.get_pixel(x, y).0;
            let diff: i32 = (0..4).map(|i: usize| (cur[i] as i32 - target[i] as i32).abs()).sum();
            if diff > tolerance { continue; }
            buf.put_pixel(x, y, Rgba(fill));
            if x > 0 { stack.push((x - 1, y)); }
            if x + 1 < width { stack.push((x + 1, y)); }
            if y > 0 { stack.push((x, y - 1)); }
            if y + 1 < height { stack.push((x, y + 1)); }
        }

        self.image = Some(DynamicImage::ImageRgba8(buf));
        self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn sample_color(&mut self, x: u32, y: u32) {
        if let Some(img) = &self.image {
            let p: [u8; 4] = img.get_pixel(x, y).0;
            self.color = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            self.add_color_to_history();
            self.hex_input = super::ie_main::RgbaColor::from_egui(self.color).to_hex();
        }
    }

    pub(super) fn stamp_all_text_layers(&self, base: &DynamicImage) -> DynamicImage {
        if self.text_layers.is_empty() { return base.clone(); }

        let ub_reg: FontRef<'_> = FontRef::try_from_slice(FONT_UB_REG).expect("Ubuntu-Regular");
        let ub_bld: FontRef<'_> = FontRef::try_from_slice(FONT_UB_BLD).expect("Ubuntu-Bold");
        let ub_itl: FontRef<'_> = FontRef::try_from_slice(FONT_UB_ITL).expect("Ubuntu-Italic");
        let rb_reg: FontRef<'_> = FontRef::try_from_slice(FONT_RB_REG).expect("Roboto-Regular");
        let rb_bld: FontRef<'_> = FontRef::try_from_slice(FONT_RB_BLD).expect("Roboto-Bold");
        let rb_itl: FontRef<'_> = FontRef::try_from_slice(FONT_RB_ITL).expect("Roboto-Italic");

        let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = base.to_rgba8();
        let (iw, ih) = (buf.width(), buf.height());

        for layer in &self.text_layers {
            let font: &FontRef = match (layer.font_name.as_str(), layer.bold, layer.italic) {
                ("Roboto", true, _) => &rb_bld,
                ("Roboto", _, true) => &rb_itl,
                ("Roboto", _, _) => &rb_reg,
                (_, true, _) => &ub_bld,
                (_, _, true) => &ub_itl,
                _ => &ub_reg,
            };

            let scale: PxScale  = PxScale::from(layer.font_size);
            let scaled: ab_glyph::PxScaleFont<&FontRef<'_>> = font.as_scaled(scale);
            let line_h: f32 = layer.font_size * 1.35;
            let wrap_w: f32 = layer.box_width.unwrap_or(f32::MAX);

            let mut visual_lines: Vec<String> = Vec::new();
            for paragraph in layer.content.split('\n') {
                if paragraph.is_empty() { visual_lines.push(String::new()); continue; }
                let mut cur_line: String = String::new();
                let mut cur_w: f32 = 0.0f32;
                for word in paragraph.split_inclusive(' ') {
                    let w: f32 = word.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
                    if cur_w + w > wrap_w && !cur_line.is_empty() {
                        visual_lines.push(cur_line.trim_end().to_string());
                        cur_line = word.to_string(); cur_w = w;
                    } else { cur_line.push_str(word); cur_w += w; }
                }
                visual_lines.push(cur_line);
            }

            let actual_h = if layer.rendered_height > 0.0 { layer.rendered_height + 4.0 }
                else { visual_lines.len() as f32 * line_h + 4.0 };

            let bw: f32  = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0)) + 4.0;
            let bh: f32  = layer.box_height.unwrap_or(actual_h);
            let ibw: usize = bw.ceil() as usize;
            let ibh: usize = bh.ceil() as usize;

            let mut tbuf: Vec<[f32; 4]> = vec![[0.0; 4]; ibw * ibh];
            let (cr, cg, cb, ca) = (
                layer.color.r() as f32 / 255.0, layer.color.g() as f32 / 255.0,
                layer.color.b() as f32 / 255.0, layer.color.a() as f32 / 255.0,
            );
            let put = |tbuf: &mut Vec<[f32; 4]>, tx: i32, ty: i32, cov: f32| {
                if tx < 0 || ty < 0 || tx >= ibw as i32 || ty >= ibh as i32 { return; }
                let idx: usize   = ty as usize * ibw + tx as usize;
                let src_a: f32 = (cov * ca).min(1.0);
                let dst: &mut [f32; 4]   = &mut tbuf[idx];
                let out_a: f32 = src_a + dst[3] * (1.0 - src_a);
                if out_a < 1e-5 { return; }
                dst[0] = (cr * src_a + dst[0] * dst[3] * (1.0 - src_a)) / out_a;
                dst[1] = (cg * src_a + dst[1] * dst[3] * (1.0 - src_a)) / out_a;
                dst[2] = (cb * src_a + dst[2] * dst[3] * (1.0 - src_a)) / out_a;
                dst[3] = out_a;
            };

            for (line_idx, line) in visual_lines.iter().enumerate() {
                let base_y: f32  = line_idx as f32 * line_h + scaled.ascent();
                let mut cx2: f32 = 0.0f32;
                for ch in line.chars() {
                    let gid: ab_glyph::GlyphId  = font.glyph_id(ch);
                    let adv: f32  = scaled.h_advance(gid);
                    let glyph: ab_glyph::Glyph = gid.with_scale_and_position(scale, ab_glyph::point(cx2, 0.0));
                    if let Some(outlined) = font.outline_glyph(glyph) {
                        let bounds: ab_glyph::Rect = outlined.px_bounds();
                        outlined.draw(|gx, gy, cov| {
                            let tx: i32 = (bounds.min.x + gx as f32) as i32;
                            let ty: i32 = (base_y + bounds.min.y + gy as f32) as i32;
                            put(&mut tbuf, tx, ty, cov);
                        });
                    }
                    if layer.underline {
                        let uly: i32 = (base_y + scaled.descent() + 2.0) as i32;
                        for ux in cx2 as i32..(cx2 + adv) as i32 { put(&mut tbuf, ux, uly, 1.0); }
                    }
                    cx2 += adv;
                }
            }

            let rcx: f32 = layer.img_x + bw / 2.0;
            let rcy: f32 = layer.img_y + bh / 2.0;
            let angle_rad: f32 = layer.rotation.to_radians();
            let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
            let half_w: f32 = bw / 2.0; let half_h: f32 = bh / 2.0;

            let corners: [(f32, f32); 4] = [
                (rcx - half_w * cos_a + half_h * sin_a, rcy - half_w * sin_a - half_h * cos_a),
                (rcx + half_w * cos_a + half_h * sin_a, rcy + half_w * sin_a - half_h * cos_a),
                (rcx + half_w * cos_a - half_h * sin_a, rcy + half_w * sin_a + half_h * cos_a),
                (rcx - half_w * cos_a - half_h * sin_a, rcy - half_w * sin_a + half_h * cos_a),
            ];
            let min_x: i32 = corners.iter().map(|c: &(f32, f32)| c.0).fold(f32::MAX, f32::min).max(0.0) as i32;
            let max_x: i32 = (corners.iter().map(|c: &(f32, f32)| c.0).fold(f32::MIN, f32::max).ceil() as i32).min(iw as i32);
            let min_y: i32 = corners.iter().map(|c: &(f32, f32)| c.1).fold(f32::MAX, f32::min).max(0.0) as i32;
            let max_y: i32 = (corners.iter().map(|c: &(f32, f32)| c.1).fold(f32::MIN, f32::max).ceil() as i32).min(ih as i32);

            for py in min_y..max_y {
                for px in min_x..max_x {
                    let dx: f32 = px as f32 - rcx; let dy: f32 = py as f32 - rcy;
                    let ux: f32 = dx * cos_a + dy * sin_a;
                    let uy: f32 = -dx * sin_a + dy * cos_a;
                    let tx: i32 = (ux + half_w) as i32; let ty: i32 = (uy + half_h) as i32;
                    if tx < 0 || ty < 0 || tx >= ibw as i32 || ty >= ibh as i32 { continue; }
                    let src: [f32; 4] = tbuf[ty as usize * ibw + tx as usize];
                    let src_a: f32 = src[3];
                    if src_a < 1e-5 { continue; }
                    let dst: [u8; 4] = buf.get_pixel(px as u32, py as u32).0;
                    let dst_a: f32 = dst[3] as f32 / 255.0;
                    let out_a: f32 = (src_a + dst_a * (1.0 - src_a)).min(1.0);
                    if out_a < 1e-5 { continue; }
                    let blend = |s: f32, d: u8| -> u8 {
                        ((s * src_a + d as f32 / 255.0 * dst_a * (1.0 - src_a)) / out_a * 255.0).min(255.0) as u8
                    };
                    buf.put_pixel(px as u32, py as u32, Rgba([
                        blend(src[0], dst[0]), blend(src[1], dst[1]),
                        blend(src[2], dst[2]), (out_a * 255.0).min(255.0) as u8,
                    ]));
                }
            }
        }

        DynamicImage::ImageRgba8(buf)
    }

    pub(super) fn hit_text_layer(&self, pos: egui::Pos2) -> Option<u64> {
        for layer in self.text_layers.iter().rev() {
            let anchor = self.image_to_screen(layer.img_x, layer.img_y);
            if layer.screen_rect(anchor, self.zoom).contains(pos) { return Some(layer.id); }
        }
        None
    }

    pub(super) fn text_transform_handles(&self) -> Option<TransformHandleSet> {
        let id: u64 = self.selected_text?;
        let layer: &super::ie_main::TextLayer = self.text_layers.iter().find(|l| l.id == id)?;
        let anchor: egui::Pos2 = self.image_to_screen(layer.img_x, layer.img_y);
        Some(TransformHandleSet::with_rotation(layer.screen_rect(anchor, self.zoom), layer.rotation.to_radians()))
    }

    pub(super) fn commit_or_discard_active_text(&mut self) {
        if let Some(id) = self.selected_text {
            let empty: bool = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id).map(|l: &super::ie_main::TextLayer| l.content.is_empty()).unwrap_or(true);
            if empty { self.text_layers.retain(|l: &super::ie_main::TextLayer| l.id != id); }
        }
        self.selected_text = None; self.editing_text = false;
        self.text_drag = None; self.text_cursor = 0; self.text_sel_anchor = None;
    }

    pub(super) fn process_text_input(&mut self, ctx: &egui::Context) {
        if !self.editing_text || self.selected_text.is_none() { return; }
        let id: u64 = self.selected_text.unwrap();
        let (events, _shift, ctrl) = ctx.input(|i: &egui::InputState| {
            (i.events.clone(), i.modifiers.shift, i.modifiers.ctrl || i.modifiers.mac_cmd)
        });
        for event in &events {
            let cursor: usize = self.text_cursor;
            let sel: Option<usize> = self.text_sel_anchor;
            match event {
                egui::Event::Text(t) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut super::ie_main::TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        }
                        let c: usize = self.text_cursor; layer.content.insert_str(c, t); self.text_cursor += t.len();
                    }
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut super::ie_main::TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor; layer.content.insert(c, '\n'); self.text_cursor += 1;
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut super::ie_main::TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        } else if cursor > 0 {
                            let prev: usize = layer.content[..cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                            layer.content.drain(prev..cursor); self.text_cursor = prev;
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Delete, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        } else if cursor < layer.content.len() {
                            let next: usize = layer.content[cursor..].char_indices().nth(1).map(|(i, _)| cursor + i).unwrap_or(layer.content.len());
                            layer.content.drain(cursor..next);
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::ArrowLeft, pressed: true, modifiers, .. } => {
                    let shift: bool = modifiers.shift;
                    if let Some(layer) = self.text_layers.iter().find(|l| l.id == id) {
                        if !shift && sel.is_some() {
                            self.text_cursor = cursor.min(sel.unwrap()); self.text_sel_anchor = None;
                        } else {
                            if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                            if cursor > 0 {
                                let prev: usize = layer.content[..cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                                self.text_cursor = prev;
                            }
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::ArrowRight, pressed: true, modifiers, .. } => {
                    let shift: bool = modifiers.shift;
                    if let Some(layer) = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id) {
                        if !shift && sel.is_some() {
                            self.text_cursor = cursor.max(sel.unwrap()); self.text_sel_anchor = None;
                        } else {
                            if shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                            if cursor < layer.content.len() {
                                let next: usize = layer.content[cursor..].char_indices().nth(1).map(|(i, _)| cursor + i).unwrap_or(layer.content.len());
                                self.text_cursor = next;
                            }
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Home, pressed: true, modifiers, .. } => {
                    if modifiers.shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                    else if !modifiers.shift { self.text_sel_anchor = None; }
                    self.text_cursor = 0;
                }
                egui::Event::Key { key: egui::Key::End, pressed: true, modifiers, .. } => {
                    let len: usize = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id).map(|l: &super::ie_main::TextLayer| l.content.len()).unwrap_or(0);
                    if modifiers.shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                    else if !modifiers.shift { self.text_sel_anchor = None; }
                    self.text_cursor = len;
                }
                egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. }
                    if modifiers.ctrl || modifiers.mac_cmd =>
                {
                    let len: usize = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id).map(|l| l.content.len()).unwrap_or(0);
                    self.text_sel_anchor = Some(0); self.text_cursor = len;
                }
                egui::Event::Copy => {
                    if let (Some(anchor), Some(layer)) = (sel, self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id)) {
                        let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                        if lo < hi && hi <= layer.content.len() { ctx.copy_text(layer.content[lo..hi].to_string()); }
                    }
                }
                egui::Event::Cut => {
                    if let Some(anchor) = sel {
                        if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut super::ie_main::TextLayer| l.id == id) {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            if lo < hi && hi <= layer.content.len() {
                                ctx.copy_text(layer.content[lo..hi].to_string());
                                layer.content.drain(lo..hi);
                                self.text_cursor = lo; self.text_sel_anchor = None;
                            }
                        }
                    }
                }
                egui::Event::Paste(text) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut super::ie_main::TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor; layer.content.insert_str(c, text); self.text_cursor += text.len();
                    }
                }
                _ => {}
            }
        }
        if let Some(layer) = self.text_layers.iter().find(|l: &&super::ie_main::TextLayer| l.id == id) {
            let clamp = |c: usize| -> usize {
                let c: usize = c.min(layer.content.len());
                if layer.content.is_char_boundary(c) { c }
                else { (0..c).rev().find(|&i| layer.content.is_char_boundary(i)).unwrap_or(0) }
            };
            self.text_cursor = clamp(self.text_cursor);
            if let Some(a) = self.text_sel_anchor { self.text_sel_anchor = Some(clamp(a)); }
        }
        let _ = ctrl;
    }

    pub(super) fn apply_crop(&mut self) {
        let img: &DynamicImage = match &self.image { Some(i) => i, None => return };
        let (s, e) = match (self.crop_state.start, self.crop_state.end) { (Some(s), Some(e)) => (s, e), _ => return };

        let x0: u32 = s.0.min(e.0).max(0.0) as u32; let y0: u32 = s.1.min(e.1).max(0.0) as u32;
        let x1: u32 = (s.0.max(e.0) as u32).min(img.width()); let y1: u32 = (s.1.max(e.1) as u32).min(img.height());

        if x1 <= x0 || y1 <= y0 { return; }
        let cropped: DynamicImage = img.crop_imm(x0, y0, x1 - x0, y1 - y0);
        self.resize_w = cropped.width(); self.resize_h = cropped.height();
        self.image = Some(cropped); self.texture_dirty = true; self.dirty = true;
        self.crop_state = CropState::default(); self.fit_on_next_frame = true;
    }

    pub(super) fn apply_brightness_contrast(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        let b: f32 = self.brightness; let c = 1.0 + self.contrast / 100.0;
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);

        self.is_processing = true; *progress.lock().unwrap() = 0.0;
        thread::spawn(move || {
            let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
            let total: usize = (buf.width() * buf.height()) as usize;
            let mut processed: i32 = 0;
            for pixel in buf.pixels_mut() {
                for i in 0..3 {
                    let val: f32 = pixel[i] as f32;
                    pixel[i] = ((val - 128.0) * c + 128.0 + b).clamp(0.0, 255.0) as u8;
                }
                processed += 1;
                if processed % 5000 == 0 { *progress.lock().unwrap() = processed as f32 / total as f32; }
            }
            *result.lock().unwrap() = Some(DynamicImage::ImageRgba8(buf));
            *progress.lock().unwrap() = 1.0;
        });
    }

    pub(super) fn apply_hue_saturation(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        let sat_factor: f32 = 1.0 + self.saturation / 100.0;
        let hue_shift: f32 = self.hue;
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);

        self.is_processing = true; *progress.lock().unwrap() = 0.0;
        thread::spawn(move || {
            let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
            for y in 0..buf.height() {
                for x in 0..buf.width() {
                    let p: [u8; 4] = buf.get_pixel(x, y).0;
                    let (h, s, v) = rgb_to_hsv(p[0], p[1], p[2]);
                    let (nr, ng, nb) = hsv_to_rgb((h + hue_shift).rem_euclid(360.0), (s * sat_factor).clamp(0.0, 1.0), v);
                    buf.put_pixel(x, y, Rgba([nr, ng, nb, p[3]]));
                }
                if y % 10 == 0 { *progress.lock().unwrap() = y as f32 / buf.height() as f32; }
            }
            *result.lock().unwrap() = Some(DynamicImage::ImageRgba8(buf));
            *progress.lock().unwrap() = 1.0;
        });
    }

    pub(super) fn apply_blur(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        let radius: f32 = self.blur_radius;
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);

        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            *result.lock().unwrap() = Some(img.blur(radius));
            *progress.lock().unwrap() = 1.0;
        });
    }

    pub(super) fn apply_sharpen(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        let amount: f32 = self.sharpen_amount;
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);

        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            *result.lock().unwrap() = Some(img.unsharpen(amount, 0));
            *progress.lock().unwrap() = 1.0;
        });
    }

    pub(super) fn apply_grayscale(&mut self) {
        if let Some(img) = &self.image {
            self.image = Some(DynamicImage::ImageRgba8(img.grayscale().to_rgba8()));
            self.texture_dirty = true; self.dirty = true;
        }
    }

    pub(super) fn apply_invert(&mut self) {
        let img: &mut DynamicImage = match self.image.as_mut() { Some(i) => i, None => return };
        let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>>    = img.to_rgba8();
        let mut pixels: image::FlatSamples<&mut [u8]> = buf.as_flat_samples_mut();

        for chunk in pixels.as_mut_slice().chunks_exact_mut(4) {
            chunk[0] = 255 - chunk[0]; chunk[1] = 255 - chunk[1]; chunk[2] = 255 - chunk[2];
        }
        self.image = Some(DynamicImage::ImageRgba8(buf)); self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_sepia(&mut self) {
        let img: &mut DynamicImage = match self.image.as_mut() { Some(i) => i, None => return };
        let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();

        for pixel in buf.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let (rf, gf, bf) = (r as f32, g as f32, b as f32);
            pixel.0 = [
                (rf * 0.393 + gf * 0.769 + bf * 0.189).min(255.0) as u8,
                (rf * 0.349 + gf * 0.686 + bf * 0.168).min(255.0) as u8,
                (rf * 0.272 + gf * 0.534 + bf * 0.131).min(255.0) as u8,
                a,
            ];
        }
        self.image = Some(DynamicImage::ImageRgba8(buf)); self.texture_dirty = true; self.dirty = true;
    }

    fn transform_text_rotate_cw(&mut self, _old_w: u32, old_h: u32) {
        for layer in &mut self.text_layers {
            let bw: f32 = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let bh: f32 = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cx: f32 = layer.img_x + bw / 2.0; let cy: f32 = layer.img_y + bh / 2.0;
            let new_cx: f32 = old_h as f32 - cy; let new_cy: f32 = cx;
            layer.img_x = new_cx - bh / 2.0; layer.img_y = new_cy - bw / 2.0;
            std::mem::swap(&mut layer.box_width, &mut layer.box_height);
            layer.rotation = (layer.rotation + 90.0).rem_euclid(360.0);
        }
    }

    fn transform_text_rotate_ccw(&mut self, old_w: u32, _old_h: u32) {
        for layer in &mut self.text_layers {
            let bw: f32 = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let bh: f32 = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cx: f32 = layer.img_x + bw / 2.0; let cy: f32 = layer.img_y + bh / 2.0;
            let new_cx: f32 = cy; let new_cy: f32 = old_w as f32 - cx;
            layer.img_x = new_cx - bh / 2.0; layer.img_y = new_cy - bw / 2.0;
            std::mem::swap(&mut layer.box_width, &mut layer.box_height);
            layer.rotation = (layer.rotation - 90.0).rem_euclid(360.0);
        }
    }

    fn transform_text_flip_h(&mut self, old_w: u32) {
        for layer in &mut self.text_layers {
            let bw: f32 = layer.box_width.unwrap_or_else(|| layer.auto_width(1.0));
            let cx: f32 = layer.img_x + bw / 2.0;
            layer.img_x = old_w as f32 - cx - bw / 2.0;
            layer.rotation = -(layer.rotation).rem_euclid(360.0);
        }
    }

    fn transform_text_flip_v(&mut self, old_h: u32) {
        for layer in &mut self.text_layers {
            let bh: f32 = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cy: f32 = layer.img_y + bh / 2.0;
            layer.img_y = old_h as f32 - cy - bh / 2.0;
            layer.rotation = -(layer.rotation).rem_euclid(360.0);
        }
    }

    pub(super) fn apply_flip_h(&mut self) {
        let (old_w, flipped) = match &self.image { Some(img) => (img.width(), img.fliph()), None => return };
        self.transform_text_flip_h(old_w); self.image = Some(flipped);
        self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_flip_v(&mut self) {
        let (old_h, flipped) = match &self.image { Some(img) => (img.height(), img.flipv()), None => return };
        self.transform_text_flip_v(old_h); self.image = Some(flipped);
        self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_rotate_cw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image { Some(img) => (img.width(), img.height(), img.rotate90()), None => return };
        self.transform_text_rotate_cw(old_w, old_h); self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width(); self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true; self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn apply_rotate_ccw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image { Some(img) => (img.width(), img.height(), img.rotate270()), None => return };
        self.transform_text_rotate_ccw(old_w, old_h); self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width(); self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true; self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn apply_resize(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        if self.resize_w == 0 || self.resize_h == 0 { return; }
        let (w, h, stretch) = (self.resize_w, self.resize_h, self.resize_stretch);
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);

        self.is_processing = true;
        thread::spawn(move || {
            *progress.lock().unwrap() = 0.5;
            let final_img: DynamicImage = if stretch {
                img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
            } else {
                let mut new_buf: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(w, h, Rgba([255, 255, 255, 255]));
                image::imageops::overlay(&mut new_buf, &img, 0, 0);
                DynamicImage::ImageRgba8(new_buf)
            };
            *result.lock().unwrap() = Some(final_img);
            *progress.lock().unwrap() = 1.0;
        });
    }

    pub(super) fn export_image_to_file(&mut self) -> Result<PathBuf, String> {
        let img: &DynamicImage = match &self.image { Some(i) => i, None => return Err("No image to export".to_string()) };
        let composite: DynamicImage = self.stamp_all_text_layers(img);
        let default_name: &str = self.file_path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("export");
        let filename: String = format!("{}.{}", default_name, self.export_format.extension());
        let path: PathBuf = match rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter(self.export_format.as_str(), &[self.export_format.extension()])
            .save_file()
        {
            Some(p) => p, None => return Err("Export cancelled".to_string()),
        };
        export_image(&composite, &path, self.export_format, self.export_jpeg_quality, 6, 100.0, self.export_auto_scale_ico)?;
        self.filter_panel = FilterPanel::None;
        Ok(path)
    }
}
