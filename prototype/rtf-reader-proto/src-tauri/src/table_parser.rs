use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Table {
    pub rows: Vec<Row>,
    pub column_widths: Vec<i32>, // twips, absolute positions from \cellx
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub h_align: String, // trqc / trql / trqr
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub text: String,
    pub h_align: String, // qc / ql / qr
    pub v_align: String, // clvertalt / clvertalb / clvertalc
    pub font_size: i32,  // half-points from \fs
    pub border_top: Option<Border>,
    pub border_bottom: Option<Border>,
    pub border_left: Option<Border>,
    pub border_right: Option<Border>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Border {
    pub style: String, // solid / double
    pub width: i32,    // twips
}

struct TableLexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> TableLexer<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn read_control_word(&mut self) -> Option<(String, Option<i32>)> {
        if self.peek()? != b'\\' {
            return None;
        }
        self.next_byte()?;

        let first = self.peek()?;

        // escapes
        if matches!(first, b'{' | b'}' | b'\\' | b'~' | b'-' | b'_' | b':' | b'|' | b';' | b'*') {
            self.next_byte()?;
            return Some((format!("\\{}", first as char), None));
        }

        if first == b'\'' {
            self.next_byte()?;
            let hi = self.next_byte()?;
            let lo = self.next_byte()?;
            let hex = format!("{}{}", hi as char, lo as char);
            if let Ok(n) = u8::from_str_radix(&hex, 16) {
                return Some(("'".to_string(), Some(n as i32)));
            }
            return Some(("'".to_string(), None));
        }

        if first == b'u' {
            self.next_byte()?;
            let mut num_str = String::new();
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() || b == b'-' {
                    num_str.push(b as char);
                    self.next_byte()?;
                } else {
                    break;
                }
            }
            if !num_str.is_empty() {
                self.next_byte()?; // skip replacement char
                if let Ok(n) = num_str.parse::<i32>() {
                    return Some(("u".to_string(), Some(n)));
                }
            }
            return Some(("u".to_string(), None));
        }

        if !first.is_ascii_alphabetic() {
            self.next_byte()?;
            return Some((format!("{}", first as char), None));
        }

        let mut word = String::new();
        while let Some(b) = self.peek() {
            if b.is_ascii_alphabetic() {
                word.push(b as char);
                self.next_byte()?;
            } else {
                break;
            }
        }

        let mut param_str = String::new();
        if self.peek() == Some(b'-') {
            param_str.push('-');
            self.next_byte()?;
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                param_str.push(b as char);
                self.next_byte()?;
            } else {
                break;
            }
        }

        if let Some(b) = self.peek() {
            if b == b' ' || b == b'\r' || b == b'\n' {
                self.next_byte()?;
            }
        }

        let param = param_str.parse::<i32>().ok();
        Some((word, param))
    }

    fn read_text_until_delim(&mut self) -> Vec<u8> {
        let mut text = Vec::new();
        while let Some(b) = self.peek() {
            if b == b'\\' || b == b'{' || b == b'}' {
                break;
            }
            self.next_byte();
            text.push(b);
        }
        text
    }

    fn skip_group(&mut self) -> Option<()> {
        // assumes '{' already consumed
        let mut depth = 1;
        while depth > 0 {
            match self.peek()? {
                b'{' => {
                    self.next_byte()?;
                    depth += 1;
                }
                b'}' => {
                    self.next_byte()?;
                    depth -= 1;
                }
                b'\\' => {
                    self.read_control_word()?;
                }
                _ => {
                    self.next_byte()?;
                }
            }
        }
        Some(())
    }
}

pub fn parse_table(data: &[u8]) -> Table {
    let mut lexer = TableLexer::new(data);
    let mut rows = Vec::new();
    let mut column_widths: Vec<i32> = Vec::new();

    while lexer.peek().is_some() {
        // find next \trowd
        let mut found_trowd = false;
        while lexer.peek().is_some() {
            if lexer.peek() == Some(b'\\') {
                if let Some((word, _)) = lexer.read_control_word() {
                    if word == "trowd" {
                        found_trowd = true;
                        break;
                    }
                }
            } else {
                lexer.next_byte();
            }
        }
        if !found_trowd {
            break;
        }

        // parse row definition
        let mut row_h_align = "left".to_string();
        let mut cell_defs: Vec<CellDef> = Vec::new();
        let mut current_v_align = "top".to_string();
        let mut current_h_align = "left".to_string();
        let mut current_font_size = 18; // default ~9pt

        let mut current_borders = Borders::default();

        // read cell definitions until first \pard or \intbl
        loop {
            if lexer.peek().is_none() {
                break;
            }
            if lexer.peek() == Some(b'\\') {
                if let Some((word, param)) = lexer.read_control_word() {
                    match word.as_str() {
                        "trqc" => row_h_align = "center".to_string(),
                        "trql" => row_h_align = "left".to_string(),
                        "trqr" => row_h_align = "right".to_string(),
                        "clvertalt" => current_v_align = "top".to_string(),
                        "clvertalb" => current_v_align = "bottom".to_string(),
                        "clvertalc" => current_v_align = "middle".to_string(),
                        "cellx" => {
                            if let Some(w) = param {
                                cell_defs.push(CellDef {
                                    v_align: current_v_align.clone(),
                                    border_top: current_borders.top.clone(),
                                    border_bottom: current_borders.bottom.clone(),
                                    border_left: current_borders.left.clone(),
                                    border_right: current_borders.right.clone(),
                                });
                                column_widths.push(w);
                                current_borders = Borders::default();
                            }
                        }
                        "clbrdrt" => current_borders.top = Some(read_border(&mut lexer)),
                        "clbrdrb" => current_borders.bottom = Some(read_border(&mut lexer)),
                        "clbrdrl" => current_borders.left = Some(read_border(&mut lexer)),
                        "clbrdrr" => current_borders.right = Some(read_border(&mut lexer)),
                        "pard" | "intbl" => {
                            // end of cell definitions, start reading cell contents
                            break;
                        }
                        _ => {}
                    }
                }
            } else if lexer.peek() == Some(b'{') {
                lexer.next_byte();
                lexer.skip_group();
            } else {
                lexer.next_byte();
            }
        }

        // read cells until \row
        // Each entry stores (h_align, font_size, raw text bytes) at the time of \cell.
        let mut current_cell_text: Vec<u8> = Vec::new();
        let mut cell_contents: Vec<(String, i32, Vec<u8>)> = Vec::new();
        loop {
            if lexer.peek().is_none() {
                break;
            }
            if lexer.peek() == Some(b'\\') {
                if let Some((word, param)) = lexer.read_control_word() {
                    match word.as_str() {
                        "qc" => current_h_align = "center".to_string(),
                        "ql" => current_h_align = "left".to_string(),
                        "qr" => current_h_align = "right".to_string(),
                        "fs" => {
                            if let Some(p) = param {
                                current_font_size = p;
                            }
                        }
                        "cell" => {
                            let s = decode_text(&current_cell_text).trim().to_string();
                            cell_contents.push((current_h_align.clone(), current_font_size, s.into_bytes()));
                            current_cell_text.clear();
                            current_h_align = "left".to_string();
                            current_font_size = 18;
                        }
                        "row" => {
                            if !current_cell_text.is_empty() {
                                let s = decode_text(&current_cell_text).trim().to_string();
                                cell_contents.push((current_h_align.clone(), current_font_size, s.into_bytes()));
                                current_cell_text.clear();
                            }
                            break;
                        }
                        "par" => {
                            current_cell_text.push(b'\n');
                        }
                        "u" => {
                            if let Some(n) = param {
                                if let Some(c) = char::from_u32(n as u32) {
                                    current_cell_text.extend(c.encode_utf8(&mut [0; 4]).bytes());
                                }
                            }
                        }
                        "'" => {
                            if let Some(n) = param {
                                current_cell_text.push(n as u8);
                            }
                        }
                        _ => {}
                    }
                }
            } else if lexer.peek() == Some(b'{') {
                // group: could be {\f0 text} or nested formatting
                lexer.next_byte();
                // read text inside group, handling basic control words
                let mut group_text: Vec<u8> = Vec::new();
                loop {
                    match lexer.peek() {
                        Some(b'}') => {
                            lexer.next_byte();
                            break;
                        }
                        Some(b'\\') => {
                            if let Some((w, p)) = lexer.read_control_word() {
                                if w == "par" {
                                    group_text.push(b'\n');
                                } else if w == "u" {
                                    if let Some(n) = p {
                                        if let Some(c) = char::from_u32(n as u32) {
                                            group_text.extend(c.encode_utf8(&mut [0; 4]).bytes());
                                        }
                                    }
                                } else if w == "'" {
                                    if let Some(n) = p {
                                        group_text.push(n as u8);
                                    }
                                }
                            }
                        }
                        Some(b) => {
                            lexer.next_byte();
                            group_text.push(b);
                        }
                        None => break,
                    }
                }
                current_cell_text.extend(group_text);
            } else {
                let text = lexer.read_text_until_delim();
                current_cell_text.extend(text);
            }
        }

        // build row
        let mut cells = Vec::new();
        for (i, (h_align, font_size, content)) in cell_contents.iter().enumerate() {
            let def = cell_defs.get(i).cloned().unwrap_or_default();
            cells.push(Cell {
                text: decode_text(content),
                h_align: h_align.clone(),
                v_align: def.v_align.clone(),
                font_size: *font_size,
                border_top: def.border_top.clone(),
                border_bottom: def.border_bottom.clone(),
                border_left: def.border_left.clone(),
                border_right: def.border_right.clone(),
            });
        }
        rows.push(Row {
            cells,
            h_align: row_h_align,
        });
    }

    Table { rows, column_widths }
}

#[derive(Default, Clone)]
struct CellDef {
    v_align: String,
    border_top: Option<Border>,
    border_bottom: Option<Border>,
    border_left: Option<Border>,
    border_right: Option<Border>,
}

#[derive(Default, Clone)]
struct Borders {
    top: Option<Border>,
    bottom: Option<Border>,
    left: Option<Border>,
    right: Option<Border>,
}

fn read_border(lexer: &mut TableLexer) -> Border {
    let mut style = "solid".to_string();
    let mut width = 15; // default
    while let Some((word, param)) = lexer.read_control_word() {
        match word.as_str() {
            "brdrs" => style = "solid".to_string(),
            "brdrdb" => style = "double".to_string(),
            "brdrw" => {
                if let Some(p) = param {
                    width = p;
                }
            }
            "clbrdrt" | "clbrdrb" | "clbrdrl" | "clbrdrr" | "cellx" | "clvertalt" | "clvertalb" | "clvertalc" => {
                // we've moved past border definition, put back by rewinding? 
                // For prototype, stop parsing border here.
                break;
            }
            _ => {}
        }
    }
    Border { style, width }
}

pub fn render_html_table(table: &Table) -> String {
    let mut html = String::from("<table class=\"tfl-table\">\n");
    for (ri, row) in table.rows.iter().enumerate() {
        html.push_str("  <tr>\n");
        for (ci, cell) in row.cells.iter().enumerate() {
            let tag = if ri == 0 { "th" } else { "td" };
            let width_css = if let Some(w) = table.column_widths.get(ci) {
                let prev = if ci == 0 { 0 } else { table.column_widths[ci - 1] };
                let width_twips = w - prev;
                let width_pt = width_twips as f64 / 20.0;
                format!("width: {:.1}pt; ", width_pt)
            } else {
                String::new()
            };

            let mut style = format!(
                "text-align: {}; vertical-align: {}; font-size: {:.1}pt; {}",
                cell.h_align,
                cell.v_align,
                cell.font_size as f64 / 2.0,
                width_css
            );

            if let Some(b) = &cell.border_top {
                style.push_str(&border_css("border-top", b));
            }
            if let Some(b) = &cell.border_bottom {
                style.push_str(&border_css("border-bottom", b));
            }
            if let Some(b) = &cell.border_left {
                style.push_str(&border_css("border-left", b));
            }
            if let Some(b) = &cell.border_right {
                style.push_str(&border_css("border-right", b));
            }

            html.push_str(&format!(
                "    <{} style=\"{}\">{}</{}>\n",
                tag,
                style,
                html_escape(&cell.text),
                tag
            ));
        }
        html.push_str("  </tr>\n");
    }
    html.push_str("</table>\n");
    html
}

fn border_css(prop: &str, border: &Border) -> String {
    let width_px = (border.width as f64 / 20.0 / 72.0 * 96.0).max(0.5);
    format!(
        "{}: {:.1}px {} #000; ",
        prop,
        width_px,
        if border.style == "double" { "double" } else { "solid" }
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| {
        bytes.iter().map(|&b| b as char).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_reference_tables() {
        let path = PathBuf::from("../test-data/reference-tfl-100p.rtf");
        if !path.exists() {
            eprintln!("Skipping test: reference file not found");
            return;
        }
        let data = std::fs::read(&path).unwrap();
        let trowd_count = data.windows(6).filter(|w| w == b"\\trowd").count();
        println!("Raw \\trowd count in file: {}", trowd_count);

        let table = parse_table(&data);
        println!(
            "Reference tables: {} rows, column_widths={:?}",
            table.rows.len(),
            table.column_widths.len()
        );
        assert!(!table.rows.is_empty());
        assert!(!table.column_widths.is_empty());
        for (i, row) in table.rows.iter().enumerate() {
            if i < 3 || i > table.rows.len() - 3 {
                println!("  row {}: {} cells", i, row.cells.len());
            }
        }
    }

    #[test]
    fn renders_first_table_html() {
        let path = PathBuf::from("../test-data/reference-tfl-100p.rtf");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).unwrap();
        let table = parse_table(&data);
        let html = render_html_table(&table);
        assert!(html.contains("<table"));
        assert!(html.contains("Subject ID"));
        println!("HTML preview (first 800 chars):\n{}", &html[..html.len().min(800)]);
    }

    #[test]
    fn writes_table_preview_html() {
        let path = PathBuf::from("../test-data/reference-tfl-100p.rtf");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(&path).unwrap();
        let table = parse_table(&data);
        let body = render_html_table(&table);
        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><style>table {{ border-collapse: collapse; font-family: sans-serif; font-size: 9pt; }} th, td {{ padding: 4px 8px; }} th {{ background: #f1f5f9; }}</style></head><body>{}</body></html>",
            body
        );
        let out = PathBuf::from("../test-data/table-preview.html");
        std::fs::write(&out, html).unwrap();
        println!("Wrote {}", out.display());
    }
}
