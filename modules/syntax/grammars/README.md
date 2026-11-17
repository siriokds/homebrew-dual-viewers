# TextMate grammars

The languages made of lines and dialects, where a tree-sitter parser stops at the first form it
does not know, are coloured with a TextMate grammar through
[syntect](https://github.com/trishume/syntect) (`src/textmate.rs`). One engine per file, chosen by
the language; the other languages stay with tree-sitter.

| Language | Extensions | Grammar |
|---|---|---|
| Z80 assembly (sjasmplus) | `asm`, `z80`, `a80` | `z80-sjasmplus.sublime-syntax`, with Lua inside `LUA ... ENDLUA` (`lua.sublime-syntax`) |

## Where they come from

- `upstream/z80-macroasm.tmLanguage.json`: the grammar of the VS Code extension
  [mborik/z80-macroasm-vscode](https://github.com/mborik/z80-macroasm-vscode), commit
  `0e1eb169fa7b1e128d9bd9962ba17cfbd34a2957` (2026-11-04). MIT, `LICENSE-z80-macroasm`.
- `upstream/lua.tmLanguage.json`: VS Code's Lua grammar
  ([microsoft/vscode](https://github.com/microsoft/vscode/blob/main/extensions/lua/syntaxes/lua.tmLanguage.json),
  taken on 2026-11-17), converted from
  [sumneko/lua.tmbundle](https://github.com/sumneko/lua.tmbundle). Permissive licence,
  `LICENSE-lua-tmbundle`.

## What changes for sjasmplus

The source grammar serves several assemblers at once. `sjasmplus.py` keeps its structure and changes
what the [sjasmplus documentation](https://github.com/z00m128/sjasmplus/blob/master/docs/documentation.xml)
defines differently:

- numbers: all the documented forms (`12d`, `0ch`, `0q14`, `14o`...) with `'` and `_` between
  digits (`%1111'0000`); a number starts with a digit, `$`, `#` or `%` (`FCH` is a label);
  `$`, `$$`, `$$$`, `$$$$`;
- strings: inside apostrophes `''` is one apostrophe and `\` a plain character; the `Z` and `C`
  suffixes; the shadow registers `af'`, `bc'`, `de'`, `hl'`; an unclosed string ends with its line;
- labels: a definition at the start of the line (also after `>`, with `+1` or `+*`), the
  characters `_ . ! ? # @`; temporary labels `1_B`, `1_F` and the old `2B` in branches;
- directives: those sjasmplus registers in its source (`sjasm/directives.cpp`), with `EQU`,
  `DEFL`, `MEMORYMAP`, `MAP`/`FIELD`/`ENDMAP`, and those of the old Sjasm 0.39/0.42 still found
  in its users' sources (`REPEAT ... ENDREPEAT`, `CODE PAGE n`, `DEFPAGE`, the `IFIDN` family,
  the repeat count `[3] add a,a`); the source grammar's words of other assemblers (`read`,
  `stop`, `print`, `local`...) are labels. A directive is recognised only where a statement
  starts (start of the line, after a label, after `:`, after a dot-repeater), with or without
  a leading dot (`.db`): as an operand (`jp .stop`, `call update`) the word is a label;
- statements after `:` (`ORG 100h:LD A,10`); the dot-repeater (`.3 INC A`, `.(n)`); `#line`;
  OPT options; `__FILE__` and the other built-in names; the device names;
- `{ ... }` is not a comment (it reads memory in sjasmplus); the functions of the AS assembler
  (`sin`, `val`, `ld`...) are not sjasmplus words;
- `DG` bits and `DH`/`HEX` hexadecimal data; after `END` the rest of the file is a comment;
- `LUA ... ENDLUA` blocks are coloured as Lua.

The Lua grammar loses its two `\G` anchors, which syntect does not support (the spaces around a
comment and the `@tags` of documentation comments).

## Making them again

```sh
python3 grammars/sjasmplus.py grammars/upstream/z80-macroasm.tmLanguage.json \
    grammars/upstream/lua.tmLanguage.json grammars
cargo run --example grammars
cargo test
```

`examples/grammars.rs` converts the `.tmLanguage.json` files into syntect's `.sublime-syntax`
format with [syntect-tmlanguage](https://crates.io/crates/syntect-tmlanguage), a development
dependency only: the plugin ships the converted files and syntect. The grammars are loaded with
each line's newline, as VS Code gives them. The test `converted_files_are_current` fails while
the converted files are out of date.

## Checked against

- two real projects: one built with sjasmplus (55 files, 29,600 lines) and one with the old
  Sjasm 0.42 (988 files, 360,000 lines): no invalid mark, no word left plain except one file in
  another assembler's format; every change of colour against the source grammar was looked at;
- the sjasmplus sources (z00m128/sjasmplus, commit `5c44bea`, 2026-11-14): every example is
  coloured with no word left plain and no invalid mark; in the tests, only the lines that are
  errors on purpose (their expected listings say so) stay marked;
- its colour test `tests/integration/KateSyntaxHighLightTest.asm` and the keyword lists of its
  Kate syntax file `asm-z80-sj.xml`;
- syntect against VS Code's own engine (vscode-textmate with Oniguruma) on 436,000 lines: the same
  scopes for every visible character.
