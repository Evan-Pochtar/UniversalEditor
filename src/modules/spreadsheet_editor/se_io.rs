use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use ahash::AHashMap;
use super::se_model::{Sheet, CellFmt, HAlign};
use super::se_formula::{eval_cell, FVal};

fn estr<E: std::fmt::Debug>(e: E) -> String { format!("{:?}", e) }

fn char_width_to_px(w: f32) -> f32 { (w * 7.0 + 5.0).round() }
fn px_to_char_width(px: f32) -> f32 { ((px - 5.0) / 7.0).max(0.0) }

fn parse_sheet_col_widths(xml: &str) -> AHashMap<u32, f32> {
    use quick_xml::{Reader, events::Event};
    let mut out = AHashMap::default();
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"col" => {
                let get = |k: &[u8]| e.attributes().filter_map(|a| a.ok()).find(|a| a.key.local_name().as_ref()==k).and_then(|a| std::str::from_utf8(&a.value).ok().map(str::to_string));
                if let (Some(min), Some(max), Some(w)) = (get(b"min"), get(b"max"), get(b"width")) {
                    if let (Ok(min), Ok(max), Ok(w)) = (min.parse::<u32>(), max.parse::<u32>(), w.parse::<f32>()) {
                        for c in min..=max.min(min+2000) { out.insert(c-1, char_width_to_px(w)); }
                    }
                }
            }
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"sheetData" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

fn load_xlsx_col_widths(path: &Path) -> Vec<AHashMap<u32, f32>> {
    let mut out = Vec::new();
    let Ok(file) = std::fs::File::open(path) else { return out };
    let Ok(mut zip) = zip::ZipArchive::new(file) else { return out };
    let mut names: Vec<String> = (0..zip.len()).filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string())).filter(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml")).collect();
    names.sort_by_key(|n| n.trim_start_matches("xl/worksheets/sheet").trim_end_matches(".xml").parse::<u32>().unwrap_or(0));
    for name in names {
        if let Ok(mut f) = zip.by_name(&name) {
            let mut buf = Vec::new();
            let _ = f.by_ref().take(65536).read_to_end(&mut buf);
            out.push(parse_sheet_col_widths(&String::from_utf8_lossy(&buf)));
        }
    }
    out
}

fn col_widths_path(src: &Path) -> PathBuf {
    let abs = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    let mut h = DefaultHasher::new();
    abs.hash(&mut h);
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("universal_editor"); p.push("col_widths");
    p.push(format!("{:016x}.json", h.finish()));
    p
}

pub fn load_col_widths_sidecar(path: &Path, sheets: &mut [Sheet]) {
    let p = col_widths_path(path);
    let Ok(s) = std::fs::read_to_string(&p) else { return };
    let Ok(all) = serde_json::from_str::<Vec<Vec<(u32,f32)>>>(&s) else { return };
    for (sheet, widths) in sheets.iter_mut().zip(all) {
        for (c, w) in widths { sheet.col_widths.insert(c, w); }
    }
}

pub fn save_col_widths_sidecar(path: &Path, sheets: &[Sheet]) {
    let p = col_widths_path(path);
    if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
    let all: Vec<Vec<(u32,f32)>> = sheets.iter().map(|s| s.col_widths.iter().map(|(&c,&w)| (c,w)).collect()).collect();
    if let Ok(json) = serde_json::to_string(&all) { let _ = std::fs::write(p, json); }
}

pub fn load_delim(path: &Path, delim: u8) -> Sheet {
    let mut sheet = Sheet::new("Sheet1");
    if let Ok(content) = std::fs::read(path) {
        let mut rdr = csv::ReaderBuilder::new().delimiter(delim).has_headers(false).flexible(true).from_reader(content.as_slice());
        for (r, rec) in rdr.records().enumerate() {
            if let Ok(rec) = rec { for (c, field) in rec.iter().enumerate() { if !field.is_empty() { sheet.set_raw(r as u32, c as u32, field.to_string()); } } }
        }
    }
    load_col_widths_sidecar(path, std::slice::from_mut(&mut sheet));
    sheet
}

pub fn load_workbook(path: &Path) -> Vec<Sheet> {
    use calamine::Reader;
    let mut sheets = Vec::new();
    if let Ok(mut wb) = calamine::open_workbook_auto(path) {
        for name in wb.sheet_names() {
            if let Ok(range) = wb.worksheet_range(&name) {
                let (r0, c0) = range.start().unwrap_or((0, 0));
                let mut sheet = Sheet::new(name.clone());
                for (r, row) in range.rows().enumerate() { for (c, cell) in row.iter().enumerate() { let s = data_to_string(cell); if !s.is_empty() { sheet.set_raw(r as u32 + r0, c as u32 + c0, s); } } }
                if let Ok(formulas) = wb.worksheet_formula(&name) {
                    let (fr0, fc0) = formulas.start().unwrap_or((0, 0));
                    for (r, row) in formulas.rows().enumerate() { for (c, f) in row.iter().enumerate() { if !f.is_empty() { sheet.set_raw(r as u32 + fr0, c as u32 + fc0, format!("={}", f)); } } }
                }
                sheets.push(sheet);
            }
        }
    }
    if sheets.is_empty() { sheets.push(Sheet::new("Sheet1")); }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.eq_ignore_ascii_case("xlsx") || ext.eq_ignore_ascii_case("xlsm") {
            for (sheet, widths) in sheets.iter_mut().zip(load_xlsx_col_widths(path)) {
                for (c, w) in widths { sheet.col_widths.entry(c).or_insert(w); }
            }
        }
    }
    load_col_widths_sidecar(path, &mut sheets);
    sheets
}

fn data_to_string(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Float(f) => if f.fract()==0.0 && f.abs()<1e15 { format!("{}", *f as i64) } else { f.to_string() },
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Bool(b) => b.to_string(),
        calamine::Data::Error(_) | calamine::Data::Empty => String::new(),
        other => format!("{:?}", other).trim_matches('"').to_string(),
    }
}

pub fn save_delim(sheets: &[Sheet], path: &Path, delim: u8) -> Result<(), String> {
    let sheet = sheets.first().ok_or("No sheet to export")?;
    let file = std::fs::File::create(path).map_err(estr)?;
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_writer(file);
    let mut cache = AHashMap::default();
    for r in 0..=sheet.max_used_row() {
        let row: Vec<String> = (0..=sheet.max_used_col()).map(|c| { let mut stack = Vec::new(); eval_cell(sheet, r, c, &mut cache, &mut stack).display() }).collect();
        wtr.write_record(&row).map_err(estr)?;
    }
    wtr.flush().map_err(estr)
}

pub fn save_xlsx(sheets: &[Sheet], path: &Path) -> Result<(), String> {
    let mut wb = rust_xlsxwriter::Workbook::new();
    for sheet in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(&sheet.name).map_err(estr)?;
        for (&c, &w) in &sheet.col_widths { ws.set_column_width(c as u16, px_to_char_width(w) as f64).map_err(estr)?; }
        for (&(r,c), cell) in sheet.cells.iter() {
            if cell.raw.is_empty() && cell.fmt == CellFmt::default() { continue; }
            let fmt = xlsx_format(&cell.fmt);
            if let Some(f) = cell.raw.strip_prefix('=') { ws.write_with_format(r, c as u16, rust_xlsxwriter::Formula::new(format!("={}", f)), &fmt).map_err(estr)?; }
            else if let Ok(n) = cell.raw.parse::<f64>() { ws.write_with_format(r, c as u16, n, &fmt).map_err(estr)?; }
            else { ws.write_with_format(r, c as u16, cell.raw.as_str(), &fmt).map_err(estr)?; }
        }
    }
    wb.save(path).map_err(estr)
}

fn xlsx_format(fmt: &CellFmt) -> rust_xlsxwriter::Format {
    let mut f = rust_xlsxwriter::Format::new();
    if fmt.bold { f = f.set_bold(); }
    if fmt.italic { f = f.set_italic(); }
    if fmt.underline { f = f.set_underline(rust_xlsxwriter::FormatUnderline::Single); }
    if let Some(c) = fmt.color { f = f.set_font_color(rust_xlsxwriter::Color::RGB(rgb_u32(c))); }
    if let Some(c) = fmt.bg { f = f.set_background_color(rust_xlsxwriter::Color::RGB(rgb_u32(c))); }
    match fmt.align { HAlign::Left => f.set_align(rust_xlsxwriter::FormatAlign::Left), HAlign::Center => f.set_align(rust_xlsxwriter::FormatAlign::Center), HAlign::Right => f.set_align(rust_xlsxwriter::FormatAlign::Right), HAlign::General => f }
}

fn rgb_u32(c: [u8;3]) -> u32 { ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32 }

const ODS_MIME: &str = "application/vnd.oasis.opendocument.spreadsheet";
const ODS_MANIFEST: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.spreadsheet\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>";

fn xml_esc(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;") }

pub fn save_ods(sheets: &[Sheet], path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(estr)?;
    let mut zip = zip::ZipWriter::new(file);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default();
    zip.start_file("mimetype", stored).map_err(estr)?; zip.write_all(ODS_MIME.as_bytes()).map_err(estr)?;
    zip.start_file("META-INF/manifest.xml", deflated).map_err(estr)?; zip.write_all(ODS_MANIFEST.as_bytes()).map_err(estr)?;
    zip.start_file("content.xml", deflated).map_err(estr)?; zip.write_all(build_ods_content(sheets).as_bytes()).map_err(estr)?;
    zip.finish().map_err(estr)?;
    Ok(())
}

fn build_ods_content(sheets: &[Sheet]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\"><office:body><office:spreadsheet>");
    for sheet in sheets {
        let mut cache = AHashMap::default();
        out.push_str(&format!("<table:table table:name=\"{}\">", xml_esc(&sheet.name)));
        let max_c = sheet.max_used_col(); let max_r = sheet.max_used_row();
        for _ in 0..=max_c { out.push_str("<table:table-column/>"); }
        for r in 0..=max_r {
            out.push_str("<table:table-row>");
            for c in 0..=max_c {
                let mut stack = Vec::new();
                let val = eval_cell(sheet, r, c, &mut cache, &mut stack);
                let disp = val.display();
                if disp.is_empty() { out.push_str("<table:table-cell/>"); continue; }
                if matches!(val, FVal::Num(_)) { out.push_str(&format!("<table:table-cell office:value-type=\"float\" office:value=\"{}\"><text:p>{}</text:p></table:table-cell>", disp, xml_esc(&disp))); }
                else { out.push_str(&format!("<table:table-cell office:value-type=\"string\"><text:p>{}</text:p></table:table-cell>", xml_esc(&disp))); }
            }
            out.push_str("</table:table-row>");
        }
        out.push_str("</table:table>");
    }
    out.push_str("</office:spreadsheet></office:body></office:document-content>");
    out
}
