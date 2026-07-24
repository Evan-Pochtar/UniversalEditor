use super::se_model::{Sheet, Cell};

pub fn col_label(mut c: u32) -> String {
    let mut s = Vec::new();
    loop { s.push((b'A' + (c % 26) as u8) as char); if c < 26 { break; } c = c / 26 - 1; }
    s.iter().rev().collect()
}

pub fn col_from_label(label: &str) -> Option<u32> {
    if label.is_empty() { return None; }
    let mut c: u32 = 0;
    for ch in label.chars() { if !ch.is_ascii_alphabetic() { return None; } c = c*26 + (ch.to_ascii_uppercase() as u32 - 'A' as u32 + 1); }
    Some(c - 1)
}

pub fn parse_cell_ref(s: &str) -> Option<(u32,u32)> {
    let i = s.find(|c: char| c.is_ascii_digit())?;
    let (col_s, row_s) = s.split_at(i);
    let col = col_from_label(col_s)?;
    let row: u32 = row_s.parse().ok()?;
    if row == 0 { return None; }
    Some((row - 1, col))
}

pub fn cell_ref(r: u32, c: u32) -> String { format!("{}{}", col_label(c), r+1) }

pub fn sort_by_column(sheet: &mut Sheet, col: u32, asc: bool, has_header: bool) {
    let start = if has_header {1} else {0};
    let max_r = sheet.max_used_row(); let max_c = sheet.max_used_col();
    if max_r < start { return; }
    let mut rows: Vec<Vec<Option<Cell>>> = (start..=max_r).map(|r| (0..=max_c).map(|c| sheet.cells.get(&(r,c)).cloned()).collect()).collect();
    rows.sort_by(|a, b| {
        let av = a.get(col as usize).and_then(|c| c.as_ref()).map(|c| c.raw.as_str()).unwrap_or("");
        let bv = b.get(col as usize).and_then(|c| c.as_ref()).map(|c| c.raw.as_str()).unwrap_or("");
        let ord = match (av.parse::<f64>(), bv.parse::<f64>()) { (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal), _ => av.cmp(bv) };
        if asc { ord } else { ord.reverse() }
    });
    for (i, row) in rows.into_iter().enumerate() {
        let r = start + i as u32;
        for (c, cell) in row.into_iter().enumerate() {
            let key = (r, c as u32);
            match cell { Some(cell) => { sheet.cells.insert(key, cell); } None => { sheet.cells.remove(&key); } }
        }
    }
}

pub fn insert_row(sheet: &mut Sheet, at: u32) {
    let old = std::mem::take(&mut sheet.cells);
    for ((r,c), cell) in old { sheet.cells.insert((if r>=at {r+1} else {r}, c), cell); }
    sheet.rows += 1;
}
pub fn delete_row(sheet: &mut Sheet, at: u32) {
    let old = std::mem::take(&mut sheet.cells);
    for ((r,c), cell) in old { if r==at { continue; } sheet.cells.insert((if r>at {r-1} else {r}, c), cell); }
    sheet.rows = sheet.rows.saturating_sub(1);
}
pub fn insert_col(sheet: &mut Sheet, at: u32) {
    let old = std::mem::take(&mut sheet.cells);
    for ((r,c), cell) in old { sheet.cells.insert((r, if c>=at {c+1} else {c}), cell); }
    sheet.cols += 1;
}
pub fn delete_col(sheet: &mut Sheet, at: u32) {
    let old = std::mem::take(&mut sheet.cells);
    for ((r,c), cell) in old { if c==at { continue; } sheet.cells.insert((r, if c>at {c-1} else {c}), cell); }
    sheet.cols = sheet.cols.saturating_sub(1);
}

pub fn range_to_tsv(sheet: &Sheet, r0: u32, c0: u32, r1: u32, c1: u32) -> String {
    (r0..=r1).map(|r| (c0..=c1).map(|c| sheet.raw(r,c).replace('\t'," ")).collect::<Vec<_>>().join("\t")).collect::<Vec<_>>().join("\n")
}
pub fn paste_tsv(sheet: &mut Sheet, r0: u32, c0: u32, text: &str) {
    for (i, line) in text.lines().enumerate() { for (j, val) in line.split('\t').enumerate() { sheet.set_raw(r0+i as u32, c0+j as u32, val.to_string()); } }
}
