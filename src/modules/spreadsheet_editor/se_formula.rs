use super::se_model::{Sheet, NumFmt};
use super::se_tools::parse_cell_ref;
use ahash::AHashMap;

#[derive(Clone, Debug)]
pub enum FVal { Num(f64), Str(String), Bool(bool), Err(&'static str) }

impl FVal {
    pub fn display(&self) -> String {
        match self {
            FVal::Num(n) => if n.fract()==0.0 && n.abs()<1e15 { format!("{}", *n as i64) } else { format!("{}", n) },
            FVal::Str(s) => s.clone(),
            FVal::Bool(b) => if *b {"TRUE".into()} else {"FALSE".into()},
            FVal::Err(e) => format!("#{}", e),
        }
    }
    pub fn num(&self) -> f64 { match self { FVal::Num(n)=>*n, FVal::Bool(b)=>if *b {1.0} else {0.0}, FVal::Str(s)=>s.trim().parse().unwrap_or(0.0), FVal::Err(_)=>0.0 } }
}

pub fn format_display(val: &FVal, fmt: NumFmt) -> String {
    match (fmt, val) {
        (NumFmt::Number, FVal::Num(n)) => format!("{:.2}", n),
        (NumFmt::Currency, FVal::Num(n)) => format!("${:.2}", n),
        (NumFmt::Percent, FVal::Num(n)) => format!("{:.1}%", n*100.0),
        _ => val.display(),
    }
}

enum Expr { Num(f64), Str(String), Ref(u32,u32), Range(u32,u32,u32,u32), Neg(Box<Expr>), Bin(u8, Box<Expr>, Box<Expr>), Call(String, Vec<Expr>) }

struct FParser<'a> { s: &'a [u8], i: usize }

impl<'a> FParser<'a> {
    fn new(s: &'a str) -> Self { Self { s: s.as_bytes(), i: 0 } }
    fn peek(&self) -> u8 { *self.s.get(self.i).unwrap_or(&0) }
    fn skip_ws(&mut self) { while self.peek()==b' ' { self.i+=1; } }
    fn eat(&mut self, c: u8) -> bool { self.skip_ws(); if self.peek()==c { self.i+=1; true } else { false } }
    fn parse(&mut self) -> Expr { self.compare() }
    fn compare(&mut self) -> Expr {
        let mut l = self.concat();
        loop {
            self.skip_ws();
            let op = match self.peek() { b'='=>b'=', b'<'=>b'<', b'>'=>b'>', _=>0 };
            if op==0 { break; }
            self.i+=1;
            if op==b'<' && self.peek()==b'>' { self.i+=1; l = Expr::Bin(b'!', Box::new(l), Box::new(self.concat())); continue; }
            l = Expr::Bin(op, Box::new(l), Box::new(self.concat()));
        }
        l
    }
    fn concat(&mut self) -> Expr { let mut l = self.addsub(); while self.eat(b'&') { l = Expr::Bin(b'&', Box::new(l), Box::new(self.addsub())); } l }
    fn addsub(&mut self) -> Expr {
        let mut l = self.muldiv();
        loop { self.skip_ws(); match self.peek() { b'+' => { self.i+=1; l=Expr::Bin(b'+',Box::new(l),Box::new(self.muldiv())); }, b'-' => { self.i+=1; l=Expr::Bin(b'-',Box::new(l),Box::new(self.muldiv())); }, _ => break } }
        l
    }
    fn muldiv(&mut self) -> Expr {
        let mut l = self.unary();
        loop { self.skip_ws(); match self.peek() { b'*' => { self.i+=1; l=Expr::Bin(b'*',Box::new(l),Box::new(self.unary())); }, b'/' => { self.i+=1; l=Expr::Bin(b'/',Box::new(l),Box::new(self.unary())); }, _ => break } }
        l
    }
    fn unary(&mut self) -> Expr { self.skip_ws(); if self.peek()==b'-' { self.i+=1; return Expr::Neg(Box::new(self.unary())); } self.power() }
    fn power(&mut self) -> Expr { let l = self.primary(); if self.eat(b'^') { return Expr::Bin(b'^', Box::new(l), Box::new(self.unary())); } l }
    fn primary(&mut self) -> Expr {
        self.skip_ws();
        if self.eat(b'(') { let e = self.compare(); self.eat(b')'); return e; }
        if self.peek()==b'"' {
            self.i+=1; let start=self.i;
            while self.peek()!=b'"' && self.peek()!=0 { self.i+=1; }
            let s = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
            if self.peek()==b'"' { self.i+=1; }
            return Expr::Str(s);
        }
        if self.peek().is_ascii_digit() || self.peek()==b'.' {
            let start=self.i;
            while self.peek().is_ascii_digit() || self.peek()==b'.' { self.i+=1; }
            return Expr::Num(std::str::from_utf8(&self.s[start..self.i]).unwrap_or("0").parse().unwrap_or(0.0));
        }
        if self.peek().is_ascii_alphabetic() {
            let start=self.i;
            while self.peek().is_ascii_alphanumeric() { self.i+=1; }
            let word = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_string();
            self.skip_ws();
            if self.peek()==b'(' {
                self.i+=1; let mut args=Vec::new(); self.skip_ws();
                if self.peek()!=b')' { loop { args.push(self.compare()); self.skip_ws(); if self.eat(b',') { continue; } break; } }
                self.eat(b')');
                return Expr::Call(word, args);
            }
            if let Some((r0,c0)) = parse_cell_ref(&word) {
                if self.peek()==b':' {
                    self.i+=1; self.skip_ws(); let start2=self.i;
                    while self.peek().is_ascii_alphanumeric() { self.i+=1; }
                    let word2 = std::str::from_utf8(&self.s[start2..self.i]).unwrap_or("");
                    if let Some((r1,c1)) = parse_cell_ref(word2) { return Expr::Range(r0.min(r1), c0.min(c1), r0.max(r1), c0.max(c1)); }
                }
                return Expr::Ref(r0, c0);
            }
            return Expr::Str(word);
        }
        self.i+=1; Expr::Num(0.0)
    }
}

pub type Cache = AHashMap<(u32,u32), FVal>;

pub fn eval_cell(sheet: &Sheet, r: u32, c: u32, cache: &mut Cache, stack: &mut Vec<(u32,u32)>) -> FVal {
    if let Some(v) = cache.get(&(r,c)) { return v.clone(); }
    if stack.contains(&(r,c)) { return FVal::Err("REF"); }
    let raw = sheet.raw(r,c);
    let val = if let Some(f) = raw.strip_prefix('=') {
        stack.push((r,c));
        let v = eval_expr(&FParser::new(f).parse(), sheet, cache, stack);
        stack.pop();
        v
    } else if raw.is_empty() { FVal::Str(String::new()) }
    else if let Ok(n) = raw.parse::<f64>() { FVal::Num(n) }
    else { FVal::Str(raw.to_string()) };
    cache.insert((r,c), val.clone());
    val
}

fn flatten(e: &Expr, sheet: &Sheet, cache: &mut Cache, stack: &mut Vec<(u32,u32)>, out: &mut Vec<FVal>) {
    if let Expr::Range(r0,c0,r1,c1) = e { for r in *r0..=*r1 { for c in *c0..=*c1 { out.push(eval_cell(sheet,r,c,cache,stack)); } } }
    else { out.push(eval_expr(e, sheet, cache, stack)); }
}

fn eval_expr(e: &Expr, sheet: &Sheet, cache: &mut Cache, stack: &mut Vec<(u32,u32)>) -> FVal {
    match e {
        Expr::Num(n) => FVal::Num(*n),
        Expr::Str(s) => FVal::Str(s.clone()),
        Expr::Ref(r,c) => eval_cell(sheet, *r, *c, cache, stack),
        Expr::Range(r0,c0,_,_) => eval_cell(sheet, *r0, *c0, cache, stack),
        Expr::Neg(a) => FVal::Num(-eval_expr(a,sheet,cache,stack).num()),
        Expr::Bin(op,a,b) => {
            let (x,y) = (eval_expr(a,sheet,cache,stack), eval_expr(b,sheet,cache,stack));
            match op {
                b'&' => FVal::Str(format!("{}{}", x.display(), y.display())),
                b'=' => FVal::Bool(x.display()==y.display()),
                b'!' => FVal::Bool(x.display()!=y.display()),
                b'+' => FVal::Num(x.num()+y.num()),
                b'-' => FVal::Num(x.num()-y.num()),
                b'*' => FVal::Num(x.num()*y.num()),
                b'/' => if y.num()==0.0 { FVal::Err("DIV0") } else { FVal::Num(x.num()/y.num()) },
                b'^' => FVal::Num(x.num().powf(y.num())),
                b'<' => FVal::Bool(x.num()<y.num()),
                b'>' => FVal::Bool(x.num()>y.num()),
                _ => FVal::Err("OP"),
            }
        }
        Expr::Call(name, args) => call_fn(name, args, sheet, cache, stack),
    }
}

fn call_fn(name: &str, args: &[Expr], sheet: &Sheet, cache: &mut Cache, stack: &mut Vec<(u32,u32)>) -> FVal {
    match name.to_ascii_uppercase().as_str() {
        "IF" => {
            let c = args.first().map(|a| eval_expr(a,sheet,cache,stack)).unwrap_or(FVal::Bool(false));
            let truthy = matches!(c, FVal::Bool(true)) || (!matches!(c, FVal::Bool(_)) && c.num()!=0.0);
            if truthy { args.get(1).map(|a| eval_expr(a,sheet,cache,stack)).unwrap_or(FVal::Num(0.0)) } else { args.get(2).map(|a| eval_expr(a,sheet,cache,stack)).unwrap_or(FVal::Num(0.0)) }
        }
        "ROUND" => {
            let n = args.first().map(|a| eval_expr(a,sheet,cache,stack).num()).unwrap_or(0.0);
            let d = args.get(1).map(|a| eval_expr(a,sheet,cache,stack).num()).unwrap_or(0.0);
            let m = 10f64.powf(d);
            FVal::Num((n*m).round()/m)
        }
        "ABS" => FVal::Num(args.first().map(|a| eval_expr(a,sheet,cache,stack).num()).unwrap_or(0.0).abs()),
        upper => {
            let mut flat = Vec::new();
            for a in args { flatten(a, sheet, cache, stack, &mut flat); }
            match upper {
                "SUM" => FVal::Num(flat.iter().map(|v| v.num()).sum()),
                "AVERAGE" => FVal::Num(if flat.is_empty() {0.0} else {flat.iter().map(|v| v.num()).sum::<f64>()/flat.len() as f64}),
                "MIN" => FVal::Num(if flat.is_empty() {0.0} else {flat.iter().map(|v| v.num()).fold(f64::INFINITY, f64::min)}),
                "MAX" => FVal::Num(if flat.is_empty() {0.0} else {flat.iter().map(|v| v.num()).fold(f64::NEG_INFINITY, f64::max)}),
                "COUNT" => FVal::Num(flat.iter().filter(|v| matches!(v, FVal::Num(_))).count() as f64),
                "COUNTA" => FVal::Num(flat.iter().filter(|v| !v.display().is_empty()).count() as f64),
                "CONCAT" | "CONCATENATE" => FVal::Str(flat.iter().map(|v| v.display()).collect()),
                _ => FVal::Err("NAME"),
            }
        }
    }
}
