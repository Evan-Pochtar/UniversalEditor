use eframe::egui;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba};
use crate::modules::helpers::image_export::export_image;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::thread;
use ab_glyph::{Font as AbFont, FontRef, PxScale, ScaleFont, point};
use crate::style::{FONT_UB_REG, FONT_UB_BLD, FONT_UB_ITL, FONT_RB_REG, FONT_RB_BLD, FONT_RB_ITL};
use super::ie_helpers::{ rgb_to_hsv, hsv_to_rgb, srgb_to_linear };
use super::ie_main::{
    ImageEditor, Tool, FilterPanel, TextLayer, BrushSettings, CropState, TransformHandleSet,
    BrushShape, BrushTextureMode, RetouchMode, LayerKind, RgbaColor,
};

static FONT_CACHE: OnceLock<[FontRef<'static>; 6]> = OnceLock::new();

fn cached_fonts() -> &'static [FontRef<'static>; 6] {
    FONT_CACHE.get_or_init(|| [
        FontRef::try_from_slice(FONT_UB_REG).expect("ub"),
        FontRef::try_from_slice(FONT_UB_BLD).expect("ub-b"),
        FontRef::try_from_slice(FONT_UB_ITL).expect("ub-i"),
        FontRef::try_from_slice(FONT_RB_REG).expect("rb"),
        FontRef::try_from_slice(FONT_RB_BLD).expect("rb-b"),
        FontRef::try_from_slice(FONT_RB_ITL).expect("rb-i"),
    ])
}

impl ImageEditor {
    pub(super) fn apply_brush_stroke(&mut self) {
        let active_id = self.active_layer_id;
        let (kind, locked) = self.layers.iter()
            .find(|l| l.id == active_id)
            .map(|l| (l.kind, l.locked))
            .unwrap_or((LayerKind::Background, false));
        if locked || matches!(kind, LayerKind::Text) { return; }

        if kind == LayerKind::Image {
            self.apply_brush_stroke_on_image_layer();
            return;
        }

        let swapped_bg: Option<DynamicImage> = if matches!(kind, LayerKind::Raster) {
            self.layer_images.remove(&active_id).map(|layer_img| {
                self.image.replace(layer_img).unwrap_or_else(|| {
                    DynamicImage::ImageRgba8(ImageBuffer::new(1, 1))
                })
            })
        } else { None };

        if let Some(img) = self.image.as_mut() {
            if !matches!(img, DynamicImage::ImageRgba8(_)) {
                *img = DynamicImage::ImageRgba8(img.to_rgba8());
            }
        }
        let buf: &mut ImageBuffer<Rgba<u8>, Vec<u8>> = match self.image.as_mut() {
            Some(DynamicImage::ImageRgba8(b)) => b, _ => return
        };
        if self.stroke_points.len() < 2 { return; }

        let width: u32  = buf.width();
        let height: u32 = buf.height();

        let is_eraser: bool = self.tool == Tool::Eraser;
        let eraser_transparent_eff = is_eraser && (self.eraser_transparent || matches!(kind, LayerKind::Raster));
        let (r, g, b_ch, base_a) = if is_eraser {
            if eraser_transparent_eff { (0u8, 0u8, 0u8, 0u8) } else { (255u8, 255u8, 255u8, 255u8) }
        } else {
            (self.color.r(), self.color.g(), self.color.b(), self.color.a())
        };

        let bs: BrushSettings = self.brush.clone();
        let radius: f32 = if is_eraser { self.eraser_size / 2.0 } else { bs.size / 2.0 };
        let opacity: f32 = if is_eraser { 1.0 } else { bs.opacity };
        let softness: f32 = if is_eraser { 0.0 } else { bs.softness };
        let flow: f32 = if is_eraser { 1.0 } else { bs.flow };
        let shape: BrushShape  = if is_eraser { BrushShape::Circle } else { bs.shape };
        let scatter: f32 = if is_eraser { 0.0 } else { bs.scatter };
        let angle_rad: f32 = if is_eraser { 0.0 } else { bs.angle.to_radians() };
        let angle_jitter_rad: f32 = if is_eraser { 0.0 } else { bs.angle_jitter.to_radians() };
        let tex_mode: BrushTextureMode = if is_eraser { BrushTextureMode::None } else { bs.texture_mode };
        let tex_str: f32 = if is_eraser { 0.0 } else { bs.texture_strength };
        let aspect: f32 = bs.aspect_ratio.clamp(0.05, 1.0);
        let wetness: f32 = if is_eraser { 0.0 } else { bs.wetness.clamp(0.0, 1.0) };
        let spray_mode: bool = !is_eraser && bs.spray_mode;

        let step_dist: f32 = if spray_mode {
            radius.max(1.0)
        } else {
            (radius * 2.0 * bs.step).max(0.5)
        };

        if spray_mode {
            let mut dr_x0: u32 = u32::MAX;
            let mut dr_y0: u32 = u32::MAX;
            let mut dr_x1: u32 = 0;
            let mut dr_y1: u32 = 0;
            for (si, &(cx, cy)) in self.stroke_points.iter().enumerate() {
                let n = bs.spray_particles as usize;
                let spray_x0: u32 = ((cx - radius - 1.0).max(0.0)) as u32;
                let spray_y0: u32 = ((cy - radius - 1.0).max(0.0)) as u32;
                let spray_x1: u32 = ((cx + radius + 1.0).max(0.0).ceil() as u32).min(width);
                let spray_y1: u32 = ((cy + radius + 1.0).max(0.0).ceil() as u32).min(height);
                
                dr_x0 = dr_x0.min(spray_x0);
                dr_y0 = dr_y0.min(spray_y0);
                dr_x1 = dr_x1.max(spray_x1);
                dr_y1 = dr_y1.max(spray_y1);

                for pi in 0..n {
                    let seed: u64 = si as u64 * 65537 + pi as u64 * 1031 + cx as u64 * 17 + cy as u64 * 13;
                    let r1: f32 = brush_rand(seed).sqrt();
                    let r2: f32 = brush_rand(seed.wrapping_add(1));
                    let particle_angle: f32 = r2 * std::f32::consts::TAU;
                    let dist: f32 = r1 * radius;
                    let px_f: f32 = cx + particle_angle.cos() * dist;
                    let py_f: f32 = cy + particle_angle.sin() * dist;
                    if px_f < 0.0 || py_f < 0.0 { continue; }
                    let px: u32 = px_f as u32;
                    let py: u32 = py_f as u32;
                    if px >= width || py >= height { continue; }

                    let t: f32 = dist / radius;
                    let falloff: f32 = 1.0 - t * t;
                    let alpha: u8 = ((falloff * flow * opacity) * 255.0).clamp(0.0, 255.0) as u8;
                    if alpha == 0 { continue; }

                    unsafe {
                        let pixel: Rgba<u8> = buf.unsafe_get_pixel(px, py);
                        let [er, eg, eb, ea] = pixel.0;
                        let fa: u16 = alpha as u16;
                        let base_factor: u16 = (base_a as u16 * fa) / 255;
                        let fb: u16 = 255 - base_factor;
                        buf.unsafe_put_pixel(px, py, Rgba([
                            ((r as u16 * base_factor + er as u16 * fb) / 255) as u8,
                            ((g as u16 * base_factor + eg as u16 * fb) / 255) as u8,
                            ((b_ch as u16 * base_factor + eb as u16 * fb) / 255) as u8,
                            ((base_factor + ea as u16 * fb / 255).min(255)) as u8,
                        ]));
                    }
                }
            }
            self.dirty = true;
            if dr_x1 > dr_x0 && dr_y1 > dr_y0 {
                self.expand_dirty_rect(dr_x0, dr_y0, dr_x1, dr_y1);
            }
            self.texture_dirty = true;
            if let Some(old_bg) = swapped_bg { self.restore_layer_swap(active_id, old_bg); } else { self.promote_dirty_to_composite(); }
            return;
        }

        let mut dr_x0: u32 = u32::MAX;
        let mut dr_y0: u32 = u32::MAX;
        let mut dr_x1: u32 = 0;
        let mut dr_y1: u32 = 0;

        let backdrop_raw: Option<(*const u8, u32, u32)> = self.stroke_backdrop.as_ref().map(|b| {
            (b.as_raw().as_ptr() as *const u8, b.width(), b.height())
        });

        for i in 0..self.stroke_points.len().saturating_sub(1) {
            let (x0, y0) = self.stroke_points[i];
            let (x1, y1) = self.stroke_points[i + 1];
            let dx: f32 = x1 - x0;
            let dy: f32 = y1 - y0;
            let seg_len: f32 = (dx * dx + dy * dy).sqrt();
            let steps: usize = (seg_len / step_dist).ceil() as usize;

            for s in 0..=steps {
                let t: f32 = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let mut cx: f32 = x0 + dx * t;
                let mut cy: f32 = y0 + dy * t;

                let stamp_seed: u64 = (i as u64).wrapping_mul(99991)
                    .wrapping_add(s as u64 * 7919)
                    .wrapping_add(cx as u64 * 131)
                    .wrapping_add(cy as u64 * 97);

                if scatter > 0.0 {
                    let sx: f32 = (brush_rand(stamp_seed) * 2.0 - 1.0) * scatter;
                    let sy: f32 = (brush_rand(stamp_seed.wrapping_add(1)) * 2.0 - 1.0) * scatter;
                    cx += sx; cy += sy;
                }

                let cur_angle: f32 = if angle_jitter_rad > 0.0 {
                    let j: f32 = (brush_rand(stamp_seed.wrapping_add(2)) * 2.0 - 1.0) * angle_jitter_rad;
                    angle_rad + j
                } else {
                    angle_rad
                };

                let min_x: u32 = ((cx - radius - 1.0).max(0.0)) as u32;
                let max_x: u32 = ((cx + radius + 1.0).max(0.0).ceil() as u32).min(width);
                let min_y: u32 = ((cy - radius - 1.0).max(0.0)) as u32;
                let max_y: u32 = ((cy + radius + 1.0).max(0.0).ceil() as u32).min(height);

                dr_x0 = dr_x0.min(min_x);
                dr_y0 = dr_y0.min(min_y);
                dr_x1 = dr_x1.max(max_x);
                dr_y1 = dr_y1.max(max_y);

                for py in min_y..max_y {
                    let dy_local: f32 = py as f32 - cy;
                    for px in min_x..max_x {
                        let dx_local: f32 = px as f32 - cx;

                        let falloff: f32 = brush_shape_falloff(
                            dx_local, dy_local, radius, aspect, cur_angle, softness, shape,
                        );
                        if falloff <= 0.0 { continue; }

                        let tex_mul: f32 = if tex_str > 0.0 {
                            let noise: f32 = brush_texture_noise(px, py, tex_mode);
                            1.0 - tex_str * noise
                        } else { 1.0 };

                        let alpha: u8 = (falloff * flow * opacity * tex_mul * 255.0).clamp(0.0, 255.0) as u8;
                        if alpha == 0 { continue; }

                        unsafe {
                            let pixel: Rgba<u8> = buf.unsafe_get_pixel(px, py);
                            let [er, eg, eb, ea] = pixel.0;

                            let new_pixel: Rgba<u8> = if is_eraser && eraser_transparent_eff {
                                Rgba([er, eg, eb, ea.saturating_sub(alpha)])
                            } else {
                                let fa: u16 = alpha as u16;
                                let base_factor: u16 = (base_a as u16 * fa) / 255;
                                let fb: u16 = 255 - base_factor;

                                let (paint_r, paint_g, paint_b) = if wetness > 0.0 {
                                    let (vis_r, vis_g, vis_b) = if let Some((bd_ptr, bd_w, bd_h)) = backdrop_raw {
                                        if px < bd_w && py < bd_h {
                                            let off = ((py * bd_w + px) * 4) as usize;
                                            let bd = std::slice::from_raw_parts(bd_ptr.add(off), 4);
                                            let la = ea as f32 / 255.0;
                                            let bda = bd[3] as f32 / 255.0;
                                            let out_a = la + bda * (1.0 - la);
                                            if out_a > 1e-6 {
                                                (
                                                    ((er as f32/255.0*la + bd[0] as f32/255.0*bda*(1.0-la))/out_a*255.0) as u8,
                                                    ((eg as f32/255.0*la + bd[1] as f32/255.0*bda*(1.0-la))/out_a*255.0) as u8,
                                                    ((eb as f32/255.0*la + bd[2] as f32/255.0*bda*(1.0-la))/out_a*255.0) as u8,
                                                )
                                            } else { (er, eg, eb) }
                                        } else { (er, eg, eb) }
                                    } else { (er, eg, eb) };
                                    let w: f32 = wetness;
                                    (
                                        ((r as f32 * (1.0 - w) + vis_r as f32 * w) as u16).min(255) as u8,
                                        ((g as f32 * (1.0 - w) + vis_g as f32 * w) as u16).min(255) as u8,
                                        ((b_ch as f32 * (1.0 - w) + vis_b as f32 * w) as u16).min(255) as u8,
                                    )
                                } else { (r, g, b_ch) };

                                Rgba([
                                    ((paint_r as u16 * base_factor + er as u16 * fb) / 255) as u8,
                                    ((paint_g as u16 * base_factor + eg as u16 * fb) / 255) as u8,
                                    ((paint_b as u16 * base_factor + eb as u16 * fb) / 255) as u8,
                                    ((base_factor + ea as u16 * fb / 255).min(255)) as u8,
                                ])
                            };
                            buf.unsafe_put_pixel(px, py, new_pixel);
                        }
                    }
                }
            }
        }
        self.dirty = true;
        if dr_x1 > dr_x0 && dr_y1 > dr_y0 {
            self.expand_dirty_rect(dr_x0, dr_y0, dr_x1, dr_y1);
        }
        self.texture_dirty = true;

        if let Some(old_bg) = swapped_bg { self.restore_layer_swap(active_id, old_bg); } else { self.promote_dirty_to_composite(); }
    }

    fn promote_dirty_to_composite(&mut self) {
        if self.layers.iter().any(|l| l.visible && l.kind == LayerKind::Image) {
            let rect = self.texture_dirty_rect.take();
            self.texture_dirty = false;
            self.composite_dirty = true;
            if let Some(r) = rect {
                match &mut self.composite_dirty_rect {
                    None => self.composite_dirty_rect = Some(r),
                    Some(cr) => { cr[0]=cr[0].min(r[0]); cr[1]=cr[1].min(r[1]); cr[2]=cr[2].max(r[2]); cr[3]=cr[3].max(r[3]); }
                }
            }
            self.texture_dirty = true;
        }
    }

    fn restore_layer_swap(&mut self, active_id: u64, old_bg: DynamicImage) {
        let rect = self.texture_dirty_rect.take();
        self.texture_dirty = false;
        if let Some(painted) = self.image.take() { self.layer_images.insert(active_id, painted); }
        self.image = Some(old_bg);
        if let Some(r) = rect {
            match self.raster_layer_dirty_rects.get_mut(&active_id) {
                None => { self.raster_layer_dirty_rects.insert(active_id, r); }
                Some(cr) => { cr[0]=cr[0].min(r[0]); cr[1]=cr[1].min(r[1]); cr[2]=cr[2].max(r[2]); cr[3]=cr[3].max(r[3]); }
            }
        }
        self.raster_layer_texture_dirty.insert(active_id);
    }

    pub(super) fn flood_fill(&mut self, start_x: u32, start_y: u32) {
        let active_id = self.active_layer_id;
        let (kind, locked) = self.layers.iter().find(|l| l.id == active_id)
            .map(|l| (l.kind, l.locked)).unwrap_or((LayerKind::Background, false));
        if locked || matches!(kind, LayerKind::Text) { return; }

        if kind == LayerKind::Image {
            self.flood_fill_image_layer(start_x, start_y);
            return;
        }

        let swapped_bg: Option<DynamicImage> = if matches!(kind, LayerKind::Raster) {
            self.layer_images.remove(&active_id).map(|layer_img| {
                self.image.replace(layer_img).unwrap_or_else(|| DynamicImage::ImageRgba8(ImageBuffer::new(1,1)))
            })
        } else { None };

        let img: &mut DynamicImage = match self.image.as_mut() { Some(i) => i, None => return };
        let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = img.to_rgba8();
        let width: u32 = buf.width(); let height: u32 = buf.height();
        let target: [u8; 4] = buf.get_pixel(start_x, start_y).0;
        let fill: [u8; 4] = [self.color.r(), self.color.g(), self.color.b(), self.color.a()];

        if target == fill {
            if let Some(old_bg) = swapped_bg {
                self.layer_images.insert(active_id, self.image.take().unwrap());
                self.image = Some(old_bg);
            }
            return;
        }
        let mut visited: Vec<bool> = vec![false; (width * height) as usize];
        let mut stack: Vec<(u32, u32)> = Vec::with_capacity(1024);
        stack.push((start_x, start_y));

        let tolerance: i32 = 30i32;
        while let Some((x, y)) = stack.pop() {
            let idx: usize = (y * width + x) as usize;
            if visited[idx] { continue; }
            visited[idx] = true;
            let cur: [u8; 4] = buf.get_pixel(x, y).0;
            let diff: i32 = (0..4).map(|i: usize| (cur[i] as i32 - target[i] as i32).abs()).sum();
            if diff > tolerance { continue; }
            buf.put_pixel(x, y, Rgba(fill));
            if x > 0 { stack.push((x - 1, y)); }
            if x + 1 < width { stack.push((x + 1, y)); }
            if y > 0 { stack.push((x, y - 1)); }
            if y + 1 < height { stack.push((x, y + 1)); }
        }

        let result = DynamicImage::ImageRgba8(buf);
        if let Some(old_bg) = swapped_bg {
            self.layer_images.insert(active_id, result);
            self.image = Some(old_bg);
            self.composite_dirty = true;
        } else {
            self.image = Some(result);
        }
        self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn flood_fill_image_layer(&mut self, start_x: u32, start_y: u32) {
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        let ild = match self.image_layer_data.get_mut(&iid) { Some(d) => d, None => return };
        
        let (lx_f, ly_f) = ild.canvas_to_local_f32(start_x as f32, start_y as f32);
        if lx_f < 0.0 || ly_f < 0.0 || lx_f >= ild.orig_w() as f32 || ly_f >= ild.orig_h() as f32 { return; }
        let lx = lx_f as u32; let ly = ly_f as u32;

        if !matches!(ild.image, DynamicImage::ImageRgba8(_)) {
            ild.image = DynamicImage::ImageRgba8(ild.image.to_rgba8());
        }
        let buf = if let DynamicImage::ImageRgba8(b) = &mut ild.image { b } else { return };
        
        let width = buf.width(); let height = buf.height();
        if lx >= width || ly >= height { return; }

        let target = buf.get_pixel(lx, ly).0;
        let fill = [self.color.r(), self.color.g(), self.color.b(), self.color.a()];

        if target == fill { return; }
        let mut visited = vec![false; (width * height) as usize];
        let mut stack = Vec::with_capacity(1024);
        stack.push((lx, ly));

        let tolerance = 30i32;
        let mut dr_x0 = width; let mut dr_y0 = height; let mut dr_x1 = 0u32; let mut dr_y1 = 0u32;
        
        while let Some((x, y)) = stack.pop() {
            let idx = (y * width + x) as usize;
            if visited[idx] { continue; }
            visited[idx] = true;
            let cur = buf.get_pixel(x, y).0;
            let diff: i32 = (0..4).map(|i| (cur[i] as i32 - target[i] as i32).abs()).sum();
            if diff > tolerance { continue; }
            buf.put_pixel(x, y, Rgba(fill));
            
            dr_x0 = dr_x0.min(x); dr_y0 = dr_y0.min(y);
            dr_x1 = dr_x1.max(x); dr_y1 = dr_y1.max(y);

            if x > 0 { stack.push((x - 1, y)); }
            if x + 1 < width { stack.push((x + 1, y)); }
            if y > 0 { stack.push((x, y - 1)); }
            if y + 1 < height { stack.push((x, y + 1)); }
        }

        if dr_x1 >= dr_x0 && dr_y1 >= dr_y0 {
            let entry = self.image_layer_stroke_rects.entry(iid).or_insert([width, height, 0, 0]);
            entry[0] = entry[0].min(dr_x0); entry[1] = entry[1].min(dr_y0);
            entry[2] = entry[2].max(dr_x1); entry[3] = entry[3].max(dr_y1);
        }
        
        self.image_layer_texture_dirty.insert(iid);
        self.composite_dirty = true;
        self.composite_dirty_rect = None;
        self.texture_dirty = true;
        self.dirty = true;
    }

    pub(super) fn sample_color(&mut self, x: u32, y: u32) {
        if let Some(composite) = self.composite_all_layers() {
            let p = composite.get_pixel(x, y).0;
            self.color = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            self.add_color_to_history();
            self.hex_input = RgbaColor::from_egui(self.color).to_hex();
        }
    }

    pub(super) fn stamp_single_text_layer(&self, base: &DynamicImage, tl: &TextLayer, opacity: f32) -> DynamicImage {
        let fonts = cached_fonts();
        let font: &FontRef = match (tl.font_name.as_str(), tl.bold, tl.italic) {
            ("Roboto", true, _) => &fonts[4], ("Roboto", _, true) => &fonts[5], ("Roboto", ..) => &fonts[3],
            (_, true, _)  => &fonts[1], (_, _, true) => &fonts[2], _ => &fonts[0],
        };
        let wrap_w = tl.box_width.unwrap_or(f32::MAX);
        let early_scale = PxScale::from(tl.font_size);
        let early_scaled = font.as_scaled(early_scale);

        let visual_lines: Vec<String> = if !tl.cached_lines.is_empty() {
            tl.cached_lines.clone()
        } else {
            let mut lines: Vec<String> = Vec::new();
            for paragraph in tl.content.split('\n') {
                if paragraph.is_empty() { lines.push(String::new()); continue; }
                let mut cur_line = String::new(); let mut cur_w = 0.0f32;
                for word in paragraph.split_inclusive(' ') {
                    let w: f32 = word.chars().map(|c| early_scaled.h_advance(font.glyph_id(c))).sum();
                    if w > wrap_w {
                        for ch in word.chars() {
                            let cw = early_scaled.h_advance(font.glyph_id(ch));
                            if cur_w + cw > wrap_w && !cur_line.is_empty() { lines.push(cur_line.clone()); cur_line.clear(); cur_w = 0.0; }
                            cur_line.push(ch); cur_w += cw;
                        }
                    } else if cur_w + w > wrap_w && !cur_line.is_empty() {
                        lines.push(cur_line.trim_end().to_string()); cur_line = word.to_string(); cur_w = w;
                    } else { cur_line.push_str(word); cur_w += w; }
                }
                lines.push(cur_line);
            }
            lines
        };
        let num_lines = visual_lines.len().max(1);
        let line_h = if tl.rendered_height > 0.0 { tl.rendered_height / num_lines as f32 } else { tl.font_size * 1.35 };
        let actual_h = if tl.rendered_height > 0.0 { tl.rendered_height } else { num_lines as f32 * line_h };
        let bw = tl.box_width.unwrap_or_else(|| tl.auto_width(1.0));
        let scale = PxScale::from(line_h);
        let scaled = font.as_scaled(scale);
        let ibw = bw.ceil() as usize; let ibh = actual_h.ceil() as usize;
        let mut tbuf: Vec<[f32; 4]> = vec![[0.0; 4]; ibw * ibh];
        let (cr, cg, cb) = (srgb_to_linear(tl.color.r()), srgb_to_linear(tl.color.g()), srgb_to_linear(tl.color.b()));
        let ca = tl.color.a() as f32/255.0 * opacity;
        let put = |tbuf: &mut Vec<[f32;4]>, tx: i32, ty: i32, cov: f32| {
            if tx < 0 || ty < 0 || tx >= ibw as i32 || ty >= ibh as i32 { return; }
            let idx = ty as usize * ibw + tx as usize;
            let src_a = (cov * ca).min(1.0); let dst = &mut tbuf[idx];
            let out_a = src_a + dst[3] * (1.0 - src_a);
            if out_a < 1e-5 { return; }
            dst[0] = (cr * src_a + dst[0] * dst[3] * (1.0 - src_a)) / out_a;
            dst[1] = (cg * src_a + dst[1] * dst[3] * (1.0 - src_a)) / out_a;
            dst[2] = (cb * src_a + dst[2] * dst[3] * (1.0 - src_a)) / out_a;
            dst[3] = out_a;
        };
        for (li, line) in visual_lines.iter().enumerate() {
            let base_y = li as f32 * line_h + scaled.ascent();
            let mut cx2 = 0.0f32;
            for ch in line.chars() {
                let gid = font.glyph_id(ch); let adv = scaled.h_advance(gid);
                let glyph = gid.with_scale_and_position(scale, point(cx2, 0.0));
                if let Some(o) = font.outline_glyph(glyph) {
                    let b = o.px_bounds();
                    o.draw(|gx, gy, cov| put(&mut tbuf, (b.min.x + gx as f32) as i32, (base_y + b.min.y + gy as f32) as i32, cov));
                }
                if tl.underline { let uly = (base_y + scaled.descent() + 2.0) as i32; for ux in cx2 as i32..(cx2+adv) as i32 { put(&mut tbuf, ux, uly, 1.0); } }
                cx2 += adv;
            }
        }
        let rcx = tl.img_x + bw/2.0; let rcy = tl.img_y + actual_h/2.0;
        let ar = tl.rotation.to_radians();
        let (cos_a, sin_a) = (ar.cos(), ar.sin());
        let (hw, hh) = (bw/2.0, actual_h/2.0);
        let corners = [(rcx-hw*cos_a+hh*sin_a, rcy-hw*sin_a-hh*cos_a),(rcx+hw*cos_a+hh*sin_a,rcy+hw*sin_a-hh*cos_a),(rcx+hw*cos_a-hh*sin_a,rcy+hw*sin_a+hh*cos_a),(rcx-hw*cos_a-hh*sin_a,rcy-hw*sin_a+hh*cos_a)];
        let mut buf = base.to_rgba8();
        let (iw, ih) = (buf.width(), buf.height());
        let min_xi = corners.iter().map(|c| c.0).fold(f32::MAX, f32::min).max(0.0) as i32;
        let max_xi = corners.iter().map(|c| c.0).fold(f32::MIN, f32::max).min(iw as f32).ceil() as i32;
        let min_yi = corners.iter().map(|c| c.1).fold(f32::MAX, f32::min).max(0.0) as i32;
        let max_yi = corners.iter().map(|c| c.1).fold(f32::MIN, f32::max).min(ih as f32).ceil() as i32;
        for py in min_yi..max_yi {
            for px in min_xi..max_xi {
                let lx = (px as f32 - rcx)*cos_a + (py as f32 - rcy)*sin_a + hw;
                let ly = -(px as f32 - rcx)*sin_a + (py as f32 - rcy)*cos_a + hh;
                if lx < 0.0 || ly < 0.0 || lx >= bw || ly >= actual_h { continue; }
                let tx0 = lx as usize; let ty0 = ly as usize;
                let tx1 = (tx0 + 1).min(ibw.saturating_sub(1));
                let ty1 = (ty0 + 1).min(ibh.saturating_sub(1));
                let (fx, fy) = (lx - tx0 as f32, ly - ty0 as f32);
                let lerp4 = |a: [f32;4], b: [f32;4], t: f32| -> [f32;4] {
                    [a[0]+(b[0]-a[0])*t, a[1]+(b[1]-a[1])*t, a[2]+(b[2]-a[2])*t, a[3]+(b[3]-a[3])*t]
                };
                let row0 = lerp4(tbuf[ty0*ibw+tx0], tbuf[ty0*ibw+tx1], fx);
                let row1 = lerp4(tbuf[ty1*ibw+tx0], tbuf[ty1*ibw+tx1], fx);
                let texel = lerp4(row0, row1, fy);
                if texel[3] < 1e-5 { continue; }
                let e = buf.get_pixel(px as u32, py as u32).0;
                let ea = e[3] as f32/255.0;
                let sa = texel[3]; let out_a = sa + ea*(1.0-sa);
                if out_a < 1e-5 { buf.put_pixel(px as u32, py as u32, Rgba([0,0,0,0])); continue; }
                buf.put_pixel(px as u32, py as u32, Rgba([
                    ((texel[0]*sa + e[0] as f32/255.0*ea*(1.0-sa))/out_a*255.0).clamp(0.0,255.0) as u8,
                    ((texel[1]*sa + e[1] as f32/255.0*ea*(1.0-sa))/out_a*255.0).clamp(0.0,255.0) as u8,
                    ((texel[2]*sa + e[2] as f32/255.0*ea*(1.0-sa))/out_a*255.0).clamp(0.0,255.0) as u8,
                    (out_a*255.0).clamp(0.0,255.0) as u8,
                ]));
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
        let layer: &TextLayer = self.text_layers.iter().find(|l| l.id == id)?;
        let anchor: egui::Pos2 = self.image_to_screen(layer.img_x, layer.img_y);
        Some(TransformHandleSet::with_rotation(layer.screen_rect(anchor, self.zoom), layer.rotation.to_radians()))
    }

    pub(super) fn commit_or_discard_active_text(&mut self) {
        if let Some(id) = self.selected_text {
            let empty: bool = self.text_layers.iter().find(|l: &&TextLayer| l.id == id).map(|l: &TextLayer| l.content.is_empty()).unwrap_or(true);
            if empty {
                self.text_layers.retain(|l: &TextLayer| l.id != id);
                self.layers.retain(|l| l.linked_text_id != Some(id));
                self.active_layer_id = self.layers.last().map(|l| l.id).unwrap_or(0);
            }
        }
        self.selected_text = None; self.editing_text = false;
        self.text_drag = None; self.text_cursor = 0; self.text_sel_anchor = None;
        self.composite_dirty = true;
    }

    pub(super) fn process_text_input(&mut self, ctx: &egui::Context) {
        if !self.editing_text || self.selected_text.is_none() { return; }
        let id: u64 = self.selected_text.unwrap();
        let (events, _shift, ctrl) = ctx.input(|i: &egui::InputState| {
            (i.events.clone(), i.modifiers.shift, i.modifiers.ctrl || i.modifiers.mac_cmd)
        });
        let mut text_content_changed = false;
        let mut should_deselect = false;
        for event in &events {
            let cursor: usize = self.text_cursor;
            let sel: Option<usize> = self.text_sel_anchor;
            match event {
                egui::Event::Text(t) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        }
                        let c: usize = self.text_cursor; layer.content.insert_str(c, t); self.text_cursor += t.len();
                        text_content_changed = true;
                    }
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. } => {
                    if modifiers.shift {
                        if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                            if let Some(anchor) = sel {
                                let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                                layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                            }
                            let c = self.text_cursor; layer.content.insert(c, '\n'); self.text_cursor += 1;
                            text_content_changed = true;
                        }
                    } else {
                        should_deselect = true;
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                            text_content_changed = true;
                        } else if cursor > 0 {
                            let prev: usize = layer.content[..cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                            layer.content.drain(prev..cursor); self.text_cursor = prev;
                            text_content_changed = true;
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Delete, pressed: true, .. } => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                            text_content_changed = true;
                        } else if cursor < layer.content.len() {
                            let next: usize = layer.content[cursor..].char_indices().nth(1).map(|(i, _)| cursor + i).unwrap_or(layer.content.len());
                            layer.content.drain(cursor..next);
                            text_content_changed = true;
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
                    if let Some(layer) = self.text_layers.iter().find(|l: &&TextLayer| l.id == id) {
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
                    let len: usize = self.text_layers.iter().find(|l: &&TextLayer| l.id == id).map(|l: &TextLayer| l.content.len()).unwrap_or(0);
                    if modifiers.shift && self.text_sel_anchor.is_none() { self.text_sel_anchor = Some(cursor); }
                    else if !modifiers.shift { self.text_sel_anchor = None; }
                    self.text_cursor = len;
                }
                egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. }
                    if modifiers.ctrl || modifiers.mac_cmd =>
                {
                    let len: usize = self.text_layers.iter().find(|l: &&TextLayer| l.id == id).map(|l| l.content.len()).unwrap_or(0);
                    self.text_sel_anchor = Some(0); self.text_cursor = len;
                }
                egui::Event::Copy => {
                    if let (Some(anchor), Some(layer)) = (sel, self.text_layers.iter().find(|l: &&TextLayer| l.id == id)) {
                        let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                        if lo < hi && hi <= layer.content.len() { ctx.copy_text(layer.content[lo..hi].to_string()); }
                    }
                }
                egui::Event::Cut => {
                    if let Some(anchor) = sel {
                        if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            if lo < hi && hi <= layer.content.len() {
                                ctx.copy_text(layer.content[lo..hi].to_string());
                                layer.content.drain(lo..hi);
                                self.text_cursor = lo; self.text_sel_anchor = None;
                                text_content_changed = true;
                            }
                        }
                    }
                }
                egui::Event::Paste(text) => {
                    if let Some(layer) = self.text_layers.iter_mut().find(|l: &&mut TextLayer| l.id == id) {
                        if let Some(anchor) = sel {
                            let (lo, hi) = (anchor.min(cursor), anchor.max(cursor));
                            layer.content.drain(lo..hi); self.text_cursor = lo; self.text_sel_anchor = None;
                        }
                        let c = self.text_cursor; layer.content.insert_str(c, text); self.text_cursor += text.len();
                        text_content_changed = true;
                    }
                }
                _ => {}
            }
        }
        if let Some(layer) = self.text_layers.iter().find(|l: &&TextLayer| l.id == id) {
            let clamp = |c: usize| -> usize {
                let c: usize = c.min(layer.content.len());
                if layer.content.is_char_boundary(c) { c }
                else { (0..c).rev().find(|&i| layer.content.is_char_boundary(i)).unwrap_or(0) }
            };
            self.text_cursor = clamp(self.text_cursor);
            if let Some(a) = self.text_sel_anchor { self.text_sel_anchor = Some(clamp(a)); }
        }
        if text_content_changed { self.composite_dirty = true; }
        if should_deselect { self.commit_or_discard_active_text(); }
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
        self.image = Some(cropped); self.texture_dirty = true; self.composite_dirty = true; self.dirty = true;
        self.crop_state = CropState::default(); self.fit_on_next_frame = true;
    }

    pub(super) fn apply_brightness_contrast(&mut self) {
        let img: DynamicImage = match self.active_raster_image().cloned() {
            Some(i) => i, None => match &self.image { Some(i) => i.clone(), None => return },
        };
        self.filter_target_layer_id = self.active_layer_id;
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
        let img: DynamicImage = match self.active_raster_image().cloned() {
            Some(i) => i, None => match &self.image { Some(i) => i.clone(), None => return },
        };
        self.filter_target_layer_id = self.active_layer_id;
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
        let img: DynamicImage = match self.active_raster_image().cloned() {
            Some(i) => i, None => match &self.image { Some(i) => i.clone(), None => return },
        };
        self.filter_target_layer_id = self.active_layer_id;
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
        let img: DynamicImage = match self.active_raster_image().cloned() {
            Some(i) => i, None => match &self.image { Some(i) => i.clone(), None => return },
        };
        self.filter_target_layer_id = self.active_layer_id;
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
        let id = self.active_layer_id;
        let kind = self.layers.iter().find(|l| l.id == id).map(|l| l.kind).unwrap_or(LayerKind::Background);
        match kind {
            LayerKind::Background => { if let Some(img) = &self.image { self.image = Some(DynamicImage::ImageRgba8(img.grayscale().to_rgba8())); } }
            LayerKind::Raster => { if let Some(img) = self.layer_images.get(&id) { let g = img.grayscale().to_rgba8(); self.layer_images.insert(id, DynamicImage::ImageRgba8(g)); self.raster_layer_texture_dirty.insert(id); self.raster_layer_dirty_rects.remove(&id); } }
            LayerKind::Image => {
                if let Some(iid) = self.image_layer_for_active() {
                    if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                        ild.image = DynamicImage::ImageRgba8(ild.image.grayscale().to_rgba8());
                        self.image_layer_texture_dirty.insert(iid);
                    }
                }
            }
            LayerKind::Text => return,
        }
        self.composite_dirty = true; self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_invert(&mut self) {
        let id = self.active_layer_id;
        let kind = self.layers.iter().find(|l| l.id == id).map(|l| l.kind).unwrap_or(LayerKind::Background);
        if kind == LayerKind::Image {
            if let Some(iid) = self.image_layer_for_active() {
                if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                    let mut buf = ild.image.to_rgba8();
                    for chunk in buf.as_flat_samples_mut().as_mut_slice().chunks_exact_mut(4) { chunk[0]=255-chunk[0]; chunk[1]=255-chunk[1]; chunk[2]=255-chunk[2]; }
                    ild.image = DynamicImage::ImageRgba8(buf);
                    self.image_layer_texture_dirty.insert(iid);
                }
            }
            self.composite_dirty = true; self.texture_dirty = true; self.dirty = true;
            return;
        }
        let img_ref = match kind {
            LayerKind::Background => self.image.as_ref(),
            LayerKind::Raster => self.layer_images.get(&id),
            LayerKind::Text | LayerKind::Image => return,
        };
        if let Some(src) = img_ref {
            let mut buf = src.to_rgba8();
            for chunk in buf.as_flat_samples_mut().as_mut_slice().chunks_exact_mut(4) { chunk[0] = 255 - chunk[0]; chunk[1] = 255 - chunk[1]; chunk[2] = 255 - chunk[2]; }
            let res = DynamicImage::ImageRgba8(buf);
            match kind {
                LayerKind::Background => self.image = Some(res),
                LayerKind::Raster => { self.layer_images.insert(id, res); self.raster_layer_texture_dirty.insert(id); self.raster_layer_dirty_rects.remove(&id); }
                _ => {}
            }
        }
        self.composite_dirty = true; self.texture_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_sepia(&mut self) {
        let id = self.active_layer_id;
        let kind = self.layers.iter().find(|l| l.id == id).map(|l| l.kind).unwrap_or(LayerKind::Background);
        let sepia = |buf: &mut ImageBuffer<Rgba<u8>, Vec<u8>>| {
            for pixel in buf.pixels_mut() {
                let [r, g, b, a] = pixel.0;
                let (rf, gf, bf) = (r as f32, g as f32, b as f32);
                pixel.0 = [(rf*0.393+gf*0.769+bf*0.189).min(255.0) as u8, (rf*0.349+gf*0.686+bf*0.168).min(255.0) as u8, (rf*0.272+gf*0.534+bf*0.131).min(255.0) as u8, a];
            }
        };
        if kind == LayerKind::Image {
            if let Some(iid) = self.image_layer_for_active() {
                if let Some(ild) = self.image_layer_data.get_mut(&iid) {
                    let mut buf = ild.image.to_rgba8();
                    sepia(&mut buf);
                    ild.image = DynamicImage::ImageRgba8(buf);
                    self.image_layer_texture_dirty.insert(iid);
                }
            }
            self.composite_dirty = true; self.texture_dirty = true; self.dirty = true;
            return;
        }
        let img_ref = match kind {
            LayerKind::Background => self.image.as_ref(),
            LayerKind::Raster => self.layer_images.get(&id),
            LayerKind::Text | LayerKind::Image => return,
        };
        if let Some(src) = img_ref {
            let mut buf = src.to_rgba8();
            sepia(&mut buf);
            let res = DynamicImage::ImageRgba8(buf);
            match kind {
                LayerKind::Background => self.image = Some(res),
                LayerKind::Raster => { self.layer_images.insert(id, res); self.raster_layer_texture_dirty.insert(id); self.raster_layer_dirty_rects.remove(&id); }
                _ => {}
            }
        }
        self.composite_dirty = true; self.texture_dirty = true; self.dirty = true;
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
            layer.rotation = (-layer.rotation).rem_euclid(360.0);
        }
    }

    fn transform_text_flip_v(&mut self, old_h: u32) {
        for layer in &mut self.text_layers {
            let bh: f32 = layer.box_height.unwrap_or_else(|| layer.auto_height(1.0));
            let cy: f32 = layer.img_y + bh / 2.0;
            layer.img_y = old_h as f32 - cy - bh / 2.0;
            layer.rotation = (180.0 - layer.rotation).rem_euclid(360.0);
        }
    }

    pub(super) fn init_smudge_sample(&mut self, ix: u32, iy: u32) {
        let active_id = self.active_layer_id;
        let kind = self.layers.iter().find(|l| l.id == active_id).map(|l| l.kind).unwrap_or(LayerKind::Background);
        let img_ref = match kind {
            LayerKind::Background => self.image.as_ref(),
            LayerKind::Raster => self.layer_images.get(&active_id),
            LayerKind::Text | LayerKind::Image => return,
        };
        if let Some(DynamicImage::ImageRgba8(buf)) = img_ref {
            if ix < buf.width() && iy < buf.height() {
                let p = buf.get_pixel(ix, iy);
                self.retouch_smudge_sample = [
                    p.0[0] as f32 / 255.0,
                    p.0[1] as f32 / 255.0,
                    p.0[2] as f32 / 255.0,
                    p.0[3] as f32 / 255.0,
                ];
            }
        }
    }

    pub(super) fn apply_retouch_stroke(&mut self) {
        let active_id = self.active_layer_id;
        let (kind, locked) = self.layers.iter().find(|l| l.id == active_id)
            .map(|l| (l.kind, l.locked)).unwrap_or((LayerKind::Background, false));
        if locked || matches!(kind, LayerKind::Text) { return; }

        if kind == LayerKind::Image {
            self.apply_retouch_stroke_on_image_layer();
            return;
        }

        let swapped_bg: Option<DynamicImage> = if matches!(kind, LayerKind::Raster) {
            self.layer_images.remove(&active_id).map(|layer_img| {
                self.image.replace(layer_img).unwrap_or_else(|| DynamicImage::ImageRgba8(ImageBuffer::new(1,1)))
            })
        } else { None };

        if let Some(img) = self.image.as_mut() {
            if !matches!(img, DynamicImage::ImageRgba8(_)) { *img = DynamicImage::ImageRgba8(img.to_rgba8()); }
        }
        if self.stroke_points.len() < 2 { return; }

        let mode: RetouchMode = self.retouch_mode;
        let radius: f32 = (self.retouch_size / 2.0).max(1.0);
        let strength: f32 = self.retouch_strength.clamp(0.0, 1.0);
        let softness: f32 = self.retouch_softness.clamp(0.0, 1.0);
        let stroke: Vec<(f32, f32)> = self.stroke_points.clone();
        let mut smudge: [f32; 4] = self.retouch_smudge_sample;
        let pixelate_block: u32 = self.retouch_pixelate_block.max(2);

        let buf: &mut ImageBuffer<Rgba<u8>, Vec<u8>> = match self.image.as_mut() {
            Some(DynamicImage::ImageRgba8(b)) => b, _ => return,
        };
        let width: u32 = buf.width();
        let height: u32 = buf.height();
        let stride: usize = width as usize * 4;
        let mut flat_samples = buf.as_flat_samples_mut();
        let raw: &mut [u8] = flat_samples.as_mut_slice();

        let step_dist: f32 = (radius * 0.4).max(0.5);
        let (mut dr_x0, mut dr_y0, mut dr_x1, mut dr_y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
        let mut snap_buf: Vec<u8> = Vec::new();

        let vib_delta: f32 = (strength - 0.5) * 2.0;
        let temp_delta: f32 = (strength - 0.5) * 2.0;
        let bri_delta: f32 = (strength - 0.5) * 2.0 * 45.0;

        for i in 0..stroke.len().saturating_sub(1) {
            let (x0, y0) = stroke[i];
            let (x1, y1) = stroke[i + 1];
            let (dx, dy) = (x1 - x0, y1 - y0);
            let seg_len: f32 = (dx * dx + dy * dy).sqrt();
            let steps: usize = ((seg_len / step_dist).ceil() as usize).max(1);

            for s in 0..=steps {
                let t: f32 = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let cx: f32 = x0 + dx * t;
                let cy: f32 = y0 + dy * t;
                let min_x: u32 = ((cx - radius - 1.0).max(0.0)) as u32;
                let max_x: u32 = ((cx + radius + 1.0).max(0.0).ceil() as u32).min(width);
                let min_y: u32 = ((cy - radius - 1.0).max(0.0)) as u32;
                let max_y: u32 = ((cy + radius + 1.0).max(0.0).ceil() as u32).min(height);

                dr_x0 = dr_x0.min(min_x); dr_y0 = dr_y0.min(min_y);
                dr_x1 = dr_x1.max(max_x); dr_y1 = dr_y1.max(max_y);

                match mode {
                    RetouchMode::Blur | RetouchMode::Sharpen => {
                        let sx0 = min_x.saturating_sub(1);
                        let sy0 = min_y.saturating_sub(1);
                        let sx1 = (max_x + 1).min(width);
                        let sy1 = (max_y + 1).min(height);
                        let srw = (sx1 - sx0) as usize;
                        let srh = (sy1 - sy0) as usize;
                        snap_buf.resize(srw * srh * 4, 0);
                        for (ri, py) in (sy0..sy1).enumerate() {
                            let src = py as usize * stride + sx0 as usize * 4;
                            snap_buf[ri * srw * 4..ri * srw * 4 + srw * 4].copy_from_slice(&raw[src..src + srw * 4]);
                        }
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let lx = (px - sx0) as usize;
                                let ly = (py - sy0) as usize;
                                let (mut sr, mut sg, mut sb, mut sa, mut cnt) = (0u32, 0u32, 0u32, 0u32, 0u32);
                                for ky in -1i32..=1 { for kx in -1i32..=1 {
                                    let nlx = (lx as i32 + kx).clamp(0, srw as i32 - 1) as usize;
                                    let nly = (ly as i32 + ky).clamp(0, srh as i32 - 1) as usize;
                                    let so = nly * srw * 4 + nlx * 4;
                                    sr += snap_buf[so] as u32; sg += snap_buf[so+1] as u32;
                                    sb += snap_buf[so+2] as u32; sa += snap_buf[so+3] as u32;
                                    cnt += 1;
                                }}
                                let so = ly * srw * 4 + lx * 4;
                                let off = py as usize * stride + px as usize * 4;
                                if matches!(mode, RetouchMode::Blur) {
                                    let blend = falloff * strength;
                                    raw[off]   = retouch_lerp_u8(snap_buf[so],   (sr/cnt) as u8, blend);
                                    raw[off+1] = retouch_lerp_u8(snap_buf[so+1], (sg/cnt) as u8, blend);
                                    raw[off+2] = retouch_lerp_u8(snap_buf[so+2], (sb/cnt) as u8, blend);
                                    raw[off+3] = retouch_lerp_u8(snap_buf[so+3], (sa/cnt) as u8, blend);
                                } else {
                                    let amt = falloff * strength * 0.16;
                                    let [r, g, b] = [snap_buf[so], snap_buf[so+1], snap_buf[so+2]];
                                    raw[off]   = ((r as i32 + ((r as i32 - (sr/cnt) as i32) as f32 * amt) as i32).clamp(0, 255)) as u8;
                                    raw[off+1] = ((g as i32 + ((g as i32 - (sg/cnt) as i32) as f32 * amt) as i32).clamp(0, 255)) as u8;
                                    raw[off+2] = ((b as i32 + ((b as i32 - (sb/cnt) as i32) as f32 * amt) as i32).clamp(0, 255)) as u8;
                                    raw[off+3] = snap_buf[so+3];
                                }
                            }
                        }
                    }

                    RetouchMode::Smudge => {
                        let [sm0, sm1, sm2] = [(smudge[0]*255.0) as u8, (smudge[1]*255.0) as u8, (smudge[2]*255.0) as u8];
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let blend = falloff * strength;
                                let off = py as usize * stride + px as usize * 4;
                                raw[off]   = retouch_lerp_u8(raw[off],   sm0, blend);
                                raw[off+1] = retouch_lerp_u8(raw[off+1], sm1, blend);
                                raw[off+2] = retouch_lerp_u8(raw[off+2], sm2, blend);
                            }
                        }
                        let cxi = cx.clamp(0.0, (width-1) as f32) as usize;
                        let cyi = cy.clamp(0.0, (height-1) as f32) as usize;
                        let off = cyi * stride + cxi * 4;
                        smudge = [raw[off] as f32/255.0, raw[off+1] as f32/255.0, raw[off+2] as f32/255.0, raw[off+3] as f32/255.0];
                    }

                    RetouchMode::Vibrance => {
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let off = py as usize * stride + px as usize * 4;
                                let (h, s, v) = rgb_to_hsv(raw[off], raw[off+1], raw[off+2]);
                                let vib_factor = if vib_delta >= 0.0 { 1.0 - s } else { s };
                                let (nr, ng, nb) = hsv_to_rgb((h + vib_delta * vib_factor * falloff).clamp(0.0, 1.0), (s + vib_delta * vib_factor * falloff).clamp(0.0, 1.0), v);
                                raw[off] = nr; raw[off+1] = ng; raw[off+2] = nb;
                            }
                        }
                    }

                    RetouchMode::Saturation => {
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let off = py as usize * stride + px as usize * 4;
                                let (h, s, v) = rgb_to_hsv(raw[off], raw[off+1], raw[off+2]);
                                let new_s = if vib_delta >= 0.0 {
                                    (s + vib_delta * (1.0 - s) * falloff).clamp(0.0, 1.0)
                                } else {
                                    (s + vib_delta * s * falloff).clamp(0.0, 1.0)
                                };
                                let (nr, ng, nb) = hsv_to_rgb(h, new_s, v);
                                raw[off] = nr; raw[off+1] = ng; raw[off+2] = nb;
                            }
                        }
                    }

                    RetouchMode::Temperature => {
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let shift = (temp_delta * falloff * 35.0) as i32;
                                let off = py as usize * stride + px as usize * 4;
                                raw[off] = (raw[off] as i32 + shift).clamp(0, 255) as u8;
                                raw[off+2] = (raw[off+2] as i32 - shift).clamp(0, 255) as u8;
                            }
                        }
                    }

                    RetouchMode::Brightness => {
                        for py in min_y..max_y {
                            for px in min_x..max_x {
                                let falloff = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                if falloff <= 0.0 { continue; }
                                let d = (bri_delta * falloff) as i32;
                                let off = py as usize * stride + px as usize * 4;
                                raw[off]   = (raw[off] as i32 + d).clamp(0, 255) as u8;
                                raw[off+1] = (raw[off+1] as i32 + d).clamp(0, 255) as u8;
                                raw[off+2] = (raw[off+2] as i32 + d).clamp(0, 255) as u8;
                            }
                        }
                    }

                    RetouchMode::Pixelate => {
                        let block = pixelate_block;
                        let bx0 = (min_x / block) * block;
                        let by0 = (min_y / block) * block;
                        let mut bx = bx0;
                        while bx < max_x {
                            let mut by = by0;
                            while by < max_y {
                                let bx1 = (bx + block).min(width);
                                let by1 = (by + block).min(height);
                                let (mut sr, mut sg, mut sb, mut sa, mut cnt) = (0u32, 0u32, 0u32, 0u32, 0u32);
                                let mut max_fo = 0.0f32;
                                for py in by..by1 { for px in bx..bx1 {
                                    let fo = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                    if fo <= 0.0 { continue; }
                                    let off = py as usize * stride + px as usize * 4;
                                    sr += raw[off] as u32; sg += raw[off+1] as u32;
                                    sb += raw[off+2] as u32; sa += raw[off+3] as u32;
                                    cnt += 1;
                                    if fo > max_fo { max_fo = fo; }
                                }}
                                if cnt > 0 {
                                    let avg = [(sr/cnt) as u8, (sg/cnt) as u8, (sb/cnt) as u8, (sa/cnt) as u8];
                                    for py in by..by1 { for px in bx..bx1 {
                                        let fo = brush_shape_falloff(px as f32 - cx, py as f32 - cy, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                        if fo <= 0.0 { continue; }
                                        let off = py as usize * stride + px as usize * 4;
                                        raw[off] = retouch_lerp_u8(raw[off],   avg[0], max_fo);
                                        raw[off+1] = retouch_lerp_u8(raw[off+1], avg[1], max_fo);
                                        raw[off+2] = retouch_lerp_u8(raw[off+2], avg[2], max_fo);
                                        raw[off+3] = retouch_lerp_u8(raw[off+3], avg[3], max_fo);
                                    }}
                                }
                                by += block;
                            }
                            bx += block;
                        }
                    }
                }
            }
        }

        let _ = raw;
        self.retouch_smudge_sample = smudge;
        self.dirty = true;
        if dr_x1 > dr_x0 && dr_y1 > dr_y0 { self.expand_dirty_rect(dr_x0, dr_y0, dr_x1, dr_y1); }
        self.texture_dirty = true;
        if let Some(old_bg) = swapped_bg { self.restore_layer_swap(active_id, old_bg); } else { self.promote_dirty_to_composite(); }
    }

    fn apply_brush_stroke_on_image_layer(&mut self) {
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        if self.stroke_points.len() < 2 { return; }
        let ild = match self.image_layer_data.get_mut(&iid) { Some(d) => d, None => return };
        if !matches!(ild.image, DynamicImage::ImageRgba8(_)) {
            ild.image = DynamicImage::ImageRgba8(ild.image.to_rgba8());
        }
        let is_eraser = self.tool == Tool::Eraser;
        let (r, g, b_ch, base_a) = if is_eraser { (0u8, 0u8, 0u8, 0u8) } else { (self.color.r(), self.color.g(), self.color.b(), self.color.a()) };
        
        let pixel_scale = ild.pixel_scale();
        let canvas_radius = if is_eraser { self.eraser_size / 2.0 } else { self.brush.size / 2.0 };
        let radius = canvas_radius * pixel_scale;
        let opacity = if is_eraser { 1.0 } else { self.brush.opacity };
        let flow = if is_eraser { 1.0 } else { self.brush.flow };
        let softness = if is_eraser { 0.0 } else { self.brush.softness };
        let shape = if is_eraser { BrushShape::Circle } else { self.brush.shape };
        let eraser_transparent = is_eraser;
        let step_dist = (radius * (if is_eraser { 0.25 } else { self.brush.step })).max(0.5);
        let (flip_h, flip_v, display_w, display_h, orig_w, orig_h) =
            (ild.flip_h, ild.flip_v, ild.display_w, ild.display_h, ild.orig_w(), ild.orig_h());
        let (ctr_cx, ctr_cy) = ild.center_canvas();
        let angle = -ild.rotation.to_radians();
        let (cos_a, sin_a) = (angle.cos(), angle.sin());
        let canvas_to_img = |cx: f32, cy: f32| -> (f32, f32) {
            let (dx, dy) = (cx - ctr_cx, cy - ctr_cy);
            let lx = dx * cos_a - dy * sin_a + display_w / 2.0;
            let ly = dx * sin_a + dy * cos_a + display_h / 2.0;
            let mut sx = lx / display_w.max(1.0) * orig_w as f32;
            let mut sy = ly / display_h.max(1.0) * orig_h as f32;
            if flip_h { sx = orig_w as f32 - 1.0 - sx; }
            if flip_v { sy = orig_h as f32 - 1.0 - sy; }
            (sx, sy)
        };
        let points = self.stroke_points.clone();
        let buf = if let DynamicImage::ImageRgba8(b) = &mut ild.image { b } else { return };
        let (bw, bh) = (buf.width(), buf.height());
        let mut dr_x0 = bw; let mut dr_y0 = bh; let mut dr_x1 = 0u32; let mut dr_y1 = 0u32;
        
        let mut canvas_dr_x0 = f32::MAX; let mut canvas_dr_y0 = f32::MAX;
        let mut canvas_dr_x1 = f32::MIN; let mut canvas_dr_y1 = f32::MIN;

        for i in 0..points.len().saturating_sub(1) {
            let (x0c, y0c) = points[i]; let (x1c, y1c) = points[i + 1];
            let (dxc, dyc) = (x1c - x0c, y1c - y0c);
            let (s0, s1) = (canvas_to_img(x0c, y0c), canvas_to_img(x1c, y1c));
            let seg_len = ((s1.0-s0.0).powi(2) + (s1.1-s0.1).powi(2)).sqrt();
            let steps = ((seg_len / step_dist).ceil() as usize).max(1);
            for s in 0..=steps {
                let t = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let cx_c = x0c + dxc * t;
                let cy_c = y0c + dyc * t;
                
                canvas_dr_x0 = canvas_dr_x0.min(cx_c - canvas_radius - 1.0);
                canvas_dr_y0 = canvas_dr_y0.min(cy_c - canvas_radius - 1.0);
                canvas_dr_x1 = canvas_dr_x1.max(cx_c + canvas_radius + 1.0);
                canvas_dr_y1 = canvas_dr_y1.max(cy_c + canvas_radius + 1.0);

                let (cx_img, cy_img) = canvas_to_img(cx_c, cy_c);
                let min_px = (cx_img - radius - 1.0).max(0.0) as u32;
                let max_px = ((cx_img + radius + 1.0).max(0.0).ceil() as u32).min(bw);
                let min_py = (cy_img - radius - 1.0).max(0.0) as u32;
                let max_py = ((cy_img + radius + 1.0).max(0.0).ceil() as u32).min(bh);
                
                dr_x0 = dr_x0.min(min_px); dr_y0 = dr_y0.min(min_py);
                dr_x1 = dr_x1.max(max_px); dr_y1 = dr_y1.max(max_py);
                for py in min_py..max_py {
                    for px in min_px..max_px {
                        let falloff = brush_shape_falloff(px as f32 - cx_img, py as f32 - cy_img, radius, 1.0, 0.0, softness, shape);
                        if falloff <= 0.0 { continue; }
                        let alpha = (falloff * flow * opacity * 255.0).clamp(0.0, 255.0) as u8;
                        if alpha == 0 { continue; }
                        unsafe {
                            let pixel = buf.unsafe_get_pixel(px, py);
                            let [er, eg, eb, ea] = pixel.0;
                            let new_pixel = if eraser_transparent {
                                Rgba([er, eg, eb, ea.saturating_sub(alpha)])
                            } else {
                                let fa = alpha as u16; let bf = (base_a as u16 * fa) / 255; let ba = 255 - bf;
                                Rgba([((r as u16*bf + er as u16*ba)/255) as u8, ((g as u16*bf + eg as u16*ba)/255) as u8, ((b_ch as u16*bf + eb as u16*ba)/255) as u8, ((bf + ea as u16*ba/255).min(255)) as u8])
                            };
                            buf.unsafe_put_pixel(px, py, new_pixel);
                        }
                    }
                }
            }
        }
        if dr_x1 > dr_x0 && dr_y1 > dr_y0 {
            let entry = self.image_layer_stroke_rects.entry(iid).or_insert([bw, bh, 0, 0]);
            entry[0] = entry[0].min(dr_x0); entry[1] = entry[1].min(dr_y0);
            entry[2] = entry[2].max(dr_x1); entry[3] = entry[3].max(dr_y1);
        }
        
        if canvas_dr_x1 > canvas_dr_x0 && canvas_dr_y1 > canvas_dr_y0 {
            let cx0 = canvas_dr_x0.max(0.0) as u32; let cy0 = canvas_dr_y0.max(0.0) as u32;
            let cx1 = canvas_dr_x1.ceil() as u32; let cy1 = canvas_dr_y1.ceil() as u32;
            let bg_w = self.image.as_ref().map(|i| i.width()).unwrap_or(u32::MAX);
            let bg_h = self.image.as_ref().map(|i| i.height()).unwrap_or(u32::MAX);
            let r = [cx0.min(bg_w), cy0.min(bg_h), cx1.min(bg_w), cy1.min(bg_h)];
            if r[2] > r[0] && r[3] > r[1] {
                match &mut self.composite_dirty_rect {
                    None => self.composite_dirty_rect = Some(r),
                    Some(cr) => { cr[0]=cr[0].min(r[0]); cr[1]=cr[1].min(r[1]); cr[2]=cr[2].max(r[2]); cr[3]=cr[3].max(r[3]); }
                }
            }
        }

        self.image_layer_texture_dirty.insert(iid);
        self.composite_dirty = true;
        self.texture_dirty = true;
        self.dirty = true;
    }

    fn apply_retouch_stroke_on_image_layer(&mut self) {
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        if self.stroke_points.len() < 2 { return; }
        let ild = match self.image_layer_data.get_mut(&iid) { Some(d) => d, None => return };
        if !matches!(ild.image, DynamicImage::ImageRgba8(_)) {
            ild.image = DynamicImage::ImageRgba8(ild.image.to_rgba8());
        }
        let mode = self.retouch_mode;
        let pixel_scale = ild.pixel_scale();
        let canvas_radius = self.retouch_size / 2.0;
        let radius = (canvas_radius * pixel_scale).max(1.0);
        let strength = self.retouch_strength.clamp(0.0, 1.0);
        let softness = self.retouch_softness.clamp(0.0, 1.0);
        let step_dist = (radius * 0.4).max(0.5);
        let pixelate_block = self.retouch_pixelate_block.max(2);
        let (flip_h, flip_v, display_w, display_h, orig_w, orig_h) =
            (ild.flip_h, ild.flip_v, ild.display_w, ild.display_h, ild.orig_w(), ild.orig_h());
        let (ctr_cx, ctr_cy) = ild.center_canvas();
        let angle = -ild.rotation.to_radians();
        let (cos_a, sin_a) = (angle.cos(), angle.sin());
        let canvas_to_img = |cx: f32, cy: f32| -> (f32, f32) {
            let (dx, dy) = (cx - ctr_cx, cy - ctr_cy);
            let lx = dx * cos_a - dy * sin_a + display_w / 2.0;
            let ly = dx * sin_a + dy * cos_a + display_h / 2.0;
            let mut sx = lx / display_w.max(1.0) * orig_w as f32;
            let mut sy = ly / display_h.max(1.0) * orig_h as f32;
            if flip_h { sx = orig_w as f32 - 1.0 - sx; }
            if flip_v { sy = orig_h as f32 - 1.0 - sy; }
            (sx, sy)
        };
        let points = self.stroke_points.clone();
        let mut smudge = self.retouch_smudge_sample;
        let vib_delta = (strength - 0.5) * 2.0;
        let temp_delta = (strength - 0.5) * 2.0;
        let bri_delta = (strength - 0.5) * 2.0 * 45.0;
        let buf = if let DynamicImage::ImageRgba8(b) = &mut ild.image { b } else { return };
        let (bw, bh) = (buf.width(), buf.height());
        let stride = bw as usize * 4;
        let mut dr_x0 = bw; let mut dr_y0 = bh; let mut dr_x1 = 0u32; let mut dr_y1 = 0u32;
        
        let mut canvas_dr_x0 = f32::MAX; let mut canvas_dr_y0 = f32::MAX;
        let mut canvas_dr_x1 = f32::MIN; let mut canvas_dr_y1 = f32::MIN;

        let mut flat = buf.as_flat_samples_mut();
        let raw = flat.as_mut_slice();
        for i in 0..points.len().saturating_sub(1) {
            let (x0c, y0c) = points[i]; let (x1c, y1c) = points[i+1];
            let (dxc, dyc) = (x1c-x0c, y1c-y0c);
            let (s0, s1) = (canvas_to_img(x0c, y0c), canvas_to_img(x1c, y1c));
            let seg_len = ((s1.0-s0.0).powi(2)+(s1.1-s0.1).powi(2)).sqrt();
            let steps = ((seg_len/step_dist).ceil() as usize).max(1);
            for s in 0..=steps {
                let t = if steps == 0 { 0.0 } else { s as f32/steps as f32 };
                let cx_c = x0c + dxc * t;
                let cy_c = y0c + dyc * t;

                canvas_dr_x0 = canvas_dr_x0.min(cx_c - canvas_radius - 1.0);
                canvas_dr_y0 = canvas_dr_y0.min(cy_c - canvas_radius - 1.0);
                canvas_dr_x1 = canvas_dr_x1.max(cx_c + canvas_radius + 1.0);
                canvas_dr_y1 = canvas_dr_y1.max(cy_c + canvas_radius + 1.0);

                let (cx_img, cy_img) = canvas_to_img(cx_c, cy_c);
                let min_px = (cx_img - radius - 1.0).max(0.0) as i32;
                let max_px = ((cx_img + radius + 1.0).max(0.0).ceil() as i32).min(bw as i32);
                let min_py = (cy_img - radius - 1.0).max(0.0) as i32;
                let max_py = ((cy_img + radius + 1.0).max(0.0).ceil() as i32).min(bh as i32);
                
                dr_x0 = dr_x0.min(min_px.max(0) as u32); dr_y0 = dr_y0.min(min_py.max(0) as u32);
                dr_x1 = dr_x1.max(max_px as u32); dr_y1 = dr_y1.max(max_py as u32);
                match mode {
                    RetouchMode::Blur | RetouchMode::Sharpen => {
                        let blur_r = ((strength * 5.0) as i32).max(1);
                        let sx0 = (min_px - blur_r).max(0) as usize;
                        let sy0 = (min_py - blur_r).max(0) as usize;
                        let sx1 = (max_px + blur_r).min(bw as i32) as usize;
                        let sy1 = (max_py + blur_r).min(bh as i32) as usize;
                        let srw = sx1.saturating_sub(sx0);
                        let srh = sy1.saturating_sub(sy0);
                        let mut snap = vec![0u8; srw * srh * 4];
                        for (ri, py2) in (sy0..sy1).enumerate() {
                            let src = py2 * stride + sx0 * 4;
                            snap[ri * srw * 4..ri * srw * 4 + srw * 4].copy_from_slice(&raw[src..src + srw * 4]);
                        }

                        let sum = |px: u32, py: u32, r: i32| -> [f32;4] {
                            let mut acc = [0f32;4]; let mut cnt = 0i32;
                            for ky in (-r)..=r { for kx in (-r)..=r {
                                let nx = px as i32+kx; let ny = py as i32+ky;
                                if nx < sx0 as i32 || ny < sy0 as i32 || nx >= sx1 as i32 || ny >= sy1 as i32 { continue; }
                                let lx = (nx - sx0 as i32) as usize;
                                let ly = (ny - sy0 as i32) as usize;
                                let o = ly * srw * 4 + lx * 4;
                                for c in 0..4 { acc[c] += snap[o+c] as f32; } cnt += 1;
                            }}
                            if cnt > 0 { [acc[0]/cnt as f32,acc[1]/cnt as f32,acc[2]/cnt as f32,acc[3]/cnt as f32] } else { acc }
                        };
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let blurred = sum(px2, py2, blur_r);
                            let off = py2 as usize*stride+px2 as usize*4;
                            if mode == RetouchMode::Blur {
                                for c in 0..4 { raw[off+c] = retouch_lerp_u8(raw[off+c], blurred[c] as u8, fo*strength); }
                            } else {
                                let orig = [raw[off] as f32, raw[off+1] as f32, raw[off+2] as f32, raw[off+3] as f32];
                                for c in 0..3 { let sv = orig[c]+(orig[c]-blurred[c])*strength*2.0; raw[off+c]=sv.clamp(0.0,255.0) as u8; }
                            }
                        }}
                    }
                    RetouchMode::Smudge => {
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let off = py2 as usize*stride+px2 as usize*4;
                            for c in 0..4 { smudge[c]=smudge[c]*(1.0-fo*strength)+raw[off+c] as f32/255.0*(fo*strength); raw[off+c]=(smudge[c]*255.0).clamp(0.0,255.0) as u8; }
                        }}
                    }
                    RetouchMode::Vibrance => {
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let off = py2 as usize*stride+px2 as usize*4;
                            let (r2,g2,b2) = (raw[off] as f32/255.0,raw[off+1] as f32/255.0,raw[off+2] as f32/255.0);
                            let max_c = r2.max(g2).max(b2); let min_c = r2.min(g2).min(b2);
                            let sat = if max_c > 0.0 { (max_c-min_c)/max_c } else { 0.0 };
                            let vib_mult = 1.0 + vib_delta * fo * (1.0-sat);
                            raw[off]=((r2*vib_mult).clamp(0.0,1.0)*255.0) as u8;
                            raw[off+1]=((g2*vib_mult).clamp(0.0,1.0)*255.0) as u8;
                            raw[off+2]=((b2*vib_mult).clamp(0.0,1.0)*255.0) as u8;
                        }}
                    }
                    RetouchMode::Saturation => {
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let off = py2 as usize*stride+px2 as usize*4;
                            let (h,s,v) = rgb_to_hsv(raw[off],raw[off+1],raw[off+2]);
                            let (nr,ng,nb) = hsv_to_rgb(h,(s*(1.0+vib_delta*fo)).clamp(0.0,1.0),v);
                            raw[off]=nr; raw[off+1]=ng; raw[off+2]=nb;
                        }}
                    }
                    RetouchMode::Temperature => {
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let off = py2 as usize*stride+px2 as usize*4;
                            let shift = (temp_delta*fo*35.0) as i32;
                            raw[off]=(raw[off] as i32+shift).clamp(0,255) as u8;
                            raw[off+2]=(raw[off+2] as i32-shift).clamp(0,255) as u8;
                        }}
                    }
                    RetouchMode::Brightness => {
                        for py2 in min_py.max(0) as u32..max_py.max(0) as u32 { for px2 in min_px.max(0) as u32..max_px.max(0) as u32 {
                            let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                            if fo <= 0.0 { continue; }
                            let off = py2 as usize*stride+px2 as usize*4;
                            let d = (bri_delta*fo) as i32;
                            raw[off]=(raw[off] as i32+d).clamp(0,255) as u8;
                            raw[off+1]=(raw[off+1] as i32+d).clamp(0,255) as u8;
                            raw[off+2]=(raw[off+2] as i32+d).clamp(0,255) as u8;
                        }}
                    }
                    RetouchMode::Pixelate => {
                        let block = pixelate_block;
                        let bx0 = (min_px.max(0) as u32/block)*block;
                        let by0 = (min_py.max(0) as u32/block)*block;
                        let mut bx = bx0;
                        while bx < (max_px as u32).min(bw) {
                            let mut by = by0;
                            while by < (max_py as u32).min(bh) {
                                let bx1=(bx+block).min(bw); let by1=(by+block).min(bh);
                                let (mut sr,mut sg,mut sb,mut sa,mut cnt,mut mfo)=(0u32,0u32,0u32,0u32,0u32,0f32);
                                for py2 in by..by1 { for px2 in bx..bx1 {
                                    let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                    if fo <= 0.0 { continue; }
                                    let off = py2 as usize*stride+px2 as usize*4;
                                    sr+=raw[off] as u32; sg+=raw[off+1] as u32; sb+=raw[off+2] as u32; sa+=raw[off+3] as u32; cnt+=1;
                                    if fo > mfo { mfo=fo; }
                                }}
                                if cnt > 0 {
                                    let avg=[(sr/cnt) as u8,(sg/cnt) as u8,(sb/cnt) as u8,(sa/cnt) as u8];
                                    for py2 in by..by1 { for px2 in bx..bx1 {
                                        let fo = brush_shape_falloff(px2 as f32-cx_img, py2 as f32-cy_img, radius, 1.0, 0.0, softness, BrushShape::Circle);
                                        if fo <= 0.0 { continue; }
                                        let off = py2 as usize*stride+px2 as usize*4;
                                        raw[off]=retouch_lerp_u8(raw[off],avg[0],mfo); raw[off+1]=retouch_lerp_u8(raw[off+1],avg[1],mfo);
                                        raw[off+2]=retouch_lerp_u8(raw[off+2],avg[2],mfo); raw[off+3]=retouch_lerp_u8(raw[off+3],avg[3],mfo);
                                    }}
                                }
                                by += block;
                            }
                            bx += block;
                        }
                    }
                }
            }
        }
        let _ = raw;
        self.retouch_smudge_sample = smudge;
        if dr_x1 > dr_x0 && dr_y1 > dr_y0 {
            let entry = self.image_layer_stroke_rects.entry(iid).or_insert([bw, bh, 0, 0]);
            entry[0] = entry[0].min(dr_x0); entry[1] = entry[1].min(dr_y0);
            entry[2] = entry[2].max(dr_x1); entry[3] = entry[3].max(dr_y1);
        }

        if canvas_dr_x1 > canvas_dr_x0 && canvas_dr_y1 > canvas_dr_y0 {
            let cx0 = canvas_dr_x0.max(0.0) as u32; let cy0 = canvas_dr_y0.max(0.0) as u32;
            let cx1 = canvas_dr_x1.ceil() as u32; let cy1 = canvas_dr_y1.ceil() as u32;
            let bg_w = self.image.as_ref().map(|i| i.width()).unwrap_or(u32::MAX);
            let bg_h = self.image.as_ref().map(|i| i.height()).unwrap_or(u32::MAX);
            let r = [cx0.min(bg_w), cy0.min(bg_h), cx1.min(bg_w), cy1.min(bg_h)];
            if r[2] > r[0] && r[3] > r[1] {
                match &mut self.composite_dirty_rect {
                    None => self.composite_dirty_rect = Some(r),
                    Some(cr) => { cr[0]=cr[0].min(r[0]); cr[1]=cr[1].min(r[1]); cr[2]=cr[2].max(r[2]); cr[3]=cr[3].max(r[3]); }
                }
            }
        }

        self.image_layer_texture_dirty.insert(iid);
        self.composite_dirty = true;
        self.texture_dirty = true;
        self.dirty = true;
    }

    pub(super) fn apply_crop_to_image_layer(&mut self) {
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        let (s, e) = match (self.crop_state.start, self.crop_state.end) { (Some(s), Some(e)) => (s, e), _ => return };
        let ild = match self.image_layer_data.get_mut(&iid) { Some(d) => d, None => return };
        let (lx0, ly0) = ild.canvas_to_local_f32(s.0, s.1);
        let (lx1, ly1) = ild.canvas_to_local_f32(e.0, e.1);
        let x0 = lx0.min(lx1).max(0.0) as u32;
        let y0 = ly0.min(ly1).max(0.0) as u32;
        let x1 = (lx0.max(lx1).ceil() as u32).min(ild.orig_w());
        let y1 = (ly0.max(ly1).ceil() as u32).min(ild.orig_h());
        if x1 <= x0 || y1 <= y0 { return; }
        self.push_undo();
        let ild = self.image_layer_data.get_mut(&iid).unwrap();
        let scale_x = ild.display_w / ild.orig_w() as f32;
        let scale_y = ild.display_h / ild.orig_h() as f32;
        let cropped = ild.image.crop_imm(x0, y0, x1-x0, y1-y0);
        let new_dw = (x1-x0) as f32 * scale_x;
        let new_dh = (y1-y0) as f32 * scale_y;
        ild.canvas_x += x0 as f32 * scale_x;
        ild.canvas_y += y0 as f32 * scale_y;
        ild.display_w = new_dw;
        ild.display_h = new_dh;
        ild.image = cropped;
        self.image_layer_texture_dirty.insert(iid);
        self.crop_state = CropState::default();
        self.composite_dirty = true;
        self.dirty = true;
    }

    pub(super) fn init_smudge_sample_image_layer(&mut self, canvas_x: f32, canvas_y: f32) {
        let iid = match self.image_layer_for_active() { Some(id) => id, None => return };
        if let Some(ild) = self.image_layer_data.get(&iid) {
            if let Some((px, py)) = ild.canvas_to_local(canvas_x, canvas_y) {
                if let DynamicImage::ImageRgba8(ref buf) = ild.image {
                    if px < buf.width() && py < buf.height() {
                        let p = buf.get_pixel(px, py);
                        self.retouch_smudge_sample = [p.0[0] as f32/255.0, p.0[1] as f32/255.0, p.0[2] as f32/255.0, p.0[3] as f32/255.0];
                    }
                }
            }
        }
    }

    pub(super) fn apply_flip_h(&mut self) {
        let (old_w, flipped) = match &self.image { Some(img) => (img.width(), img.fliph()), None => return };
        self.transform_text_flip_h(old_w); self.image = Some(flipped);
        self.texture_dirty = true; self.composite_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_flip_v(&mut self) {
        let (old_h, flipped) = match &self.image { Some(img) => (img.height(), img.flipv()), None => return };
        self.transform_text_flip_v(old_h); self.image = Some(flipped);
        self.texture_dirty = true; self.composite_dirty = true; self.dirty = true;
    }

    pub(super) fn apply_rotate_cw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image { Some(img) => (img.width(), img.height(), img.rotate90()), None => return };
        self.transform_text_rotate_cw(old_w, old_h); self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width(); self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true; self.composite_dirty = true; self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn apply_rotate_ccw(&mut self) {
        let (old_w, old_h, rotated) = match &self.image { Some(img) => (img.width(), img.height(), img.rotate270()), None => return };
        self.transform_text_rotate_ccw(old_w, old_h); self.image = Some(rotated);
        self.resize_w = self.image.as_ref().unwrap().width(); self.resize_h = self.image.as_ref().unwrap().height();
        self.texture_dirty = true; self.composite_dirty = true; self.dirty = true; self.fit_on_next_frame = true;
    }

    pub(super) fn apply_resize(&mut self) {
        let img: DynamicImage = match self.image.clone() { Some(i) => i, None => return };
        if self.resize_w == 0 || self.resize_h == 0 { return; }
        let (w, h, stretch) = (self.resize_w, self.resize_h, self.resize_stretch);
        let result: Arc<std::sync::Mutex<Option<DynamicImage>>> = Arc::clone(&self.pending_filter_result);
        let progress: Arc<std::sync::Mutex<f32>> = Arc::clone(&self.filter_progress);
        self.filter_target_layer_id = 0;

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
        let composite: DynamicImage = self.composite_all_layers()
            .ok_or_else(|| "No image to export".to_string())?;
        let default_name: &str = self.file_path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("export");
        let filename: String = format!("{}.{}", default_name, self.export_format.extension());
        let path: PathBuf = match rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter(self.export_format.as_str(), &[self.export_format.extension()])
            .save_file()
        {
            Some(p) => p, None => return Err("Export cancelled".to_string()),
        };
        export_image(&composite, &path, self.export_format, self.export_jpeg_quality, 6, 100.0, self.export_auto_scale_ico, self.export_avif_quality, self.export_avif_speed)?;
        self.filter_panel = FilterPanel::None;
        Ok(path)
    }

    pub(super) fn render_brush_preview_to_pixels(&self, w: u32, h: u32, is_dark: bool) -> Vec<egui::Color32> {
        let bg = if is_dark { [18u8, 18, 23, 255] } else { [238u8, 238, 244, 255] };
        let mut buf: Vec<[u8; 4]> = vec![bg; (w * h) as usize];

        let (r, g, b_ch, base_a) = (
            self.color.r(), self.color.g(), self.color.b(),
            self.color.a().max(180),
        );

        let max_r = h as f32 * 0.36;
        let radius = (self.brush.size / 2.0).min(max_r).max(1.5);
        let step_dist = (radius * 2.0 * self.brush.step).max(0.5);
        let n_pts = 100usize;
        let pts: Vec<(f32, f32)> = (0..n_pts).map(|i| {
            let t = i as f32 / (n_pts - 1) as f32;
            let x = w as f32 * 0.06 + t * (w as f32 * 0.88);
            let y = h as f32 * 0.5 + (t * std::f32::consts::TAU * 1.1).sin() * (h as f32 * 0.29);
            (x, y)
        }).collect();

        for seg_i in 0..pts.len().saturating_sub(1) {
            let (x0, y0) = pts[seg_i];
            let (x1, y1) = pts[seg_i + 1];
            let (dx, dy) = (x1 - x0, y1 - y0);
            let seg_len = (dx * dx + dy * dy).sqrt();
            let steps = ((seg_len / step_dist).ceil() as usize).max(1);

            for s in 0..=steps {
                let t = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                let seed = (seg_i as u64).wrapping_mul(99991).wrapping_add(s as u64 * 7919);
                let mut cx = x0 + dx * t;
                let mut cy = y0 + dy * t;

                if self.brush.scatter > 0.0 && !self.brush.spray_mode {
                    let sc = self.brush.scatter.min(radius * 0.6);
                    cx += (brush_rand(seed) * 2.0 - 1.0) * sc;
                    cy += (brush_rand(seed.wrapping_add(1)) * 2.0 - 1.0) * sc;
                }

                let min_px = ((cx - radius - 1.0).max(0.0)) as u32;
                let max_px = ((cx + radius + 1.0).ceil() as u32).min(w);
                let min_py = ((cy - radius - 1.0).max(0.0)) as u32;
                let max_py = ((cy + radius + 1.0).ceil() as u32).min(h);

                for py in min_py..max_py {
                    for px in min_px..max_px {
                        let falloff = brush_shape_falloff(
                            px as f32 - cx, py as f32 - cy,
                            radius, self.brush.aspect_ratio,
                            self.brush.angle.to_radians(),
                            self.brush.softness, self.brush.shape,
                        );
                        if falloff <= 0.0 { continue; }

                        let tex_mul = if self.brush.texture_strength > 0.0 {
                            1.0 - self.brush.texture_strength * brush_texture_noise(px, py, self.brush.texture_mode)
                        } else { 1.0 };

                        let alpha = (falloff * self.brush.flow * self.brush.opacity * tex_mul * 255.0)
                            .clamp(0.0, 255.0) as u8;
                        if alpha == 0 { continue; }

                        let idx = (py * w + px) as usize;
                        if idx >= buf.len() { continue; }
                        let [er, eg, eb, ea] = buf[idx];
                        let fa = alpha as u16;
                        let bf = (base_a as u16 * fa) / 255;
                        let ba = 255u16.saturating_sub(bf);
                        buf[idx] = [
                            ((r as u16 * bf + er as u16 * ba) / 255) as u8,
                            ((g as u16 * bf + eg as u16 * ba) / 255) as u8,
                            ((b_ch as u16 * bf + eb as u16 * ba) / 255) as u8,
                            ((bf + ea as u16 * ba / 255).min(255)) as u8,
                        ];
                    }
                }
            }
        }

        buf.iter().map(|&[r, g, b, a]| egui::Color32::from_rgba_unmultiplied(r, g, b, a)).collect()
    }
}

#[inline(always)]
pub(super) fn retouch_lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 255.0) as u8
}

#[inline(always)]
pub(super) fn brush_rand(seed: u64) -> f32 {
    let x: u64 = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let x: u64 = x ^ (x >> 33);
    let x: u64 = x.wrapping_mul(0xff51afd7ed558ccd);
    let x: u64 = x ^ (x >> 33);
    let x: u64 = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    let x: u64 = x ^ (x >> 33);
    (x >> 11) as f32 / (1u64 << 53) as f32
}

#[inline]
pub(super) fn brush_shape_falloff(dx: f32, dy: f32, radius: f32, aspect: f32, angle: f32, softness: f32, shape: BrushShape) -> f32 {
    let (cos_a, sin_a) = (angle.cos(), angle.sin());
    let lx: f32 = dx * cos_a + dy * sin_a;
    let ly: f32 = -dx * sin_a + dy * cos_a;

    let t: f32 = match shape {
        BrushShape::Circle => {
            ((dx * dx + dy * dy) / (radius * radius)).sqrt()
        }
        BrushShape::Square => {
            lx.abs().max(ly.abs()) / radius
        }
        BrushShape::Diamond => {
            (lx.abs() + ly.abs()) / radius
        }
        BrushShape::CalligraphyFlat => {
            let r_minor: f32 = radius * aspect;
            ((lx / radius).powi(2) + (ly / r_minor).powi(2)).sqrt()
        }
    };

    if t >= 1.0 { return 0.0; }
    if softness < 0.001 { return 1.0; }
    let soft_inner: f32 = 1.0 - softness;
    if t <= soft_inner { return 1.0; }
    let s: f32 = ((t - soft_inner) / softness).clamp(0.0, 1.0);
    1.0 - s * s * (3.0 - 2.0 * s)
}

fn smooth_hash_2d(px: u32, py: u32, scale: u32, seed: u64) -> f32 {
    let s = scale.max(1);
    let gx = px / s; let gy = py / s;
    let fx = (px % s) as f32 / s as f32; let fy = (py % s) as f32 / s as f32;
    let ux = fx * fx * (3.0 - 2.0 * fx); let uy = fy * fy * (3.0 - 2.0 * fy);
    let h = |x: u32, y: u32| -> f32 { brush_rand((x as u64).wrapping_mul(0x517CC1B7) ^ (y as u64).wrapping_mul(0x9E3779B9) ^ seed,) };
    let n00 = h(gx,gy); let n10 = h(gx + 1, gy);
    let n01 = h(gx, gy + 1); let n11 = h(gx + 1, gy + 1);
    let x0 = n00 + (n10 - n00) * ux; let x1 = n01 + (n11 - n01) * ux;
    x0 + (x1 - x0) * uy
}

fn paper_noise(px: u32, py: u32) -> f32 {
    let n0 = smooth_hash_2d(px, py,  2, 1); let n1 = smooth_hash_2d(px, py,  5, 2) * 0.60;
    let n2 = smooth_hash_2d(px, py, 13, 3) * 0.40; let n3 = smooth_hash_2d(px, py, 28, 4) * 0.28;
    let wx = smooth_hash_2d(px.wrapping_add(17), py.wrapping_add(31), 9, 5); let wy = smooth_hash_2d(px.wrapping_add(43), py.wrapping_add(7),  9, 6);
    let warp_x = (px as i32 + ((wx - 0.5) * 6.0) as i32).max(0) as u32; let warp_y = (py as i32 + ((wy - 0.5) * 6.0) as i32).max(0) as u32;
    let n_warped = smooth_hash_2d(warp_x, warp_y, 3, 7) * 0.35;
    let micro = brush_rand(8) * 0.08;
    let raw = (n0 + n1 + n2 + n3 + n_warped + micro) / 2.71;
    let c = ((raw - 0.48) * 1.55 + 0.5).clamp(0.0, 1.0);
    c
}

#[inline]
pub(super) fn brush_texture_noise(px: u32, py: u32, mode: BrushTextureMode) -> f32 {
    match mode {
        BrushTextureMode::None => 0.0,
        BrushTextureMode::Rough => {
            let coarse = brush_rand((px as u64).wrapping_mul(37) ^ (py as u64).wrapping_mul(1009) ^ 0xDEAD);
            let smooth = smooth_hash_2d(px, py, 3, 0xBEEF0011) * 0.45;
            ((coarse + smooth) / 1.45).clamp(0.0, 1.0)
        }
        BrushTextureMode::Canvas => {
            let cx: u64 = (px / 4) as u64;
            let cy: u64 = (py / 4) as u64;
            let cell: f32 = brush_rand(cx * 31 ^ cy * 127 ^ 0xCAFE);
            let fine: f32 = brush_rand(px as u64 * 53 ^ py as u64 * 79 ^ 0xBEEF) * 0.25;
            (cell * 0.75 + fine).clamp(0.0, 1.0)
        }
        BrushTextureMode::Paper  => paper_noise(px, py),
    }
}