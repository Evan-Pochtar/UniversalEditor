use super::te_main::TextEditor;

impl TextEditor {
    pub(super) fn insert_table(&mut self, rows: usize, cols: usize) {
        let header: String = (0..cols).map(|i| format!("Header {}", i + 1)).collect::<Vec<_>>().join(" | ");
        let sep: String = (0..cols).map(|_| "---").collect::<Vec<_>>().join(" | ");
        let data_row: String = (0..cols).map(|_| "Cell").collect::<Vec<_>>().join(" | ");
        let data_rows: String = (0..rows).map(|_| format!("| {} |", data_row)).collect::<Vec<_>>().join("\n");
        let table: String = format!("| {} |\n| {} |\n{}\n", header, sep, data_rows);
        let byte_idx: usize = self.last_cursor_range
            .map(|r| self.char_index_to_byte_index(r.primary.index))
            .unwrap_or(self.content.len());
        let needs_newline: bool = byte_idx > 0 && !self.content[..byte_idx].ends_with('\n');
        let insert: String = if needs_newline { format!("\n{}", table) } else { table };
        self.content.insert_str(byte_idx, &insert);
        self.dirty = true;
        self.content_version = self.content_version.wrapping_add(1);
    }

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

    pub(super) fn is_horizontal_rule(line: &str) -> bool {
        let trimmed: &str = line.trim();
        if trimmed.len() < 3 { return false; }
        let first: char = match trimmed.chars().next() {
            Some(c) if matches!(c, '-' | '*' | '_') => c,
            _ => return false,
        };
        let count = trimmed.chars().filter(|&c| c == first).count();
        count >= 3 && trimmed.chars().all(|c| c == first || c == ' ')
    }

    pub(super) fn format_blockquote(&mut self) {
        if let Some(range) = self.last_cursor_range {
            let byte_idx: usize = self.char_index_to_byte_index(range.primary.index);
            let start_byte: usize = self.content[..byte_idx].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let end_byte: usize = self.content[byte_idx..].find('\n').map(|i| byte_idx + i).unwrap_or(self.content.len());
            let line: &str = &self.content[start_byte..end_byte];
            let new_line: String = if line.starts_with("> ") {
                line[2..].to_string()
            } else {
                format!("> {}", line)
            };
            self.content.replace_range(start_byte..end_byte, &new_line);
            self.dirty = true;
        }
    }

    pub(super) fn insert_checklist_item(&mut self) {
        if let Some(range) = self.last_cursor_range {
            let byte_idx: usize = self.char_index_to_byte_index(range.primary.index);
            let start_byte: usize = self.content[..byte_idx].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let end_byte: usize = self.content[byte_idx..].find('\n').map(|i| byte_idx + i).unwrap_or(self.content.len());
            let line: &str = &self.content[start_byte..end_byte];
            let new_line: String = if line.starts_with("- [ ] ") || line.starts_with("- [x] ") || line.starts_with("- [X] ") {
                line[6..].to_string()
            } else if line.starts_with("- ") {
                format!("- [ ] {}", &line[2..])
            } else {
                format!("- [ ] {}", line)
            };
            self.content.replace_range(start_byte..end_byte, &new_line);
            self.dirty = true;
        }
    }

    pub(super) fn try_toggle_checkbox(&mut self) {
        if let Some(range) = self.last_cursor_range {
            let cursor_char: usize = range.primary.index;
            let content_chars: Vec<char> = self.content.chars().collect();
            let safe_cursor: usize = cursor_char.min(content_chars.len());

            let line_start_char: usize = content_chars[..safe_cursor]
                .iter().rposition(|&c| c == '\n')
                .map(|i| i + 1)
                .unwrap_or(0);

            let line_start_byte: usize = self.char_index_to_byte_index(line_start_char);
            let line_end_byte: usize = {
                let after = &self.content[line_start_byte..];
                after.find('\n').map(|i| line_start_byte + i).unwrap_or(self.content.len())
            };
            let line: String = self.content[line_start_byte..line_end_byte].to_string();

            let cursor_offset_in_line: usize = safe_cursor.saturating_sub(line_start_char);
            if cursor_offset_in_line > 5 { return; }
            for prefix in &["- [ ] ", "* [ ] ", "+ [ ] "] {
                if line.starts_with(prefix) {
                    let list_char: char = line.chars().next().unwrap();
                    let checked_prefix: String = format!("{} [x] ", list_char);
                    let end: usize = line_start_byte + prefix.len();
                    self.content.replace_range(line_start_byte..end, &checked_prefix);
                    self.dirty = true;
                    self.content_version = self.content_version.wrapping_add(1);
                    return;
                }
            }
            for prefix in &["- [x] ", "- [X] ", "* [x] ", "* [X] ", "+ [x] ", "+ [X] "] {
                if line.starts_with(prefix) {
                    let list_char: char = line.chars().next().unwrap();
                    let unchecked_prefix: String = format!("{} [ ] ", list_char);
                    let end: usize = line_start_byte + prefix.len();
                    self.content.replace_range(line_start_byte..end, &unchecked_prefix);
                    self.dirty = true;
                    self.content_version = self.content_version.wrapping_add(1);
                    return;
                }
            }
        }
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

            if Self::is_horizontal_rule(&line) {
                if line_end < chars.len() { count += 1; }
                i = line_end + 1;
                continue;
            }

            let effective_line: String = if line.starts_with("> ") {
                line[2..].to_string()
            } else {
                line.clone()
            };

            for prefix in &["#### ", "### ", "## ", "# "] {
                if let Some(rest) = effective_line.strip_prefix(prefix) {
                    count += rest.chars().count();
                    if line_end < chars.len() { count += 1; }
                    i = line_end + 1;
                    break;
                }
            }
            if i == line_end + 1 || i > line_end { continue; }

            let line_chars: Vec<char> = effective_line.chars().collect();
            let mut j: usize = 0;
            let indent_count: usize = line_chars.iter().take_while(|&&c| c == ' ').count();
            let check_line: &str = &effective_line[indent_count..];

            let checkbox_variants: &[(&str, bool)] = &[
                ("- [ ] ", false), ("- [x] ", true), ("- [X] ", true),
                ("* [ ] ", false), ("* [x] ", true), ("* [X] ", true),
                ("+ [ ] ", false), ("+ [x] ", true), ("+ [X] ", true),
            ];
            let mut is_checkbox = false;
            for (prefix, _) in checkbox_variants {
                if check_line.starts_with(prefix) {
                    count += 4;
                    j = indent_count + prefix.chars().count();
                    is_checkbox = true;
                    break;
                }
            }

            if !is_checkbox {
                if check_line.starts_with("- ") || check_line.starts_with("* ") || check_line.starts_with("+ ") {
                    count += 1;
                    j = indent_count + 2;
                } else {
                    j = 0;
                    let mut k: usize = 0;
                    while k < line_chars.len() && line_chars[k].is_ascii_digit() { k += 1; }
                    if k > 0 && k < line_chars.len() && line_chars[k] == '.' && k + 1 < line_chars.len() && line_chars[k + 1] == ' ' {
                        count += k + 2;
                        j = k + 2;
                    }
                }
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

    pub(super) fn open_file_location(&self) {
        if let Some(path) = &self.file_path {
            let dir = path.parent().unwrap_or(path.as_path());
            #[cfg(target_os = "windows")]
            { let _ = std::process::Command::new("explorer").arg(dir).spawn(); }
            #[cfg(target_os = "macos")]
            { let _ = std::process::Command::new("open").arg(dir).spawn(); }
            #[cfg(target_os = "linux")]
            { let _ = std::process::Command::new("xdg-open").arg(dir).spawn(); }
        }
    }

    pub(super) fn apply_rename(&mut self) {
        if let Some(old_path) = self.file_path.take() {
            let stem = self.rename_buffer.trim().to_string();
            if stem.is_empty() { self.file_path = Some(old_path); return; }
            let ext = self.rename_ext.as_deref().unwrap_or("txt");
            let new_name = format!("{}.{}", stem, ext);
            let new_path = old_path.with_file_name(&new_name);
            if std::fs::rename(&old_path, &new_path).is_ok() {
                if let Some(tx) = &self.path_replace_tx {
                    let _ = tx.send((old_path, new_path.clone()));
                }
                self.file_path = Some(new_path.clone());
                self.view_mode = Self::detect_view_mode(&new_path);
            } else {
                self.file_path = Some(old_path);
            }
        }
    }

    pub(super) fn convert_file_extension(&mut self) {
        if let Some(path) = self.file_path.take() {
            let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());
            let new_ext = match ext.as_deref() {
                Some("md") | Some("markdown") => "txt",
                Some("txt") => "md",
                _ => { self.file_path = Some(path); return; }
            };
            let new_path = path.with_extension(new_ext);
            if std::fs::rename(&path, &new_path).is_ok() {
                if let Some(tx) = &self.path_replace_tx {
                    let _ = tx.send((path, new_path.clone()));
                }
                self.file_path = Some(new_path.clone());
                self.view_mode = Self::detect_view_mode(&new_path);
            } else {
                self.file_path = Some(path);
            }
        }
    }
}
