//! Converts the TextMate grammars of `grammars/` into the `.sublime-syntax` files syntect loads
//! at run time (`src/textmate.rs`). The conversion crate is only a development tool: the plugin
//! ships the converted files and syntect alone.
//!
//!     cargo run --example grammars
//!
//! Run it after regenerating the `.tmLanguage.json` files with `grammars/sjasmplus.py`; the test
//! `textmate::tests::converted_files_are_current` fails while they are out of date.

use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("grammars");
    for name in ["z80-sjasmplus", "lua"] {
        let json = std::fs::read_to_string(dir.join(format!("{name}.tmLanguage.json"))).expect("grammar");
        let raw = syntect_tmlanguage::RawGrammar::from_json(&json).expect("TextMate JSON");
        let yaml = syntect_tmlanguage::translate::to_sublime_syntax_yaml(&raw).expect("conversion");
        std::fs::write(dir.join(format!("{name}.sublime-syntax")), sorted_captures(&yaml)).expect("write");
        println!("grammars/{name}.sublime-syntax");
    }
}

/// The conversion lists the captures of a rule in the order of a hash map, which changes from run
/// to run: sorted by number, the same grammar gives the same text.
fn sorted_captures(yaml: &str) -> String {
    let number = |line: &str| line.trim_start().split_once(": ").and_then(|(n, _)| n.parse::<u32>().ok());
    let lines: Vec<&str> = yaml.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let mut run = vec![lines[i]];
        if number(lines[i]).is_some() {
            while i + run.len() < lines.len() && number(lines[i + run.len()]).is_some() {
                run.push(lines[i + run.len()]);
            }
            run.sort_by_key(|line| number(line));
        }
        i += run.len();
        for line in run {
            out += line;
            out.push('\n');
        }
    }
    out
}
