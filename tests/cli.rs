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

fn json(args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&stdout(args)).unwrap()
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
    assert_eq!(stdout(&["text", pdf, "abc"]), stdout(&["text", pdf, "1"]));
}

#[test]
fn a_page_out_of_range_is_an_error() {
    let pdf = fixture("multipage.pdf");
    for n in ["0", "99"] {
        let output = run(&["text", pdf.to_str().unwrap(), n]);

        assert!(!output.status.success(), "page {n}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&format!("invalid page number: {n}")),
            "page {n}"
        );
    }
}

/// `pages` output with `text` and `table` removed from each page entry.
fn without_page_content(mut payload: serde_json::Value) -> serde_json::Value {
    for entry in payload["pages"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        entry.remove("text");
        entry.remove("table");
    }
    payload
}

#[test]
fn pages_is_info_plus_text_and_table_for_every_page() {
    for name in FIXTURES {
        let path = fixture(name);
        let pdf = path.to_str().unwrap();
        let all = json(&["pages", pdf]);

        assert_eq!(without_page_content(all.clone()), json(&["info", pdf]), "{name} info");
        for (index, entry) in all["pages"].as_array().unwrap().iter().enumerate() {
            let number = (index + 1).to_string();
            assert_eq!(
                format!("{}\n", entry["text"].as_str().unwrap()),
                stdout(&["text", pdf, &number]),
                "{name} page {number} text"
            );
            assert_eq!(entry["table"], json(&["table", pdf, &number, "lines"]), "{name} page {number} table");
        }
    }
}

#[test]
fn pages_uses_the_given_table_setting() {
    let path = fixture("table_text_only.pdf");
    let pdf = path.to_str().unwrap();
    let all = json(&["pages", pdf, "text"]);

    assert_eq!(all["pages"][0]["table"], json(&["table", pdf, "1", "text"]));
    assert_ne!(all["pages"][0]["table"], serde_json::Value::Null, "text strategy finds a table");
}

#[test]
fn pages_rejects_an_unknown_table_setting() {
    let pdf = fixture("multipage.pdf");
    let output = run(&["pages", pdf.to_str().unwrap(), "nope"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown table strategy: nope"));
}
