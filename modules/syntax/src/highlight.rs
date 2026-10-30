//! I colori del codice secondo il linguaggio, con tree-sitter (D164 A di
//! Dual Rust): ogni grammatica analizza il codice per struttura e le sue
//! regole dei colori ("highlights") danno un nome a ogni pezzo (`function`,
//! `keyword`, `string`…); qui ogni nome ha un ruolo (`Role`, i numeri di
//! `dual_syntax_role_t` nell'header). I colori di ogni ruolo li sceglie Dual
//! secondo il suo tema.
//!
//! Scelto fra tre strade guardate e misurate sullo stesso file
//! (`experiments/sintassi` di Dual Rust): Lexilla di Scintilla (veloce ma
//! riconosce poco: niente funzioni né variabili), syntect (40 volte più
//! lento) e tree-sitter (utente: "il migliore a occhio").
//!
//! I linguaggi (D160 A): C, C++, C#, Rust, Python, JavaScript, HTML (con
//! JavaScript e CSS dentro), CSS, PHP (con l'HTML intorno), SQL e Markdown
//! (quando si apre come testo, senza il suo visualizzatore; anche il codice
//! dei blocchi ``` nei linguaggi qui sopra). Poi (utente, 24/10/2026): TOML,
//! Bash, Lua, TypeScript (anche TSX) e JSON, anche per i notebook di
//! Jupyter (`.ipynb`, che sono JSON).

use std::sync::OnceLock;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// Che cosa è un pezzo di testo. I numeri sono quelli di
/// `dual_syntax_role_t` nell'header: non si cambiano, si aggiungono in fondo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Role {
    Default = 0,
    Comment,
    /// Dichiarazioni e parole del linguaggio (`const`, `class`, `fn`).
    Keyword,
    /// Il percorso del codice dove la grammatica lo distingue (SQL, le
    /// regole `@media` del CSS).
    Control,
    Type,
    Function,
    Variable,
    Constant,
    String,
    Number,
    /// I tag dell'HTML.
    Tag,
    /// Gli attributi dell'HTML, i link del Markdown.
    Attribute,
    /// Le macro del preprocessore, i segni del Markdown.
    Preprocessor,
    /// Corsivo e grassetto del Markdown (del colore del testo).
    Emphasis,
    Strong,
    /// I titoli del Markdown: blu e in grassetto, come VS Code.
    Heading,
    /// Il testo delle citazioni del Markdown (`>`), verde come VS Code.
    Quote,
    /// Il barrato del Markdown: attenuato (Scintilla non sa barrare).
    Strike,
}

pub const ROLES: [Role; 18] = [
    Role::Default,
    Role::Comment,
    Role::Keyword,
    Role::Control,
    Role::Type,
    Role::Function,
    Role::Variable,
    Role::Constant,
    Role::String,
    Role::Number,
    Role::Tag,
    Role::Attribute,
    Role::Preprocessor,
    Role::Emphasis,
    Role::Strong,
    Role::Heading,
    Role::Quote,
    Role::Strike,
];

/// I nomi delle regole delle grammatiche e il loro ruolo. tree-sitter dà a
/// ogni pezzo il nome più vicino di questa lista (`function.method` →
/// `function`), quindi bastano i nomi di base più le eccezioni.
const NAMES: &[(&str, Role)] = &[
    ("attribute", Role::Attribute),
    ("boolean", Role::Keyword),
    ("charset", Role::Control),
    ("comment", Role::Comment),
    ("conditional", Role::Control),
    ("constant", Role::Constant),
    ("constant.builtin", Role::Keyword),
    ("constructor", Role::Type),
    ("delimiter", Role::Default),
    ("embedded", Role::Default),
    ("escape", Role::String),
    ("field", Role::Variable),
    ("float", Role::Number),
    ("function", Role::Function),
    ("function.macro", Role::Function),
    ("function.special", Role::Preprocessor),
    ("import", Role::Control),
    ("keyframes", Role::Control),
    ("keyword", Role::Keyword),
    ("label", Role::Default),
    ("media", Role::Control),
    ("method", Role::Function),
    ("module", Role::Type),
    ("namespace", Role::Type),
    ("none", Role::Default),
    ("number", Role::Number),
    ("operator", Role::Default),
    ("parameter", Role::Variable),
    ("preproc", Role::Preprocessor),
    ("property", Role::Variable),
    ("punctuation", Role::Default),
    ("punctuation.special", Role::Preprocessor),
    ("repeat", Role::Control),
    ("spell", Role::Default),
    ("storageclass", Role::Keyword),
    ("string", Role::String),
    // Le chiavi del JSON, come le proprietà (VS Code).
    ("string.special.key", Role::Variable),
    ("supports", Role::Control),
    ("tag", Role::Tag),
    ("text.emphasis", Role::Emphasis),
    ("text.literal", Role::String),
    ("text.reference", Role::Attribute),
    ("text.quote", Role::Quote),
    ("text.strike", Role::Strike),
    ("text.strong", Role::Strong),
    ("text.title", Role::Heading),
    ("text.uri", Role::Attribute),
    ("type", Role::Type),
    ("type.qualifier", Role::Keyword),
    ("variable", Role::Variable),
    ("variable.builtin", Role::Keyword),
];

/// I linguaggi che si colorano.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    C,
    Cpp,
    CSharp,
    Rust,
    Python,
    JavaScript,
    Html,
    Css,
    Php,
    Sql,
    Markdown,
    /// Il testo dei paragrafi del Markdown (corsivo, link, codice): una
    /// grammatica a sé, dentro quella dei blocchi.
    MarkdownInline,
    Toml,
    Bash,
    Lua,
    TypeScript,
    /// TypeScript con i tag JSX.
    Tsx,
    Json,
}

const ALL: [Language; 18] = [
    Language::C,
    Language::Cpp,
    Language::CSharp,
    Language::Rust,
    Language::Python,
    Language::JavaScript,
    Language::Html,
    Language::Css,
    Language::Php,
    Language::Sql,
    Language::Markdown,
    Language::MarkdownInline,
    Language::Toml,
    Language::Bash,
    Language::Lua,
    Language::TypeScript,
    Language::Tsx,
    Language::Json,
];

/// Il linguaggio di un file dalla sua estensione (senza punto, qualunque
/// maiuscola); `None`: testo semplice. `.h` va col C++, che comprende il C.
pub fn language_for(extension: &str) -> Option<Language> {
    Some(match extension.to_ascii_lowercase().as_str() {
        "c" => Language::C,
        "cpp" | "cc" | "cxx" | "c++" | "h" | "hpp" | "hh" | "hxx" | "inl" | "ino" => Language::Cpp,
        "cs" => Language::CSharp,
        "rs" => Language::Rust,
        "py" | "pyw" | "pyi" => Language::Python,
        "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
        "html" | "htm" | "xhtml" => Language::Html,
        "css" => Language::Css,
        "php" | "phtml" | "php3" | "php4" | "php5" | "phps" => Language::Php,
        "sql" => Language::Sql,
        "md" | "markdown" | "mdown" | "mkd" => Language::Markdown,
        "toml" => Language::Toml,
        "sh" | "bash" | "zsh" | "ksh" | "command" => Language::Bash,
        "lua" => Language::Lua,
        "ts" | "mts" | "cts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "json" | "jsonc" | "ipynb" | "geojson" => Language::Json,
        _ => return None,
    })
}

/// Il linguaggio di un blocco annidato, col nome che usano le grammatiche
/// (`<script>` → "javascript") e i blocchi ``` del Markdown ("js", "c++").
fn language_named(name: &str) -> Option<Language> {
    Some(match name.to_ascii_lowercase().as_str() {
        "markdown_inline" => Language::MarkdownInline,
        "markdown" | "md" => Language::Markdown,
        "javascript" | "js" | "jsx" | "mjs" => Language::JavaScript,
        "css" => Language::Css,
        "html" | "htm" => Language::Html,
        "php" => Language::Php,
        "c" => Language::C,
        "cpp" | "c++" | "cc" | "h" | "hpp" => Language::Cpp,
        "c_sharp" | "csharp" | "cs" | "c#" => Language::CSharp,
        "rust" | "rs" => Language::Rust,
        "python" | "py" => Language::Python,
        "sql" => Language::Sql,
        "toml" => Language::Toml,
        "bash" | "sh" | "shell" | "zsh" | "console" => Language::Bash,
        "lua" => Language::Lua,
        "typescript" | "ts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "json" | "jsonc" => Language::Json,
        _ => return None,
    })
}

/// L'HTML fuori da `<?php … ?>`: la regola sta in un file a parte della
/// grammatica (`queries/injections-text.scm` di tree-sitter-php), che il
/// pacchetto Rust non esporta.
const PHP_TEXT_INJECTION: &str = "((text) @injection.content\n (#set! injection.language \"html\")\n (#set! injection.combined))\n";

/// Le regole in più per i blocchi del Markdown (in fondo: a parità di pezzo
/// vince l'ultima).
const MARKDOWN_BLOCK_MORE: &str = r#"
(block_quote) @text.quote
(indented_code_block) @text.literal
(info_string) @punctuation.special
[(task_list_marker_checked) (task_list_marker_unchecked)] @punctuation.special
(pipe_table_delimiter_row) @punctuation.special
(pipe_table_header "|" @punctuation.special)
(pipe_table_row "|" @punctuation.special)
"#;

/// Le regole in più per il testo dei paragrafi del Markdown. I tag HTML
/// dentro il testo (`<b>`) si colorano interi come tag: la grammatica li
/// passerebbe all'HTML, ma a quel livello (blocchi, paragrafo, tag)
/// tree-sitter non restituisce colori, nemmeno analizzandoli insieme.
const MARKDOWN_INLINE_MORE: &str = r#"
(strikethrough) @text.strike
(html_tag) @tag
"#;

/// Le chiavi del JSON: la regola della grammatica sta prima di quella delle
/// stringhe, e con tree-sitter-highlight 0.25 vince l'ultima; ripetuta in
/// fondo, vince lei.
const JSON_KEYS: &str = "(pair key: (_) @string.special.key)\n";

/// La grammatica e le sue regole (colori, blocchi annidati, nomi locali).
fn build(language: Language) -> Result<HighlightConfiguration, String> {
    use Language::*;
    let php_injections = format!("{}\n{PHP_TEXT_INJECTION}", tree_sitter_php::INJECTIONS_QUERY);
    // Il Markdown, due ritocchi alle regole della grammatica:
    // - i blocchi di codice non prendono il colore delle stringhe né il
    //   "nessun colore" (`@none`): il loro contenuto ha i colori del suo
    //   linguaggio. `@none` si sovrapponeva a quei colori e ne scombinava
    //   l'ordine (il `fn` di un blocco ```rust restava bianco). I nomi nuovi
    //   non corrispondono a nessun ruolo, quindi tree-sitter li salta;
    // - il testo dei paragrafi va alla grammatica inline con i suoi figli
    //   (`injection.include-children`), se no non riceve niente.
    // - ogni parte che il Markdown interpreta si riconosce (utente:
    //   "aiuterebbe con tutte le parti che verranno interpretate che fossero
    //   riconoscibili"): i segni (`**`, gli apici, `[ ] ( )`, i ```, le `|`
    //   delle tabelle, le caselle `[x]`, il nome del linguaggio) come gli
    //   altri segni, non del colore del testo; il testo delle citazioni, il
    //   barrato e il codice indentato coi loro ruoli (`MARKDOWN_BLOCK_MORE`,
    //   `MARKDOWN_INLINE_MORE`).
    let markdown_highlights = tree_sitter_md::HIGHLIGHT_QUERY_BLOCK
        .replace("@text.literal", "@mdcode")
        .replace("@none", "@mdcontent")
        .replace("@punctuation.delimiter", "@punctuation.special")
        + MARKDOWN_BLOCK_MORE;
    let markdown_inline_highlights =
        tree_sitter_md::HIGHLIGHT_QUERY_INLINE.replace("@punctuation.delimiter", "@punctuation.special")
            + MARKDOWN_INLINE_MORE;
    let markdown_injections = tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
        "(#set! injection.language \"markdown_inline\"))",
        "(#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
    );
    let typescript_locals = format!("{}\n{}", tree_sitter_typescript::LOCALS_QUERY, tree_sitter_javascript::LOCALS_QUERY);
    let (grammar, highlights, injections, locals): (tree_sitter::Language, String, &str, &str) =
        match language {
            C => (tree_sitter_c::LANGUAGE.into(), tree_sitter_c::HIGHLIGHT_QUERY.into(), "", ""),
            // Le regole del C, poi quelle del C++ che le estendono, nell'ordine
            // della grammatica (`tree-sitter.json` di tree-sitter-cpp): a
            // parità di pezzo vince l'ultima (al contrario le funzioni
            // `Classe::nome` restavano variabili).
            Cpp => (
                tree_sitter_cpp::LANGUAGE.into(),
                format!("{}\n{}", tree_sitter_c::HIGHLIGHT_QUERY, tree_sitter_cpp::HIGHLIGHT_QUERY),
                "",
                "",
            ),
            CSharp => (
                tree_sitter_c_sharp::LANGUAGE.into(),
                tree_sitter_c_sharp::HIGHLIGHTS_QUERY.into(),
                "",
                "",
            ),
            Rust => (
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY.into(),
                tree_sitter_rust::INJECTIONS_QUERY,
                "",
            ),
            Python => (tree_sitter_python::LANGUAGE.into(), tree_sitter_python::HIGHLIGHTS_QUERY.into(), "", ""),
            // Le regole dal generale allo specifico: con tree-sitter-highlight
            // 0.25, fra le regole che colorano lo stesso pezzo vince l'ultima
            // (prima i tag JSX uscivano come variabili).
            JavaScript => (
                tree_sitter_javascript::LANGUAGE.into(),
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
                ),
                tree_sitter_javascript::INJECTIONS_QUERY,
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            Html => (
                tree_sitter_html::LANGUAGE.into(),
                tree_sitter_html::HIGHLIGHTS_QUERY.into(),
                tree_sitter_html::INJECTIONS_QUERY,
                "",
            ),
            Css => (tree_sitter_css::LANGUAGE.into(), tree_sitter_css::HIGHLIGHTS_QUERY.into(), "", ""),
            Php => (
                tree_sitter_php::LANGUAGE_PHP.into(),
                tree_sitter_php::HIGHLIGHTS_QUERY.into(),
                &php_injections,
                "",
            ),
            Sql => (tree_sitter_sequel::LANGUAGE.into(), tree_sitter_sequel::HIGHLIGHTS_QUERY.into(), "", ""),
            Markdown => (
                tree_sitter_md::LANGUAGE.into(),
                markdown_highlights,
                &markdown_injections,
                "",
            ),
            MarkdownInline => (
                tree_sitter_md::INLINE_LANGUAGE.into(),
                markdown_inline_highlights,
                tree_sitter_md::INJECTION_QUERY_INLINE,
                "",
            ),
            Toml => (tree_sitter_toml_ng::LANGUAGE.into(), tree_sitter_toml_ng::HIGHLIGHTS_QUERY.into(), "", ""),
            Bash => (tree_sitter_bash::LANGUAGE.into(), tree_sitter_bash::HIGHLIGHT_QUERY.into(), "", ""),
            Lua => (
                tree_sitter_lua::LANGUAGE.into(),
                tree_sitter_lua::HIGHLIGHTS_QUERY.into(),
                tree_sitter_lua::INJECTIONS_QUERY,
                tree_sitter_lua::LOCALS_QUERY,
            ),
            // TypeScript è JavaScript con in più i tipi: prima le regole di
            // JavaScript, poi quelle di TypeScript, che vincono (l'ultima
            // vince; `tree-sitter.json` della grammatica dà l'ordine opposto,
            // pensato per le versioni in cui vinceva la prima).
            TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
                tree_sitter_javascript::INJECTIONS_QUERY,
                &typescript_locals,
            ),
            Tsx => (
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                format!(
                    "{}\n{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
                tree_sitter_javascript::INJECTIONS_QUERY,
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            Json => (
                tree_sitter_json::LANGUAGE.into(),
                format!("{}\n{JSON_KEYS}", tree_sitter_json::HIGHLIGHTS_QUERY),
                "",
                "",
            ),
        };
    let mut config = HighlightConfiguration::new(grammar, format!("{language:?}"), &highlights, injections, locals)
        .map_err(|e| format!("{language:?}: {e:?}"))?;
    let names: Vec<&str> = NAMES.iter().map(|(n, _)| *n).collect();
    config.configure(&names);
    Ok(config)
}

/// Le regole di un linguaggio, preparate la prima volta che servono (qualche
/// millisecondo) e tenute per tutto il programma; `None` se non si caricano
/// (non succede: lo controllano i test).
fn configuration(language: Language) -> Option<&'static HighlightConfiguration> {
    static CONFIGS: [OnceLock<Option<HighlightConfiguration>>; ALL.len()] = [const { OnceLock::new() }; ALL.len()];
    let index = ALL.iter().position(|l| *l == language)?;
    CONFIGS[index].get_or_init(|| build(language).ok()).as_ref()
}

/// Un tratto di testo (in byte) e il suo ruolo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize,
    pub role: Role,
}

/// I tratti colorati di `text`, uno dopo l'altro dall'inizio alla fine
/// (anche quelli normali, `Role::Default`). Vuoto se la grammatica non si
/// carica o l'analisi si ferma.
pub fn highlight(language: Language, text: &[u8]) -> Vec<Span> {
    let Some(config) = configuration(language) else {
        return Vec::new();
    };
    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(config, text, None, |name| {
        language_named(name).and_then(configuration)
    }) else {
        return Vec::new();
    };
    let mut spans: Vec<Span> = Vec::new();
    let mut stack: Vec<Role> = Vec::new();
    for event in events {
        match event {
            Ok(HighlightEvent::HighlightStart(h)) => stack.push(NAMES[h.0].1),
            Ok(HighlightEvent::HighlightEnd) => {
                stack.pop();
            }
            Ok(HighlightEvent::Source { start, end }) => {
                let role = stack.last().copied().unwrap_or(Role::Default);
                // Due tratti vicini dello stesso ruolo diventano uno.
                match spans.last_mut() {
                    Some(last) if last.role == role && last.start + last.len == start => {
                        last.len += end - start;
                    }
                    _ => spans.push(Span { start, len: end - start, role }),
                }
            }
            Err(_) => return Vec::new(),
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Il ruolo del primo `needle` di `text`.
    fn role_of(language: Language, text: &str, needle: &str) -> Role {
        let at = text.find(needle).expect("il pezzo");
        highlight(language, text.as_bytes())
            .into_iter()
            .find(|s| s.start <= at && at < s.start + s.len)
            .map_or(Role::Default, |s| s.role)
    }

    #[test]
    fn every_grammar_loads() {
        for language in ALL {
            assert!(build(language).is_ok(), "{:?}", build(language).err());
        }
    }

    #[test]
    fn languages_by_extension() {
        assert_eq!(language_for("cpp"), Some(Language::Cpp));
        assert_eq!(language_for("H"), Some(Language::Cpp));
        assert_eq!(language_for("SQL"), Some(Language::Sql));
        assert_eq!(language_for("php"), Some(Language::Php));
        assert_eq!(language_for("md"), Some(Language::Markdown));
        assert_eq!(language_for("txt"), None);
        assert_eq!(language_for(""), None);
    }

    #[test]
    fn spans_cover_the_whole_text() {
        let text = "int main(void) { return 0; } // fine\n";
        let spans = highlight(Language::C, text.as_bytes());
        let mut at = 0;
        for span in &spans {
            assert_eq!(span.start, at);
            at += span.len;
        }
        assert_eq!(at, text.len());
    }

    #[test]
    fn cpp_like_the_comparison() {
        let text = "#ifdef NIENTE\nbool Audio::Start(int rate) {\n  if (!rate) return false;\n  m_impl->fills.store(0); // nota\n}\n#endif\n";
        assert_eq!(role_of(Language::Cpp, text, "Start"), Role::Function);
        assert_eq!(role_of(Language::Cpp, text, "store"), Role::Function);
        assert_eq!(role_of(Language::Cpp, text, "return"), Role::Keyword);
        assert_eq!(role_of(Language::Cpp, text, "fills"), Role::Variable);
        assert_eq!(role_of(Language::Cpp, text, "// nota"), Role::Comment);
        assert_eq!(role_of(Language::Cpp, text, "0)"), Role::Number);
    }

    #[test]
    fn the_other_languages() {
        assert_eq!(role_of(Language::Rust, "fn conta() -> usize { 1 }", "conta"), Role::Function);
        assert_eq!(role_of(Language::Python, "def f(a):\n    return 'x'\n", "'x'"), Role::String);
        assert_eq!(role_of(Language::CSharp, "class A { void F() {} }", "class"), Role::Keyword);
        assert_eq!(role_of(Language::JavaScript, "const n = 1; // x", "const"), Role::Keyword);
        assert_eq!(role_of(Language::JavaScript, "const e = <div className=\"c\">x</div>;", "div"), Role::Tag);
        assert_eq!(role_of(Language::Css, "a { color: red; } /* x */", "/* x */"), Role::Comment);
        assert_eq!(role_of(Language::Sql, "SELECT a FROM t; -- x", "SELECT"), Role::Keyword);
    }

    /// I linguaggi della fase 1 (utente, 24/10/2026).
    #[test]
    fn toml_bash_lua_typescript_json() {
        let toml = "[package]\nname = \"dual\" # nome\nversion = 1\n";
        assert_eq!(role_of(Language::Toml, toml, "package"), Role::Type);
        assert_eq!(role_of(Language::Toml, toml, "name"), Role::Variable);
        assert_eq!(role_of(Language::Toml, toml, "\"dual\""), Role::String);
        assert_eq!(role_of(Language::Toml, toml, "# nome"), Role::Comment);
        assert_eq!(role_of(Language::Toml, toml, "1\n"), Role::Number);
        let sh = "#!/bin/bash\nif [ -f \"$f\" ]; then\n  echo ciao # x\nfi\n";
        assert_eq!(role_of(Language::Bash, sh, "if"), Role::Keyword);
        assert_eq!(role_of(Language::Bash, sh, "echo"), Role::Function);
        assert_eq!(role_of(Language::Bash, sh, "# x"), Role::Comment);
        let lua = "local function somma(a, b)\n  for i = 1, 3 do end\n  return a + b -- x\nend\n";
        assert_eq!(role_of(Language::Lua, lua, "local"), Role::Keyword);
        assert_eq!(role_of(Language::Lua, lua, "somma"), Role::Function);
        assert_eq!(role_of(Language::Lua, lua, "for"), Role::Control);
        assert_eq!(role_of(Language::Lua, lua, "-- x"), Role::Comment);
        let ts = "interface Punto { x: number }\nconst p: Punto = { x: 1 }; // fine\n";
        assert_eq!(role_of(Language::TypeScript, ts, "interface"), Role::Keyword);
        assert_eq!(role_of(Language::TypeScript, ts, "number"), Role::Type);
        assert_eq!(role_of(Language::TypeScript, ts, "const"), Role::Keyword);
        assert_eq!(role_of(Language::TypeScript, ts, "// fine"), Role::Comment);
        let tsx = "const a: string = \"x\";\nconst e = <div className=\"c\">{a}</div>;\n";
        assert_eq!(role_of(Language::Tsx, tsx, "string"), Role::Type);
        assert_eq!(role_of(Language::Tsx, tsx, "div"), Role::Tag);
        let json = "{\"cells\": [1, true, null], \"nome\": \"x\"}";
        assert_eq!(role_of(Language::Json, json, "\"cells\""), Role::Variable);
        assert_eq!(role_of(Language::Json, json, "\"x\""), Role::String);
        assert_eq!(role_of(Language::Json, json, "1,"), Role::Number);
        assert_eq!(role_of(Language::Json, json, "true"), Role::Keyword);
        for (ext, language) in [
            ("toml", Language::Toml),
            ("sh", Language::Bash),
            ("zsh", Language::Bash),
            ("lua", Language::Lua),
            ("ts", Language::TypeScript),
            ("tsx", Language::Tsx),
            ("ipynb", Language::Json),
            ("JSON", Language::Json),
        ] {
            assert_eq!(language_for(ext), Some(language), "{ext}");
        }
    }

    /// Con file veri presi da internet (utente, 24/10/2026: "i test coi vari
    /// tipi di file … dovresti farli tu con dei file scaricati da
    /// internet"): per ognuno il linguaggio, quanto ci mette, quanto testo si
    /// colora e quanti errori trova la grammatica. A mano:
    ///
    ///   DUAL_FILE_VERI=<cartella> cargo test --lib \
    ///       highlight::tests::real_files -- --ignored --nocapture
    #[test]
    #[ignore = "vuole una cartella di file veri (DUAL_FILE_VERI)"]
    fn real_files() {
        let dir = std::env::var("DUAL_FILE_VERI").expect("DUAL_FILE_VERI");
        let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
        names.sort();
        for path in names {
            let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
            let Some(language) = language_for(&ext) else { continue };
            let text = std::fs::read(&path).unwrap();
            let start = std::time::Instant::now();
            let spans = highlight(language, &text);
            let took = start.elapsed();
            let covered: usize = spans.iter().map(|s| s.len).sum();
            assert_eq!(covered, text.len(), "{}: i tratti coprono tutto", path.display());
            let coloured: usize = spans.iter().filter(|s| s.role != Role::Default).map(|s| s.len).sum();
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&configuration(language).unwrap().language).unwrap();
            let tree = parser.parse(&text, None).unwrap();
            let mut errors = 0;
            let mut cursor = tree.walk();
            'walk: loop {
                let node = cursor.node();
                if node.is_error() || node.is_missing() {
                    errors += 1;
                }
                if cursor.goto_first_child() {
                    continue;
                }
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        break 'walk;
                    }
                }
            }
            eprintln!(
                "{:<20} {:<10} {:>7} byte  {:>8.1?}  colorato {:>3}%  errori {}",
                path.file_name().unwrap().to_string_lossy(),
                format!("{language:?}"),
                text.len(),
                took,
                coloured * 100 / text.len().max(1),
                errors
            );
        }
    }

    #[test]
    fn nested_languages() {
        // JavaScript dentro l'HTML, l'HTML intorno al PHP, il codice e il
        // testo dentro il Markdown.
        let page = "<p class=\"a\">x</p>\n<script>\nconst n = 1;\n</script>\n";
        assert_eq!(role_of(Language::Html, page, "p class"), Role::Tag);
        assert_eq!(role_of(Language::Html, page, "const"), Role::Keyword);
        let php = "<b>ciao</b>\n<?php echo $nome; ?>\n";
        assert_eq!(role_of(Language::Php, php, "nome"), Role::Variable);
        assert_eq!(role_of(Language::Php, php, "b>ciao"), Role::Tag);
        let md = "# Titolo\n\nUn `codice` e *corsivo*.\n\n```rust\nfn f() {}\n```\n";
        assert_eq!(role_of(Language::Markdown, md, "Titolo"), Role::Heading);
        assert_eq!(role_of(Language::Markdown, md, "codice"), Role::String);
        assert_eq!(role_of(Language::Markdown, md, "fn f"), Role::Keyword);
    }

    #[test]
    fn every_markdown_part_is_recognisable() {
        let md = "# Titolo\n\nTesto **forte**, *corsivo*, ~~via~~, `cod`, [link](http://a.b), <b>x</b>.\n\n> citata\n\n- [x] fatto\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn f() {}\n```\n\n    indentato\n";
        let at = |needle: &str| role_of(Language::Markdown, md, needle);
        // I segni della sintassi.
        assert_eq!(at("**forte"), Role::Preprocessor);
        assert_eq!(at("*corsivo"), Role::Preprocessor);
        assert_eq!(at("`cod"), Role::Preprocessor);
        assert_eq!(at("[link"), Role::Preprocessor);
        assert_eq!(at("](http"), Role::Preprocessor);
        assert_eq!(at("[x]"), Role::Preprocessor);
        assert_eq!(at("| a"), Role::Preprocessor);
        assert_eq!(at("|---"), Role::Preprocessor);
        assert_eq!(at("```rust"), Role::Preprocessor);
        assert_eq!(at("rust\n"), Role::Preprocessor);
        // Il contenuto coi suoi ruoli.
        assert_eq!(at("Titolo"), Role::Heading);
        assert_eq!(at("forte"), Role::Strong);
        assert_eq!(at("corsivo"), Role::Emphasis);
        assert_eq!(at("via"), Role::Strike);
        assert_eq!(at("cod`"), Role::String);
        assert_eq!(at("citata"), Role::Quote);
        assert_eq!(at("indentato"), Role::String);
        assert_eq!(at("b>x"), Role::Tag);
        assert_eq!(at("fn f"), Role::Keyword);
    }

    #[test]
    fn markdown_rules_were_patched() {
        // I ritocchi valgono solo se il testo della grammatica è quello
        // atteso: una versione nuova che lo cambia deve farsene accorgere.
        assert!(tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.contains("@none"));
        assert!(tree_sitter_md::INJECTION_QUERY_BLOCK
            .contains("(#set! injection.language \"markdown_inline\"))"));
    }
}
