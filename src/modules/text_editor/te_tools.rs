use super::te_main::TextEditor;

impl TextEditor {
    pub(super) fn char_index_to_byte_index(&self, char_index: usize) -> usize {
        self.content.char_indices()
            .nth(char_index)
            .map(|(b, _)| b)
            .unwrap_or(self.content.len())
    }

    pub(super) fn insert_wrapper_at_cursor(&mut self, wrapper: &str) {
        if let Some(range) = self.last_cursor_range {
            let cursor_pos: usize = self.char_index_to_byte_index(range.primary.index);
            self.content.insert_str(cursor_pos, &format!("{}{}", wrapper, wrapper));
            self.dirty = true;
            self.pending_cursor_pos = Some(range.primary.index + wrapper.chars().count());
        }
    }

    pub(super) fn wrap_selection(&mut self, wrapper: &str) {
        if let Some(range) = self.last_cursor_range {
            let start_char: usize = range.primary.index.min(range.secondary.index);
            let end_char: usize = range.primary.index.max(range.secondary.index);

            if start_char == end_char {
                self.insert_wrapper_at_cursor(wrapper);
                return;
            }

            let start_byte: usize = self.char_index_to_byte_index(start_char);
            let end_byte: usize = self.char_index_to_byte_index(end_char);
            let selected: String = self.content[start_byte..end_byte].to_string();

            let wlen: usize = wrapper.chars().count();
            let prefix_start_char: usize = start_char.saturating_sub(wlen);
            let prefix_start_byte: usize = self.char_index_to_byte_index(prefix_start_char);
            let suffix_end_char: usize = end_char + wlen;
            let suffix_end_byte: usize = if suffix_end_char >= self.content.chars().count() {
                self.content.len()
            } else {
                self.char_index_to_byte_index(suffix_end_char)
            };

            let has_prefix: bool = start_char >= wlen && &self.content[prefix_start_byte..start_byte] == wrapper;
            let has_suffix: bool = suffix_end_byte <= self.content.len() && &self.content[end_byte..suffix_end_byte] == wrapper;

            if has_prefix && has_suffix {
                self.content.replace_range(end_byte..suffix_end_byte, "");
                self.content.replace_range(prefix_start_byte..start_byte, "");
                self.pending_cursor_pos = Some(start_char + selected.chars().count());
            } else {
                let wrapped: String = format!("{}{}{}", wrapper, selected, wrapper);
                self.content.replace_range(start_byte..end_byte, &wrapped);
                self.pending_cursor_pos = Some(start_char + selected.chars().count() + wlen * 2);
            }

            self.dirty = true;
        }
    }

    pub(super) fn format_bold(&mut self) { self.wrap_selection("**"); }
    pub(super) fn format_italic(&mut self) { self.wrap_selection("*"); }
    pub(super) fn format_underline(&mut self) { self.wrap_selection("__"); }
    pub(super) fn format_strikethrough(&mut self) { self.wrap_selection("~~"); }
    pub(super) fn format_code(&mut self) { self.wrap_selection("`"); }

    pub(super) fn format_heading(&mut self, level: usize) {
        if let Some(range) = self.last_cursor_range {
            let byte_idx: usize = self.char_index_to_byte_index(range.primary.index);
            let start_byte: usize = self.content[..byte_idx].rfind('\n').map(|i: usize| i + 1).unwrap_or(0);
            let end_byte: usize = self.content[byte_idx..].find('\n').map(|i: usize| byte_idx + i).unwrap_or(self.content.len());
            let line: &str = &self.content[start_byte..end_byte];
            let content_start: usize = line.find(|c: char| c != '#' && !c.is_whitespace()).unwrap_or(line.len());
            let clean: &str = &line[content_start..];
            let new_line: String = if level > 0 { format!("{} {}", "#".repeat(level), clean) } else { clean.to_string() };
            self.content.replace_range(start_byte..end_byte, &new_line);
            self.dirty = true;
        }
    }

    pub(super) fn count_words(&self) -> usize {
        self.content.split_whitespace().filter(|w: &&str| !w.is_empty()).count()
    }

    pub(super) fn count_visible_chars(&self) -> usize {
        use super::te_main::ViewMode;
        if self.view_mode != ViewMode::Markdown {
            return self.content.chars().count();
        }

        let mut count: usize = 0;
        let chars: Vec<char> = self.content.chars().collect();
        let mut i: usize = 0;
        let mut in_code_block: bool = false;

        while i < chars.len() {
            let line_start: usize = i;
            let mut line_end: usize = i;
            while line_end < chars.len() && chars[line_end] != '\n' { line_end += 1; }
            let line: String = chars[line_start..line_end].iter().collect();

            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                count += line.chars().count();
                if line_end < chars.len() { count += 1; }
                i = line_end + 1;
                continue;
            }

            if in_code_block {
                count += line.chars().count();
                if line_end < chars.len() { count += 1; }
                i = line_end + 1;
                continue;
            }

            for prefix in &["#### ", "### ", "## ", "# "] {
                if let Some(rest) = line.strip_prefix(prefix) {
                    count += rest.chars().count();
                    if line_end < chars.len() { count += 1; }
                    i = line_end + 1;
                    break;
                }
            }
            if i == line_end + 1 || i > line_end { continue; }

            let line_chars: Vec<char> = line.chars().collect();
            let mut j: usize = 0;

            if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
                count += 1;
                j = 2;
            }

            let mut k: usize = 0;
            while k < line_chars.len() && line_chars[k].is_ascii_digit() { k += 1; }
            if k > 0 && k < line_chars.len() && line_chars[k] == '.' && k + 1 < line_chars.len() && line_chars[k + 1] == ' ' {
                count += k + 2;
                j = k + 2;
            }

            while j < line_chars.len() {
                if j + 1 < line_chars.len() && line_chars[j] == '~' && line_chars[j + 1] == '~' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "~~") {
                        if end > j + 2 { count += end - (j + 2); j = end + 2; continue; }
                    }
                }
                if line_chars[j] == '~' && j + 1 < line_chars.len() && !line_chars[j + 1].is_whitespace() && line_chars[j + 1] != '~' {
                    let mut end: usize = j + 1;
                    while end < line_chars.len() && line_chars[end] != '~' {
                        if line_chars[end].is_whitespace() || line_chars[end].is_ascii_punctuation() { break; }
                        end += 1;
                    }
                    if end > j + 1 { count += end - (j + 1); j = end; continue; }
                }
                if j + 1 < line_chars.len() && line_chars[j] == '*' && line_chars[j + 1] == '*' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "**") {
                        if end > j + 2 { count += end - (j + 2); j = end + 2; continue; }
                    }
                }
                if line_chars[j] == '*' && !(j + 1 < line_chars.len() && line_chars[j + 1] == '*') {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 1, "*") {
                        if end > j + 1 { count += end - (j + 1); j = end + 1; continue; }
                    }
                }
                if j + 1 < line_chars.len() && line_chars[j] == '_' && line_chars[j + 1] == '_' {
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 2, "__") {
                        if end > j + 2 { count += end - (j + 2); j = end + 2; continue; }
                    }
                }
                if line_chars[j] == '`' {
                    if j + 2 < line_chars.len() && line_chars[j + 1] == '`' && line_chars[j + 2] == '`' {
                        count += 1; j += 1; continue;
                    }
                    if let Some(end) = Self::find_closing_marker(&line_chars, j + 1, "`") {
                        if end > j + 1 { count += end - (j + 1); j = end + 1; continue; }
                    }
                }
                if line_chars[j] == '^' && j + 1 < line_chars.len() && !line_chars[j + 1].is_whitespace() {
                    let mut end: usize = j + 1;
                    while end < line_chars.len() && line_chars[end] != '^' {
                        if line_chars[end].is_whitespace() || line_chars[end].is_ascii_punctuation() { break; }
                        end += 1;
                    }
                    if end > j + 1 { count += end - (j + 1); j = end; continue; }
                }
                if line_chars[j] == '[' {
                    if let Some(text_end) = Self::find_closing_bracket(&line_chars, j + 1) {
                        if text_end + 1 < line_chars.len() && line_chars[text_end + 1] == '(' {
                            if let Some(url_end) = Self::find_closing_paren(&line_chars, text_end + 2) {
                                count += text_end - (j + 1); j = url_end + 1; continue;
                            }
                        }
                    }
                }
                count += 1;
                j += 1;
            }

            if line_end < chars.len() { count += 1; }
            i = line_end + 1;
        }

        count
    }

    pub(super) fn find_link_at_offset(chars: &[char], cursor_idx: usize) -> Option<String> {
        let search_start: usize = cursor_idx.saturating_sub(1000);
        let mut start_bracket: Option<usize> = None;
        for i in (search_start..=cursor_idx).rev() {
            if i < chars.len() && chars[i] == '[' { start_bracket = Some(i); break; }
        }
        if let Some(start) = start_bracket {
            if let Some(text_end) = Self::find_closing_bracket(chars, start + 1) {
                if text_end + 1 < chars.len() && chars[text_end + 1] == '(' {
                    if let Some(url_end) = Self::find_closing_paren(chars, text_end + 2) {
                        let end: usize = url_end + 1;
                        if cursor_idx >= start && cursor_idx <= end {
                            return Some(chars[text_end + 2..url_end].iter().collect());
                        }
                    }
                }
            }
        }
        None
    }

    pub(super) fn find_closing_marker(chars: &[char], start: usize, marker: &str) -> Option<usize> {
        let marker_chars: Vec<char> = marker.chars().collect();
        let mlen: usize = marker_chars.len();
        let mut i: usize = start;
        while i + mlen <= chars.len() {
            if chars[i..i + mlen] == marker_chars[..] { return Some(i); }
            i += 1;
        }
        None
    }

    pub(super) fn find_closing_bracket(chars: &[char], start: usize) -> Option<usize> {
        chars[start..].iter().position(|&c| c == ']').map(|i: usize| start + i)
    }

    pub(super) fn find_closing_paren(chars: &[char], start: usize) -> Option<usize> {
        chars[start..].iter().position(|&c| c == ')').map(|i: usize| start + i)
    }
}
