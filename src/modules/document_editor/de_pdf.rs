use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use crate::style::{
    FONT_GS_BLD, FONT_GS_BLD_ITL, FONT_GS_ITL, FONT_GS_REG, FONT_OS_BLD, FONT_OS_BLD_ITL,
    FONT_OS_ITL, FONT_OS_REG, FONT_RB_BLD, FONT_RB_BLD_ITL, FONT_RB_ITL, FONT_RB_REG, FONT_UB_BLD,
    FONT_UB_BLD_ITL, FONT_UB_ITL, FONT_UB_REG,
};
use printpdf::{
    Color, FontId, Greyscale, Line, LinePoint, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle,
    PdfPage, PdfSaveOptions, PdfWarnMsg, Point, Pt, RawImage, Rgb, TextItem, XObjectTransform,
};
use std::{io::BufWriter, path::PathBuf};
use super::de_tools::*;

fn font_bytes(c: FontChoice, bold: bool, italic: bool) -> &'static [u8] {
    match (c, bold, italic) {
        (FontChoice::Ubuntu, true, true) => FONT_UB_BLD_ITL,
        (FontChoice::Ubuntu, true, false) => FONT_UB_BLD,
        (FontChoice::Ubuntu, false, true) => FONT_UB_ITL,
        (FontChoice::Ubuntu, ..) => FONT_UB_REG,
        (FontChoice::Roboto, true, true) => FONT_RB_BLD_ITL,
        (FontChoice::Roboto, true, false) => FONT_RB_BLD,
        (FontChoice::Roboto, false, true) => FONT_RB_ITL,
        (FontChoice::Roboto, ..) => FONT_RB_REG,
        (FontChoice::GoogleSans, true, true) => FONT_GS_BLD_ITL,
        (FontChoice::GoogleSans, true, false) => FONT_GS_BLD,
        (FontChoice::GoogleSans, false, true) => FONT_GS_ITL,
        (FontChoice::GoogleSans, ..) => FONT_GS_REG,
        (FontChoice::OpenSans, true, true) => FONT_OS_BLD_ITL,
        (FontChoice::OpenSans, true, false) => FONT_OS_BLD,
        (FontChoice::OpenSans, false, true) => FONT_OS_ITL,
        (FontChoice::OpenSans, ..) => FONT_OS_REG,
    }
}

fn mw(s: &str, c: FontChoice, bold: bool, italic: bool, pt: f32) -> f32 {
    FontRef::try_from_slice(font_bytes(c, bold, italic))
        .map(|f| {
            let sf = f.as_scaled(PxScale::from(pt));
            let units_per_em = f.units_per_em().unwrap_or(1000.0);
            let height = f.height_unscaled();
            let correction = height / units_per_em;
            s.chars()
                .map(|ch| sf.h_advance(f.glyph_id(ch)) * correction)
                .sum()
        })
        .unwrap_or_else(|_| s.len() as f32 * pt * 0.55)
}

fn pm(pt: f32) -> Mm { Mm(pt * 0.352_777_78) }
fn rgba_col(c: Option<[u8; 3]>, fb: [u8; 3]) -> Color {
    let [r, g, b] = c.unwrap_or(fb);
    Color::Rgb(Rgb::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        None,
    ))
}

struct Fonts { ids: Vec<FontId>, }
impl Fonts {
    fn new(doc: &mut PdfDocument) -> Self {
        const VARIANTS: &[(FontChoice, bool, bool)] = &[
            (FontChoice::Ubuntu, false, false),
            (FontChoice::Ubuntu, true, false),
            (FontChoice::Ubuntu, false, true),
            (FontChoice::Ubuntu, true, true),
            (FontChoice::Roboto, false, false),
            (FontChoice::Roboto, true, false),
            (FontChoice::Roboto, false, true),
            (FontChoice::Roboto, true, true),
            (FontChoice::GoogleSans, false, false),
            (FontChoice::GoogleSans, true, false),
            (FontChoice::GoogleSans, false, true),
            (FontChoice::GoogleSans, true, true),
            (FontChoice::OpenSans, false, false),
            (FontChoice::OpenSans, true, false),
            (FontChoice::OpenSans, false, true),
            (FontChoice::OpenSans, true, true),
        ];
        const NAMES: &[&str] = &[
            "Ubuntu", "Ubuntu-B", "Ubuntu-I", "Ubuntu-BI", "Roboto", "Roboto-B", "Roboto-I",
            "Roboto-BI", "GSans", "GSans-B", "GSans-I", "GSans-BI", "OSans", "OSans-B", "OSans-I",
            "OSans-BI",
        ];
        let mut ids = Vec::with_capacity(16);
        for (&(c, b, i), _) in VARIANTS.iter().zip(NAMES.iter()) {
            let bytes = font_bytes(c, b, i);
            let mut font_warn = Vec::new();
            let parsed = ParsedFont::from_bytes(bytes, 0, &mut font_warn).unwrap_or_else(|| {
                ParsedFont::from_bytes(FONT_UB_REG, 0, &mut font_warn).unwrap()
            });
            ids.push(doc.add_font(&parsed));
        }
        Self { ids }
    }

    fn handle(&self, c: FontChoice, bold: bool, italic: bool) -> PdfFontHandle {
        let family = match c {
            FontChoice::Ubuntu => 0,
            FontChoice::Roboto => 1,
            FontChoice::GoogleSans => 2,
            FontChoice::OpenSans => 3,
        };
        PdfFontHandle::External(self.ids[family * 4 + bold as usize + italic as usize * 2].clone())
    }
}

struct PdfCtx { doc: PdfDocument, pages: Vec<PdfPage>, ops: Vec<Op>, y: f32,
    pw: f32, ph: f32, warn: Vec<PdfWarnMsg>, fonts: Fonts, }
impl PdfCtx {
    fn new(title: &str, layout: &PageLayout) -> Self {
        let mut doc = PdfDocument::new(title);  
        let warn = Vec::new();
        let fonts = Fonts::new(&mut doc);
        Self {
            doc, pages: Vec::new(), ops: Vec::new(), y: layout.margin_top,
            pw: layout.width, ph: layout.height, warn, fonts,
        }
    }

    fn new_page(&mut self, margin_top: f32) {
        let ops = std::mem::take(&mut self.ops);
        self.pages.push(PdfPage::new(pm(self.pw), pm(self.ph), ops));
        self.y = margin_top;
    }

    fn ensure(&mut self, h: f32, layout: &PageLayout) {
        if self.y + h > layout.height - layout.margin_bot {
            self.new_page(layout.margin_top);
        }
    }

    fn draw_text(&mut self, text: &str, seg: &Seg, x: f32, y_bl: f32) {
        if text.is_empty() { return; }
        let total_w = mw(text, seg.font, seg.bold, seg.italic, seg.size);
        let yy = y_bl + seg.y_shift;

        let fh = self.fonts.handle(seg.font, seg.bold, seg.italic);
        self.ops.push(Op::StartTextSection);
        self.ops.push(Op::SetFillColor {
            col: rgba_col(seg.color, [0, 0, 0]),
        });
        self.ops.push(Op::SetFont {
            font: fh,
            size: Pt(seg.size),
        });
        self.ops.push(Op::SetTextCursor {
            pos: Point::new(pm(x), pm(yy)),
        });
        self.ops.push(Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        });
        self.ops.push(Op::EndTextSection);

        if seg.underline {
            self.push_line(
                x,
                yy - seg.size * 0.10,
                x + total_w,
                yy - seg.size * 0.10,
                rgba_col(seg.color, [0, 0, 0]),
                (seg.size * 0.05).max(0.4),
            );
        }
        if seg.strike {
            self.push_line(
                x,
                yy + seg.size * 0.30,
                x + total_w,
                yy + seg.size * 0.30,
                rgba_col(seg.color, [0, 0, 0]),
                (seg.size * 0.05).max(0.4),
            );
        }
    }

    fn push_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, col: Color, thick: f32) {
        self.ops.push(Op::SetOutlineColor { col });
        self.ops.push(Op::SetOutlineThickness { pt: Pt(thick) });
        self.ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point::new(pm(x0), pm(y0)),
                        bezier: false,
                    },
                    LinePoint {
                        p: Point::new(pm(x1), pm(y1)),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    }
}

#[derive(Clone)]
struct Seg { text: String, font: FontChoice, bold: bool, italic: bool, size: f32,
    color: Option<[u8; 3]>, underline: bool, strike: bool, y_shift: f32, }
impl Seg {
    fn w(&self) -> f32 { mw(&self.text, self.font, self.bold, self.italic, self.size) }
}

fn build_segs(para: &DocParagraph) -> Vec<Seg> {
    let sb = para.style.is_bold();
    let si = para.style.is_italic();
    let base = para.style.default_font_size_pt() as f32;
    let mut out = Vec::new();
    let mut pos = 0usize;
    for span in para.spans.iter() {
        let end = (pos + span.len).min(para.text.len());
        if pos < end {
            let eff = span.fmt.size_hp.map(|h| h as f32 / 2.0).unwrap_or(base);
            let (size, y_shift) = if span.fmt.sup {
                (eff * 0.68, eff * 0.28)
            } else if span.fmt.sub {
                (eff * 0.68, -eff * 0.10)
            } else {
                (eff, 0.0)
            };
            let text = para.text[pos..end].to_string();
            out.push(Seg {
                text, font: span.fmt.font.unwrap_or(DEFAULT_BASE_FONT),
                bold: sb || span.fmt.bold, italic: si || span.fmt.italic,
                size, color: span.fmt.color,
                underline: span.fmt.underline || span.fmt.link.is_some(), strike: span.fmt.strike,
                y_shift,
            });
        }
        pos = end;
    }
    out
}

fn wrap_segs(segs: &[Seg], max_w: f32) -> Vec<Vec<Seg>> {
    let mut lines: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut x = 0.0f32;
    for seg in segs {
        for (pi, part) in seg.text.split('\n').enumerate() {
            if pi > 0 { lines.push(Vec::new()); x = 0.0; }
            let mut pos = 0usize;
            while pos < part.len() {
                let wl = part[pos..].find(' ').unwrap_or(part.len() - pos);
                let we = pos + wl;
                let sl = part[we..].find(|c: char| c != ' ').unwrap_or(part.len() - we);
                let ce = we + sl;
                let word = &part[pos..we];
                let chunk = &part[pos..ce];
                if chunk.is_empty() { pos = ce; continue; }
                let w = mw(word, seg.font, seg.bold, seg.italic, seg.size);
                if x > 0.0 && x + w > max_w {
                    if let Some(l) = lines.last_mut().and_then(|v| v.last_mut()) {
                        l.text = l.text.trim_end_matches(' ').to_string();
                    }
                    lines.push(Vec::new()); x = 0.0;
                }
                x += mw(chunk, seg.font, seg.bold, seg.italic, seg.size);
                lines.last_mut().unwrap().push(Seg { text: chunk.to_string(), ..seg.clone() });
                pos = ce;
            }
        }
    }
    if let Some(l) = lines.last_mut().and_then(|v| v.last_mut()) {
        l.text = l.text.trim_end_matches(' ').to_string();
    }
    while lines.len() > 1 && lines.last().map_or(false, |v| v.is_empty()) { lines.pop(); }
    lines
}

fn lh_for(line: &[Seg], base: f32, para_lh: f32) -> f32 { line.iter().map(|s| s.size).fold(base, f32::max) * para_lh }
fn render_line(ctx: &mut PdfCtx, line: &[Seg], x0: f32, avail_w: f32, y_bl: f32, align: Align) {
    let line_w: f32 = line.iter().map(|s| s.w()).sum();
    let offset = match align {
        Align::Center => ((avail_w - line_w) / 2.0).max(0.0),
        Align::Right => (avail_w - line_w).max(0.0),
        _ => 0.0,
    };
    let mut x = x0 + offset;
    for seg in line {
        ctx.draw_text(&seg.text, seg, x, y_bl);
        x += seg.w();
    }
}

fn render_image(ctx: &mut PdfCtx, para: &DocParagraph, layout: &PageLayout) {
    let img = match &para.image {
        Some(i) if !i.data.is_empty() => i,
        _ => return,
    };
    let raw = match RawImage::decode_from_bytes(&img.data, &mut ctx.warn) {
        Ok(r) => r,
        Err(_) => return,
    };
    let max_h = layout.height - layout.margin_top - layout.margin_bot;
    let h = img.display_h.min(max_h).max(1.0);
    let w = img.display_w.max(1.0);
    ctx.ensure(h + 4.0, layout);
    let ix = match para.align {
        Align::Center => layout.margin_left + (layout.content_width() - w) / 2.0,
        Align::Right => layout.margin_left + layout.content_width() - w,
        _ => layout.margin_left,
    };
    let iy = layout.height - ctx.y - h;
    let dyn_img = match image::load_from_memory(&img.data) {
        Ok(v) => v,
        Err(_) => return,
    };
    let scale_x = w / dyn_img.width().max(1) as f32;
    let scale_y = h / dyn_img.height().max(1) as f32;
    let id = ctx.doc.add_image(&raw);
    ctx.ops.push(Op::UseXobject {
        id,
        transform: XObjectTransform {
            translate_x: Some(Pt(ix)),
            translate_y: Some(Pt(iy)),
            rotate: None,
            scale_x: Some(scale_x),
            scale_y: Some(scale_y),
            dpi: Some(72.0),
        },
    });
    ctx.y += h + 4.0;
}

fn render_table(ctx: &mut PdfCtx, tbl: &TableData, layout: &PageLayout) {
    let rows = &tbl.rows;
    if rows.is_empty() {
        return;
    }
    let nc = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    let col_px: Vec<f32> = if tbl.col_widths.len() == nc {
        tbl.col_widths.iter().map(|f| f * layout.content_width()).collect()
    } else {
        vec![layout.content_width() / nc as f32; nc]
    };
    let pad = 6.0f32;
    let bsz = DEFAULT_BASE_SIZE as f32 * 0.9;
    let bw = tbl.border_width.max(0.25);
    ctx.y += 4.0;
    for row in rows {
        let rh = row.iter().enumerate().fold(18.0f32, |acc, (ci, cell)| {
            let cw = col_px.get(ci).copied().unwrap_or(layout.content_width() / nc as f32);
            let seg = Seg {
                text: cell.text.clone(), font: DEFAULT_BASE_FONT,
                bold: false, italic: false,
                size: bsz, color: None,
                underline: false, strike: false,
                y_shift: 0.0,
            };
            let n = wrap_segs(&[seg], (cw - pad * 2.0).max(5.0)).len() as f32;
            acc.max((n * bsz * 1.35 + 8.0).max(18.0))
        });

        ctx.ensure(rh, layout);
        let top = layout.height - ctx.y;
        let mut x = layout.margin_left;
        for (ci, cell) in row.iter().enumerate() {
            let cw = col_px.get(ci).copied().unwrap_or(layout.content_width() / nc as f32);
            let (left, right, bot) = (x, x + cw, top - rh);
            let border = rgba_col(Some(tbl.border_color), [110, 110, 110]);
            ctx.push_line(left, top, right, top, border.clone(), bw);
            ctx.push_line(left, bot, right, bot, border.clone(), bw);
            ctx.push_line(left, bot, left, top, border.clone(), bw);
            ctx.push_line(right, bot, right, top, border.clone(), bw);

            let cell_seg = Seg {
                text: cell.text.clone(), font: cell.spans.first().and_then(|s| s.fmt.font).unwrap_or(DEFAULT_BASE_FONT),
                bold: cell.spans.first().map(|s| s.fmt.bold).unwrap_or(false), italic: cell.spans.first().map(|s| s.fmt.italic).unwrap_or(false),
                size: bsz, color: cell.spans.first().and_then(|s| s.fmt.color),
                underline: false, strike: false,
                y_shift: 0.0,
            };
            let wrapped = wrap_segs(&[cell_seg], (cw - pad * 2.0).max(5.0));
            let mut y_c = ctx.y + 6.0 + bsz * 0.8;
            for line in &wrapped {
                render_line(ctx, line, left + pad, cw - pad * 2.0, layout.height - y_c, Align::Left);
                y_c += bsz * 1.35;
            }
            x += cw;
        }
        ctx.y += rh;
    }
    ctx.y += 4.0;
}

fn render_para(ctx: &mut PdfCtx, para: &DocParagraph, layout: &PageLayout, list_n: u32) {
    match para.style {
        ParaStyle::HRule => {
            ctx.ensure(12.0, layout);
            let y = layout.height - ctx.y - 6.0;
            ctx.push_line(layout.margin_left, y, layout.margin_left + layout.content_width(), y,
                Color::Greyscale(Greyscale::new(0.6, None)), 0.6 );
            ctx.y += 12.0;
            return;
        }
        ParaStyle::Table => {
            if let Some(tbl) = para.table.as_deref() {
                render_table(ctx, tbl, layout);
            }
            return;
        }
        ParaStyle::Image => {
            render_image(ctx, para, layout);
            return;
        }
        _ => {}
    }

    let base = para.style.default_font_size_pt() as f32;
    let indent = para.indent_left;
    let cw = (layout.content_width() - indent).max(10.0);
    let x0 = layout.margin_left + indent;
    ctx.y += para.space_before;
    let segs = build_segs(para);
    let lines = if segs.is_empty() {
        vec![Vec::new()]
    } else {
        wrap_segs(&segs, cw)
    };

    for (li, line) in lines.iter().enumerate() {
        let lh = lh_for(line, base, para.line_height);
        ctx.ensure(lh, layout);
        let ascent = base * 0.78;
        let y_bl = layout.height - ctx.y - ascent;
        if li == 0 {
            let marker: Option<String> = match para.style {
                ParaStyle::ListBullet => Some("\u{2022}".into()),
                ParaStyle::ListCheck => Some(if para.checked { "[x]" } else { "[ ]" }.into()),
                ParaStyle::ListOrdered => Some(format!("{}.", list_n)),
                _ => None,
            };
            if let Some(m) = marker {
                let mseg = Seg {
                    text: m, font: DEFAULT_BASE_FONT,
                    bold: false, italic: false,
                    size: base * 0.9, color: Some([110, 110, 110]),
                    underline: false, strike: false,
                    y_shift: 0.0,
                };
                let mx = (layout.margin_left + indent - base * 1.5).max(2.0);
                ctx.draw_text(&mseg.text, &mseg, mx, y_bl);
            }
        }
        render_line(ctx, line, x0, cw, y_bl, para.align);
        ctx.y += lh;
    }
    ctx.y += para.space_after;
}

pub fn export_to_pdf(paras: &[DocParagraph], layout: &PageLayout, path: &PathBuf) -> Result<(), String> {
    let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Document");
    let mut ctx = PdfCtx::new(title, layout);
    let mut list_n = 0u32;

    for para in paras {
        list_n = if para.style == ParaStyle::ListOrdered { list_n + 1 } else { 0 };
        render_para(&mut ctx, para, layout, list_n);
    }
    if !ctx.ops.is_empty() || ctx.pages.is_empty() {
        let ops = std::mem::take(&mut ctx.ops);
        ctx.pages.push(PdfPage::new(pm(ctx.pw), pm(ctx.ph), ops));
    }

    let mut warn = std::mem::take(&mut ctx.warn);
    let PdfCtx { mut doc, pages, .. } = ctx;
    let save_opts = PdfSaveOptions {
        subset_fonts: false,
        ..Default::default()
    };
    let bytes = doc.with_pages(pages).save(&save_opts, &mut warn);
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut w = BufWriter::new(file);
    use std::io::Write;
    w.write_all(&bytes).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}
