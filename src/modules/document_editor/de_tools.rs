use eframe::egui;
use std::{io::{Read, Write}, path::PathBuf};
use crate::style::ColorPalette;

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpg" ContentType="image/jpeg"/><Default Extension="jpeg" ContentType="image/jpeg"/><Default Extension="webp" ContentType="image/webp"/><Default Extension="bmp" ContentType="image/bmp"/><Default Extension="gif" ContentType="image/gif"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;
const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
const ODT_MANIFEST: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.text\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>";
pub const DEFAULT_BASE_SIZE: u32 = 11;
pub const DEFAULT_BASE_FONT: FontChoice = FontChoice::Ubuntu;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontChoice { #[default] Ubuntu, Roboto, GoogleSans, OpenSans }
impl FontChoice {
    pub fn label(self) -> &'static str { match self { Self::Ubuntu => "Ubuntu", Self::Roboto => "Roboto", Self::GoogleSans => "Google Sans", Self::OpenSans => "Open Sans" } }
    pub fn egui_family(self, bold: bool, italic: bool) -> egui::FontFamily {
        egui::FontFamily::Name(match (self, bold, italic) {
            (Self::Ubuntu, true, true) => "Ubuntu-BoldItalic", (Self::Ubuntu, true, _) => "Ubuntu-Bold",
            (Self::Ubuntu, _, true) => "Ubuntu-Italic", (Self::Ubuntu, _, _) => "Ubuntu",
            (Self::Roboto, true, true) => "Roboto-BoldItalic", (Self::Roboto, true, _) => "Roboto-Bold",
            (Self::Roboto, _, true) => "Roboto-Italic", (Self::Roboto, _, _) => "Roboto",
            (Self::GoogleSans, true, true) => "GoogleSans-BoldItalic", (Self::GoogleSans, true, _) => "GoogleSans-Bold",
            (Self::GoogleSans, _, true) => "GoogleSans-Italic", (Self::GoogleSans, _, _) => "GoogleSans",
            (Self::OpenSans, true, true) => "OpenSans-BoldItalic", (Self::OpenSans, true, _) => "OpenSans-Bold",
            (Self::OpenSans, _, true) => "OpenSans-Italic", (Self::OpenSans, _, _) => "OpenSans",
        }.into())
    }
    pub fn all() -> &'static [FontChoice] { &[Self::Ubuntu, Self::Roboto, Self::GoogleSans, Self::OpenSans] }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ParaStyle {
    #[default] Normal, H1, H2, H3, H4, H5, H6,
    Title, Subtitle, BlockQuote, Code, ListBullet, ListOrdered, ListCheck, HRule, Table, Image,
}
impl ParaStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal", Self::H1 => "Heading 1", Self::H2 => "Heading 2",
            Self::H3 => "Heading 3", Self::H4 => "Heading 4", Self::H5 => "Heading 5",
            Self::H6 => "Heading 6", Self::Title => "Title", Self::Subtitle => "Subtitle",
            Self::BlockQuote => "Block Quote", Self::Code => "Code Block",
            Self::ListBullet => "Bullet List", Self::ListOrdered => "Numbered List", Self::ListCheck => "Checklist",
            Self::HRule => "Horizontal Rule", Self::Table => "Table", Self::Image => "Image",
        }
    }
    pub fn all() -> &'static [ParaStyle] {
        &[Self::Normal, Self::H1, Self::H2, Self::H3, Self::H4, Self::H5, Self::H6,
          Self::Title, Self::Subtitle, Self::BlockQuote, Self::Code, Self::ListBullet, Self::ListOrdered, Self::ListCheck]
    }
    pub fn is_heading(self) -> bool { matches!(self, Self::H1|Self::H2|Self::H3|Self::H4|Self::H5|Self::H6|Self::Title|Self::Subtitle) }
    pub fn is_bold(self) -> bool { matches!(self, Self::H1|Self::H2|Self::H3|Self::H4|Self::H5|Self::H6|Self::Title) }
    pub fn is_italic(self) -> bool { matches!(self, Self::Subtitle|Self::BlockQuote) }
    pub fn space_before(self) -> f32 { match self { Self::H1|Self::H2 => 16.0, Self::H3|Self::H4 => 12.0, Self::H5|Self::H6|Self::Title => 8.0, _ => 0.0 } }
    pub fn space_after(self) -> f32 { match self { Self::H1 | Self::H2 => 8.0, Self::H3 | Self::H4 => 8.0, Self::Normal | Self::Table | Self::Image | Self::ListBullet | Self::ListOrdered | Self::ListCheck | Self::HRule => 0.0, _ => 6.0, } }
    pub fn default_indent(self) -> f32 { match self { Self::ListBullet|Self::ListOrdered|Self::ListCheck => 36.0, Self::BlockQuote => 36.0, _ => 0.0 } }
    pub fn default_font_size_pt(self) -> u32 {
        match self {
            Self::Title => 26, Self::H1 => 20, Self::H2 => 16, Self::H3 => 14,
            Self::H4 => 13, Self::H5 => 12, Self::Subtitle => 15, _ => DEFAULT_BASE_SIZE,
        }
    }
    pub fn outline_depth(self) -> Option<u8> {
        match self { Self::Title|Self::Subtitle => Some(0), Self::H1 => Some(1), Self::H2 => Some(2), Self::H3 => Some(3), Self::H4 => Some(4), Self::H5 => Some(5), Self::H6 => Some(6), _ => None }
    }
    pub fn docx_id(self) -> &'static str {
        match self { Self::Normal => "Normal", Self::H1 => "Heading1", Self::H2 => "Heading2", Self::H3 => "Heading3",
            Self::H4 => "Heading4", Self::H5 => "Heading5", Self::H6 => "Heading6",
            Self::Title => "Title", Self::Subtitle => "Subtitle", Self::BlockQuote => "Quote",
            Self::Code => "CodeBlock", Self::ListBullet => "ListBullet", Self::ListOrdered => "ListNumber", Self::ListCheck => "ListCheck",
            Self::HRule => "HRule", Self::Table => "Table", Self::Image => "Image" }
    }
    pub fn from_docx_id(s: &str) -> Self {
        match s {
            "Heading1"|"Heading 1" => Self::H1, "Heading2"|"Heading 2" => Self::H2,
            "Heading3"|"Heading 3" => Self::H3, "Heading4"|"Heading 4" => Self::H4,
            "Heading5"|"Heading 5" => Self::H5, "Heading6"|"Heading 6" => Self::H6,
            "Title" => Self::Title, "Subtitle" => Self::Subtitle, "Quote"|"BlockText" => Self::BlockQuote,
            "CodeBlock"|"Code" => Self::Code,
            "ListBullet"|"ListBullet2"|"List Bullet"|"List Bullet 2"|"ListParagraph" => Self::ListBullet,
            "ListNumber"|"ListNumber2"|"List Number"|"List Number 2" => Self::ListOrdered,
            "ListCheck"|"CheckList"|"Checklist"|"Check List"|"Task List" => Self::ListCheck, _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Align { #[default] Left, Center, Right, Justify }
impl Align {
    pub fn label(self) -> &'static str { match self { Self::Left => "L", Self::Center => "C", Self::Right => "R", Self::Justify => "J" } }
    pub fn full_label(self) -> &'static str { match self { Self::Left => "Left", Self::Center => "Center", Self::Right => "Right", Self::Justify => "Justify" } }
    pub fn docx_val(self) -> &'static str { match self { Self::Left => "left", Self::Center => "center", Self::Right => "right", Self::Justify => "both" } }
    pub fn egui_align(self) -> egui::Align { match self { Self::Center => egui::Align::Center, Self::Right => egui::Align::RIGHT, _ => egui::Align::LEFT } }
    pub fn all() -> &'static [Align] { &[Self::Left, Self::Center, Self::Right, Self::Justify] }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpanFmt {
    pub bold: bool, pub italic: bool, pub underline: bool, pub strike: bool,
    pub sub: bool, pub sup: bool, pub size_hp: Option<u32>,
    pub font: Option<FontChoice>, pub color: Option<[u8; 3]>,
    pub highlight: Option<[u8; 3]>,
    pub link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DocSpan { pub len: usize, pub fmt: SpanFmt }

#[derive(Debug, Clone, Default)]
pub struct TableCell { pub text: String, pub spans: Vec<DocSpan>, pub bg_color: Option<[u8; 3]> }
#[derive(Debug, Clone)]
pub struct TableData { pub rows: Vec<Vec<TableCell>>, pub col_widths: Vec<f32>, pub border_color: [u8; 3], pub border_width: f32 }

#[derive(Debug, Clone, Default)]
pub struct DocImage { pub data: Vec<u8>, pub display_w: f32, pub display_h: f32, pub name: String, pub uid: u64, }

#[derive(Debug, Clone)]
pub struct DocParagraph {
    pub text: String, pub spans: Vec<DocSpan>, pub style: ParaStyle, pub align: Align,
    pub indent_left: f32, pub indent_first: f32, pub space_before: f32, pub space_after: f32,
    pub line_height: f32, pub list_num: Option<u32>, pub checked: bool, pub is_split: bool, pub table: Option<Box<TableData>>, pub image: Option<DocImage>,
}
impl DocParagraph {
    pub fn new() -> Self {
        Self { text: String::new(), spans: vec![DocSpan { len: 0, fmt: SpanFmt::default() }],
            style: ParaStyle::Normal, align: Align::Left, indent_left: 0.0, indent_first: 0.0,
            space_before: 0.0, space_after: 0.0, line_height: 1.15, list_num: None, checked: false, is_split: false, table: None, image: None, }
    }
    pub fn with_style(s: ParaStyle) -> Self {
        let mut p = Self::new();
        p.style = s; p.space_before = s.space_before(); p.space_after = s.space_after(); p.indent_left = s.default_indent(); p.checked = false; p
    }
}

#[derive(Debug, Clone)]
pub struct PageLayout {
    pub width: f32, pub height: f32, pub margin_top: f32, pub margin_bot: f32, pub margin_left: f32, pub margin_right: f32,
}
impl Default for PageLayout {
    fn default() -> Self { Self { width: 612.0, height: 792.0, margin_top: 72.0, margin_bot: 72.0, margin_left: 72.0, margin_right: 72.0 } }
}
impl PageLayout {
    pub const PTS_PER_INCH: f32 = 72.0;
    pub const CUSTOM: usize = 9;
    pub fn content_width(&self) -> f32 { self.width - self.margin_left - self.margin_right }
    pub fn content_height(&self) -> f32 { self.height - self.margin_top - self.margin_bot }
    fn inch(w: f32, h: f32, mt: f32, mb: f32, ml: f32, mr: f32) -> Self { let p = Self::PTS_PER_INCH; Self { width: w * p, height: h * p, margin_top: mt * p, margin_bot: mb * p, margin_left: ml * p, margin_right: mr * p } }
    pub fn letter() -> Self { Self::inch(8.5, 11.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn letter_word() -> Self { Self::inch(8.5, 11.0, 1.0, 1.0, 1.25, 1.25) }
    pub fn a4() -> Self { Self::inch(8.2677, 11.6929, 1.0, 1.0, 1.0, 1.0) }
    pub fn legal() -> Self { Self::inch(8.5, 14.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn a3() -> Self { Self::inch(11.6929, 16.5354, 1.0, 1.0, 1.0, 1.0) }
    pub fn a5() -> Self { Self::inch(5.8268, 8.2677, 1.0, 1.0, 1.0, 1.0) }
    pub fn executive() -> Self { Self::inch(7.25, 10.5, 1.0, 1.0, 1.0, 1.0) }
    pub fn tabloid() -> Self { Self::inch(11.0, 17.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn b5() -> Self { Self::inch(6.9291, 9.8425, 1.0, 1.0, 1.0, 1.0) }
    pub fn presets() -> &'static [(&'static str, &'static str)] { &[("Letter", "8.5 x 11 in  — Google Docs default"), ("Letter (Word)", "8.5 x 11 in  — Word default (1.25\" sides)"), ("A4", "210 x 297 mm  — International standard"), ("Legal", "8.5 x 14 in  — US legal"), ("A3", "297 x 420 mm"), ("A5", "148 x 210 mm"), ("Executive", "7.25 x 10.5 in"), ("Tabloid", "11 x 17 in"), ("B5", "176 x 250 mm"), ("Custom", "Enter your own page size")] }
    pub fn from_preset(i: usize) -> Self { match i { 0 => Self::letter(), 1 => Self::letter_word(), 2 => Self::a4(), 3 => Self::legal(), 4 => Self::a3(), 5 => Self::a5(), 6 => Self::executive(), 7 => Self::tabloid(), 8 => Self::b5(), _ => Self::default() } }
    pub fn custom_in(w: f32, h: f32, mt: f32, mb: f32, ml: f32, mr: f32) -> Result<Self, &'static str> {
        if [w, h, mt, mb, ml, mr].iter().any(|v| !v.is_finite()) { return Err("Values must be numbers."); }
        if w <= 0.0 || h <= 0.0 { return Err("Page size must be greater than zero."); }
        if w > 200.0 || h > 200.0 { return Err("Page size is too large."); }
        if [mt, mb, ml, mr].iter().any(|v| *v < 0.0) { return Err("Margins cannot be negative."); }
        if ml + mr >= w || mt + mb >= h { return Err("Margins must fit inside the page."); }
        Ok(Self::inch(w, h, mt, mb, ml, mr))
    }
    pub fn preset_idx(&self) -> usize {
        let p = Self::PTS_PER_INCH; let e = |a: f32, b: f32| (a - b).abs() < 0.05;
        [(8.5, 11.0, 1.0, 1.0, 1.0, 1.0), (8.5, 11.0, 1.0, 1.0, 1.25, 1.25), (8.2677, 11.6929, 1.0, 1.0, 1.0, 1.0), (8.5, 14.0, 1.0, 1.0, 1.0, 1.0), (11.6929, 16.5354, 1.0, 1.0, 1.0, 1.0), (5.8268, 8.2677, 1.0, 1.0, 1.0, 1.0), (7.25, 10.5, 1.0, 1.0, 1.0, 1.0), (11.0, 17.0, 1.0, 1.0, 1.0, 1.0), (6.9291, 9.8425, 1.0, 1.0, 1.0, 1.0)].iter().position(|&(w, h, mt, mb, ml, mr)| e(self.width / p, w) && e(self.height / p, h) && e(self.margin_top / p, mt) && e(self.margin_bot / p, mb) && e(self.margin_left / p, ml) && e(self.margin_right / p, mr)).unwrap_or(Self::CUSTOM)
    }
}

pub fn highlight_color32(rgb: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (0.40 * 255.0_f32).round() as u8)
}

fn ensure_non_empty_spans(spans: &mut Vec<DocSpan>) { if spans.is_empty() { spans.push(DocSpan { len: 0, fmt: SpanFmt::default() }); }}
fn append_span(spans: &mut Vec<DocSpan>, span: DocSpan) {
    if span.len == 0 { return; }
    if let Some(last) = spans.last_mut() {
        if last.fmt == span.fmt { last.len += span.len; return; }
    }
    spans.push(span);
}

pub fn ensure_boundary(para: &mut DocParagraph, byte: usize) {
    if byte == 0 || byte >= para.text.len() { return; }
    let mut consumed = 0usize;
    for index in 0..para.spans.len() {
        let span_len = para.spans[index].len;
        let next = consumed + span_len;
        if next == byte { return; }
        if next > byte {
            let right_len = next - byte;
            para.spans[index].len -= right_len;
            let right = DocSpan { len: right_len, fmt: para.spans[index].fmt.clone() };
            para.spans.insert(index + 1, right);
            return;
        }
        consumed = next;
    }
}

pub fn merge_adjacent(para: &mut DocParagraph) {
    let mut merged: Vec<DocSpan> = Vec::with_capacity(para.spans.len());
    for span in para.spans.drain(..) {
        append_span(&mut merged, span);
    }
    ensure_non_empty_spans(&mut merged);
    para.spans = merged;
}

pub fn apply_fmt_range(para: &mut DocParagraph, start: usize, end: usize, op: impl Fn(&mut SpanFmt)) {
    if start >= end { return; }
    ensure_boundary(para, start); ensure_boundary(para, end);
    let mut pos = 0usize;
    for span in &mut para.spans {
        let span_end = pos + span.len;
        if pos >= end { break; }
        if span_end > start && pos < end { op(&mut span.fmt); }
        pos = span_end;
    }
    merge_adjacent(para);
}

pub fn all_set_range(para: &DocParagraph, start: usize, end: usize, get: impl Fn(&SpanFmt) -> bool) -> bool {
    let mut pos = 0usize; let mut any = false;
    for span in &para.spans {
        let span_end = pos + span.len;
        let overlaps = pos < end && span_end > start;
        if overlaps {
            any = true;
            if !get(&span.fmt) { return false; }
        }
        pos = span_end;
    }
    any
}

pub fn char_to_byte(text: &str, ci: usize) -> usize { text.char_indices().nth(ci).map(|(i, _)| i).unwrap_or(text.len()) }
pub fn merge_paragraphs(paras: &mut Vec<DocParagraph>, idx: usize) {
    if idx + 1 >= paras.len() { return; }
    let next = paras.remove(idx + 1);
    let next_has_text = next.spans.iter().any(|s| s.len > 0);
    if next_has_text {
        if paras[idx].spans.last().map(|span| span.len == 0).unwrap_or(false) {  
            paras[idx].spans.pop(); 
        }
    }
    paras[idx].text.push_str(&next.text);
    for span in next.spans { append_span(&mut paras[idx].spans, span); }
    ensure_non_empty_spans(&mut paras[idx].spans);
}

fn is_checkbox_marker(text: &str) -> Option<bool> {
    const UNCHECKED_MARKERS: &[char] = &['☐', '□', '\u{F0A8}', '\u{E000}', '\u{F0B1}'];
    const CHECKED_MARKERS: &[char] = &['☑', '☒', '\u{F0FE}', '✓', '✔', '\u{E001}'];
    let mut decoded = text.to_owned();
    if text.len() == 4 {
        if let Ok(code_point) = u32::from_str_radix(text, 16) {
            if let Some(ch) = std::char::from_u32(code_point) {
                decoded.push(ch);
            }
        }
    }
    if decoded.chars().any(|ch| UNCHECKED_MARKERS.contains(&ch)) {
        Some(false)
    } else if decoded.chars().any(|ch| CHECKED_MARKERS.contains(&ch)) {
        Some(true)
    } else {
        None
    }
}

pub fn para_fmt_at(para: &DocParagraph, byte: usize) -> SpanFmt {
    let mut pos = 0usize;
    for span in &para.spans {
        let span_end = pos + span.len;
        if byte >= pos && byte < span_end {
            return span.fmt.clone();
        }
        if byte == 0 && span_end == 0 {
            return span.fmt.clone();
        }
        pos = span_end;
    }

    let mut fmt = para.spans.last().map(|span| span.fmt.clone()).unwrap_or_default();
    fmt.link = None;
    fmt
}

pub fn rebuild_spans(para: &mut DocParagraph, new_text: String, cur_fmt: &SpanFmt) {
    let old_len = para.text.len();
    let new_len = new_text.len();
    if old_len == new_len { para.text = new_text; return; }
    if new_len == 0 { para.text = new_text; para.spans = vec![DocSpan { len: 0, fmt: cur_fmt.clone() }]; return; }

    let common_prefix = para.text.bytes().zip(new_text.bytes()).take_while(|(a, b)| a == b).count();
    let max_suf = old_len.min(new_len).saturating_sub(common_prefix);
    let common_suffix = para.text.bytes().rev().zip(new_text.bytes().rev()).take(max_suf).take_while(|(a, b)| a == b).count();
    let del_start = common_prefix; let del_end = old_len - common_suffix;
    let ins_len = new_len - common_suffix - common_prefix;

    if del_end > del_start {
        ensure_boundary(para, del_start); ensure_boundary(para, del_end);
        let mut p = 0usize;
        para.spans.retain(|s| { let e = p + s.len; let keep = !(p >= del_start && e <= del_end); p = e; keep });
        let is_single = para.spans.len() == 1;
        para.spans.retain(|s| s.len > 0 || is_single);
    }

    if ins_len > 0 {
        let mut p = 0usize; let mut done = false;
        for i in 0..para.spans.len() {
            if p == del_start {
                if i > 0 && para.spans[i-1].fmt == *cur_fmt { para.spans[i-1].len += ins_len; }
                else if para.spans[i].fmt == *cur_fmt { para.spans[i].len += ins_len; }
                else { para.spans.insert(i, DocSpan { len: ins_len, fmt: cur_fmt.clone() }); }
                done = true; break;
            }
            p += para.spans[i].len;
        }
        if !done {
            if para.spans.last().map(|s| s.fmt == *cur_fmt).unwrap_or(false) { para.spans.last_mut().unwrap().len += ins_len; }
            else { if para.spans.last().map(|s| s.len == 0).unwrap_or(false) { para.spans.pop(); } para.spans.push(DocSpan { len: ins_len, fmt: cur_fmt.clone() }); }
        }
    }

    para.text = new_text;
    if para.spans.is_empty() { para.spans.push(DocSpan { len: 0, fmt: SpanFmt::default() }); }
    merge_adjacent(para);
}

pub fn build_layout_job(spans: &[DocSpan], text: &str, para: &DocParagraph, wrap_w: f32, is_dark: bool, zoom: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_w;
    job.halign = egui::Align::LEFT;
    job.justify = false;

    let default_size = para.style.default_font_size_pt() as f32 * zoom;
    let (style_bold, style_italic) = (para.style.is_bold(), para.style.is_italic());
    let code_background = if is_dark { egui::Color32::from_rgb(28, 28, 34) } else { egui::Color32::from_rgb(244, 244, 248) };
    let base_color = match para.style {
        ParaStyle::H1 | ParaStyle::H2 => if is_dark { ColorPalette::ZINC_100 } else { ColorPalette::ZINC_900 },
        ParaStyle::H3 | ParaStyle::H4 => if is_dark { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 },
        ParaStyle::Subtitle | ParaStyle::BlockQuote => if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 },
        _ => if is_dark { ColorPalette::ZINC_200 } else { egui::Color32::from_rgb(22, 22, 22) },
    };

    let mut pos = 0usize;
    for span in spans {
        if span.len == 0 || pos >= text.len() { continue; }
        let end = (pos + span.len).min(text.len());
        let segment = &text[pos..end];
        let first_segment = pos == 0;
        pos = end;
        if segment.is_empty() { continue; }

        let effective_size = span.fmt.size_hp.map(|hp| hp as f32 / 2.0 * zoom).unwrap_or(default_size);
        let font_size = if span.fmt.sub || span.fmt.sup { effective_size * 0.68 } else { effective_size };
        let font_choice = span.fmt.font.unwrap_or(DEFAULT_BASE_FONT);
        let mut color = span.fmt.color.map(|rgb| egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])).unwrap_or(base_color);
        if span.fmt.link.is_some() && span.fmt.color.is_none() {
            color = if is_dark { egui::Color32::from_rgb(96, 165, 250) } else { egui::Color32::from_rgb(37, 99, 235) };
        }
        let background = span.fmt.highlight.map(highlight_color32).unwrap_or_else(|| {
            if para.style == ParaStyle::Code { code_background } else { egui::Color32::TRANSPARENT }
        });
        let stroke_width = (effective_size * 0.07).max(1.0);

        job.append(
            segment,
            if first_segment { para.indent_first * zoom } else { 0.0 },
            egui::TextFormat {
                font_id: egui::FontId::new(font_size, font_choice.egui_family(style_bold || span.fmt.bold, style_italic || span.fmt.italic)),
                color, background,
                underline: if span.fmt.underline || span.fmt.link.is_some() { egui::Stroke::new(stroke_width, color) } else { egui::Stroke::NONE },
                strikethrough: if span.fmt.strike { egui::Stroke::new(stroke_width, color) } else { egui::Stroke::NONE },
                valign: if span.fmt.sup { egui::Align::TOP } else if span.fmt.sub { egui::Align::BOTTOM } else { egui::Align::Center },
                line_height: if span.fmt.sup || span.fmt.sub { None } else { Some(effective_size * para.line_height) },
                ..Default::default()
            },
        );
    }

    if job.sections.is_empty() {
        let first_fmt = para.spans.first().map(|s| &s.fmt).cloned().unwrap_or_default();
        let effective_size = first_fmt.size_hp.map(|hp| hp as f32 / 2.0 * zoom).unwrap_or(default_size);
        let font_size = if first_fmt.sub || first_fmt.sup { effective_size * 0.68 } else { effective_size };
        let font_choice = first_fmt.font.unwrap_or(DEFAULT_BASE_FONT);
        let mut color = first_fmt.color.map(|rgb| egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])).unwrap_or(base_color);
        if first_fmt.link.is_some() && first_fmt.color.is_none() {
            color = if is_dark { egui::Color32::from_rgb(96, 165, 250) } else { egui::Color32::from_rgb(37, 99, 235) };
        }
        let background = first_fmt.highlight.map(highlight_color32).unwrap_or_else(|| {
            if para.style == ParaStyle::Code { code_background } else { egui::Color32::TRANSPARENT }
        });
        let stroke_width = (effective_size * 0.07).max(1.0);
        job.append(
            "",
            para.indent_first * zoom,
            egui::TextFormat {
                font_id: egui::FontId::new(font_size, font_choice.egui_family(style_bold || first_fmt.bold, style_italic || first_fmt.italic)),
                color,
                background,
                underline: if first_fmt.underline || first_fmt.link.is_some() { egui::Stroke::new(stroke_width, color) } else { egui::Stroke::NONE },
                strikethrough: if first_fmt.strike { egui::Stroke::new(stroke_width, color) } else { egui::Stroke::NONE },
                valign: if first_fmt.sup { egui::Align::TOP } else if first_fmt.sub { egui::Align::BOTTOM } else { egui::Align::Center },
                line_height: if first_fmt.sup || first_fmt.sub { None } else { Some(effective_size * para.line_height) },
                ..Default::default()
            },
        );
    }
    job
}

pub fn convert_leading_tabs_to_indent(paras: &mut Vec<DocParagraph>) {
    for para in paras {
        let tab_count = para.text.bytes().take_while(|byte| *byte == b'\t').count();
        if tab_count == 0 { continue; }
        para.indent_first += 36.0 * tab_count as f32;
        para.text = para.text[tab_count..].to_string();
        let mut remaining = tab_count;
        for span in &mut para.spans {
            if remaining == 0 { break; }
            let trimmed = span.len.min(remaining);
            span.len -= trimmed;
            remaining -= trimmed;
        }
        para.spans.retain(|span| span.len > 0);
        ensure_non_empty_spans(&mut para.spans);
    }
}

fn table_text_count<F>(table: &TableData, count: F) -> usize where F: Fn(&str) -> usize + Copy, {
    table.rows.iter().flat_map(|row| row.iter()).map(|cell| count(&cell.text)).sum()
}
pub fn word_count(paras: &[DocParagraph]) -> usize {
    paras.iter().map(|para| {
            para.text.split_whitespace().count()
                + para.table.as_ref().map_or(0, |table| table_text_count(table, |text| text.split_whitespace().count()))
        }).sum()
}
pub fn char_count(paras: &[DocParagraph]) -> usize {
    paras.iter().map(|para| {
            para.text.chars().count() + para.table.as_ref().map_or(0, |table| table_text_count(table, |text| text.chars().count()))
        }).sum()
}

fn style_size_hp(style: ParaStyle) -> Option<u32> {
    match style {
        ParaStyle::Title => Some(52),
        ParaStyle::H1 => Some(40), ParaStyle::H2 => Some(32), ParaStyle::H3 => Some(28),
        ParaStyle::H4 => Some(26), ParaStyle::H5 => Some(24), ParaStyle::H6 => Some(22),
        ParaStyle::Subtitle => Some(30),
        ParaStyle::Code => Some(22),
        _ => None,
    }
}

fn docx_pstyle_block(style: ParaStyle) -> String {
    let (style_id, display_name, based_on, outline, spacing_before, spacing_after, indent_left, size_hp, bold, italic, font_name) = match style {
        ParaStyle::Normal => return String::new(),
        ParaStyle::Table => return String::new(),
        ParaStyle::Image => return String::new(),
        ParaStyle::H1 => ("Heading1", "Heading 1", Some("Normal"), Some(0), 320, 80, None, style_size_hp(style), true, false, None),
        ParaStyle::H2 => ("Heading2", "Heading 2", Some("Normal"), Some(1), 240, 80, None, style_size_hp(style), true, false, None),
        ParaStyle::H3 => ("Heading3", "Heading 3", Some("Normal"), Some(2), 200, 60, None, style_size_hp(style), true, false, None),
        ParaStyle::H4 => ("Heading4", "Heading 4", Some("Normal"), Some(3), 160, 60, None, style_size_hp(style), true, false, None),
        ParaStyle::H5 => ("Heading5", "Heading 5", Some("Normal"), Some(4), 120, 40, None, style_size_hp(style), true, false, None),
        ParaStyle::H6 => ("Heading6", "Heading 6", Some("Normal"), Some(5), 100, 40, None, style_size_hp(style), true, false, None),
        ParaStyle::Title => ("Title", "Title", Some("Normal"), Some(0), 0, 120, None, style_size_hp(style), true, false, None),
        ParaStyle::Subtitle => ("Subtitle", "Subtitle", Some("Normal"), Some(0), 0, 120, None, style_size_hp(style), false, true, None),
        ParaStyle::BlockQuote => ("Quote", "Quote", Some("Normal"), None, 80, 80, Some(480), style_size_hp(style), false, true, None),
        ParaStyle::Code => ("CodeBlock", "Code Block", Some("Normal"), None, 80, 80, Some(480), style_size_hp(style), false, false, Some("Consolas")),
        ParaStyle::ListBullet => ("ListBullet", "List Bullet", Some("Normal"), None, 0, 0, Some(360), Some(22), false, false, None),
        ParaStyle::ListOrdered => ("ListNumber", "List Number", Some("Normal"), None, 0, 0, Some(360), Some(22), false, false, None),
        ParaStyle::ListCheck => ("ListCheck", "List Check", Some("Normal"), None, 0, 0, Some(360), Some(22), false, false, None),
        ParaStyle::HRule => return String::new(),
    };

    let mut out = format!(r#"<w:style w:type="paragraph" w:styleId="{}"><w:name w:val="{}"/>"#, style_id, display_name);
    if let Some(base) = based_on { out.push_str(&format!(r#"<w:basedOn w:val="{}"/>"#, base)); }
    out.push_str("<w:qFormat/>");
    if let Some(lvl) = outline { out.push_str(&format!(r#"<w:outlineLvl w:val="{}"/>"#, lvl)); }
    out.push_str("<w:pPr>");
    if spacing_before != 0 || spacing_after != 0 { out.push_str(&format!(r#"<w:spacing w:before="{}" w:after="{}"/>"#, spacing_before, spacing_after)); }
    if let Some(left) = indent_left { out.push_str(&format!(r#"<w:ind w:left="{}"/>"#, left)); }
    out.push_str("</w:pPr><w:rPr>");
    if bold { out.push_str("<w:b/>"); }
    if italic { out.push_str("<w:i/>"); }
    if let Some(sz) = size_hp { out.push_str(&format!(r#"<w:sz w:val="{}"/><w:szCs w:val="{}"/>"#, sz, sz)); }
    if let Some(font) = font_name { out.push_str(&format!(r#"<w:rFonts w:ascii="{}" w:hAnsi="{}" w:cs="{}"/>"#, font, font, font)); }
    out.push_str("</w:rPr></w:style>");
    out
}

fn build_word_rels(hyperlinks: &[(String, String)], image_names: &[String]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>");
    for (id, url) in hyperlinks { out.push_str(&format!("<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"{}\" TargetMode=\"External\"/>", id, xml_esc(url))); }
    for (idx, name) in image_names.iter().enumerate() { out.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{}\"/>", idx + 1003, name)); }
    out.push_str("</Relationships>");
    out
}

fn build_numbering_xml() -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:abstractNum w:abstractNumId=\"0\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"{}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Arial\" w:hAnsi=\"Arial\"/><w:sz w:val=\"22\"/></w:rPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId=\"1\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/><w:lvlText w:val=\"%1.\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr><w:rPr><w:sz w:val=\"22\"/></w:rPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId=\"2\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"{}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Wingdings\" w:hAnsi=\"Wingdings\" w:cs=\"Wingdings\"/><w:sz w:val=\"22\"/></w:rPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId=\"3\"><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"bullet\"/><w:lvlText w:val=\"{}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr><w:rPr><w:rFonts w:ascii=\"Wingdings\" w:hAnsi=\"Wingdings\" w:cs=\"Wingdings\"/><w:sz w:val=\"22\"/></w:rPr></w:lvl></w:abstractNum><w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num><w:num w:numId=\"2\"><w:abstractNumId w:val=\"1\"/></w:num><w:num w:numId=\"3\"><w:abstractNumId w:val=\"2\"/></w:num><w:num w:numId=\"4\"><w:abstractNumId w:val=\"3\"/></w:num></w:numbering>", "\u{2022}", "\u{F0A8}", "\u{F0FE}")
}

fn run_rpr(span: &DocSpan, base_font: FontChoice) -> String {
    let fname = match span.fmt.font.unwrap_or(base_font) {
        FontChoice::Ubuntu => "Ubuntu", FontChoice::Roboto => "Roboto",
        FontChoice::GoogleSans => "Google Sans", FontChoice::OpenSans => "Open Sans",
    };
    let mut r = format!("<w:rPr><w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\" w:cs=\"{}\"/>", fname, fname, fname);
    if let Some(sz) = span.fmt.size_hp { r.push_str(&format!("<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>", sz, sz)); }
    if span.fmt.bold { r.push_str("<w:b/>"); }
    if span.fmt.italic { r.push_str("<w:i/>"); }
    if span.fmt.underline || span.fmt.link.is_some() { r.push_str("<w:u w:val=\"single\"/>"); }
    if span.fmt.strike { r.push_str("<w:strike/>"); }
    if span.fmt.sub { r.push_str("<w:vertAlign w:val=\"subscript\"/>"); }
    if span.fmt.sup { r.push_str("<w:vertAlign w:val=\"superscript\"/>"); }
    if let Some(c) = span.fmt.color { r.push_str(&format!("<w:color w:val=\"{:02X}{:02X}{:02X}\"/>", c[0], c[1], c[2])); }
    else if span.fmt.link.is_some() { r.push_str("<w:color w:val=\"1155CC\"/>"); }
    if let Some(h) = span.fmt.highlight { r.push_str(&format!("<w:shd w:val=\"clear\" w:fill=\"{:02X}{:02X}{:02X}\" w:color=\"auto\"/>", h[0], h[1], h[2])); }
    r.push_str("</w:rPr>");
    r
}

fn write_run(out: &mut String, txt: &str, rpr: &str) {
    out.push_str("<w:r>");
    out.push_str(rpr);
    for (i, part) in txt.split('\t').enumerate() {
        if i > 0 { out.push_str("<w:tab/>"); }
        if !part.is_empty() {
            let sp = if part.starts_with(' ') || part.ends_with(' ') { " xml:space=\"preserve\"" } else { "" };
            out.push_str(&format!("<w:t{}>{}</w:t>", sp, xml_esc(part)));
        }
    }
    out.push_str("</w:r>\n");
}

fn build_docx_styles_xml() -> String {
    let mut out = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);
    out.push_str(r#"<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="120"/></w:pPr></w:pPrDefault></w:docDefaults>"#);
    out.push_str(r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:rPr><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr><w:pPr><w:spacing w:after="120"/></w:pPr></w:style>"#);
    for style in [ParaStyle::Title, ParaStyle::Subtitle, ParaStyle::H1, ParaStyle::H2, ParaStyle::H3, ParaStyle::H4, ParaStyle::H5, ParaStyle::H6, ParaStyle::BlockQuote, ParaStyle::Code, ParaStyle::ListBullet, ParaStyle::ListOrdered, ParaStyle::ListCheck] {
        out.push_str(&docx_pstyle_block(style));
    }
    out.push_str("</w:styles>");
    out
}

fn xml_esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn build_table_xml(tbl: &TableData, content_w_twips: u32) -> String {
    let ncols = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    let col_tw = content_w_twips / ncols as u32;
    let bc = format!("{:02X}{:02X}{:02X}", tbl.border_color[0], tbl.border_color[1], tbl.border_color[2]);
    let bsz = ((tbl.border_width * 8.0).round() as u32).max(2);
    let b = |s: &str| format!("<w:{0} w:val=\"single\" w:sz=\"{1}\" w:color=\"{2}\"/>", s, bsz, bc);
    let mut out = format!("<w:tbl><w:tblPr><w:tblW w:w=\"{}\" w:type=\"dxa\"/><w:tblBorders>{}{}{}{}{}{}</w:tblBorders></w:tblPr><w:tblGrid>",
        content_w_twips, b("top"), b("left"), b("bottom"), b("right"), b("insideH"), b("insideV"));
    for _ in 0..ncols { out.push_str(&format!("<w:gridCol w:w=\"{}\"/>", col_tw)); }
    out.push_str("</w:tblGrid>");
    for row in &tbl.rows {
        out.push_str("<w:tr>");
        for cell in row {
            let shd = cell.bg_color.map(|c| format!("<w:shd w:val=\"clear\" w:fill=\"{:02X}{:02X}{:02X}\" w:color=\"auto\"/>", c[0], c[1], c[2])).unwrap_or_default();
            out.push_str(&format!("<w:tc><w:tcPr><w:tcW w:w=\"{}\" w:type=\"dxa\"/>{}</w:tcPr><w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p></w:tc>",
                col_tw, shd, xml_esc(&cell.text)));
        }
        out.push_str("</w:tr>");
    }
    out.push_str("</w:tbl>");
    out
}

fn build_document_xml(paras: &[DocParagraph], layout: &PageLayout) -> (String, Vec<String>, Vec<(String, Vec<u8>)>) {
    let content_w_twips = ((layout.width - layout.margin_left - layout.margin_right) * 20.0).round() as u32;
    let mut hyperlinks: Vec<String> = Vec::new();
    let mut images: Vec<(String, Vec<u8>)> = Vec::new();
    let mut img_idx = 0usize;
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\n<w:body>\n");
    for para in paras {
        if para.style == ParaStyle::Table {
            if let Some(ref t) = para.table { out.push_str(&build_table_xml(t, content_w_twips)); }
            continue;
        }
        if para.style == ParaStyle::HRule {
            out.push_str("<w:p><w:pPr><w:pBdr><w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"1\" w:color=\"auto\"/></w:pBdr></w:pPr></w:p>\n");
            continue;
        }
        if para.style == ParaStyle::Image {
            if let Some(ref img) = para.image {
                if !img.data.is_empty() {
                    let ext = img.name.rsplit('.').next().unwrap_or("png");
                    let media_name = format!("image{}.{}", img_idx + 1, ext);
                    let rid = format!("rId{}", img_idx + 1003);
                    let cx = (img.display_w * 12700.0) as u64;
                    let cy = (img.display_h * 12700.0) as u64;
                    out.push_str("<w:p><w:pPr>");
                    if para.align != Align::Left { out.push_str(&format!("<w:jc w:val=\"{}\"/>", para.align.docx_val())); }
                    out.push_str(&format!("</w:pPr><w:r><w:drawing><wp:inline><wp:extent cx=\"{}\" cy=\"{}\"/><wp:docPr id=\"{}\" name=\"{}\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"0\" name=\"{}\"/><pic:cNvPicPr preferRelativeResize=\"0\"/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"{}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"rect\"/></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>\n",
                        cx, cy, img_idx + 1, xml_esc(&media_name), xml_esc(&media_name), rid, cx, cy));
                    images.push((media_name, img.data.clone()));
                    img_idx += 1;
                }
            }
            continue;
        }
        out.push_str("<w:p>\n<w:pPr>\n");
        if para.style != ParaStyle::Normal { out.push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", para.style.docx_id())); }
        if para.align != Align::Left { out.push_str(&format!("<w:jc w:val=\"{}\"/>\n", para.align.docx_val())); }
        out.push_str(&format!("<w:spacing w:before=\"{}\" w:after=\"{}\" w:line=\"{}\" w:lineRule=\"auto\"/>\n",
            (para.space_before * 20.0) as u32, (para.space_after * 20.0) as u32, (para.line_height * 240.0) as u32));
        match para.style {
            ParaStyle::ListBullet => { out.push_str(&format!("<w:ind w:left=\"{}\" w:hanging=\"360\"/>\n", 720u32.max((para.indent_left * 20.0) as u32))); out.push_str("<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr>\n"); }
            ParaStyle::ListOrdered => { out.push_str(&format!("<w:ind w:left=\"{}\" w:hanging=\"360\"/>\n", 720u32.max((para.indent_left * 20.0) as u32))); out.push_str("<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"2\"/></w:numPr>\n"); }
            ParaStyle::ListCheck => { let num_id = if para.checked { 4u32 } else { 3u32 }; out.push_str(&format!("<w:ind w:left=\"{}\" w:hanging=\"360\"/>\n<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"{}\"/></w:numPr>\n", 720u32.max((para.indent_left * 20.0) as u32), num_id)); }
            _ if para.indent_left != 0.0 || para.indent_first != 0.0 => { out.push_str(&format!("<w:ind w:left=\"{}\" w:firstLine=\"{}\"/>\n", (para.indent_left * 20.0) as u32, (para.indent_first * 20.0) as u32)); }
            _ => {}
        }
        out.push_str("</w:pPr>\n");
        let mut pos = 0;
        let mut has_run = false;
        for span in &para.spans {
            if span.len == 0 { pos += span.len; continue; }
            if pos >= para.text.len() { break; }
            let end = (pos + span.len).min(para.text.len());
            let seg = &para.text[pos..end];
            pos = end;
            if seg.is_empty() { continue; }
            let rpr = run_rpr(span, DEFAULT_BASE_FONT);
            if let Some(ref url) = span.fmt.link {
                let idx = hyperlinks.iter().position(|u| u == url).unwrap_or_else(|| { hyperlinks.push(url.clone()); hyperlinks.len() - 1 });
                out.push_str(&format!("<w:hyperlink r:id=\"rId{}\">", idx + 3));
                write_run(&mut out, seg, &rpr);
                out.push_str("</w:hyperlink>\n");
            } else {
                write_run(&mut out, seg, &rpr);
                has_run = true;
            }
        }
        if !has_run && para.text.is_empty() && !para.spans.is_empty() {
            let span = &para.spans[0];
            let rpr = run_rpr(span, DEFAULT_BASE_FONT);
            out.push_str("<w:r>");
            out.push_str(&rpr);
            out.push_str("<w:t/>");
            out.push_str("</w:r>\n");
        }
        out.push_str("</w:p>\n");
    }
    let (w, h, mt, mb, ml, mr) = ((layout.width*20.0) as u32, (layout.height*20.0) as u32, (layout.margin_top*20.0) as u32, (layout.margin_bot*20.0) as u32, (layout.margin_left*20.0) as u32, (layout.margin_right*20.0) as u32);
    out.push_str(&format!("<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\"/></w:sectPr>\n</w:body>\n</w:document>", w, h, mt, mr, mb, ml));
    (out, hyperlinks, images)
}

pub fn save_docx(path: &PathBuf, paras: &[DocParagraph], layout: &PageLayout) -> Result<(), String> {
    let (doc, hyperlinks, images) = build_document_xml(paras, layout);
    let hl_pairs: Vec<(String, String)> = hyperlinks.iter().enumerate().map(|(i, url)| (format!("rId{}", i + 3), url.clone())).collect();
    let image_names: Vec<String> = images.iter().map(|(n, _)| n.clone()).collect();
    let rels = build_word_rels(&hl_pairs, &image_names);
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in [("[Content_Types].xml", CONTENT_TYPES.as_bytes()), ("_rels/.rels", ROOT_RELS.as_bytes())] {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    zip.start_file("word/_rels/document.xml.rels", opts).map_err(|e| e.to_string())?;
    zip.write_all(rels.as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file("word/document.xml", opts).map_err(|e| e.to_string())?;
    zip.write_all(doc.as_bytes()).map_err(|e| e.to_string())?;
    for (name, bytes) in &images {
        zip.start_file(format!("word/media/{}", name), opts).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())?;
    }
    let styles = build_docx_styles_xml();
    zip.start_file("word/styles.xml", opts).map_err(|e| e.to_string())?;
    zip.write_all(styles.as_bytes()).map_err(|e| e.to_string())?;
    let numbering = build_numbering_xml();
    zip.start_file("word/numbering.xml", opts).map_err(|e| e.to_string())?;
    zip.write_all(numbering.as_bytes()).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn get_attr(e: &quick_xml::events::BytesStart, key: &[u8]) -> Option<String> {
    e.attributes().filter_map(|a| a.ok()).find(|a| a.key.local_name().as_ref() == key)
        .and_then(|a| std::str::from_utf8(&a.value).ok().map(|s| s.to_string()))
}

pub fn load_docx(path: &PathBuf) -> Result<(Vec<DocParagraph>, PageLayout), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut arch = zip::ZipArchive::new(file).map_err(|_| "Not a valid DOCX".to_string())?;
    let num_map = parse_docx_numbering(&mut arch);
    let mut rels_map: std::collections::HashMap<String, String> = Default::default();
    let mut image_rels: std::collections::HashMap<String, String> = Default::default();
    if let Ok(mut e) = arch.by_name("word/_rels/document.xml.rels") {
        let mut s = String::new();
        if e.read_to_string(&mut s).is_ok() {
            let mut reader = quick_xml::Reader::from_str(&s);
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Empty(ref ev)) | Ok(quick_xml::events::Event::Start(ref ev)) => {
                        if ev.local_name().as_ref() == b"Relationship" {
                            if let (Some(id), Some(target), Some(typ)) = (get_attr(ev, b"Id"), get_attr(ev, b"Target"), get_attr(ev, b"Type")) {
                                if typ.ends_with("/hyperlink") { rels_map.insert(id, target); }
                                else if typ.ends_with("/image") { image_rels.insert(id, target); }
                            }
                        }
                    }
                    Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }
    let xml = { let mut e = arch.by_name("word/document.xml").map_err(|_| "Missing document.xml".to_string())?; let mut s = String::new(); e.read_to_string(&mut s).map_err(|e| e.to_string())?; s };
    let (mut paras, layout) = parse_docx_xml(&xml, &num_map, &rels_map)?;
    let mut uid = 0u64;
    for p in &mut paras {
        if p.style == ParaStyle::Image {
            if let Some(ref mut img) = p.image {
                if let Some(media_path) = image_rels.get(&img.name) {
                    let zip_path = format!("word/{}", media_path);
                    if let Ok(mut entry) = arch.by_name(&zip_path) {
                        let mut data = Vec::new();
                        let _ = entry.read_to_end(&mut data);
                        img.data = data;
                    }
                    img.name = media_path.rsplit('/').next().unwrap_or("image.png").to_string();
                }
                img.uid = uid;
                uid += 1;
            }
        }
    }
    Ok((paras, layout))
}

fn parse_docx_numbering(arch: &mut zip::ZipArchive<std::fs::File>) -> std::collections::HashMap<u32, (ParaStyle, Option<bool>)> {
    let xml = match arch.by_name("word/numbering.xml") {
        Ok(mut e) => { let mut s = String::new(); let _ = e.read_to_string(&mut s); s }
        Err(_) => return Default::default(),
    };
    use quick_xml::{Reader, events::Event};
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    
    let mut abstract_kind: std::collections::HashMap<u32, (ParaStyle, Option<bool>)> = Default::default();
    let mut num_to_abstract: std::collections::HashMap<u32, u32> = Default::default();
    let mut cur_abstract: Option<u32> = None;
    let mut in_num = false;
    let mut cur_num: Option<u32> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"abstractNum" => { 
                    cur_abstract = get_attr(e, b"abstractNumId").and_then(|v| v.parse().ok()); 
                }
                b"num" => { 
                    in_num = true; 
                    cur_num = get_attr(e, b"numId").and_then(|v| v.parse().ok()); 
                }
                _ => {}
            },
            Ok(Event::Empty(ref e)) => match e.local_name().as_ref() {
                b"numFmt" => if let Some(aid) = cur_abstract {
                    let entry = abstract_kind.entry(aid).or_insert((ParaStyle::ListBullet, None));
                    if entry.0 != ParaStyle::ListCheck {
                        entry.0 = match get_attr(e, b"val").as_deref() {
                            Some("decimal"|"lowerLetter"|"upperLetter"|"lowerRoman"|"upperRoman") => ParaStyle::ListOrdered,
                            _ => ParaStyle::ListBullet,
                        };
                    }
                },
                b"lvlText" | b"text" | b"sym" => if let Some(aid) = cur_abstract {
                    let text_attr = get_attr(e, b"val").or_else(|| get_attr(e, b"text")).or_else(|| get_attr(e, b"char"));
                    if let Some(v) = text_attr {
                        if let Some(state) = is_checkbox_marker(&v) {
                            let entry = abstract_kind.entry(aid).or_insert((ParaStyle::ListCheck, Some(state)));
                            entry.0 = ParaStyle::ListCheck;
                            if entry.1.is_none() { entry.1 = Some(state); }
                        }
                    }
                },
                b"abstractNumId" if in_num => if let (Some(nid), Some(aid)) = (cur_num, get_attr(e, b"val").and_then(|v| v.parse().ok())) {
                    num_to_abstract.insert(nid, aid);
                },
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"abstractNum" => { cur_abstract = None; }
                b"num" => { in_num = false; cur_num = None; }
                _ => {}
            },
            _ => {}
        }
    }
    
    num_to_abstract.into_iter().filter_map(|(nid, aid)| abstract_kind.get(&aid).copied().map(|v| (nid, v))).collect()
}

fn parse_docx_xml(xml: &str, num_map: &std::collections::HashMap<u32, (ParaStyle, Option<bool>)>, rels_map: &std::collections::HashMap<String, String>) -> Result<(Vec<DocParagraph>, PageLayout), String> {
    use quick_xml::{Reader, events::Event};
    let mut reader = Reader::from_str(xml); reader.config_mut().trim_text(false);
    let mut paras: Vec<DocParagraph> = Vec::new();
    let mut para_numids: Vec<Option<u32>> = Vec::new();
    let mut layout = PageLayout::default();
    let mut cur_para: Option<DocParagraph> = None;
    let mut cur_fmt = SpanFmt::default();
    let mut cur_run_text = String::new();
    let mut in_run = false; let mut in_rpr = false; let mut in_ppr = false; let mut in_t = false;
    let (mut in_drawing, mut para_has_drawing, mut drawing_rid, mut drawing_cx, mut drawing_cy) = (false, false, String::new(), 0u64, 0u64);
    let mut in_pbdr = false; let mut has_hborder = false; let mut in_pict = false;
    let mut in_numpr = false; let mut pending_num_id: Option<u32> = None;
    let mut cur_para_numid: Option<u32> = None;
    let mut cur_link_url: Option<String> = None;
    let (mut in_tbl, mut in_tc) = (false, false);
    let (mut in_tbl_pr, mut in_tbl_borders, mut in_tc_pr) = (false, false, false);
    let (mut cur_tbl_bc, mut cur_tbl_bw, mut cur_tc_bg) = ([100u8, 100, 110], 1.0f32, None::<[u8; 3]>);
    let mut cur_tbl_rows: Vec<Vec<TableCell>> = Vec::new();
    let mut cur_tbl_row: Vec<TableCell> = Vec::new();
    let mut cur_tc_text = String::new();

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(ref e) => {
                match e.local_name().as_ref() {
                    b"p" => if !in_tbl { cur_para = Some(DocParagraph::new()); in_ppr = false; has_hborder = false; cur_para_numid = None; }
                    b"pPr" => in_ppr = true,
                    b"pBdr" => if in_ppr { in_pbdr = true; },
                    b"numPr" => if in_ppr { in_numpr = true; },
                    b"pict" => in_pict = true,
                    b"bottom" | b"top" | b"left" | b"right" | b"insideH" | b"insideV" => {
                        if in_pbdr {
                            let val = get_attr(e, b"val");
                            if val.as_deref() != Some("nil") && val.as_deref() != Some("none") { has_hborder = true; }
                        }
                        if in_tbl_borders {
                            if let Some(c) = get_attr(e, b"color").filter(|v| v != "auto") {
                                let h = c.trim_start_matches('#');
                                if h.len() == 6 { if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&h[0..2],16),u8::from_str_radix(&h[2..4],16),u8::from_str_radix(&h[4..6],16)) { cur_tbl_bc = [r,g,bv]; } }
                            }
                            if let Some(sz) = get_attr(e, b"sz").and_then(|v| v.parse::<f32>().ok()) { cur_tbl_bw = (sz/8.0).max(0.5); }
                        }
                    }
                    b"pStyle" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"val") { p.style = ParaStyle::from_docx_id(&v); p.space_before = p.style.space_before(); p.space_after = p.style.space_after(); p.indent_left = p.style.default_indent(); } } } }
                    b"jc" => { if in_ppr { if let Some(ref mut p) = cur_para { p.align = match get_attr(e, b"val").as_deref() { Some("center") => Align::Center, Some("right") => Align::Right, Some("both") => Align::Justify, _ => Align::Left }; } } }
                    b"spacing" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"before") { p.space_before = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"after") { p.space_after = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"line") { p.line_height = v.parse::<f32>().unwrap_or(240.0)/240.0; } } } }
                    b"ind" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"left") { p.indent_left = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"firstLine") { p.indent_first = v.parse::<f32>().unwrap_or(0.0)/20.0; } } } }
                    b"hyperlink" => { if let Some(id) = get_attr(e, b"id") { cur_link_url = rels_map.get(&id).cloned(); } }
                    b"r" => { in_run = true; cur_fmt = SpanFmt::default(); cur_fmt.link = cur_link_url.clone(); cur_run_text.clear(); }
                    b"rPr" => in_rpr = true,
                    b"t" => { in_t = true; }
                    b"tab" => { if in_run { cur_run_text.push('\t'); } }
                    b"br" => { if in_run { cur_run_text.push('\n'); } }
                    b"b" => { if in_rpr { cur_fmt.bold = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"i" => { if in_rpr { cur_fmt.italic = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"u" => { if in_rpr { cur_fmt.underline = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"strike" => { if in_rpr { cur_fmt.strike = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"tbl" => { in_tbl = true; cur_tbl_rows.clear(); }
                    b"tblPr" if in_tbl => in_tbl_pr = true,
                    b"tblBorders" if in_tbl_pr => in_tbl_borders = true,
                    b"tcPr" if in_tbl => { in_tc_pr = true; cur_tc_bg = None; }
                    b"tr" if in_tbl => { cur_tbl_row.clear(); }
                    b"tc" if in_tbl => { in_tc = true; cur_tc_text.clear(); }
                    b"drawing" => { in_drawing = true; }
                    b"extent" if in_drawing => {
                        if let Some(v) = get_attr(e, b"cx") { drawing_cx = v.parse().unwrap_or(0); }
                        if let Some(v) = get_attr(e, b"cy") { drawing_cy = v.parse().unwrap_or(0); }
                    }
                    b"blip" if in_drawing => { if let Some(v) = get_attr(e, b"embed") { drawing_rid = v; } }
                    b"vertAlign" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val") {
                            if v.eq_ignore_ascii_case("subscript") { cur_fmt.sub = true; cur_fmt.sup = false; }
                            else if v.eq_ignore_ascii_case("superscript") { cur_fmt.sup = true; cur_fmt.sub = false; }
                        } }
                    }
                    b"position" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val").and_then(|v| v.parse::<i32>().ok()) {
                            if v < 0 { cur_fmt.sub = true; cur_fmt.sup = false; }
                            else if v > 0 { cur_fmt.sup = true; cur_fmt.sub = false; }
                        } }
                    }
                    b"sz" => { if in_rpr { if let Some(v) = get_attr(e, b"val").and_then(|v| v.parse().ok()) { cur_fmt.size_hp = Some(v); } } }
                    b"color" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val") {
                            if v != "auto" && v != "000000" && v.len() == 6 {
                                if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&v[0..2],16), u8::from_str_radix(&v[2..4],16), u8::from_str_radix(&v[4..6],16)) { cur_fmt.color = Some([r,g,b]); }
                            }
                        } }
                    }
                    b"highlight" => {
                        if in_rpr { if let Some(val) = get_attr(e, b"val") {
                            let rgb = match val.as_str() {
                                "yellow" => Some([255, 235, 59]), "green" => Some([167, 243, 208]), "cyan" => Some([125, 211, 252]),
                                "magenta" => Some([196, 181, 253]), "blue" => Some([147, 197, 253]), "red" => Some([255, 171, 145]),
                                "darkBlue" => Some([59, 130, 246]), "darkCyan" => Some([20, 184, 166]), "darkGreen" => Some([22, 163, 74]),
                                "darkMagenta" => Some([168, 85, 247]), "darkRed" => Some([220, 38, 38]), "darkYellow" => Some([234, 179, 8]),
                                "darkGray" => Some([102, 102, 102]), "lightGray" => Some([204, 204, 204]), "black" => Some([0, 0, 0]),
                                _ => None,
                            };
                            if rgb.is_some() { cur_fmt.highlight = rgb; }
                        } }
                    }
                    b"shd" => {
                        if let Some(v) = get_attr(e, b"fill") {
                            if in_rpr && v != "auto" && v.len() == 6 {
                                if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&v[0..2],16),u8::from_str_radix(&v[2..4],16),u8::from_str_radix(&v[4..6],16)) { cur_fmt.highlight = Some([r,g,bv]); }
                            } else if in_tc_pr && !in_rpr && v.len() == 6 && v != "auto" {
                                if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&v[0..2],16),u8::from_str_radix(&v[2..4],16),u8::from_str_radix(&v[4..6],16)) { cur_tc_bg = Some([r,g,bv]); }
                            }
                        }
                    }
                    b"rFonts" => {
                        if in_rpr { if let Some(font_name) = get_attr(e, b"ascii").or_else(|| get_attr(e, b"hAnsi")) {
                            cur_fmt.font = match font_name.as_str() {
                                "Ubuntu" => Some(FontChoice::Ubuntu), "Roboto" => Some(FontChoice::Roboto),
                                "Google Sans" => Some(FontChoice::GoogleSans), "Open Sans" => Some(FontChoice::OpenSans),
                                _ => None,
                            };
                        } }
                    }
                    _ => {}
                }
            }
            Event::Empty(ref e) => {
                match e.local_name().as_ref() {
                    b"rect" if in_pict => {
                        if get_attr(e, b"hr").as_deref() == Some("t") { has_hborder = true; }
                    }
                    b"pStyle" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"val") { p.style = ParaStyle::from_docx_id(&v); p.space_before = p.style.space_before(); p.space_after = p.style.space_after(); p.indent_left = p.style.default_indent(); } } } }
                    b"jc" => { if in_ppr { if let Some(ref mut p) = cur_para { p.align = match get_attr(e, b"val").as_deref() { Some("center") => Align::Center, Some("right") => Align::Right, Some("both") => Align::Justify, _ => Align::Left }; } } }
                    b"spacing" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"before") { p.space_before = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"after") { p.space_after = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"line") { p.line_height = v.parse::<f32>().unwrap_or(240.0)/240.0; } } } }
                    b"ind" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"left") { p.indent_left = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"firstLine") { p.indent_first = v.parse::<f32>().unwrap_or(0.0)/20.0; } } } }
                    b"bottom" | b"top" | b"left" | b"right" | b"insideH" | b"insideV" => {
                        if in_pbdr && in_ppr {
                            let val = get_attr(e, b"val");
                            if val.as_deref() != Some("nil") && val.as_deref() != Some("none") { has_hborder = true; }
                        }
                        if in_tbl_borders {
                            if let Some(c) = get_attr(e, b"color").filter(|v| v != "auto") {
                                let h = c.trim_start_matches('#');
                                if h.len() == 6 { if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&h[0..2],16),u8::from_str_radix(&h[2..4],16),u8::from_str_radix(&h[4..6],16)) { cur_tbl_bc = [r,g,bv]; } }
                            }
                            if let Some(sz) = get_attr(e, b"sz").and_then(|v| v.parse::<f32>().ok()) { cur_tbl_bw = (sz/8.0).max(0.5); }
                        }
                    }
                    b"numId" => {
                        if in_numpr {
                            let nid = get_attr(e, b"val").and_then(|v| v.parse().ok());
                            pending_num_id = nid;
                            cur_para_numid = nid;
                        }
                    }
                    b"tab" => { if in_run { cur_run_text.push('\t'); } }
                    b"br" => { if in_run { cur_run_text.push('\n'); } }
                    b"b" => { if in_rpr { cur_fmt.bold = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"i" => { if in_rpr { cur_fmt.italic = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"u" => { if in_rpr { cur_fmt.underline = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"strike" => { if in_rpr { cur_fmt.strike = get_attr(e, b"val").map_or(true, |v| v != "0" && v != "false" && v != "none"); } }
                    b"extent" if in_drawing => {
                        if let Some(v) = get_attr(e, b"cx") { drawing_cx = v.parse().unwrap_or(0); }
                        if let Some(v) = get_attr(e, b"cy") { drawing_cy = v.parse().unwrap_or(0); }
                    }
                    b"blip" if in_drawing => { if let Some(v) = get_attr(e, b"embed") { drawing_rid = v; } }
                    b"vertAlign" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val") {
                            if v.eq_ignore_ascii_case("subscript") { cur_fmt.sub = true; cur_fmt.sup = false; }
                            else if v.eq_ignore_ascii_case("superscript") { cur_fmt.sup = true; cur_fmt.sub = false; }
                        } }
                    }
                    b"position" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val").and_then(|v| v.parse::<i32>().ok()) {
                            if v < 0 { cur_fmt.sub = true; cur_fmt.sup = false; }
                            else if v > 0 { cur_fmt.sup = true; cur_fmt.sub = false; }
                        } }
                    }
                    b"sz" => { if in_rpr { if let Some(v) = get_attr(e, b"val").and_then(|v| v.parse().ok()) { cur_fmt.size_hp = Some(v); } } }
                    b"color" => {
                        if in_rpr { if let Some(v) = get_attr(e, b"val") {
                            if v != "auto" && v != "000000" && v.len() == 6 {
                                if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&v[0..2],16), u8::from_str_radix(&v[2..4],16), u8::from_str_radix(&v[4..6],16)) { cur_fmt.color = Some([r,g,b]); }
                            }
                        } }
                    }
                    b"highlight" => {
                        if in_rpr { if let Some(val) = get_attr(e, b"val") {
                            let rgb = match val.as_str() {
                                "yellow" => Some([255, 235, 59]), "green" => Some([167, 243, 208]), "cyan" => Some([125, 211, 252]),
                                "magenta" => Some([196, 181, 253]), "blue" => Some([147, 197, 253]), "red" => Some([255, 171, 145]),
                                "darkBlue" => Some([59, 130, 246]), "darkCyan" => Some([20, 184, 166]), "darkGreen" => Some([22, 163, 74]),
                                "darkMagenta" => Some([168, 85, 247]), "darkRed" => Some([220, 38, 38]), "darkYellow" => Some([234, 179, 8]),
                                "darkGray" => Some([102, 102, 102]), "lightGray" => Some([204, 204, 204]), "black" => Some([0, 0, 0]),
                                _ => None,
                            };
                            if rgb.is_some() { cur_fmt.highlight = rgb; }
                        } }
                    }
                    b"shd" => {
                        if let Some(v) = get_attr(e, b"fill") {
                            if in_rpr && v != "auto" && v.len() == 6 {
                                if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&v[0..2],16),u8::from_str_radix(&v[2..4],16),u8::from_str_radix(&v[4..6],16)) { cur_fmt.highlight = Some([r,g,bv]); }
                            } else if in_tc_pr && !in_rpr && v.len() == 6 && v != "auto" {
                                if let (Ok(r),Ok(g),Ok(bv)) = (u8::from_str_radix(&v[0..2],16),u8::from_str_radix(&v[2..4],16),u8::from_str_radix(&v[4..6],16)) { cur_tc_bg = Some([r,g,bv]); }
                            }
                        }
                    }
                    b"rFonts" => {
                        if in_rpr { if let Some(font_name) = get_attr(e, b"ascii").or_else(|| get_attr(e, b"hAnsi")) {
                            cur_fmt.font = match font_name.as_str() {
                                "Ubuntu" => Some(FontChoice::Ubuntu), "Roboto" => Some(FontChoice::Roboto),
                                "Google Sans" => Some(FontChoice::GoogleSans), "Open Sans" => Some(FontChoice::OpenSans),
                                _ => None,
                            };
                        } }
                    }
                    b"pgSz" => { if let Some(v) = get_attr(e, b"w") { layout.width = v.parse::<f32>().unwrap_or(12240.0)/20.0; } if let Some(v) = get_attr(e, b"h") { layout.height = v.parse::<f32>().unwrap_or(15840.0)/20.0; } }
                    b"pgMar" => { if let Some(v) = get_attr(e, b"top") { layout.margin_top = v.parse::<f32>().unwrap_or(1440.0)/20.0; } if let Some(v) = get_attr(e, b"bottom") { layout.margin_bot = v.parse::<f32>().unwrap_or(1440.0)/20.0; } if let Some(v) = get_attr(e, b"left") { layout.margin_left = v.parse::<f32>().unwrap_or(1800.0)/20.0; } if let Some(v) = get_attr(e, b"right") { layout.margin_right = v.parse::<f32>().unwrap_or(1800.0)/20.0; } }
                    _ => {}
                }
            }
            Event::End(ref e) => {
                match e.local_name().as_ref() {
                    b"pict" => in_pict = false,
                    b"tbl" if in_tbl => {
                        in_tbl = false; in_tc = false; in_tbl_pr = false; in_tbl_borders = false; in_tc_pr = false;
                        let mut p = DocParagraph::with_style(ParaStyle::Table);
                        p.table = Some(Box::new(TableData { rows: std::mem::take(&mut cur_tbl_rows), col_widths: Vec::new(), border_color: cur_tbl_bc, border_width: cur_tbl_bw.max(0.5) }));
                        paras.push(p);
                        para_numids.push(None);
                        cur_tbl_bc = [100, 100, 110]; cur_tbl_bw = 1.0;
                    },
                    b"tblPr" => in_tbl_pr = false,
                    b"tblBorders" => in_tbl_borders = false,
                    b"tcPr" => in_tc_pr = false,
                    b"tr" if in_tbl => { cur_tbl_rows.push(std::mem::take(&mut cur_tbl_row)); }
                    b"tc" if in_tbl => {
                        let text = std::mem::take(&mut cur_tc_text);
                        let spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] }
                            else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                        cur_tbl_row.push(TableCell { text, spans, bg_color: cur_tc_bg.take() });
                        in_tc = false; in_tc_pr = false;
                    },
                    b"drawing" => { if in_drawing { para_has_drawing = true; } in_drawing = false; }
                    b"p" => {
                        if !in_tbl {
                            if para_has_drawing && !drawing_rid.is_empty() {
                                let mut imgp = DocParagraph::with_style(ParaStyle::Image);
                                imgp.align = cur_para.as_ref().map(|p| p.align).unwrap_or(Align::Left);
                                imgp.image = Some(DocImage { data: vec![], display_w: drawing_cx as f32 / 12700.0, display_h: drawing_cy as f32 / 12700.0, name: drawing_rid.clone(), uid: 0 });
                                paras.push(imgp);
                                para_numids.push(None);
                                cur_para = None;
                            } else if let Some(mut p) = cur_para.take() {
                                if has_hborder {
                                    if p.text.trim().is_empty() {
                                        p.style = ParaStyle::HRule;
                                        paras.push(p);
                                        para_numids.push(None);
                                    } else {
                                        paras.push(p);
                                        para_numids.push(cur_para_numid);
                                        paras.push(DocParagraph::with_style(ParaStyle::HRule));
                                        para_numids.push(None);
                                    }
                                } else {
                                    paras.push(p);
                                    para_numids.push(cur_para_numid);
                                }
                            }
                            has_hborder = false;
                            para_has_drawing = false;
                            drawing_rid.clear();
                            drawing_cx = 0;
                            drawing_cy = 0;
                        }
                    }
                    b"pPr" => {
                        in_ppr = false;
                        if let (Some(nid), Some(ref mut p)) = (pending_num_id.take(), cur_para.as_mut()) {
                            if let Some((style, checked)) = num_map.get(&nid).copied() {
                                p.style = style;
                                if let Some(c) = checked { p.checked = c; }
                            } else {
                                p.style = ParaStyle::ListBullet;
                            }
                            p.space_before = p.style.space_before(); p.space_after = p.style.space_after(); p.indent_left = p.style.default_indent();
                        }
                    }
                    b"hyperlink" => { cur_link_url = None; }
                    b"numPr" => in_numpr = false,
                    b"pBdr" => in_pbdr = false,
                    b"r" => {
                        in_run = false;
                        if in_tbl && in_tc { cur_tc_text.push_str(&cur_run_text); }
                        else if let Some(ref mut para) = cur_para {
                            let blen = cur_run_text.len();
                            para.text.push_str(&cur_run_text);
                            if para.spans.last().map(|s| s.fmt == cur_fmt && s.len > 0).unwrap_or(false) { para.spans.last_mut().unwrap().len += blen; }
                            else { if para.spans.last().map(|s| s.len == 0).unwrap_or(false) { para.spans.pop(); } para.spans.push(DocSpan { len: blen, fmt: cur_fmt.clone() }); }
                        }
                    }
                    b"rPr" => in_rpr = false,
                    b"t" => in_t = false,
                    _ => {}
                }
            }
            Event::Text(ref e) => { if in_t && in_run { if let Ok(s) = std::str::from_utf8(e.as_ref()) { cur_run_text.push_str(s); } } }
            Event::Eof => break,
            _ => {}
        }
    }

    {
        let mut numid_groups: std::collections::HashMap<u32, Vec<usize>> = Default::default();
        for (i, (&nid_opt, p)) in para_numids.iter().zip(paras.iter()).enumerate() {
            if let Some(nid) = nid_opt {
                if p.style == ParaStyle::ListBullet {
                    numid_groups.entry(nid).or_default().push(i);
                }
            }
        }
        for indices in numid_groups.values() {
            let any_strike = indices.iter().any(|&i| {
                paras[i].spans.iter().any(|s| s.len > 0)
                    && paras[i].spans.iter().filter(|s| s.len > 0).all(|s| s.fmt.strike)
            });
            if any_strike {
                for &i in indices {
                    let all_strike = paras[i].spans.iter().filter(|s| s.len > 0).all(|s| s.fmt.strike);
                    paras[i].style = ParaStyle::ListCheck;
                    paras[i].checked = all_strike;
                    if all_strike {
                        for s in &mut paras[i].spans { s.fmt.strike = false; }
                    }
                }
            }
        }
    }

    if paras.is_empty() { paras.push(DocParagraph::new()); }
    convert_leading_tabs_to_indent(&mut paras);
    Ok((paras, layout))
}

pub fn load_txt_as_doc(path: &PathBuf) -> Result<Vec<DocParagraph>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut paras: Vec<DocParagraph> = content.lines().map(|line| {
        let mut p = DocParagraph::new(); p.text = line.to_string();
        p.spans = vec![DocSpan { len: line.len(), fmt: SpanFmt::default() }]; p
    }).collect();
    if paras.is_empty() { paras.push(DocParagraph::new()); }
    convert_leading_tabs_to_indent(&mut paras);
    Ok(paras)
}

#[derive(Clone, Default)]
struct OdtStyle { bold:bool, italic:bool, underline:bool, strike:bool, size_hp:Option<u32>, color:Option<[u8;3]>, highlight:Option<[u8;3]>, align:Align, h_border:bool, parent:String }

fn odt_attr(e: &quick_xml::events::BytesStart, k: &[u8]) -> Option<String> {
    e.attributes().filter_map(|a| a.ok()).find(|a| a.key.local_name().as_ref()==k).and_then(|a| std::str::from_utf8(&a.value).ok().map(String::from))
}

fn odt_parse_units(v: &str) -> f32 {
    let v = v.trim();
    if let Some(n) = v.strip_suffix("cm") { n.parse::<f32>().unwrap_or(0.0) * 28.3465 }
    else if let Some(n) = v.strip_suffix("mm") { n.parse::<f32>().unwrap_or(0.0) * 2.83465 }
    else if let Some(n) = v.strip_suffix("in") { n.parse::<f32>().unwrap_or(0.0) * 72.0 }
    else if let Some(n) = v.strip_suffix("pt") { n.parse::<f32>().unwrap_or(0.0) }
    else { v.parse::<f32>().unwrap_or(0.0) }
}

fn odt_apply_text_props(e: &quick_xml::events::BytesStart, s: &mut OdtStyle) {
    if let Some(v) = odt_attr(e, b"font-weight") { s.bold = v == "bold"; }
    if let Some(v) = odt_attr(e, b"font-style") { s.italic = v == "italic"; }
    if let Some(v) = odt_attr(e, b"text-underline-style") { s.underline = v != "none"; }
    if let Some(v) = odt_attr(e, b"text-line-through-style") { s.strike = v != "none"; }
    if let Some(v) = odt_attr(e, b"font-size") { let p = odt_parse_units(&v); if p > 0.0 { s.size_hp = Some((p*2.0).round() as u32); } }
    if let Some(c) = odt_attr(e, b"color") { let h = c.trim_start_matches('#'); if h.len()==6 { if let (Ok(r),Ok(g),Ok(b)) = (u8::from_str_radix(&h[0..2],16),u8::from_str_radix(&h[2..4],16),u8::from_str_radix(&h[4..6],16)) { s.color=Some([r,g,b]); } } }
    if let Some(c) = odt_attr(e, b"background-color") { let h = c.trim_start_matches('#'); if h.len()==6 { if let (Ok(r),Ok(g),Ok(b)) = (u8::from_str_radix(&h[0..2],16),u8::from_str_radix(&h[2..4],16),u8::from_str_radix(&h[4..6],16)) { s.highlight=Some([r,g,b]); } } }
}

fn odt_resolve_span(name: &str, map: &std::collections::HashMap<String, OdtStyle>) -> SpanFmt {
    let mut fmt = SpanFmt::default();
    let mut cur = name.to_string();
    for _ in 0..6 {
        match map.get(&cur) {
            Some(s) => {
                if s.bold { fmt.bold = true; } if s.italic { fmt.italic = true; }
                if s.underline { fmt.underline = true; } if s.strike { fmt.strike = true; }
                if fmt.size_hp.is_none() { fmt.size_hp = s.size_hp; }
                if fmt.color.is_none() { fmt.color = s.color; }
                if fmt.highlight.is_none() { fmt.highlight = s.highlight; }
                if s.parent.is_empty() { break; } else { cur = s.parent.clone(); }
            }
            None => break,
        }
    }
    fmt
}

fn odt_resolve_para(name: &str, map: &std::collections::HashMap<String, OdtStyle>, outline: u8) -> (ParaStyle, Align, bool) {
    if outline > 0 { return (match outline { 1=>ParaStyle::H1,2=>ParaStyle::H2,3=>ParaStyle::H3,4=>ParaStyle::H4,5=>ParaStyle::H5,_=>ParaStyle::H6 }, Align::Left, false); }
    let mut cur = name.to_string();
    let mut align = Align::Left;
    let mut h_border = false;
    for _ in 0..8 {
        match cur.as_str() {
            "Heading_20_1"|"Heading 1"|"Heading1" => return (ParaStyle::H1, align, h_border),
            "Heading_20_2"|"Heading 2"|"Heading2" => return (ParaStyle::H2, align, h_border),
            "Heading_20_3"|"Heading 3"|"Heading3" => return (ParaStyle::H3, align, h_border),
            "Heading_20_4"|"Heading 4"|"Heading4" => return (ParaStyle::H4, align, h_border),
            "Heading_20_5"|"Heading 5"|"Heading5" => return (ParaStyle::H5, align, h_border),
            "Heading_20_6"|"Heading 6"|"Heading6" => return (ParaStyle::H6, align, h_border),
            "Title" => return (ParaStyle::Title, align, h_border),
            "Subtitle" => return (ParaStyle::Subtitle, align, h_border),
            "Quotations"|"Quotation"|"BlockText"|"Quotation_20_Cont" => return (ParaStyle::BlockQuote, align, h_border),
            "Preformatted_20_Text"|"Code"|"Source_20_Code" => return (ParaStyle::Code, align, h_border),
            "List_20_Bullet"|"List Bullet"|"List_20_Bullet_20_2" => return (ParaStyle::ListBullet, align, h_border),
            "List_20_Number"|"List Number"|"List_20_Number_20_2" => return (ParaStyle::ListOrdered, align, h_border),
            "List_20_Check"|"List Check"|"Checklist"|"CheckList" => return (ParaStyle::ListCheck, align, h_border),
            "Horizontal_20_Line"|"Horizontal Line" => return (ParaStyle::HRule, align, true),
            _ => {}
        }
        match map.get(&cur) {
            Some(s) => { 
                if s.align != Align::Left { align = s.align; } 
                if s.h_border { h_border = true; }
                if s.parent.is_empty() { break; } else { cur = s.parent.clone(); } 
            }
            None => break,
        }
    }
    (ParaStyle::Normal, align, h_border)
}

fn para_to_odt_style(s: ParaStyle) -> &'static str {
    match s {
        ParaStyle::Normal=>"Standard", ParaStyle::H1=>"Heading_20_1", ParaStyle::H2=>"Heading_20_2",
        ParaStyle::H3=>"Heading_20_3", ParaStyle::H4=>"Heading_20_4", ParaStyle::H5=>"Heading_20_5",
        ParaStyle::H6=>"Heading_20_6", ParaStyle::Title=>"Title", ParaStyle::Subtitle=>"Subtitle",
        ParaStyle::BlockQuote=>"Quotations", ParaStyle::Code=>"Preformatted_20_Text",
        ParaStyle::ListBullet=>"List_20_Bullet", ParaStyle::ListOrdered=>"List_20_Number", ParaStyle::ListCheck=>"List_20_Check",
        ParaStyle::HRule=>"Standard", ParaStyle::Table => "Standard", ParaStyle::Image => "Standard",
    }
}

fn fmt_to_odt_id(fmt: &SpanFmt) -> String {
    let mut s = String::from("T");
    if fmt.bold { s.push('B'); } if fmt.italic { s.push('I'); }
    if fmt.underline { s.push('U'); } if fmt.strike { s.push('K'); }
    if fmt.sup { s.push('P'); } if fmt.sub { s.push('D'); }
    if let Some(sz) = fmt.size_hp { s.push_str(&sz.to_string()); }
    if let Some(c) = fmt.color { s.push_str(&format!("{:02x}{:02x}{:02x}", c[0],c[1],c[2])); }
    s
}

fn build_odt_styles(layout: &PageLayout) -> String {
    let cm = |pt: f32| format!("{:.3}cm", pt / 28.3465);
    let pt = |pt: f32| format!("{:.1}pt", pt);
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-styles xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\"><office:styles>");
    for (name, parent, size, bold, italic, before, after, left, outline) in [
        ("Standard", "", Some(11.0), false, false, 0.0, 6.0, 0.0, None),
        ("Title", "Standard", Some(26.0), true, false, 0.0, 6.0, 0.0, Some(0)),
        ("Subtitle", "Standard", Some(15.0), false, true, 0.0, 6.0, 0.0, Some(0)),
        ("Heading_20_1", "Standard", Some(20.0), true, false, 16.0, 8.0, 0.0, Some(1)),
        ("Heading_20_2", "Standard", Some(16.0), true, false, 16.0, 8.0, 0.0, Some(2)),
        ("Heading_20_3", "Standard", Some(14.0), true, false, 12.0, 6.0, 0.0, Some(3)),
        ("Heading_20_4", "Standard", Some(13.0), true, false, 12.0, 6.0, 0.0, Some(4)),
        ("Heading_20_5", "Standard", Some(12.0), true, false, 8.0, 4.0, 0.0, Some(5)),
        ("Heading_20_6", "Standard", Some(11.0), true, false, 8.0, 4.0, 0.0, Some(6)),
        ("Quotations", "Standard", Some(12.0), false, true, 6.0, 6.0, 24.0, None),
        ("Preformatted_20_Text", "Standard", Some(11.0), false, false, 6.0, 6.0, 24.0, None),
        ("List_20_Bullet", "Standard", Some(11.0), false, false, 0.0, 0.0, 18.0, None),
        ("List_20_Number", "Standard", Some(11.0), false, false, 0.0, 0.0, 18.0, None),
        ("List_20_Check", "Standard", Some(11.0), false, false, 0.0, 0.0, 18.0, None),
    ] {
        out.push_str("<style:style style:name=\"");
        out.push_str(name);
        out.push_str("\" style:family=\"paragraph\"");
        if !parent.is_empty() {
            out.push_str(&format!(" style:parent-style-name=\"{}\"", parent));
        }
        if let Some(lvl) = outline {
            out.push_str(&format!(" style:default-outline-level=\"{}\"", lvl));
        }
        out.push_str(">");
        out.push_str("<style:paragraph-properties");
        if before != 0.0 { out.push_str(&format!(" fo:margin-top=\"{}\"", pt(before))); }
        if after != 0.0 { out.push_str(&format!(" fo:margin-bottom=\"{}\"", pt(after))); }
        if left != 0.0 { out.push_str(&format!(" fo:margin-left=\"{}\"", pt(left))); }
        out.push_str("/>");
        out.push_str("<style:text-properties");
        if let Some(sz) = size { out.push_str(&format!(" fo:font-size=\"{}\"", pt(sz))); }
        if bold { out.push_str(" fo:font-weight=\"bold\""); }
        if italic { out.push_str(" fo:font-style=\"italic\""); }
        if name == "Preformatted_20_Text" { out.push_str(" style:font-name=\"Liberation Mono\" fo:font-family=\"Liberation Mono\""); }
        out.push_str("/>");
        out.push_str("</style:style>");
    }
    out.push_str(&format!("</office:styles><office:automatic-styles><style:page-layout style:name=\"pm1\"><style:page-layout-properties fo:page-width=\"{}\" fo:page-height=\"{}\" fo:margin-top=\"{}\" fo:margin-bottom=\"{}\" fo:margin-left=\"{}\" fo:margin-right=\"{}\"/></style:page-layout></office:automatic-styles><office:master-styles><style:master-page style:name=\"Standard\" style:page-layout-name=\"pm1\"/></office:master-styles></office:document-styles>",
        cm(layout.width), cm(layout.height), cm(layout.margin_top), cm(layout.margin_bot), cm(layout.margin_left), cm(layout.margin_right)));
    out
}

fn build_odt_content(paras: &[DocParagraph]) -> String {
    let mut span_styles: std::collections::BTreeMap<String, SpanFmt> = Default::default();
    for p in paras {
        for s in &p.spans { if s.len > 0 && s.fmt != SpanFmt::default() { span_styles.entry(fmt_to_odt_id(&s.fmt)).or_insert_with(|| s.fmt.clone()); } }
    }
    let ns = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\"";    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content {}><office:automatic-styles>", ns);
    for (id, fmt) in &span_styles {
        out.push_str(&format!("<style:style style:name=\"{}\" style:family=\"text\"><style:text-properties", id));
        if fmt.bold { out.push_str(" fo:font-weight=\"bold\""); }
        if fmt.italic { out.push_str(" fo:font-style=\"italic\""); }
        if fmt.underline { out.push_str(" style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\""); }
        if fmt.strike { out.push_str(" style:text-line-through-style=\"solid\""); }
        if fmt.sup { out.push_str(" style:text-position=\"super 58%\""); }
        if fmt.sub { out.push_str(" style:text-position=\"sub 58%\""); }
        if let Some(sz) = fmt.size_hp { out.push_str(&format!(" fo:font-size=\"{}pt\"", (sz as f32 / 2.0).round() as u32)); }
        if let Some(c) = fmt.color { out.push_str(&format!(" fo:color=\"#{:02X}{:02X}{:02X}\"", c[0],c[1],c[2])); }
        out.push_str("/></style:style>");
    }
    out.push_str("</office:automatic-styles><office:body><office:text>");
    for para in paras {
        if para.style == ParaStyle::Image { continue; }
        if para.style == ParaStyle::Table {
            if let Some(ref tbl) = para.table {
                let ncols = tbl.rows.iter().map(|r| r.len()).max().unwrap_or(1);
                out.push_str("<table:table table:name=\"Table\">");
                for _ in 0..ncols { out.push_str("<table:table-column/>"); }
                for row in &tbl.rows {
                    out.push_str("<table:table-row>");
                    for cell in row {
                        out.push_str(&format!("<table:table-cell><text:p>{}</text:p></table:table-cell>", xml_esc(&cell.text)));
                    }
                    out.push_str("</table:table-row>");
                }
                out.push_str("</table:table>");
            }
            continue;
        }
        if para.style == ParaStyle::HRule { out.push_str("<text:p text:style-name=\"Standard\"><text:s/></text:p>"); continue; }
        let sname = para_to_odt_style(para.style);
        let is_h = matches!(para.style, ParaStyle::H1|ParaStyle::H2|ParaStyle::H3|ParaStyle::H4|ParaStyle::H5|ParaStyle::H6);
        let align_str = match para.align { Align::Center=>"center", Align::Right=>"end", Align::Justify=>"justify", _=>"" };
        if is_h {
            let lvl = match para.style { ParaStyle::H1=>1,ParaStyle::H2=>2,ParaStyle::H3=>3,ParaStyle::H4=>4,ParaStyle::H5=>5,_=>6 };
            out.push_str(&format!("<text:h text:style-name=\"{}\" text:outline-level=\"{}\">", sname, lvl));
        } else if !align_str.is_empty() {
            out.push_str(&format!("<text:p text:style-name=\"{}\" fo:text-align=\"{}\">", sname, align_str));
        } else {
            out.push_str(&format!("<text:p text:style-name=\"{}\">", sname));
        }
        if para.style == ParaStyle::ListCheck { out.push_str(if para.checked { "☑ " } else { "☐ " }); }
        let mut pos = 0;
        if para.text.is_empty() {
            if let Some(span) = para.spans.first() {
                if span.fmt != SpanFmt::default() {
                    let style_id = fmt_to_odt_id(&span.fmt);
                    out.push_str(&format!("<text:span text:style-name=\"{}\"/>", style_id));
                }
            }
        } else {
            for span in &para.spans {
                if span.len == 0 { pos += span.len; continue; }
                if pos >= para.text.len() { break; }
                let end = (pos + span.len).min(para.text.len());
                let txt = &para.text[pos..end]; pos = end;
                if txt.is_empty() { continue; }
                let esc = xml_esc(txt);
                if span.fmt == SpanFmt::default() { out.push_str(&esc); }
                else { out.push_str(&format!("<text:span text:style-name=\"{}\">{}</text:span>", fmt_to_odt_id(&span.fmt), esc)); }
            }
        }
        if is_h { out.push_str("</text:h>"); } else { out.push_str("</text:p>"); }
    }
    out.push_str("</office:text></office:body></office:document-content>");
    out
}

fn parse_odt_xml(xml: &str) -> Result<(Vec<DocParagraph>, PageLayout), String> {
    use quick_xml::{Reader, events::Event};
    fn push_text(ps: &mut Option<(DocParagraph, Vec<(SpanFmt, String)>)>, ss: &mut Vec<(SpanFmt, String)>, text: &str) {
        if text.is_empty() { return; }
        if let Some(sp) = ss.last_mut() { sp.1.push_str(text); return; }
        if let Some((_, chunks)) = ps.as_mut() {
            match chunks.last_mut() {
                Some((f, t)) if *f == SpanFmt::default() => t.push_str(text),
                _ => chunks.push((SpanFmt::default(), text.to_string())),
            }
        }
    }
    let mut smap: std::collections::HashMap<String, OdtStyle> = Default::default();
    let mut paras: Vec<DocParagraph> = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let (mut in_auto, mut in_body) = (false, false);
    let (mut in_tbl, mut in_tc_odt) = (false, false);
    let mut cur_tbl_rows: Vec<Vec<TableCell>> = Vec::new();
    let mut cur_tbl_row: Vec<TableCell> = Vec::new();
    let mut cur_tc_text = String::new();
    let mut cur_sty: Option<(String, OdtStyle)> = None;
    let mut para_state: Option<(DocParagraph, Vec<(SpanFmt, String)>)> = None;
    let mut span_stack: Vec<(SpanFmt, String)> = Vec::new();
    let mut list_style_map: std::collections::HashMap<String, (ParaStyle, Option<bool>)> = Default::default();
    let mut cur_list_style: Option<String> = None;
    let mut list_stack: Vec<(ParaStyle, Option<bool>)> = Vec::new();
    let mut in_list_item = false;
    let mut cur_para_h_border = false;
    
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(ref e) => {
                let local_name = e.local_name();
                let tag = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match tag {
                    "automatic-styles" => in_auto = true,
                    "body" => in_body = true,
                    "style" if in_auto => { cur_sty = Some((odt_attr(e, b"name").unwrap_or_default(), OdtStyle { parent: odt_attr(e, b"parent-style-name").unwrap_or_default(), ..Default::default() })); }
                    "list-style" if in_auto => { cur_list_style = odt_attr(e, b"name"); }
                    "text-properties" if cur_sty.is_some() => { if let Some((_, ref mut s)) = cur_sty { odt_apply_text_props(e, s); } }
                    "table" if in_body => { in_tbl = true; cur_tbl_rows.clear(); }
                    "table-row" if in_tbl => { cur_tbl_row.clear(); }
                    "table-cell" if in_tbl => { in_tc_odt = true; cur_tc_text.clear(); }
                    "paragraph-properties" if cur_sty.is_some() => { 
                        if let Some((_, ref mut s)) = cur_sty { 
                            if let Some(v) = odt_attr(e, b"text-align") { s.align = match v.as_str() { "center"=>Align::Center,"right"|"end"=>Align::Right,"justify"=>Align::Justify,_=>Align::Left }; }
                            if odt_attr(e, b"border-bottom").is_some() || odt_attr(e, b"border-top").is_some() { s.h_border = true; }
                        } 
                    }
                    "list" if in_body => {
                        let sname = odt_attr(e, b"style-name").unwrap_or_default();
                        list_stack.push(list_style_map.get(&sname).copied().unwrap_or((ParaStyle::ListBullet, None)));
                    }
                    "list-item" if in_body => in_list_item = true,
                    "p" | "h" if in_body => if !in_tbl {
                        let sname = odt_attr(e, b"style-name").unwrap_or_default();
                        let outline: u8 = if tag=="h" { odt_attr(e, b"outline-level").and_then(|v| v.parse().ok()).unwrap_or(1) } else { 0 };
                        let (mut ps, align, h_border) = odt_resolve_para(&sname, &smap, outline);
                        if in_list_item && !matches!(ps, ParaStyle::ListBullet | ParaStyle::ListOrdered | ParaStyle::ListCheck) {
                            ps = list_stack.last().copied().map(|v| v.0).unwrap_or(ParaStyle::ListBullet);
                        }
                        let mut p = DocParagraph::with_style(ps); p.align = align;
                        cur_para_h_border = h_border;
                        para_state = Some((p, Vec::new())); span_stack.clear();
                    }
                    "a" if para_state.is_some() => {
                        let mut fmt = SpanFmt::default();
                        if let Some((parent_fmt, _)) = span_stack.last() { fmt = parent_fmt.clone(); }
                        fmt.link = odt_attr(e, b"href");
                        span_stack.push((fmt, String::new()));
                    }
                    "span" if para_state.is_some() => {
                        let mut fmt = odt_resolve_span(&odt_attr(e, b"style-name").unwrap_or_default(), &smap);
                        if let Some((parent_fmt, _)) = span_stack.last() {
                            if fmt.link.is_none() { fmt.link = parent_fmt.link.clone(); }
                        }
                        span_stack.push((fmt, String::new()));
                    }
                    _ => {}
                }
            }
            Event::Empty(ref e) => {
                let local_name = e.local_name();
                let tag = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match tag {
                    "text-properties" if cur_sty.is_some() => { if let Some((_, ref mut s)) = cur_sty { odt_apply_text_props(e, s); } }
                    "paragraph-properties" if cur_sty.is_some() => { 
                        if let Some((_, ref mut s)) = cur_sty { 
                            if let Some(v) = odt_attr(e, b"text-align") { s.align = match v.as_str() { "center"=>Align::Center,"right"|"end"=>Align::Right,"justify"=>Align::Justify,_=>Align::Left }; }
                            if odt_attr(e, b"border-bottom").is_some() || odt_attr(e, b"border-top").is_some() { s.h_border = true; }
                        } 
                    }
                    "list-level-style-bullet" => if let Some(ref n) = cur_list_style {
                        let bullet = odt_attr(e, b"bullet-char").unwrap_or_default();
                        let kind = if let Some(state) = is_checkbox_marker(&bullet) {
                            (ParaStyle::ListCheck, Some(state))
                        } else {
                            (ParaStyle::ListBullet, None)
                        };
                        list_style_map.insert(n.clone(), kind);
                    }
                    "list-level-style-number" => if let Some(ref n) = cur_list_style { list_style_map.insert(n.clone(), (ParaStyle::ListOrdered, None)); }
                    "line-break" => push_text(&mut para_state, &mut span_stack, "\n"),
                    "s" => {
                        let n = odt_attr(e, b"c").and_then(|v| v.parse().ok()).unwrap_or(1usize);
                        if in_tbl && in_tc_odt { cur_tc_text.push_str(&" ".repeat(n)); }
                        else { push_text(&mut para_state, &mut span_stack, &" ".repeat(n)); }
                    }
                    "tab" => {
                        if in_tbl && in_tc_odt { cur_tc_text.push('\t'); }
                        else { push_text(&mut para_state, &mut span_stack, "\t"); }
                    }
                    _ => {}
                }
            }
            Event::End(ref e) => {
                let local_name = e.local_name();
                let tag = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match tag {
                    "automatic-styles" => in_auto = false,
                    "style" if in_auto => { if let Some((n, s)) = cur_sty.take() { smap.insert(n, s); } }
                    "list-style" => cur_list_style = None,
                    "list" => { list_stack.pop(); }
                    "list-item" => in_list_item = false,
                    "table" if in_tbl => {
                        in_tbl = false; in_tc_odt = false;
                        let mut p = DocParagraph::with_style(ParaStyle::Table);
                        p.table = Some(Box::new(TableData { rows: std::mem::take(&mut cur_tbl_rows), col_widths: Vec::new(), border_color: [100, 100, 110], border_width: 1.0 }));
                        paras.push(p);
                    }
                    "table-row" if in_tbl => { cur_tbl_rows.push(std::mem::take(&mut cur_tbl_row)); }
                    "table-cell" if in_tbl => {
                        let text = std::mem::take(&mut cur_tc_text);
                        let spans = if text.is_empty() { vec![DocSpan { len: 0, fmt: SpanFmt::default() }] }
                            else { vec![DocSpan { len: text.len(), fmt: SpanFmt::default() }] };
                        cur_tbl_row.push(TableCell { text, spans, bg_color: None });
                        in_tc_odt = false;
                    }
                    "p" | "h" if in_body && !in_tbl => {
                        while let Some((fmt, text)) = span_stack.pop() { if !text.is_empty() { if let Some((_, chunks)) = para_state.as_mut() { chunks.push((fmt, text)); } } }
                        if let Some((mut p, chunks)) = para_state.take() {
                            for (fmt, text) in &chunks {
                                let len = text.len(); p.text.push_str(text);
                                if p.spans.last().map(|s: &DocSpan| &s.fmt == fmt).unwrap_or(false) { p.spans.last_mut().unwrap().len += len; }
                                else { if p.spans.last().map(|s| s.len==0).unwrap_or(false) { p.spans.pop(); } p.spans.push(DocSpan { len, fmt: fmt.clone() }); }
                            }
                            if p.spans.is_empty() { p.spans.push(DocSpan { len: 0, fmt: SpanFmt::default() }); }
                            if cur_para_h_border && p.text.trim().is_empty() { p.style = ParaStyle::HRule; }
                            paras.push(p);
                        }
                    }
                    "span" | "a" => { if let Some((fmt, text)) = span_stack.pop() { if !text.is_empty() { if let Some((_, chunks)) = para_state.as_mut() { chunks.push((fmt, text)); } } } }
                    _ => {}
                }
            }
            Event::Text(ref e) => {
                if let Ok(s) = std::str::from_utf8(e.as_ref()) {
                    if in_tbl && in_tc_odt { cur_tc_text.push_str(s); }
                    else if para_state.is_some() { push_text(&mut para_state, &mut span_stack, s); }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if paras.is_empty() { paras.push(DocParagraph::new()); }
    convert_leading_tabs_to_indent(&mut paras);
    Ok((paras, PageLayout::default()))
}

pub fn load_odt(path: &PathBuf) -> Result<(Vec<DocParagraph>, PageLayout), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut arch = zip::ZipArchive::new(file).map_err(|_| "Not a valid ODT".to_string())?;
    let content = { let mut e = arch.by_name("content.xml").map_err(|_| "Missing content.xml".to_string())?; let mut s = String::new(); e.read_to_string(&mut s).map_err(|e| e.to_string())?; s };
    parse_odt_xml(&content)
}

pub fn save_odt(path: &PathBuf, paras: &[DocParagraph], layout: &PageLayout) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default();
    zip.start_file("mimetype", stored).map_err(|e| e.to_string())?;
    zip.write_all(b"application/vnd.oasis.opendocument.text").map_err(|e| e.to_string())?;
    zip.start_file("META-INF/manifest.xml", deflated).map_err(|e| e.to_string())?;
    zip.write_all(ODT_MANIFEST.as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file("styles.xml", deflated).map_err(|e| e.to_string())?;
    zip.write_all(build_odt_styles(layout).as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file("content.xml", deflated).map_err(|e| e.to_string())?;
    zip.write_all(build_odt_content(paras).as_bytes()).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
