//! The languages coloured with a TextMate grammar through syntect instead of tree-sitter: the
//! languages made of lines and dialects, where a parser stops at the first form it does not know.
//! One engine per file, chosen by the language.
//!
//! Z80 assembly for sjasmplus: `grammars/z80-sjasmplus.sublime-syntax` (from the VS Code
//! z80-macroasm grammar, `grammars/README.md`), with Lua inside `LUA ... ENDLUA` blocks
//! (`grammars/lua.sublime-syntax`). The grammars expect each line with its newline, as VS Code
//! gives it.

use std::collections::HashMap;
use std::sync::OnceLock;

use syntect::easy::ScopeRangeIterator;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};

use crate::highlight::{Role, Span};

const Z80: &str = include_str!("../grammars/z80-sjasmplus.sublime-syntax");
const LUA: &str = include_str!("../grammars/lua.sublime-syntax");
const Z80_SCOPE: &str = "source.z80asm";

/// The scope names and their role: the most specific name that starts a scope wins
/// (`keyword.control.z80asm` before `keyword.control` before `keyword`). Names not listed
/// (`punctuation`, `meta`, `source`) let the enclosing scope decide, so the quotes of a string
/// keep the colour of the string. The colours follow VS Code's Dark+ and Light+ for these
/// scopes: instructions as keywords, directives as control words, registers as types.
const ROLES: &[(&str, Role)] = &[
    ("comment", Role::Comment),
    ("string", Role::String),
    ("constant.character.escape", Role::String),
    ("constant.numeric", Role::Number),
    // high, low, mod, sizeof...
    ("constant.numeric.operator", Role::Keyword),
    // $ (program counter), __FILE__, nil, true.
    ("constant.language", Role::Keyword),
    ("constant", Role::Constant),
    ("keyword", Role::Keyword),
    ("keyword.control", Role::Control),
    ("keyword.operator", Role::Default),
    ("storage", Role::Keyword),
    ("support.type", Role::Type),
    ("support.function", Role::Function),
    ("support.constant", Role::Constant),
    // The condition of a jump (nz, c, pe).
    ("meta.preprocessor", Role::Keyword),
    // A label where it is defined, a Lua function name.
    ("entity.name.function", Role::Function),
    ("entity.name", Role::Type),
    ("entity.other.attribute-name", Role::Attribute),
    // A label where it is used.
    ("variable", Role::Variable),
    ("variable.language", Role::Keyword),
    // Plugin roles have no error colour: plain text.
    ("invalid", Role::Default),
];

struct Grammars {
    set: SyntaxSet,
    /// The table above as syntect scopes, longest first.
    roles: Vec<(Scope, Role)>,
}

/// The grammars, loaded the first time they are needed and kept for the whole program; `None`
/// if they do not load (the tests make sure they do).
fn grammars() -> Option<&'static Grammars> {
    static GRAMMARS: OnceLock<Option<Grammars>> = OnceLock::new();
    GRAMMARS
        .get_or_init(|| {
            let mut builder = SyntaxSetBuilder::new();
            for yaml in [Z80, LUA] {
                builder.add(SyntaxDefinition::load_from_str(yaml, true, None).ok()?);
            }
            let mut roles: Vec<(Scope, Role)> =
                ROLES.iter().map(|(name, role)| Some((Scope::new(name).ok()?, *role))).collect::<Option<_>>()?;
            roles.sort_by_key(|(scope, _)| std::cmp::Reverse(scope.len()));
            Some(Grammars { set: builder.build(), roles })
        })
        .as_ref()
}

/// The role of a scope stack: the innermost scope with a role in the table.
fn role_of(stack: &ScopeStack, grammars: &Grammars, cache: &mut HashMap<Scope, Option<Role>>) -> Role {
    stack
        .as_slice()
        .iter()
        .rev()
        .find_map(|scope| {
            *cache.entry(*scope).or_insert_with(|| {
                grammars.roles.iter().find(|(prefix, _)| prefix.is_prefix_of(*scope)).map(|(_, role)| *role)
            })
        })
        .unwrap_or(Role::Default)
}

/// Adds `len` bytes of `role` after the last span, joined to it when the role is the same.
fn push(spans: &mut Vec<Span>, start: usize, len: usize, role: Role) {
    if len == 0 {
        return;
    }
    match spans.last_mut() {
        Some(last) if last.role == role && last.start + last.len == start => last.len += len,
        _ => spans.push(Span { start, len, role }),
    }
}

/// The spans of a Z80 source, one after another from the start to the end (the plain ones too),
/// like `highlight::highlight`. Empty if the grammars do not load or the parsing stops.
pub fn highlight_z80(text: &[u8]) -> Vec<Span> {
    let Some(grammars) = grammars() else {
        return Vec::new();
    };
    let Some(syntax) = Scope::new(Z80_SCOPE).ok().and_then(|s| grammars.set.find_syntax_by_scope(s)) else {
        return Vec::new();
    };
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut cache = HashMap::new();
    let mut spans = Vec::new();
    let mut line_start = 0;
    while line_start < text.len() {
        let line_end = text[line_start..].iter().position(|b| *b == b'\n').map_or(text.len(), |i| line_start + i);
        let raw = &text[line_start..line_end];
        let content = raw.strip_suffix(b"\r").unwrap_or(raw);
        // A line that is not UTF-8 is still parsed, so the lines after it keep their state, but
        // stays plain: its byte offsets would not match the text.
        let (line, valid) = match std::str::from_utf8(content) {
            Ok(line) => (format!("{line}\n"), true),
            Err(_) => (format!("{}\n", String::from_utf8_lossy(content)), false),
        };
        let Ok(ops) = state.parse_line(&line, &grammars.set) else {
            return Vec::new();
        };
        for (range, op) in ScopeRangeIterator::new(&ops, &line) {
            if stack.apply(op).is_err() {
                return Vec::new();
            }
            let end = range.end.min(content.len());
            if valid && range.start < end {
                push(&mut spans, line_start + range.start, end - range.start, role_of(&stack, grammars, &mut cache));
            }
        }
        // What has no colour of its own: the line when it is not UTF-8, its `\r` and `\n`.
        let covered = spans.last().map_or(0, |s| s.start + s.len);
        let next = (line_end + 1).min(text.len());
        push(&mut spans, covered, next - covered, Role::Default);
        line_start = next;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The role of the first `needle` after `after` in `text`.
    fn role_at(text: &str, after: &str, needle: &str) -> Role {
        let from = text.find(after).expect("the context");
        let at = from + text[from..].find(needle).expect("the piece");
        highlight_z80(text.as_bytes())
            .into_iter()
            .find(|s| s.start <= at && at < s.start + s.len)
            .map_or(Role::Default, |s| s.role)
    }

    #[test]
    fn grammars_load() {
        let grammars = grammars().expect("grammars");
        for scope in [Z80_SCOPE, "source.lua"] {
            assert!(grammars.set.find_syntax_by_scope(Scope::new(scope).unwrap()).is_some(), "{scope}");
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

    /// The `.sublime-syntax` files are the conversion of the `.tmLanguage.json` next to them
    /// (`cargo run --example grammars` makes them again).
    #[test]
    fn converted_files_are_current() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("grammars");
        for (name, shipped) in [("z80-sjasmplus", Z80), ("lua", LUA)] {
            let json = std::fs::read_to_string(dir.join(format!("{name}.tmLanguage.json"))).unwrap();
            let raw = syntect_tmlanguage::RawGrammar::from_json(&json).unwrap();
            let yaml = syntect_tmlanguage::translate::to_sublime_syntax_yaml(&raw).unwrap();
            assert!(sorted_captures(&yaml) == sorted_captures(shipped), "{name}: run `cargo run --example grammars`");
        }
    }

    /// The colour test of sjasmplus (tests/integration/KateSyntaxHighLightTest.asm) and the
    /// forms of its documentation.
    #[test]
    fn sjasmplus_forms() {
        let text = "    device zxspectrum48     ; directive\n\
                    \x20   org     $A8, 0xA8, #A8\n\
                    \x20   disp    43008d\n\
                    \x20   ld      bc, %1111'0000  ; C++ separator\n\
                    \x20   db      'apostrophe''s', \"quotes\\\"\"Z, 0\n\
                    \x20   ex      af,af'\n\
                    s:  ; some label\n\
                    \x20   call    s, s\n\
                    \x20   ret     nz\n\
                    \x20   and     low .localLab\n\
                    \x20   MACRO leMacron _arg0?\n\
                    \x20   ENDM\n\
                    answer+1:   ld a,13\n\
                    val     equ #B0\n\
                    \x20   .3 inc a\n\
                    \x20   org 100h:ld a,10:.db 1\n\
                    \x20   djnz 2_B\n";
        assert_eq!(role_at(text, "device", "device"), Role::Control);
        assert_eq!(role_at(text, "device", "zxspectrum48"), Role::Type);
        assert_eq!(role_at(text, "; directive", "; directive"), Role::Comment);
        assert_eq!(role_at(text, "org", "$A8"), Role::Number);
        assert_eq!(role_at(text, "org", "#A8"), Role::Number);
        assert_eq!(role_at(text, "disp", "43008d"), Role::Number);
        assert_eq!(role_at(text, "ld      bc", "ld"), Role::Keyword);
        assert_eq!(role_at(text, "ld      bc", "bc"), Role::Type);
        assert_eq!(role_at(text, "ld      bc", "'0000"), Role::Number);
        assert_eq!(role_at(text, "db ", "''s"), Role::String);
        assert_eq!(role_at(text, "db ", "\"Z"), Role::String);
        assert_eq!(role_at(text, "ex ", "af'"), Role::Type);
        assert_eq!(role_at(text, "s:", "s"), Role::Function);
        assert_eq!(role_at(text, "call", "s,"), Role::Keyword);
        assert_eq!(role_at(text, ", s\n", "s"), Role::Variable);
        assert_eq!(role_at(text, "ret", "nz"), Role::Keyword);
        assert_eq!(role_at(text, "and ", "low"), Role::Keyword);
        assert_eq!(role_at(text, "and ", ".localLab"), Role::Variable);
        assert_eq!(role_at(text, "MACRO", "_arg0?"), Role::Variable);
        assert_eq!(role_at(text, "answer", "answer+1"), Role::Function);
        assert_eq!(role_at(text, "val", "val"), Role::Function);
        assert_eq!(role_at(text, ".3", ".3"), Role::Control);
        assert_eq!(role_at(text, "100h:", "ld"), Role::Keyword);
        assert_eq!(role_at(text, "100h:", ".db"), Role::Control);
        assert_eq!(role_at(text, "djnz", "2_B"), Role::Variable);
    }

    #[test]
    fn lua_blocks_and_the_end() {
        let text = "    LUA ALLPASS\n\
                    \x20       -- a comment\n\
                    \x20       for i = 0, 3 do _pc(\"nop\") end\n\
                    \x20   ENDLUA\n\
                    \x20   ld a,1\n\
                    \x20   END start\n\
                    \n\
                    notes after the end: it's ignored\n";
        assert_eq!(role_at(text, "LUA", "LUA"), Role::Control);
        assert_eq!(role_at(text, "-- a", "-- a"), Role::Comment);
        assert_eq!(role_at(text, "for", "for"), Role::Control);
        assert_eq!(role_at(text, "for", "\"nop\""), Role::String);
        assert_eq!(role_at(text, "ENDLUA", "ENDLUA"), Role::Control);
        assert_eq!(role_at(text, "ld a", "ld"), Role::Keyword);
        assert_eq!(role_at(text, "END start", "END"), Role::Control);
        assert_eq!(role_at(text, "END start", "start"), Role::Variable);
        assert_eq!(role_at(text, "notes", "it's"), Role::Comment);
    }

    /// The spans cover the whole text in order, with Windows line ends and a line that is not
    /// UTF-8 (it stays plain, the next line keeps its colours at the right place).
    #[test]
    fn spans_cover_the_text() {
        let text = b"start:\r\n    ld a,'\xE8'\r\n    nop ; x\n    ld b,2";
        let spans = highlight_z80(text);
        let mut at = 0;
        for span in &spans {
            assert_eq!(span.start, at);
            at += span.len;
        }
        assert_eq!(at, text.len());
        let nop = text.windows(3).position(|w| w == b"nop").unwrap();
        let span = spans.iter().find(|s| s.start <= nop && nop < s.start + s.len).unwrap();
        assert_eq!((span.start, span.role), (nop, Role::Keyword));
        assert!(highlight_z80(b"").is_empty());
    }
}
