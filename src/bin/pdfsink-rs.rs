use image::ImageFormat;
use pdfsink_rs::{
    Page, PdfDocument, Result, SearchOptions, TableSettings, TableStrategy, TextOptions,
};
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    let Some(path) = args.next() else {
        print_usage();
        return Ok(());
    };

    // Per-page commands parse only the page they read, and the page argument
    // is consumed here. info and pages report every page, so they parse them all.
    let pdf = match command.as_str() {
        "info" | "pages" => PdfDocument::open(&path)?,
        _ => PdfDocument::open_pages(&path, &[page_number(args.next())])?,
    };

    match command.as_str() {
        "info" => {
            let pages = pdf.pages().iter().map(page_info).collect();
            println!("{}", serde_json::to_string_pretty(&document_payload(&path, &pdf, pages))?);
        }
        "pages" => {
            let settings = table_settings(args.next())?;
            let pages = pdf
                .pages()
                .iter()
                .map(|page| -> Result<serde_json::Value> {
                    let mut entry = page_info(page);
                    entry["text"] = page.extract_text().into();
                    entry["table"] = serde_json::to_value(page.extract_table(settings.clone())?)?;
                    Ok(entry)
                })
                .collect::<Result<Vec<_>>>()?;
            println!("{}", serde_json::to_string(&document_payload(&path, &pdf, pages))?);
        }
        "text" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            println!("{}", page.extract_text());
        }
        "words" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let words = page.extract_words();
            println!("{}", serde_json::to_string_pretty(&words)?);
        }
        "search" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let pattern = args.next().unwrap_or_default();
            let matches = page.search_with_options(&pattern, &SearchOptions::default(), &TextOptions::default())?;
            println!("{}", serde_json::to_string_pretty(&matches)?);
        }
        "objects" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let payload = page.to_dict(None);
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "json" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let rendered = page.to_json::<Vec<u8>>(None, None, None, None, None, Some(2))?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "csv" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let rendered = page.to_csv::<Vec<u8>>(None, None, None, None, None)?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "links" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            println!("{}", serde_json::to_string_pretty(&page.hyperlinks)?);
        }
        "table" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let table = page.extract_table(table_settings(args.next())?)?;
            println!("{}", serde_json::to_string_pretty(&table)?);
        }
        "svg" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("page.svg"));
            fs::write(&output, page.to_debug_svg())?;
            eprintln!("wrote {}", output.display());
        }
        "render" => {
            let page = pdf.page(1)?; // the document holds only the requested page
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("page.png"));
            let image = page.to_image(Some(150.0), None, None, false, false)?;
            image.save(&output, Some(ImageFormat::Png), false, 256, 8)?;
            eprintln!("wrote {}", output.display());
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

/// The per-page entry of the info payload.
fn page_info(page: &Page) -> serde_json::Value {
    serde_json::json!({
        "page_number": page.page_number,
        "rotation": page.rotation,
        "width": page.width,
        "height": page.height,
        "bbox": page.bbox,
        "object_counts": page.object_counts(),
    })
}

fn document_payload(path: &str, pdf: &PdfDocument, pages: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "path": path, "page_count": pdf.len(), "pages": pages })
}

/// Table settings for a strategy argument. Missing means `lines`.
fn table_settings(strategy: Option<String>) -> Result<TableSettings> {
    let strategy: TableStrategy = strategy.as_deref().unwrap_or("lines").parse()?;
    let mut settings = TableSettings::default();
    settings.vertical_strategy = strategy;
    settings.horizontal_strategy = strategy;
    Ok(settings)
}

/// The page argument as a number. Missing or unreadable means page 1.
fn page_number(page_arg: Option<String>) -> usize {
    page_arg
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
}

fn print_usage() {
    eprintln!(
        "\
pdfsink-rs

Usage:
  pdfsink-rs info <file.pdf>
  pdfsink-rs pages <file.pdf> [lines|lines_strict|text|explicit]
  pdfsink-rs text <file.pdf> [page]
  pdfsink-rs words <file.pdf> [page]
  pdfsink-rs search <file.pdf> [page] [pattern]
  pdfsink-rs objects <file.pdf> [page]
  pdfsink-rs json <file.pdf> [page]
  pdfsink-rs csv <file.pdf> [page]
  pdfsink-rs links <file.pdf> [page]
  pdfsink-rs table <file.pdf> [page] [lines|lines_strict|text|explicit]
  pdfsink-rs svg <file.pdf> [page] [output.svg]
  pdfsink-rs render <file.pdf> [page] [output.png]
"
    );
}
