//! Tests that run the built pdfsink-rs program.

use pdfsink_rs::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pdfsink-rs"))
        .args(args)
        .output()
        .expect("program runs")
}

fn stdout(args: &[&str]) -> String {
    let output = run(args);
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// What the program prints for a JSON value. Compared as text because
/// serde_json can read a float back one unit off (126.036 as 126.03599999999999).
fn pretty<T: Serialize>(value: &T) -> String {
    format!("{}\n", serde_json::to_string_pretty(value).unwrap())
}

fn settings(strategy: TableStrategy) -> TableSettings {
    let mut settings = TableSettings::default();
    settings.vertical_strategy = strategy;
    settings.horizontal_strategy = strategy;
    settings
}

const FIXTURES: &[&str] = &["multipage.pdf", "table_lines.pdf", "rotated_and_duplicates.pdf"];

#[test]
fn per_page_commands_match_a_full_parse() {
    for name in FIXTURES {
        let path = fixture(name);
        let pdf = path.to_str().unwrap();
        let full = PdfDocument::open(&path).unwrap();

        for n in 1..=full.len() {
            let page = full.page(n).unwrap();
            let number = n.to_string();

            assert_eq!(stdout(&["text", pdf, &number]), format!("{}\n", page.extract_text()), "{name} page {n} text");
            assert_eq!(
                stdout(&["table", pdf, &number, "lines"]),
                pretty(&page.extract_table(settings(TableStrategy::Lines)).unwrap()),
                "{name} page {n} table"
            );
            assert_eq!(
                stdout(&["words", pdf, &number]),
                pretty(&page.extract_words()),
                "{name} page {n} words"
            );
            assert_eq!(
                stdout(&["objects", pdf, &number]),
                pretty(&page.to_dict(None)),
                "{name} page {n} objects"
            );
        }
    }
}

#[test]
fn a_missing_page_argument_means_page_one() {
    let pdf = fixture("multipage.pdf");
    let pdf = pdf.to_str().unwrap();
    assert_eq!(stdout(&["text", pdf]), stdout(&["text", pdf, "1"]));
}

#[test]
fn a_page_out_of_range_is_an_error() {
    let pdf = fixture("multipage.pdf");
    let output = run(&["text", pdf.to_str().unwrap(), "99"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid page number: 99"));
}
