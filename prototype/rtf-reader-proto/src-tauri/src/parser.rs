use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
pub enum BlockType {
    Table,
    Text,
    Listing,
    Figure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ImageFormat {
    Emf,
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub block_type: BlockType,
    pub start_byte: usize,
    pub end_byte: usize,
    pub estimated_lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Image {
    pub format: ImageFormat,
    pub data: Vec<u8>,
    pub byte_offset: usize,
}

#[derive(Debug, Serialize)]
pub struct ParseResult {
    pub file_size: u64,
    pub parse_nanos: u128,
    pub peak_memory_kb: u64,
    pub block_count: usize,
    pub blocks: Vec<Block>,
    pub images: Vec<Image>,
    pub skipped_control_words: Vec<String>,
    pub page_count_estimate: usize,
}

struct Lexer<R: Read> {
    reader: BufReader<R>,
    buf: Vec<u8>,
    pos: usize,
    group_depth: usize,
    current_block_type: Option<BlockType>,
    current_block_start: usize,
    current_block_lines: usize,
    blocks: Vec<Block>,
    images: Vec<Image>,
    skipped: HashSet<String>,
    skipped_list: Vec<String>,
    skip_remaining_in_group: usize,
    total_bytes: usize,
    page_count: usize,
    // picture extraction state
    in_pict: bool,
    pict_format: Option<ImageFormat>,
}

impl<R: Read> Lexer<R> {
    fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buf: Vec::with_capacity(64 * 1024),
            pos: 0,
            group_depth: 0,
            current_block_type: None,
            current_block_start: 0,
            current_block_lines: 0,
            blocks: Vec::with_capacity(256),
            images: Vec::with_capacity(64),
            skipped: HashSet::new(),
            skipped_list: Vec::new(),
            skip_remaining_in_group: 0,
            total_bytes: 0,
            page_count: 0,
            in_pict: false,
            pict_format: None,
        }
    }

    fn fill_buffer(&mut self) -> std::io::Result<bool> {
        if self.pos >= self.buf.len() {
            self.buf.clear();
            let n = self.reader.read_to_end(&mut self.buf)?;
            self.pos = 0;
            Ok(n > 0)
        } else {
            Ok(true)
        }
    }

    fn peek(&mut self) -> std::io::Result<Option<u8>> {
        loop {
            if self.pos < self.buf.len() {
                return Ok(Some(self.buf[self.pos]));
            }
            if !self.fill_buffer()? {
                return Ok(None);
            }
        }
    }

    fn next_byte(&mut self) -> std::io::Result<Option<u8>> {
        let b = self.peek()?;
        if b.is_some() {
            self.pos += 1;
            self.total_bytes += 1;
        }
        Ok(b)
    }

    fn read_raw_bytes(&mut self, n: usize) -> std::io::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(n);
        while result.len() < n {
            if self.pos >= self.buf.len() && !self.fill_buffer()? {
                break;
            }
            let take = (n - result.len()).min(self.buf.len() - self.pos);
            result.extend_from_slice(&self.buf[self.pos..self.pos + take]);
            self.pos += take;
            self.total_bytes += take;
        }
        Ok(result)
    }

    fn flush_block(&mut self, end_byte: usize) {
        if let Some(block_type) = self.current_block_type.take() {
            let start = self.current_block_start;
            if end_byte > start {
                self.blocks.push(Block {
                    block_type,
                    start_byte: start,
                    end_byte,
                    estimated_lines: self.current_block_lines.max(1),
                });
            }
            self.current_block_lines = 0;
        }
    }

    fn start_block(&mut self, block_type: BlockType, at_byte: usize) {
        if self.current_block_type != Some(block_type) {
            self.flush_block(at_byte);
            self.current_block_type = Some(block_type);
            self.current_block_start = at_byte;
        }
    }

    fn record_skip(&mut self, word: String) {
        if self.skipped.insert(word.clone()) && self.skipped_list.len() < 20 {
            self.skipped_list.push(word);
        }
    }

    fn read_control_word(&mut self) -> std::io::Result<Option<(String, Option<i32>)>> {
        // Assumes backslash already consumed.
        let first = match self.peek()? {
            Some(b) => b,
            None => return Ok(None),
        };

        // Escapes
        if matches!(first, b'{' | b'}' | b'\\' | b'~' | b'-' | b'_' | b':' | b'|' | b';' | b'*') {
            self.next_byte()?;
            return Ok(Some((format!("\\{}", first as char), None)));
        }

        if first == b'\'' {
            self.next_byte()?;
            let hi = match self.next_byte()? {
                Some(b) => b,
                None => return Ok(None),
            };
            let lo = match self.next_byte()? {
                Some(b) => b,
                None => return Ok(None),
            };
            let hex = format!("{}{}", hi as char, lo as char);
            return Ok(Some((format!("'{}'", hex), None)));
        }

        if first == b'u' {
            self.next_byte()?;
            let mut num_str = String::new();
            loop {
                match self.peek()? {
                    Some(b) if b.is_ascii_digit() || b == b'-' => {
                        num_str.push(b as char);
                        self.next_byte()?;
                    }
                    _ => break,
                }
            }
            if !num_str.is_empty() {
                self.next_byte()?; // skip replacement char
                if let Ok(n) = num_str.parse::<i32>() {
                    return Ok(Some(("u".to_string(), Some(n))));
                }
            }
            return Ok(Some(("u".to_string(), None)));
        }

        if !first.is_ascii_alphabetic() {
            self.next_byte()?;
            return Ok(Some((format!("{}", first as char), None)));
        }

        let mut word = String::new();
        while let Some(b) = self.peek()? {
            if b.is_ascii_alphabetic() {
                word.push(b as char);
                self.next_byte()?;
            } else {
                break;
            }
        }

        let mut param_str = String::new();
        if let Some(b) = self.peek()? {
            if b == b'-' {
                param_str.push(b as char);
                self.next_byte()?;
            }
        }
        while let Some(b) = self.peek()? {
            if b.is_ascii_digit() {
                param_str.push(b as char);
                self.next_byte()?;
            } else {
                break;
            }
        }

        if let Some(b) = self.peek()? {
            if b == b' ' || b == b'\r' || b == b'\n' {
                self.next_byte()?;
            }
        }

        let param = if param_str.is_empty() {
            None
        } else {
            param_str.parse::<i32>().ok()
        };

        Ok(Some((word, param)))
    }

    fn skip_unknown_destination(&mut self) -> std::io::Result<()> {
        self.skip_remaining_in_group = self.group_depth;
        Ok(())
    }

    fn run(&mut self) -> std::io::Result<()> {
        let mut in_table_row = false;

        loop {
            let b = match self.peek()? {
                Some(b) => b,
                None => break,
            };

            let current_byte = self.total_bytes;

            if self.skip_remaining_in_group > 0 {
                if self.in_pict && self.skip_remaining_in_group == self.group_depth {
                    // we are skipping an unknown pict-related destination; stop skip
                    self.skip_remaining_in_group = 0;
                    self.in_pict = false;
                }
                if self.skip_remaining_in_group > 0 {
                    if b == b'{' {
                        self.next_byte()?;
                        self.group_depth += 1;
                    } else if b == b'}' {
                        self.next_byte()?;
                        if self.group_depth == self.skip_remaining_in_group {
                            self.skip_remaining_in_group = 0;
                        }
                        if self.group_depth > 0 {
                            self.group_depth -= 1;
                        }
                    } else if b == b'\\' {
                        self.next_byte()?;
                        self.read_control_word()?;
                    } else {
                        self.next_byte()?;
                    }
                    continue;
                }
            }

            match b {
                b'{' => {
                    self.next_byte()?;
                    self.group_depth += 1;
                    if self.peek()? == Some(b'\\') {
                        self.next_byte()?;
                        if let Some((word, _)) = self.read_control_word()? {
                            if word == "*" {
                                if let Some((dest, _)) = self.read_control_word()? {
                                    self.record_skip(dest.clone());
                                    self.skip_unknown_destination()?;
                                }
                            } else {
                                self.handle_control_word(&word, current_byte, &mut in_table_row)?;
                            }
                        }
                    }
                }
                b'}' => {
                    self.next_byte()?;
                    if self.group_depth > 0 {
                        self.group_depth -= 1;
                    }
                    if self.in_pict && self.group_depth < self.group_depth + 1 {
                        // leaving pict group
                        self.in_pict = false;
                        self.pict_format = None;
                    }
                    if in_table_row && self.group_depth == 0 {
                        in_table_row = false;
                    }
                }
                b'\\' => {
                    self.next_byte()?;
                    if let Some((word, param)) = self.read_control_word()? {
                        if word.starts_with('\\') || word.starts_with('\'') || word == "u" {
                            self.start_block(BlockType::Text, current_byte);
                            self.current_block_lines += 1;
                        } else if self.in_pict && word == "bin" {
                            if let Some(n) = param {
                                let n = n as usize;
                                let data = self.read_raw_bytes(n)?;
                                if let Some(fmt) = self.pict_format {
                                    self.images.push(Image {
                                        format: fmt,
                                        data,
                                        byte_offset: current_byte,
                                    });
                                    self.start_block(BlockType::Figure, current_byte);
                                    self.current_block_lines += 10;
                                }
                            }
                        } else {
                            self.handle_control_word(&word, current_byte, &mut in_table_row)?;
                        }
                    }
                }
                b'\r' | b'\n' => {
                    self.next_byte()?;
                }
                _ => {
                    if self.in_pict {
                        // skip any stray whitespace inside pict before/after binary data
                        self.next_byte()?;
                    } else {
                        self.start_block(BlockType::Text, current_byte);
                        let mut line_count = 0;
                        while let Some(tb) = self.peek()? {
                            if matches!(tb, b'\\' | b'{' | b'}') {
                                break;
                            }
                            self.next_byte()?;
                            if tb == b'\n' {
                                line_count += 1;
                            }
                        }
                        self.current_block_lines += line_count.max(1);
                    }
                }
            }
        }

        self.flush_block(self.total_bytes);
        Ok(())
    }

    fn handle_control_word(
        &mut self,
        word: &str,
        current_byte: usize,
        in_table_row: &mut bool,
    ) -> std::io::Result<()> {
        match word {
            "pict" => {
                self.in_pict = true;
                self.pict_format = None;
            }
            "emfblip" => {
                self.pict_format = Some(ImageFormat::Emf);
            }
            "pngblip" => {
                self.pict_format = Some(ImageFormat::Png);
            }
            "jpegblip" => {
                self.pict_format = Some(ImageFormat::Jpeg);
            }
            "trowd" => {
                *in_table_row = true;
                self.start_block(BlockType::Table, current_byte);
            }
            "row" => {
                if *in_table_row {
                    self.current_block_lines += 1;
                }
            }
            "par" => {
                if !*in_table_row {
                    self.start_block(BlockType::Text, current_byte);
                }
                self.current_block_lines += 1;
            }
            "page" => {
                self.page_count += 1;
                self.flush_block(current_byte);
            }
            "intbl" | "cellx" | "clmgf" | "clmrg" | "trleft"
            | "picw" | "pich" | "picwgoal" | "pichgoal" | "picscalex" | "picscaley" => {}
            "rtf1" | "ansi" | "ansicpg" | "deff" | "deflang" | "plain" | "f"
            | "fs" | "cf" | "cb" | "b" | "i" | "ul" | "pard" | "qc" | "ql"
            | "qr" | "li" | "ri" | "fi" | "sb" | "sa" | "sl" | "slmult"
            | "tx" | "tab" | "emdash" | "endash" | "ldblquote" | "rdblquote"
            | "lquote" | "rquote" | "bullet" | "enspace" | "qmspace"
            | "emspace" | "nofeaturethrottle" | "uc" | "u" => {}
            _ => {
                self.record_skip(word.to_string());
            }
        }
        Ok(())
    }
}

pub fn parse_file(path: &Path) -> std::io::Result<ParseResult> {
    let start = Instant::now();
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();

    let mut lexer = Lexer::new(file);
    lexer.run()?;

    let parse_nanos = start.elapsed().as_nanos();
    let peak_memory_kb = get_rss_kb();

    let block_count = lexer.blocks.len();
    let page_count_estimate = if lexer.page_count > 0 {
        lexer.page_count
    } else {
        ((file_size as f64) / 10_000.0).ceil() as usize
    }
    .max(1);

    Ok(ParseResult {
        file_size,
        parse_nanos,
        peak_memory_kb,
        block_count,
        blocks: lexer.blocks,
        images: lexer.images,
        skipped_control_words: lexer.skipped_list,
        page_count_estimate,
    })
}

#[cfg(target_os = "macos")]
fn get_rss_kb() -> u64 {
    use std::process::Command;
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
        }
        _ => 0,
    }
}

#[cfg(target_os = "windows")]
fn get_rss_kb() -> u64 {
    0
}

#[cfg(target_os = "linux")]
fn get_rss_kb() -> u64 {
    use std::io::{BufRead, BufReader};
    let path = format!("/proc/{}/status", std::process::id());
    if let Ok(file) = File::open(path) {
        for line in BufReader::new(file).lines() {
            if let Ok(l) = line {
                if l.starts_with("VmRSS:") {
                    let parts: Vec<_> = l.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].parse::<u64>().unwrap_or(0);
                    }
                }
            }
        }
    }
    0
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn get_rss_kb() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_reference_rtf() {
        let path = PathBuf::from("../test-data/reference-tfl-100p.rtf");
        if !path.exists() {
            eprintln!("Skipping test: reference file not found");
            return;
        }
        let res = parse_file(&path).unwrap();
        assert!(res.block_count > 0);
        assert!(res.page_count_estimate >= 50);
        println!("Reference: {} blocks, {} pages", res.block_count, res.page_count_estimate);
        println!("Skipped ({}): {}", res.skipped_control_words.len(), res.skipped_control_words.join(", "));
    }

    #[test]
    fn parses_image_file() {
        let path = PathBuf::from("../test-data/synthetic-tfl-images-200p.rtf");
        if !path.exists() {
            eprintln!("Skipping test: image file not found");
            return;
        }
        let res = parse_file(&path).unwrap();
        println!(
            "Image-200p: {} MB, {} blocks, {} pages, {} images, {}s",
            res.file_size / 1024 / 1024,
            res.block_count,
            res.page_count_estimate,
            res.images.len(),
            res.parse_nanos as f64 / 1e9
        );
        assert!(!res.images.is_empty());
        for (i, img) in res.images.iter().take(3).enumerate() {
            println!("  image {}: {:?} {} bytes", i, img.format, img.data.len());
        }
    }

    #[test]
    fn benchmark_all() {
        for pages in [200, 1000, 2000] {
            let path = PathBuf::from(format!("../test-data/synthetic-tfl-{pages}p.rtf"));
            if !path.exists() {
                continue;
            }
            let res = parse_file(&path).unwrap();
            let secs = res.parse_nanos as f64 / 1e9;
            let pages_per_sec = res.page_count_estimate as f64 / secs;
            println!(
                "synthetic-{pages}p: size={:.2}MB pages={} blocks={} time={:.3}s throughput={:.0}p/s memory={:.1}MB",
                res.file_size as f64 / 1024.0 / 1024.0,
                res.page_count_estimate,
                res.block_count,
                secs,
                pages_per_sec,
                res.peak_memory_kb as f64 / 1024.0
            );
        }
    }
}
