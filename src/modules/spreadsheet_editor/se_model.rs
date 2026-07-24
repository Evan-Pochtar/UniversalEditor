use ahash::AHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum HAlign { #[default] General, Left, Center, Right }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum NumFmt { #[default] General, Number, Currency, Percent }

impl NumFmt {
    pub fn label(self) -> &'static str { match self { Self::General=>"General", Self::Number=>"Number", Self::Currency=>"Currency", Self::Percent=>"Percent" } }
    pub fn all() -> &'static [NumFmt] { &[Self::General, Self::Number, Self::Currency, Self::Percent] }
}

#[derive(Clone, PartialEq, Default)]
pub struct CellFmt { pub bold: bool, pub italic: bool, pub underline: bool, pub color: Option<[u8;3]>, pub bg: Option<[u8;3]>, pub align: HAlign, pub numfmt: NumFmt }

#[derive(Clone, Default)]
pub struct Cell { pub raw: String, pub fmt: CellFmt }

#[derive(Clone)]
pub struct Sheet { pub name: String, pub cells: AHashMap<(u32,u32), Cell>, pub rows: u32, pub cols: u32, pub col_widths: AHashMap<u32,f32> }

impl Sheet {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), cells: AHashMap::default(), rows: 60, cols: 20, col_widths: AHashMap::default() } }
    pub fn raw(&self, r: u32, c: u32) -> &str { self.cells.get(&(r,c)).map_or("", |x| x.raw.as_str()) }
    pub fn fmt(&self, r: u32, c: u32) -> CellFmt { self.cells.get(&(r,c)).map_or_else(CellFmt::default, |x| x.fmt.clone()) }
    pub fn set_raw(&mut self, r: u32, c: u32, val: String) {
        if val.is_empty() { if let Some(cell) = self.cells.get_mut(&(r,c)) { cell.raw.clear(); } }
        else { self.cells.entry((r,c)).or_default().raw = val; }
        self.rows = self.rows.max(r+1); self.cols = self.cols.max(c+1);
    }
    pub fn col_width(&self, c: u32) -> f32 { self.col_widths.get(&c).copied().unwrap_or(92.0) }
    pub fn max_used_row(&self) -> u32 { self.cells.keys().map(|&(r,_)| r).max().unwrap_or(0) }
    pub fn max_used_col(&self) -> u32 { self.cells.keys().map(|&(_,c)| c).max().unwrap_or(0) }
}
