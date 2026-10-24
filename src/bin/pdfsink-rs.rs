use image::ImageFormat;
use pdfsink_rs::{
    PdfDocument, Result, SearchOptions, TableSettings, TableStrategy, TextOptions,
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

    // Per-page commands parse only the page they read. info and pages report
    // every page, so they parse them all.
    let mut args = args.peekable();
    let pdf = match command.as_str() {
        "info" | "pages" => PdfDocument::open(&path)?,
        _ => PdfDocument::open_pages(&path, &[page_number(args.peek())])?,
    };

    match command.as_str() {
        "info" => {
            println!("{}", serde_json::to_string_pretty(&info_payload(&path, &pdf))?);
        }
        "pages" => {
            let strategy = args.next().unwrap_or_else(|| "lines".to_string());
            let strat: TableStrategy = strategy.parse()?;
            let mut settings = TableSettings::default();
            settings.vertical_strategy = strat;
            settings.horizontal_strategy = strat;
            let mut payload = info_payload(&path, &pdf);
            if let Some(entries) = payload["pages"].as_array_mut() {
                for (entry, page) in entries.iter_mut().zip(pdf.pages()) {
                    entry["text"] = serde_json::Value::String(page.extract_text());
                    entry["table"] = serde_json::to_value(page.extract_table(settings.clone())?)?;
                }
            }
            println!("{}", serde_json::to_string(&payload)?);
        }
        "text" => {
            let page = resolve_page(&pdf, args.next())?;
            println!("{}", page.extract_text());
        }
        "words" => {
            let page = resolve_page(&pdf, args.next())?;
            let words = page.extract_words();
            println!("{}", serde_json::to_string_pretty(&words)?);
        }
        "search" => {
            let page = resolve_page(&pdf, args.next())?;
            let pattern = args.next().unwrap_or_default();
            let matches = page.search_with_options(&pattern, &SearchOptions::default(), &TextOptions::default())?;
            println!("{}", serde_json::to_string_pretty(&matches)?);
        }
        "objects" => {
            let page = resolve_page(&pdf, args.next())?;
            let payload = page.to_dict(None);
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        "json" => {
            let page = resolve_page(&pdf, args.next())?;
            let rendered = page.to_json::<Vec<u8>>(None, None, None, None, None, Some(2))?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "csv" => {
            let page = resolve_page(&pdf, args.next())?;
            let rendered = page.to_csv::<Vec<u8>>(None, None, None, None, None)?
                .unwrap_or_default();
            println!("{}", rendered);
        }
        "links" => {
            let page = resolve_page(&pdf, args.next())?;
            println!("{}", serde_json::to_string_pretty(&page.hyperlinks)?);
        }
        "table" => {
            let page = resolve_page(&pdf, args.next())?;
            let strategy = args.next().unwrap_or_else(|| "lines".to_string());
            let strat: TableStrategy = strategy.parse()?;
            let mut settings = TableSettings::default();
            settings.vertical_strategy = strat;
            settings.horizontal_strategy = strat;
            let table = page.extract_table(settings)?;
            println!("{}", serde_json::to_string_pretty(&table)?);
        }
        "svg" => {
            let page = resolve_page(&pdf, args.next())?;
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("page.svg"));
            fs::write(&output, page.to_debug_svg())?;
            eprintln!("wrote {}", output.display());
        }
        "render" => {
            let page = resolve_page(&pdf, args.next())?;
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

fn info_payload(path: &str, pdf: &PdfDocument) -> serde_json::Value {
    serde_json::json!({
        "path": path,
        "page_count": pdf.len(),
        "pages": pdf.pages().iter().map(|page| serde_json::json!({
            "page_number": page.page_number,
            "rotation": page.rotation,
            "width": page.width,
            "height": page.height,
            "bbox": page.bbox,
            "object_counts": page.object_counts(),
        })).collect::<Vec<_>>(),
    })
}

/// The page a per-page command reads. The document holds only that page
/// (see `run`), so it is always the first; the page argument is consumed.
fn resolve_page<'a>(pdf: &'a PdfDocument, _page_arg: Option<String>) -> Result<&'a pdfsink_rs::Page> {
    pdf.page(1)
}

/// The page argument as a number. Missing or unreadable means page 1.
fn page_number(page_arg: Option<&String>) -> usize {
    page_arg
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
