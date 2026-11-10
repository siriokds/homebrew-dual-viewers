//! dual-syntax: i colori del codice per il visualizzatore di testo di Dual.
//!
//! - `highlight`: l'analisi con tree-sitter (testo dentro, tratti col loro
//!   ruolo fuori). Dual Rust su Windows e Linux la chiama direttamente,
//!   collegando questo crate senza la feature `plugin`.
//! - `abi` (feature `plugin`): il plugin dei visualizzatori, ABI 1.2, tipo
//!   SYNTAX_HIGHLIGHT (`dual_viewer_plugin.h` qui accanto), che Dual sul Mac
//!   apre con dlopen da `/opt/homebrew/opt/dual-syntax/lib`.

pub mod highlight;

#[cfg(feature = "plugin")]
mod abi;

/// Le estensioni che si colorano (senza punto, minuscole): le stesse di
/// `highlight::language_for`. Also whole file names, lowercase: Dual asks
/// first with the whole name, then with the extension.
pub const EXTENSIONS: &[&str] = &[
    "c", "cpp", "cc", "cxx", "c++", "h", "hpp", "hh", "hxx", "inl", "ino", "cs", "rs", "py", "pyw",
    "pyi", "js", "mjs", "cjs", "jsx", "html", "htm", "xhtml", "css", "php", "phtml", "php3",
    "php4", "php5", "phps", "sql", "md", "markdown", "mdown", "mkd", "toml", "sh", "bash", "zsh",
    "ksh", "command", "lua", "ts", "mts", "cts", "tsx", "json", "jsonc", "ipynb", "geojson",
    "makefile", "dockerfile", "cmakelists.txt",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_extension_has_its_language() {
        for ext in EXTENSIONS {
            assert!(highlight::language_for(ext).is_some(), "{ext}");
        }
        assert!(highlight::language_for("txt").is_none());
    }
}
