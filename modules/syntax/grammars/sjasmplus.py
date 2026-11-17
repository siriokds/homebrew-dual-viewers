#!/usr/bin/env python3
"""Build the Z80 grammar for sjasmplus from the VS Code z80-macroasm grammar.

The source grammar (mborik/z80-macroasm-vscode, MIT) serves several assemblers at once; this
script keeps its structure and changes only what the sjasmplus documentation defines
differently (docs/documentation.xml of z00m128/sjasmplus, sections on labels, numeric
constants, strings, expressions and the dot-repeater). It also prepares the Lua grammar used
inside LUA ... ENDLUA blocks (VS Code's lua.tmLanguage.json, from sumneko/lua.tmbundle), with
its two \\G anchors removed because syntect does not support them.

    python3 sjasmplus.py <z80-macroasm.tmLanguage.json> <lua.tmLanguage.json> <output dir>
"""
import copy
import json
import sys

# A statement starts after whitespace or after ':' (sjasmplus joins instructions with colons:
# ORG 100h:LD A,10:LD B,10). Directives may carry a leading dot (.db, .INCLUDE).
STMT = r"(?<=[\s:])"

# Label characters (documentation, "Labels"): start with a letter or '_', then letters, digits
# and _ . ! ? # @ ; prefixes . (local), @ (global as-is), ! (does not affect local labels).
LABEL = r"[A-Za-z_][\w.!?#@]*"

# Official keywords the source grammar does not know (asm-z80-sj.xml lists of sjasmplus).
MISSING_DIRECTIVES = (
    "d24|dd|defp|dp|text|defdevice|endmap|hexend|hexout|ifn|memorymap|"
    "relocate_(?:start|end|table)|save3dos|saveamsdos|savecdt|savecpcsna|savecpr|savehex|sldopt"
)
BUILTINS = "SJASMPLUS|VERSION|PASS|ERRORS|WARNINGS|INCLUDE_LEVEL|BASE_FILE|FILE|LINE|COUNTER|DATE|TIME"


def statement_start(match):
    """`(?<=\\s)` at the start of a keyword list becomes `(?<=[\\s:])\\.?`."""
    old = r"(?i:(?<=\s)(?:"
    assert match.startswith(old), match
    return "(?i:" + STMT + r"\.?(?:" + match[len(old):]


def z80(source):
    g = copy.deepcopy(source)
    g["name"] = "Z80 Assembly (sjasmplus)"
    repo = g["repository"]

    # Comments: '{ ... }' is a block comment in tniASM; in sjasmplus braces read memory
    # ({address}, {b address}) and fill structures, so that rule goes.
    repo["comments"]["patterns"] = [p for p in repo["comments"]["patterns"] if p.get("begin") != "{"]

    # Keywords: after ':' too, with an optional leading dot; the missing official ones; the
    # dot-repeater (.3 INC A, .(12-len) BYTE 0, .2 .3 RET); OPT options (--syntax=abf).
    kw = repo["keywords"]["patterns"]
    for p in kw:
        if "match" in p:
            p["match"] = statement_start(p["match"])
    lua = kw[-1]
    assert lua.get("name") == "string.other.lua.z80asm"
    kw.remove(lua)
    kw.append({"name": "keyword.control.z80asm", "match": "(?i:" + STMT + r"\.?(?:" + MISSING_DIRECTIVES + r")\b)"})
    kw.insert(0, {"name": "keyword.control.repeat.z80asm", "match": STMT + r"\.(?:\d+|\([^)\n]*\))(?=\s)"})
    kw.append({"name": "support.type.flag.z80asm", "match": r"(?<=\s)--[A-Za-z][\w-]*(?:=[^\s,;:]*)?"})
    # DG bits ("-", "." and "_" are 0, any other character 1) and DH/HEX hexadecimal pairs,
    # up to the comment (documentation, DG and DH).
    data_end = r"(?=;|//|$)"
    kw[1:1] = [
        {"begin": "(?i:" + STMT + r"\.?(dg|defg)\b)", "beginCaptures": {"1": {"name": "keyword.control.z80asm"}},
         "end": data_end, "patterns": [{"include": "#strings"},
                                       {"name": "constant.numeric.integer.binary.z80asm", "match": r"[^\s;\"']+"}]},
        {"begin": "(?i:" + STMT + r"\.?(dh|defh|hex)\b)", "beginCaptures": {"1": {"name": "keyword.control.z80asm"}},
         "end": data_end, "patterns": [{"include": "#strings"},
                                       {"name": "constant.numeric.integer.hexadecimal.z80asm", "match": "[0-9A-Fa-f]+"}]},
    ]
    # LUA [ALLPASS|PASS1|PASS2|PASS3] ... ENDLUA: the code inside is Lua.
    kw.append({
        "name": "meta.embedded.block.lua",
        "begin": "(?i:" + STMT + r"\.?(lua)\b(?:\s+(allpass|pass[123])\b)?)",
        "beginCaptures": {"1": {"name": "keyword.control.z80asm"}, "2": {"name": "support.type.flag.z80asm"}},
        "end": "(?i:" + STMT + r"\.?(endlua)\b)",
        "endCaptures": {"1": {"name": "keyword.control.z80asm"}},
        "contentName": "source.lua",
        "patterns": [{"include": "source.lua"}],
    })

    # Strings (documentation, "Character and string constants"): inside apostrophes two
    # apostrophes are one and the backslash is a plain character; inside quotes the backslash
    # escapes. Z or C after the closing mark (zero-terminated, top bit set). Shadow registers.
    st = repo["strings"]["patterns"]
    assert st[0]["match"] == "(?i:(af'))"
    st[0] = {"match": r"(?i:\b((?:af|bc|de|hl)'))", "captures": {"1": {"name": "support.type.register.z80asm"}}}
    assert st[1]["name"] == "string.quoted.single.z80asm"
    # An unclosed string ends with its line (the source missed a quote that is the last
    # character: "DEFINE QUOTE \"" ran on into the following lines).
    st[1]["end"] = r"(')(?!')(?:[zZcC](?!\w))?|([^\n])$|$"
    st[1]["patterns"] = [{"name": "constant.character.escape.z80asm", "match": "''"}]
    assert st[2]["name"] == "string.quoted.double.z80asm"
    st[2]["end"] = r"(\")(?:[zZcC](?!\w))?|((?:[^\\\n])$)|(?<!\\)$"

    # Mnemonics and operators.
    mn = repo["mnemonics"]["patterns"]
    # Conditions: also s and ns (aliases of m and p).
    assert "[cmpz]|n[cz]|p[eo]" in mn[0]["match"]
    mn[0]["match"] = mn[0]["match"].replace("[cmpz]|n[cz]|p[eo]", "[cmpsz]|n[csz]|p[eo]")
    # Devices of DEVICE.
    assert "zxspectrum(?:48|128|256|512|1024)" in mn[1]["match"]
    mn[1]["match"] = mn[1]["match"].replace(
        "zxspectrum(?:48|128|256|512|1024)",
        "zxspectrum(?:48|128|256|512|1024|2048|4096|8192|next)|amstradcpc(?:464|6128|plus)|noslot64k",
    )
    # Expression operators of the documentation.
    assert mn[2]["match"] == r"\b(?i:high|low|not|mod|sh[lr])\b"
    mn[2]["match"] = r"\b(?i:high|low|not|mod|sh[lr]|abs|sizeof|exist|norel|pair|u16)\b"
    # The functions of the AS assembler (sin, val, int, ld...) are not sjasmplus words: there
    # they are labels (examples: "val equ #B0"), and "ld" must stay an instruction.
    assert mn[3]["name"] == "constant.numeric.function.z80asm"
    del mn[3]
    # Instructions: after whitespace or ':' (the source consumed the spaces around them);
    # the unofficial SLI.
    for i in (3, 4):
        m = mn[i]["match"]
        assert m.startswith(r"\s(?i:") and m.endswith(r")\s"), m
        mn[i]["match"] = r"(?:^|" + STMT + ")" + m[2:-2] + r"(?=[\s:;]|$)"
    mn[3]["match"] = mn[3]["match"].replace("|slia|", "|slia|sli|")
    mn.append({"name": "constant.language.z80asm", "match": r"\b__(?:" + BUILTINS + r")__\b"})

    # Numbers (documentation, "Numeric constants"): they start with a digit, $, # or %; ' or _
    # may separate digits. $, $$, $$$, $$$$ are the program counter and pages.
    sep = lambda d: d + "(?:['_]?" + d + ")*"
    hexd, bind, octd, decd = "[0-9a-fA-F]", "[01]", "[0-7]", r"\d"
    num = "constant.numeric.integer."
    repo["numbers"]["patterns"] = [
        {"name": "constant.language.pc.z80asm", "match": r"\${2,4}|\$(?![0-9a-fA-F])"},
        {"name": num + "hexadecimal.z80asm", "match": "[$#]" + sep(hexd) + r"\b"},
        {"name": num + "hexadecimal.z80asm", "match": r"\b0[xX]" + sep(hexd) + r"\b"},
        {"name": num + "binary.z80asm", "match": r"\b0[bB]" + sep(bind) + r"\b"},
        {"name": num + "octal.z80asm", "match": r"\b0[qQ]" + sep(octd) + r"\b"},
        {"name": "variable.parameter.label.temporary.z80asm", "match": r"\b\d+(?:_[bBfF]|[fF])\b"},
        {"name": num + "hexadecimal.z80asm", "match": r"\b\d(?:['_]?" + hexd + r")*[hH]\b"},
        {"name": num + "binary.z80asm", "match": r"\b" + sep(bind) + r"[bB]\b"},
        {"name": "variable.parameter.label.temporary.z80asm", "match": r"\b\d+[bB]\b"},
        {"name": num + "octal.z80asm", "match": r"\b" + sep(octd) + r"[qQoO]\b"},
        {"name": num + "binary.z80asm", "match": "%" + sep(bind) + r"\b"},
        {"name": num + "decimal.z80asm", "match": r"\b" + sep(decd) + r"(?:\.\d+)?[dD]?\b"},
    ]

    # INCLUDE after ':' or with a dot too; <file> as the name (quoted names are strings).
    repo["includes"]["patterns"] = [{
        "match": "(?i:" + STMT + r"\.?(include)\b)(?:\s*(<[^>\n]*>))?",
        "captures": {"1": {"name": "keyword.control.z80asm"}, "2": {"name": "string.quoted.other.z80asm"}},
    }]

    # Labels: a definition is at the start of the line (or after a leading '>'), with an
    # optional SMC offset (+1, +*) and colon; temporary labels are numbers. Elsewhere a label
    # may start with '.' or '@'.
    repo["labels"]["patterns"] = [{
        "match": r"(?<!\w)[.@]?" + LABEL,
        "name": "variable.parameter.label.z80asm",
    }]
    repo["label-definition"] = {
        "match": r"^(?:\s*>)?([@!]?\.?" + LABEL + r"|\d+)(\+[\d*])?(?:(:)|(?=\s|$))",
        "captures": {"1": {"name": "entity.name.function.z80asm"}, "2": {"name": "entity.name.function.z80asm"}},
    }
    # END [start]: "the assembler will stop at this point" (documentation, END). It is the
    # first word of its line (after an optional label); the lines after it are comments
    # ("^.+": a zero-length match on an empty line would make the engine leave the rule).
    repo["end-of-source"] = {
        "begin": r"(?i:^(?:(?:\s*>)?([@!]?\.?" + LABEL + r"|\d+)(\+[\d*])?:?)?\s+\.?(end)\b)",
        "beginCaptures": {"1": {"name": "entity.name.function.z80asm"}, "2": {"name": "entity.name.function.z80asm"},
                          "3": {"name": "keyword.control.z80asm"}},
        "end": "(?!)",
        "patterns": [{"name": "comment.block.ignored.z80asm", "match": "^.+"}, {"include": "#comments"},
                     {"include": "#numbers"}, {"include": "#labels"}],
    }
    # #line of the C preprocessor (documentation, "#line").
    repo["line-directive"] = {"match": r"^\s*(#\s*line)\b", "captures": {"1": {"name": "keyword.control.z80asm"}}}

    top = [p["include"] for p in g["patterns"]]
    assert top == ["#comments", "#keywords", "#strings", "#mnemonics", "#numbers", "#includes", "#labels"], top
    g["patterns"] = [{"include": "#" + k} for k in ("comments", "line-directive", "end-of-source", "label-definition", "keywords",
                                                    "strings", "mnemonics", "numbers", "includes", "labels")]
    return g


def lua(source):
    """VS Code's Lua grammar without its two \\G anchors (syntect never matches \\G)."""
    g = copy.deepcopy(source)
    comment = g["repository"]["comment"]["patterns"]
    wrapper = comment[0]
    assert wrapper["end"] == r"(?!\G)((?!^)[ \t]+\n)?", wrapper["end"]
    # The wrapper only gave a scope to the spaces before and after a comment.
    g["repository"]["comment"]["patterns"] = wrapper["patterns"] + comment[1:]
    text = json.dumps(g)
    assert text.count(r"\\G[ \\t]*(@)") == 1
    return json.loads(text.replace(r"\\G[ \\t]*(@)", r"[ \\t]*(@)"))


def main():
    z80_src, lua_src, out = sys.argv[1:4]
    with open(z80_src, encoding="utf-8") as f:
        z = z80(json.load(f))
    with open(lua_src, encoding="utf-8") as f:
        l = lua(json.load(f))
    assert r"\G" not in json.dumps(z) and r"\\G" not in json.dumps(l)
    for name, grammar in (("z80-sjasmplus.tmLanguage.json", z), ("lua.tmLanguage.json", l)):
        with open(f"{out}/{name}", "w", encoding="utf-8") as f:
            json.dump(grammar, f, indent=1, ensure_ascii=False)
            f.write("\n")


if __name__ == "__main__":
    main()
