use serde::{Serialize, Deserialize};
use std::{collections::{HashMap, hash_map::DefaultHasher}, fs, hash::{Hash, Hasher}, path::{Path, PathBuf}};
use image::DynamicImage;
use eframe::egui;
use super::ie_main::{ImageEditor, ImageLayer, LayerKind, BlendMode, TextLayer, ImageLayerData};

#[derive(Serialize, Deserialize)]
struct LMeta { id: u64, name: String, opacity: f32, visible: bool, locked: bool, blend: BlendMode, kind: LayerKind, ltid: Option<u64>, liid: Option<u64> }

#[derive(Serialize, Deserialize)]
struct TLMeta { id: u64, content: String, x: f32, y: f32, fs: f32, bw: Option<f32>, bh: Option<f32>, rot: f32, c: [u8; 4], bold: bool, ital: bool, ul: bool, font: String }

#[derive(Serialize, Deserialize)]
struct ILMeta { id: u64, cx: f32, cy: f32, dw: f32, dh: f32, rot: f32, fh: bool, fv: bool }

#[derive(Serialize, Deserialize)]
struct Meta { path: String, mod_ms: u64, layers: Vec<LMeta>, tls: Vec<TLMeta>, ils: Vec<ILMeta>, active: u64, nlid: u64, ntid: u64, niid: u64 }

pub struct CacheEntry { pub src_path: String, pub cache_dir: PathBuf, pub size_kb: u64 }

pub struct LoadedCache {
    pub background: Option<DynamicImage>,
    pub layers: Vec<ImageLayer>,
    pub layer_images: HashMap<u64, DynamicImage>,
    pub(super) text_layers: Vec<TextLayer>,
    pub image_layer_data: HashMap<u64, ImageLayerData>,
    pub active_layer_id: u64,
    pub next_layer_id: u64,
    pub next_text_id: u64,
    pub next_image_layer_id: u64,
}

fn cache_base() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("universal_editor"); p.push("layer_cache"); p
}

pub fn cache_dir_for(path: &Path) -> PathBuf {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut h = DefaultHasher::new(); abs.hash(&mut h);
    cache_base().join(format!("{:016x}", h.finish()))
}

fn mod_ms(path: &Path) -> u64 {
    fs::metadata(path).ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn save_cache(editor: &ImageEditor) -> Result<(), String> {
    let path = editor.file_path.as_ref().ok_or("no path")?;
    let dir = cache_dir_for(path);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() { if e.path().extension().map_or(false, |x| x == "png") { let _ = fs::remove_file(e.path()); } }
    }
    if let Some(img) = &editor.image {
        img.save(dir.join("bg.png")).map_err(|e| e.to_string())?;
    }
    for (&id, img) in &editor.layer_images {
        img.save(dir.join(format!("r{id}.png"))).map_err(|e| e.to_string())?;
    }
    for (&id, ild) in &editor.image_layer_data {
        ild.image.save(dir.join(format!("i{id}.png"))).map_err(|e| e.to_string())?;
    }
    let m = Meta {
        path: path.to_string_lossy().into_owned(), mod_ms: mod_ms(path),
        layers: editor.layers.iter().map(|l| LMeta {
            id: l.id, name: l.name.clone(), opacity: l.opacity, visible: l.visible,
            locked: l.locked, blend: l.blend_mode, kind: l.kind, ltid: l.linked_text_id, liid: l.linked_image_id,
        }).collect(),
        tls: editor.text_layers.iter().map(|t| TLMeta {
            id: t.id, content: t.content.clone(), x: t.img_x, y: t.img_y, fs: t.font_size,
            bw: t.box_width, bh: t.box_height, rot: t.rotation,
            c: [t.color.r(), t.color.g(), t.color.b(), t.color.a()],
            bold: t.bold, ital: t.italic, ul: t.underline, font: t.font_name.clone(),
        }).collect(),
        ils: editor.image_layer_data.iter().map(|(&id, ild)| ILMeta {
            id, cx: ild.canvas_x, cy: ild.canvas_y, dw: ild.display_w, dh: ild.display_h,
            rot: ild.rotation, fh: ild.flip_h, fv: ild.flip_v,
        }).collect(),
        active: editor.active_layer_id, nlid: editor.next_layer_id,
        ntid: editor.next_text_id, niid: editor.next_image_layer_id,
    };
    fs::write(dir.join("meta.json"), serde_json::to_string(&m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn load_cache(path: &Path) -> Option<LoadedCache> {
    let dir = cache_dir_for(path);
    let m: Meta = serde_json::from_str(&fs::read_to_string(dir.join("meta.json")).ok()?).ok()?;
    if m.mod_ms != 0 && mod_ms(path) != m.mod_ms { return None; }
    let background = image::open(dir.join("bg.png")).ok();
    let layer_images = m.layers.iter().filter(|l| l.kind == LayerKind::Raster)
        .filter_map(|l| image::open(dir.join(format!("r{}.png", l.id))).ok().map(|i| (l.id, i))).collect();
    let image_layer_data = m.ils.iter().filter_map(|il| {
        let img = image::open(dir.join(format!("i{}.png", il.id))).ok()?;
        Some((il.id, ImageLayerData { image: img, canvas_x: il.cx, canvas_y: il.cy, display_w: il.dw, display_h: il.dh, rotation: il.rot, flip_h: il.fh, flip_v: il.fv }))
    }).collect();
    let layers = m.layers.into_iter().map(|l| ImageLayer {
        id: l.id, name: l.name, opacity: l.opacity, visible: l.visible, locked: l.locked,
        blend_mode: l.blend, kind: l.kind, linked_text_id: l.ltid, linked_image_id: l.liid,
    }).collect();
    let text_layers = m.tls.into_iter().map(|t| TextLayer {
        id: t.id, content: t.content, img_x: t.x, img_y: t.y, font_size: t.fs,
        box_width: t.bw, box_height: t.bh, rotation: t.rot,
        color: egui::Color32::from_rgba_unmultiplied(t.c[0], t.c[1], t.c[2], t.c[3]),
        bold: t.bold, italic: t.ital, underline: t.ul, font_name: t.font,
        rendered_height: 0.0, cached_lines: Vec::new(),
    }).collect();
    Some(LoadedCache { background, layers, layer_images, text_layers, image_layer_data, active_layer_id: m.active, next_layer_id: m.nlid, next_text_id: m.ntid, next_image_layer_id: m.niid })
}

pub fn apply_cache(editor: &mut ImageEditor, c: LoadedCache) {
    if let Some(bg) = c.background { editor.image = Some(bg); }
    editor.layers = c.layers;
    editor.layer_images = c.layer_images;
    editor.text_layers = c.text_layers;
    editor.image_layer_data = c.image_layer_data;
    editor.active_layer_id = c.active_layer_id;
    editor.next_layer_id = c.next_layer_id;
    editor.next_text_id = c.next_text_id;
    editor.next_image_layer_id = c.next_image_layer_id;
    for l in &editor.layers {
        match l.kind {
            LayerKind::Raster => { editor.raster_layer_texture_dirty.insert(l.id); }
            LayerKind::Image => { if let Some(iid) = l.linked_image_id { editor.image_layer_texture_dirty.insert(iid); } }
            _ => {}
        }
    }
    editor.composite_dirty = true;
    editor.texture_dirty = true;
}

pub fn list_caches() -> Vec<CacheEntry> {
    fs::read_dir(cache_base()).ok().map(|rd| {
        rd.flatten().filter_map(|e| {
            let dir = e.path();
            let m: Meta = serde_json::from_str(&fs::read_to_string(dir.join("meta.json")).ok()?).ok()?;
            let size_kb = fs::read_dir(&dir).ok()?.flatten().map(|f| f.metadata().ok().map(|m| m.len()).unwrap_or(0)).sum::<u64>() / 1024;
            Some(CacheEntry { src_path: m.path, cache_dir: dir, size_kb })
        }).collect()
    }).unwrap_or_default()
}

pub fn delete_all_caches() { let _ = fs::remove_dir_all(cache_base()); }
