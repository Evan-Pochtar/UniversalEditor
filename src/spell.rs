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

pub fn is_correct(word: &str) -> bool {
    let dict = match DICT.get() { Some(d) => d, None => return true };
    if word.is_empty() { return true; }
    let lower = word.to_lowercase();
    let clean: String = lower.trim_matches(|c: char| !c.is_alphabetic()).chars()
        .filter(|c| c.is_alphabetic() || *c == '\'').collect();
    if clean.is_empty() { return true; }
    dict.contains(&clean)
        || dict.contains(clean.trim_matches('\''))
}

pub fn check_para(text: &str) -> Vec<(usize, usize)> {
    if DICT.get().is_none() { return Vec::new(); }
    let mut errors = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if text[i..].chars().next().map(|c| c.is_alphabetic() || c == '\'').unwrap_or(false) {
            let start = i;
            while i < len {
                let c = text[i..].chars().next().unwrap_or('\0');
                if c.is_alphabetic() || c == '\'' { i += c.len_utf8(); } else { break; }
            }
            let word = &text[start..i];
            let clean = word.trim_matches('\'');
            if clean.len() > 1 && !is_correct(clean) {
                let byte_start = start + (word.len() - word.trim_start_matches('\'').len());
                let byte_end = byte_start + clean.len();
                errors.push((byte_start, byte_end));
            }
        } else {
            let c = text[i..].chars().next().unwrap_or('\0');
            i += c.len_utf8().max(1);
        }
    }
    errors
}
