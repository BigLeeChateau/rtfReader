use serde::Serialize;
use std::path::PathBuf;

mod converter;
mod parser;
mod table_parser;
use parser::ParseResult;


#[derive(Debug, Serialize)]
pub struct ConvertedImage {
    pub index: usize,
    pub format: String,
    pub svg: String,
}

#[derive(Debug, Serialize)]
pub struct ImageParseResult {
    pub parse: ParseResult,
    pub convert_nanos: u128,
    pub converted_images: Vec<ConvertedImage>,
    pub total_images: usize,
    pub converted_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TableParseResult {
    pub row_count: usize,
    pub column_count: usize,
    pub html: String,
}

#[tauri::command]
async fn parse_rtf(path: String) -> Result<ParseResult, String> {
    let path = PathBuf::from(path);
    parser::parse_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn parse_and_convert_rtf(path: String) -> Result<ImageParseResult, String> {
    let path = PathBuf::from(path);
    let parse = parser::parse_file(&path).map_err(|e| e.to_string())?;
    let total_images = parse.images.len();
    let (svgs, convert_nanos) = converter::convert_images(&parse.images);
    let converted_count = svgs.iter().filter(|s| !s.is_empty()).count();

    let converted_images: Vec<ConvertedImage> = parse
        .images
        .iter()
        .zip(svgs.iter())
        .enumerate()
        .filter(|(_, (_, svg))| !svg.is_empty())
        .take(20)
        .map(|(i, (img, svg))| ConvertedImage {
            index: i,
            format: format!("{:?}", img.format),
            svg: svg.clone(),
        })
        .collect();

    Ok(ImageParseResult {
        parse,
        convert_nanos,
        converted_images,
        total_images,
        converted_count,
    })
}

#[tauri::command]
async fn parse_tables(path: String) -> Result<TableParseResult, String> {
    let path = PathBuf::from(path);
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let table = table_parser::parse_table(&data);
    let column_count = table.column_widths.len();
    let html = table_parser::render_html_table(&table);
    Ok(TableParseResult {
        row_count: table.rows.len(),
        column_count,
        html,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![parse_rtf, parse_and_convert_rtf, parse_tables])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
