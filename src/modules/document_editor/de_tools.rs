use eframe::egui;
use std::{io::{Read, Write}, path::PathBuf};
use crate::style::ColorPalette;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontChoice { #[default] Ubuntu, Roboto }
impl FontChoice {
    pub fn label(self) -> &'static str { match self { Self::Ubuntu => "Ubuntu", Self::Roboto => "Roboto" } }
    pub fn egui_family(self, bold: bool, italic: bool) -> egui::FontFamily {
        egui::FontFamily::Name(match (self, bold, italic) {
            (Self::Ubuntu, true, true) => "Ubuntu-BoldItalic", (Self::Ubuntu, true, _) => "Ubuntu-Bold",
            (Self::Ubuntu, _, true) => "Ubuntu-Italic", (Self::Ubuntu, _, _) => "Ubuntu",
            (Self::Roboto, true, true) => "Roboto-BoldItalic", (Self::Roboto, true, _) => "Roboto-Bold",
            (Self::Roboto, _, true) => "Roboto-Italic", (Self::Roboto, _, _) => "Roboto",
        }.into())
    }
    pub fn all() -> &'static [FontChoice] { &[Self::Ubuntu, Self::Roboto] }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ParaStyle {
    #[default] Normal, H1, H2, H3, H4, H5, H6,
    Title, Subtitle, BlockQuote, Code, ListBullet, ListOrdered,
}
impl ParaStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal", Self::H1 => "Heading 1", Self::H2 => "Heading 2",
            Self::H3 => "Heading 3", Self::H4 => "Heading 4", Self::H5 => "Heading 5",
            Self::H6 => "Heading 6", Self::Title => "Title", Self::Subtitle => "Subtitle",
            Self::BlockQuote => "Block Quote", Self::Code => "Code Block",
            Self::ListBullet => "Bullet List", Self::ListOrdered => "Numbered List",
        }
    }
    pub fn all() -> &'static [ParaStyle] {
        &[Self::Normal, Self::H1, Self::H2, Self::H3, Self::H4, Self::H5, Self::H6,
          Self::Title, Self::Subtitle, Self::BlockQuote, Self::Code, Self::ListBullet, Self::ListOrdered]
    }
    pub fn is_heading(self) -> bool { matches!(self, Self::H1|Self::H2|Self::H3|Self::H4|Self::H5|Self::H6|Self::Title|Self::Subtitle) }
    pub fn size_scale(self) -> f32 {
        match self { Self::Title => 2.4, Self::H1 => 2.0, Self::H2 => 1.6, Self::H3 => 1.3,
            Self::H4 => 1.15, Self::H5 => 1.05, Self::Subtitle => 1.4, Self::Code => 0.9, _ => 1.0 }
    }
    pub fn is_bold(self) -> bool { matches!(self, Self::H1|Self::H2|Self::H3|Self::H4|Self::H5|Self::H6|Self::Title) }
    pub fn is_italic(self) -> bool { matches!(self, Self::Subtitle|Self::BlockQuote) }
    pub fn space_before(self) -> f32 { match self { Self::H1|Self::H2 => 16.0, Self::H3|Self::H4 => 12.0, Self::H5|Self::H6|Self::Title => 8.0, _ => 0.0 } }
    pub fn space_after(self) -> f32 { match self { Self::H1|Self::H2 => 8.0, Self::H3|Self::H4 => 6.0, _ => 6.0 } }
    pub fn default_indent(self) -> f32 { match self { Self::ListBullet|Self::ListOrdered => 18.0, Self::BlockQuote => 24.0, _ => 0.0 } }
    pub fn outline_depth(self) -> Option<u8> {
        match self { Self::Title|Self::Subtitle => Some(0), Self::H1 => Some(1), Self::H2 => Some(2), Self::H3 => Some(3), Self::H4 => Some(4), Self::H5 => Some(5), Self::H6 => Some(6), _ => None }
    }
    pub fn docx_id(self) -> &'static str {
        match self { Self::Normal => "Normal", Self::H1 => "Heading1", Self::H2 => "Heading2", Self::H3 => "Heading3",
            Self::H4 => "Heading4", Self::H5 => "Heading5", Self::H6 => "Heading6",
            Self::Title => "Title", Self::Subtitle => "Subtitle", Self::BlockQuote => "Quote",
            Self::Code => "CodeBlock", Self::ListBullet => "ListBullet", Self::ListOrdered => "ListNumber" }
    }
    pub fn from_docx_id(s: &str) -> Self {
        match s {
            "Heading1"|"Heading 1" => Self::H1, "Heading2"|"Heading 2" => Self::H2,
            "Heading3"|"Heading 3" => Self::H3, "Heading4"|"Heading 4" => Self::H4,
            "Heading5"|"Heading 5" => Self::H5, "Heading6"|"Heading 6" => Self::H6,
            "Title" => Self::Title, "Subtitle" => Self::Subtitle, "Quote"|"BlockText" => Self::BlockQuote,
            "CodeBlock"|"Code" => Self::Code, "ListBullet"|"ListBullet2" => Self::ListBullet,
            "ListNumber"|"ListNumber2" => Self::ListOrdered, _ => Self::Normal,
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
}

#[derive(Debug, Clone)]
pub struct DocSpan { pub len: usize, pub fmt: SpanFmt }

#[derive(Debug, Clone)]
pub struct DocParagraph {
    pub text: String, pub spans: Vec<DocSpan>, pub style: ParaStyle, pub align: Align,
    pub indent_left: f32, pub indent_first: f32, pub space_before: f32, pub space_after: f32,
    pub line_height: f32, pub list_num: Option<u32>,
}
impl DocParagraph {
    pub fn new() -> Self {
        Self { text: String::new(), spans: vec![DocSpan { len: 0, fmt: SpanFmt::default() }],
            style: ParaStyle::Normal, align: Align::Left, indent_left: 0.0, indent_first: 0.0,
            space_before: 0.0, space_after: 6.0, line_height: 1.15, list_num: None }
    }
    pub fn with_style(s: ParaStyle) -> Self {
        let mut p = Self::new();
        p.style = s; p.space_before = s.space_before(); p.space_after = s.space_after(); p.indent_left = s.default_indent(); p
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
    pub fn content_width(&self) -> f32 { self.width - self.margin_left - self.margin_right }
    pub fn content_height(&self) -> f32 { self.height - self.margin_top - self.margin_bot }
    fn inch(w: f32, h: f32, mt: f32, mb: f32, ml: f32, mr: f32) -> Self {
        let p = Self::PTS_PER_INCH;
        Self { width: w * p, height: h * p, margin_top: mt * p, margin_bot: mb * p, margin_left: ml * p, margin_right: mr * p }
    }
    pub fn letter() -> Self { Self::inch(8.5, 11.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn letter_word() -> Self { Self::inch(8.5, 11.0, 1.0, 1.0, 1.25, 1.25) }
    pub fn a4() -> Self { Self::inch(8.2677, 11.6929, 1.0, 1.0, 1.0, 1.0) }
    pub fn legal() -> Self { Self::inch(8.5, 14.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn a3() -> Self { Self::inch(11.6929, 16.5354, 1.0, 1.0, 1.0, 1.0) }
    pub fn a5() -> Self { Self::inch(5.8268, 8.2677, 1.0, 1.0, 1.0, 1.0) }
    pub fn executive() -> Self { Self::inch(7.25, 10.5, 1.0, 1.0, 1.0, 1.0) }
    pub fn tabloid() -> Self { Self::inch(11.0, 17.0, 1.0, 1.0, 1.0, 1.0) }
    pub fn b5() -> Self { Self::inch(6.9291, 9.8425, 1.0, 1.0, 1.0, 1.0) }
    pub fn presets() -> &'static [(&'static str, &'static str)] {
        &[
            ("Letter", "8.5 x 11 in  — Google Docs default"),
            ("Letter (Word)", "8.5 x 11 in  — Word default (1.25\" sides)"),
            ("A4", "210 x 297 mm  — International standard"),
            ("Legal", "8.5 x 14 in  — US legal"),
            ("A3", "297 x 420 mm"),
            ("A5", "148 x 210 mm"),
            ("Executive", "7.25 x 10.5 in"),
            ("Tabloid", "11 x 17 in"),
            ("B5", "176 x 250 mm"),
        ]
    }
    pub fn from_preset(i: usize) -> Self {
        match i {
            0 => Self::letter(),
            1 => Self::letter_word(),
            2 => Self::a4(),
            3 => Self::legal(),
            4 => Self::a3(),
            5 => Self::a5(),
            6 => Self::executive(),
            7 => Self::tabloid(),
            8 => Self::b5(),
            _ => Self::letter(),
        }
    }
}

pub fn ensure_boundary(para: &mut DocParagraph, byte: usize) {
    if byte == 0 || byte >= para.text.len() { return; }
    let mut pos = 0;
    for i in 0..para.spans.len() {
        pos += para.spans[i].len;
        if pos == byte { return; }
        if pos > byte {
            let over = pos - byte; para.spans[i].len -= over;
            let rhs = DocSpan { len: over, fmt: para.spans[i].fmt.clone() };
            para.spans.insert(i + 1, rhs); return;
        }
    }
}

pub fn merge_adjacent(para: &mut DocParagraph) {
    let mut i = 0;
    while i + 1 < para.spans.len() {
        if para.spans[i].fmt == para.spans[i+1].fmt { para.spans[i].len += para.spans[i+1].len; para.spans.remove(i+1); }
        else { i += 1; }
    }
    if para.spans.is_empty() { para.spans.push(DocSpan { len: 0, fmt: SpanFmt::default() }); }
}

pub fn apply_fmt_range(para: &mut DocParagraph, start: usize, end: usize, op: impl Fn(&mut SpanFmt)) {
    if start >= end { return; }
    ensure_boundary(para, start); ensure_boundary(para, end);
    let mut p = 0;
    for span in &mut para.spans {
        let e = p + span.len;
        if p >= end { break; }
        if e > start && p >= start { op(&mut span.fmt); }
        p = e;
    }
    merge_adjacent(para);
}

pub fn all_set_range(para: &DocParagraph, start: usize, end: usize, get: impl Fn(&SpanFmt) -> bool) -> bool {
    let mut p = 0; let mut any = false; let mut all = true;
    for s in &para.spans {
        let e = p + s.len;
        if p < end && e > start { any = true; if !get(&s.fmt) { all = false; break; } }
        p = e;
    }
    any && all
}

pub fn toggle_fmt(para: &mut DocParagraph, start: usize, end: usize, get: impl Fn(&SpanFmt) -> bool, set: impl Fn(&mut SpanFmt, bool)) {
    let v = !all_set_range(para, start, end, &get);
    apply_fmt_range(para, start, end, |fmt| set(fmt, v));
}

pub fn char_to_byte(text: &str, ci: usize) -> usize { text.char_indices().nth(ci).map(|(i, _)| i).unwrap_or(text.len()) }

pub fn para_fmt_at(para: &DocParagraph, byte: usize) -> SpanFmt {
    let mut p = 0;
    for s in &para.spans { let e = p + s.len; if byte >= p && (byte < e || (byte == 0 && e == 0)) { return s.fmt.clone(); } p = e; }
    para.spans.last().map(|s| s.fmt.clone()).unwrap_or_default()
}


pub fn merge_paragraphs(paras: &mut Vec<DocParagraph>, idx: usize) {
    if idx + 1 >= paras.len() { return; }
    let next = paras.remove(idx + 1);
    if paras[idx].spans.last().map(|s| s.len == 0).unwrap_or(false) { paras[idx].spans.pop(); }
    paras[idx].text.push_str(&next.text);
    for s in next.spans {
        if s.len == 0 { continue; }
        if paras[idx].spans.last().map(|l| l.fmt == s.fmt).unwrap_or(false) { paras[idx].spans.last_mut().unwrap().len += s.len; }
        else { paras[idx].spans.push(s); }
    }
    if paras[idx].spans.is_empty() { paras[idx].spans.push(DocSpan { len: 0, fmt: SpanFmt::default() }); }
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
        let mut p = 0usize;
        for s in &mut para.spans {
            let e = p + s.len;
            if p < del_end && e > del_start {
                let rm = del_end.min(e) - del_start.max(p);
                s.len = s.len.saturating_sub(rm);
            }
            p = e;
        }
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

pub fn build_layout_job(spans: &[DocSpan], text: &str, para: &DocParagraph, base_font: FontChoice, base_size: f32, wrap_w: f32, is_dark: bool, zoom: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_w;
    job.halign = egui::Align::LEFT;
    job.justify = false;

    let ss = base_size * para.style.size_scale();
    let (sb, si) = (para.style.is_bold(), para.style.is_italic());
    let code_bg = if is_dark {
        egui::Color32::from_rgb(28, 28, 34)
    } else {
        egui::Color32::from_rgb(244, 244, 248)
    };

    let base_col = match para.style {
        ParaStyle::H1 | ParaStyle::H2 => {
            if is_dark { ColorPalette::ZINC_100 } else { ColorPalette::ZINC_900 }
        }
        ParaStyle::H3 | ParaStyle::H4 => {
            if is_dark { ColorPalette::ZINC_200 } else { ColorPalette::ZINC_800 }
        }
        ParaStyle::Subtitle | ParaStyle::BlockQuote => {
            if is_dark { ColorPalette::ZINC_400 } else { ColorPalette::ZINC_600 }
        }
        _ => {
            if is_dark { ColorPalette::ZINC_200 } else { egui::Color32::from_rgb(22, 22, 22) }
        }
    };

    let mut pos = 0;
    for span in spans {
        if pos >= text.len() {
            break;
        }
        if span.len == 0 {
            continue;
        }

        let end = (pos + span.len).min(text.len());
        let seg = &text[pos..end];
        pos = end;

        if seg.is_empty() {
            continue;
        }

        let eff = span
            .fmt
            .size_hp
            .map(|hp| hp as f32 / 2.0 * zoom)
            .unwrap_or(ss);

        let sz = if span.fmt.sub || span.fmt.sup { eff * 0.68 } else { eff };
        let fc = span.fmt.font.unwrap_or(base_font);
        let col = span
            .fmt
            .color
            .map(|c| egui::Color32::from_rgb(c[0], c[1], c[2]))
            .unwrap_or(base_col);

        job.append(
            seg,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(sz, fc.egui_family(sb || span.fmt.bold, si || span.fmt.italic)),
                color: col,
                background: if para.style == ParaStyle::Code { code_bg } else { egui::Color32::TRANSPARENT },
                underline: if span.fmt.underline {
                    egui::Stroke::new((eff * 0.07).max(1.0), col)
                } else {
                    egui::Stroke::NONE
                },
                strikethrough: if span.fmt.strike {
                    egui::Stroke::new((eff * 0.07).max(1.0), col)
                } else {
                    egui::Stroke::NONE
                },
                valign: if span.fmt.sup {
                    egui::Align::TOP
                } else if span.fmt.sub {
                    egui::Align::BOTTOM
                } else {
                    egui::Align::Center
                },
                line_height: Some(eff * para.line_height),
                ..Default::default()
            },
        );
    }

    if job.sections.is_empty() {
        job.append(
            "",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(ss, base_font.egui_family(sb, si)),
                color: base_col,
                line_height: Some(ss * para.line_height),
                ..Default::default()
            },
        );
    }

    job
}

pub fn word_count(paras: &[DocParagraph]) -> usize { paras.iter().map(|p| p.text.split_whitespace().count()).sum() }
pub fn char_count(paras: &[DocParagraph]) -> usize { paras.iter().map(|p| p.text.chars().count()).sum() }

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>";
const ROOT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/></Relationships>";
const WORD_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>";

fn xml_esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }

fn build_document_xml(paras: &[DocParagraph], layout: &PageLayout) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n<w:body>\n");
    for para in paras {
        out.push_str("<w:p>\n<w:pPr>\n");
        if para.style != ParaStyle::Normal { out.push_str(&format!("<w:pStyle w:val=\"{}\"/>\n", para.style.docx_id())); }
        if para.align != Align::Left { out.push_str(&format!("<w:jc w:val=\"{}\"/>\n", para.align.docx_val())); }
        out.push_str(&format!("<w:spacing w:before=\"{}\" w:after=\"{}\" w:line=\"{}\" w:lineRule=\"auto\"/>\n",
            (para.space_before * 20.0) as u32, (para.space_after * 20.0) as u32, (para.line_height * 240.0) as u32));
        if para.indent_left != 0.0 || para.indent_first != 0.0 {
            out.push_str(&format!("<w:ind w:left=\"{}\" w:firstLine=\"{}\"/>\n", (para.indent_left * 20.0) as u32, (para.indent_first * 20.0) as u32));
        }
        out.push_str("</w:pPr>\n");
        let mut pos = 0;
        for span in &para.spans {
            if span.len == 0 { pos += span.len; continue; }
            if pos >= para.text.len() { break; }
            let end = (pos + span.len).min(para.text.len()); let txt = &para.text[pos..end]; pos = end;
            if txt.is_empty() { continue; }
            out.push_str("<w:r>\n");
            let hf = span.fmt.bold||span.fmt.italic||span.fmt.underline||span.fmt.strike||span.fmt.sub||span.fmt.sup||span.fmt.size_hp.is_some()||span.fmt.color.is_some();
            if hf {
                out.push_str("<w:rPr>\n");
                if span.fmt.bold { out.push_str("<w:b/>\n"); }
                if span.fmt.italic { out.push_str("<w:i/>\n"); }
                if span.fmt.underline { out.push_str("<w:u w:val=\"single\"/>\n"); }
                if span.fmt.strike { out.push_str("<w:strike/>\n"); }
                if span.fmt.sub { out.push_str("<w:vertAlign w:val=\"subscript\"/>\n"); }
                if span.fmt.sup { out.push_str("<w:vertAlign w:val=\"superscript\"/>\n"); }
                if let Some(sz) = span.fmt.size_hp { out.push_str(&format!("<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>\n", sz, sz)); }
                if let Some(c) = span.fmt.color { out.push_str(&format!("<w:color w:val=\"{:02X}{:02X}{:02X}\"/>\n", c[0], c[1], c[2])); }
                out.push_str("</w:rPr>\n");
            }
            let ps = if txt.starts_with(' ') || txt.ends_with(' ') { " xml:space=\"preserve\"" } else { "" };
            out.push_str(&format!("<w:t{}>{}</w:t>\n</w:r>\n", ps, xml_esc(txt)));
        }
        out.push_str("</w:p>\n");
    }
    let (w, h, mt, mb, ml, mr) = ((layout.width*20.0) as u32, (layout.height*20.0) as u32, (layout.margin_top*20.0) as u32, (layout.margin_bot*20.0) as u32, (layout.margin_left*20.0) as u32, (layout.margin_right*20.0) as u32);
    out.push_str(&format!("<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\"/></w:sectPr>\n</w:body>\n</w:document>", w, h, mt, mr, mb, ml));
    out
}

pub fn save_docx(path: &PathBuf, paras: &[DocParagraph], layout: &PageLayout) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in [("[Content_Types].xml", CONTENT_TYPES.as_bytes()), ("_rels/.rels", ROOT_RELS.as_bytes()), ("word/_rels/document.xml.rels", WORD_RELS.as_bytes())] {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    let doc = build_document_xml(paras, layout);
    zip.start_file("word/document.xml", opts).map_err(|e| e.to_string())?;
    zip.write_all(doc.as_bytes()).map_err(|e| e.to_string())?;
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
    let xml = { let mut e = arch.by_name("word/document.xml").map_err(|_| "Missing document.xml".to_string())?; let mut s = String::new(); e.read_to_string(&mut s).map_err(|e| e.to_string())?; s };
    parse_docx_xml(&xml)
}

fn parse_docx_xml(xml: &str) -> Result<(Vec<DocParagraph>, PageLayout), String> {
    use quick_xml::{Reader, events::Event};
    let mut reader = Reader::from_str(xml); reader.config_mut().trim_text(false);
    let mut paras: Vec<DocParagraph> = Vec::new(); let mut layout = PageLayout::default();
    let mut cur_para: Option<DocParagraph> = None; let mut cur_fmt = SpanFmt::default();
    let mut cur_run_text = String::new();
    let mut in_run = false; let mut in_rpr = false; let mut in_ppr = false; let mut in_t = false;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(ref e) => {
                match e.local_name().as_ref() {
                    b"p" => { cur_para = Some(DocParagraph::new()); in_ppr = false; }
                    b"pPr" => in_ppr = true,
                    b"pStyle" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"val") { p.style = ParaStyle::from_docx_id(&v); p.space_before = p.style.space_before(); p.space_after = p.style.space_after(); p.indent_left = p.style.default_indent(); } } } }
                    b"jc" => { if in_ppr { if let Some(ref mut p) = cur_para { p.align = match get_attr(e, b"val").as_deref() { Some("center") => Align::Center, Some("right") => Align::Right, Some("both") => Align::Justify, _ => Align::Left }; } } }
                    b"spacing" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"before") { p.space_before = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"after") { p.space_after = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"line") { p.line_height = v.parse::<f32>().unwrap_or(240.0)/240.0; } } } }
                    b"ind" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"left") { p.indent_left = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"firstLine") { p.indent_first = v.parse::<f32>().unwrap_or(0.0)/20.0; } } } }
                    b"r" => { in_run = true; cur_fmt = SpanFmt::default(); cur_run_text.clear(); }
                    b"rPr" => in_rpr = true,
                    b"t" => { in_t = true; cur_run_text.clear(); }
                    _ => {}
                }
            }
            Event::Empty(ref e) => {
                match e.local_name().as_ref() {
                    b"pStyle" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"val") { p.style = ParaStyle::from_docx_id(&v); p.space_before = p.style.space_before(); p.space_after = p.style.space_after(); p.indent_left = p.style.default_indent(); } } } }
                    b"jc" => { if in_ppr { if let Some(ref mut p) = cur_para { p.align = match get_attr(e, b"val").as_deref() { Some("center") => Align::Center, Some("right") => Align::Right, Some("both") => Align::Justify, _ => Align::Left }; } } }
                    b"spacing" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"before") { p.space_before = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"after") { p.space_after = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"line") { p.line_height = v.parse::<f32>().unwrap_or(240.0)/240.0; } } } }
                    b"ind" => { if in_ppr { if let Some(ref mut p) = cur_para { if let Some(v) = get_attr(e, b"left") { p.indent_left = v.parse::<f32>().unwrap_or(0.0)/20.0; } if let Some(v) = get_attr(e, b"firstLine") { p.indent_first = v.parse::<f32>().unwrap_or(0.0)/20.0; } } } }
                    b"b" => { if in_rpr { cur_fmt.bold = true; } }
                    b"i" => { if in_rpr { cur_fmt.italic = true; } }
                    b"u" => { if in_rpr && get_attr(e, b"val").as_deref() != Some("none") { cur_fmt.underline = true; } }
                    b"strike" => { if in_rpr { cur_fmt.strike = true; } }
                    b"vertAlign" => { if in_rpr { match get_attr(e, b"val").as_deref() { Some("subscript") => cur_fmt.sub = true, Some("superscript") => cur_fmt.sup = true, _ => {} } } }
                    b"sz" => { if in_rpr { cur_fmt.size_hp = get_attr(e, b"val").and_then(|v| v.parse().ok()); } }
                    b"color" => { if in_rpr { if let Some(v) = get_attr(e, b"val") { if v != "auto" && v.len() == 6 { if let (Ok(r), Ok(g), Ok(b)) = (u8::from_str_radix(&v[0..2],16), u8::from_str_radix(&v[2..4],16), u8::from_str_radix(&v[4..6],16)) { cur_fmt.color = Some([r,g,b]); } } } } }
                    b"pgSz" => { if let Some(v) = get_attr(e, b"w") { layout.width = v.parse::<f32>().unwrap_or(12240.0)/20.0; } if let Some(v) = get_attr(e, b"h") { layout.height = v.parse::<f32>().unwrap_or(15840.0)/20.0; } }
                    b"pgMar" => { if let Some(v) = get_attr(e, b"top") { layout.margin_top = v.parse::<f32>().unwrap_or(1440.0)/20.0; } if let Some(v) = get_attr(e, b"bottom") { layout.margin_bot = v.parse::<f32>().unwrap_or(1440.0)/20.0; } if let Some(v) = get_attr(e, b"left") { layout.margin_left = v.parse::<f32>().unwrap_or(1800.0)/20.0; } if let Some(v) = get_attr(e, b"right") { layout.margin_right = v.parse::<f32>().unwrap_or(1800.0)/20.0; } }
                    _ => {}
                }
            }
            Event::End(ref e) => {
                match e.local_name().as_ref() {
                    b"p" => { if let Some(p) = cur_para.take() { paras.push(p); } }
                    b"pPr" => in_ppr = false,
                    b"r" => {
                        in_run = false;
                        if let Some(ref mut para) = cur_para {
                            let blen = cur_run_text.len();
                            para.text.push_str(&cur_run_text);
                            if para.spans.last().map(|s| s.fmt == cur_fmt && s.len > 0).unwrap_or(false) { para.spans.last_mut().unwrap().len += blen; }
                            else { if para.spans.last().map(|s| s.len == 0).unwrap_or(false) { para.spans.pop(); } para.spans.push(DocSpan { len: blen, fmt: cur_fmt.clone() }); }
                        }
                    }
                    b"rPr" => in_rpr = false,
                    b"t" => in_t = false,
                    b"b" => { if in_rpr { cur_fmt.bold = true; } }
                    b"i" => { if in_rpr { cur_fmt.italic = true; } }
                    _ => {}
                }
            }
            Event::Text(ref e) => { if in_t && in_run { if let Ok(s) = std::str::from_utf8(e.as_ref()) { cur_run_text.push_str(s); } } }
            Event::Eof => break,
            _ => {}
        }
    }
    if paras.is_empty() { paras.push(DocParagraph::new()); }
    Ok((paras, layout))
}

pub fn load_txt_as_doc(path: &PathBuf) -> Result<Vec<DocParagraph>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut paras: Vec<DocParagraph> = content.lines().map(|line| {
        let mut p = DocParagraph::new(); p.text = line.to_string();
        p.spans = vec![DocSpan { len: line.len(), fmt: SpanFmt::default() }]; p
    }).collect();
    if paras.is_empty() { paras.push(DocParagraph::new()); }
    Ok(paras)
}
