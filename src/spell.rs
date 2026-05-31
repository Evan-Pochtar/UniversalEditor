use ahash::AHashSet;
use std::sync::OnceLock;

static DICT: OnceLock<AHashSet<String>> = OnceLock::new();

pub fn init() {
    DICT.get_or_init(|| {
        include_str!("../assets/en_words.txt")
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().to_lowercase())
            .collect()
    });
}

#[inline]
fn is_apos(c: char) -> bool {
    matches!(c, '\'' | '\u{2018}' | '\u{2019}')
}

#[inline]
fn is_word_char(c: char) -> bool {
    c.is_alphabetic() || is_apos(c)
}

fn normalize(s: &str) -> String {
    s.chars().map(|c| if is_apos(c) { '\'' } else { c }).collect::<String>().to_lowercase()
}

pub fn check_para(text: &str) -> Vec<(usize, usize)> {
    let dict = match DICT.get() { Some(d) => d, None => return Vec::new() };
    let mut errors = Vec::new();
    let len = text.len();
    let mut i = 0;
    while i < len {
        let c = match text[i..].chars().next() { Some(c) => c, None => break };
        if c.is_alphabetic() {
            let start = i;
            i += c.len_utf8();
            while i < len {
                let ch = match text[i..].chars().next() { Some(c) => c, None => break };
                if is_word_char(ch) { i += ch.len_utf8(); } else { break; }
            }
            let word = &text[start..i];
            let ls = word.trim_start_matches(|c: char| is_apos(c));
            let trimmed = ls.trim_end_matches(|c: char| is_apos(c));
            if trimmed.chars().count() < 2 { continue; }
            let byte_start = start + (word.len() - ls.len());
            let byte_end = byte_start + trimmed.len();
            if !text.is_char_boundary(byte_start) || !text.is_char_boundary(byte_end) { continue; }
            let norm = normalize(trimmed);
            let clean = norm.trim_matches('\'');
            if !clean.is_empty() && !dict.contains(clean) {
                errors.push((byte_start, byte_end));
            }
        } else {
            i += c.len_utf8();
        }
    }
    errors
}

fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let (m, n) = (a.len(), b.len());
    let mut dp: Vec<usize> = (0..=n).collect();
    for i in 1..=m {
        let mut prev = dp[0];
        dp[0] = i;
        for j in 1..=n {
            let tmp = dp[j];
            dp[j] = if a[i - 1] == b[j - 1] { prev } else { 1 + prev.min(dp[j]).min(dp[j - 1]) };
            prev = tmp;
        }
    }
    dp[n]
}

pub fn suggestions(word: &str, max: usize) -> Vec<String> {
    let dict = match DICT.get() { Some(d) => d, None => return Vec::new() };
    let norm = normalize(word);
    let clean: String = norm.chars().filter(|c| c.is_alphabetic()).collect();
    if clean.len() < 2 { return Vec::new(); }
    let wlen = clean.len();
    let max_dist = if wlen <= 4 { 1 } else if wlen <= 8 { 2 } else { 3 };
    let cb = clean.as_bytes();
    let priority: Vec<String> = (1..wlen)
        .filter_map(|i| {
            let s = format!("{}\'{}", &clean[..i], &clean[i..]);
            if dict.contains(s.as_str()) { Some(s) } else { None }
        })
        .collect();
    let mut candidates: Vec<(usize, String)> = dict.iter()
        .filter(|w| w.len().abs_diff(wlen) <= max_dist)
        .filter_map(|w| {
            let d = levenshtein(cb, w.as_bytes());
            if d > 0 && d <= max_dist { Some((d, w.clone())) } else { None }
        })
        .collect();
    candidates.sort_unstable_by_key(|(d, _)| *d);
    let mut result = priority.clone();
    for (_, w) in candidates {
        if result.len() >= max { break; }
        if !priority.contains(&w) { result.push(w); }
    }
    result.truncate(max);
    result
}
