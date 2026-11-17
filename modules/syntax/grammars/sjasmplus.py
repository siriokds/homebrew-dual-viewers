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

# A word of a statement follows whitespace or ':' (sjasmplus joins instructions with colons:
# ORG 100h:LD A,10:LD B,10).
STMT = r"(?<=[\s:])"

# Label characters (documentation, "Labels"): start with a letter or '_', then letters, digits
# and _ . ! ? # @ ; prefixes . (local), @ (global as-is), ! (does not affect local labels).
LABEL = r"[A-Za-z_][\w.!?#@]*"

# The directives: those sjasmplus registers (sjasm/directives.cpp) with EQU, DEFL and
# MEMORYMAP of its documentation; MAP, FIELD and ENDMAP, kept by its Kate syntax file
# (asm-z80-sj.xml); and those of the old Sjasm 0.39/0.42 its users still have in their
# sources (its manual: REPEAT ... ENDREPEAT, CODE, DEFPAGE, UPDATE, the IFIDN family...).
# The source grammar's words of other assemblers (read, stop, default, print, message,
# local...) are labels here. A directive is recognised only where a statement starts
# ("directive" below): as an operand ("call update", "jp .stop") the same word is a label.
DIRECTIVES = sorted(set("""
    abyte abytec abytez align assert binary block bplist byte cspectmap d24 db dc dd defarray
    defb defd defdevice defg defh define defl defm defp defs defw dephase device dg dh disp display
    dm dp ds dup dw dword dz edup else elseif emptytap emptytrd encoding end endif endlua endm
    endmod endmodule endr ends endt endw ent equ export fpos hex hexend hexout if ifdef ifn ifndef
    ifnused ifused incbin inchob include includelua inctrd insert labelslist lua macro memorymap
    mmu module opt org outend output page phase relocate_end relocate_start relocate_table rept
    save3dos saveamsdos savebin savecdt savecpcsna savecpr savedev savehex savehob savenex savesna
    savetap savetrd setbp setbreakpoint shellexec size sldopt slot struct tapend tapout text
    textarea undefine unphase while winexec word map field endmap
    repeat endrepeat endwhile endmacro exitmacro endstruct code defpage update error rotate ascmap
    dt ifb ifnb ifid ifnid ifidn ifidni ifdif ifdifi ifin ifini ifnum ifnnum ifstr ifnstr ifexists
    ifnexists
""".split()), key=lambda w: (-len(w), w))
# Those with a rule of their own, which colours what follows them too (DG bits, DH and HEX
# data, the Lua block, INCLUDE <file>).
OWN_RULE = {"dg", "defg", "dh", "defh", "hex", "lua", "include"}
# DEFINE+ and DEFARRAY+ (the + redefines).
DIRECTIVE = r"(?:(?:define|defarray)\+(?=\s)|(?:" + "|".join(w for w in DIRECTIVES if w not in OWN_RULE) + r")\b)"
# Their leading-dot form, where a statement starts, is only a keyword (.INCLUDE "file").
DOTTED_OWN = r"\.(?:" + "|".join(sorted(OWN_RULE)) + r")\b"
# The dot-repeater: .3, .(12-len), before a single instruction.
REPEATER = r"\.(?:\d+|\([^)\n]*\))"
BUILTINS = "SJASMPLUS|VERSION|PASS|ERRORS|WARNINGS|INCLUDE_LEVEL|BASE_FILE|FILE|LINE|COUNTER|DATE|TIME"


def z80(source):
    g = copy.deepcopy(source)
    g["name"] = "Z80 Assembly (sjasmplus)"
    repo = g["repository"]

    # Comments: '{ ... }' is a block comment in tniASM; in sjasmplus braces read memory
    # ({address}, {b address}) and fill structures, so that rule goes.
    repo["comments"]["patterns"] = [p for p in repo["comments"]["patterns"] if p.get("begin") != "{"]

    # Keywords: the dot-repeater (.3 INC A, .2 .3 RET), OPT options (--syntax=abf), and the
    # directives with a rule of their own (the others are "directive", where a statement starts).
    kw = repo["keywords"]["patterns"]
    assert len(kw) == 11 and kw[-1].get("name") == "string.other.lua.z80asm"
    kw[:] = [
        {"name": "keyword.control.repeat.z80asm", "match": STMT + REPEATER + r"(?=\s)"},
        {"name": "support.type.flag.z80asm", "match": r"(?<=\s)--[A-Za-z][\w-]*(?:=[^\s,;:]*)?"},
        # CODE PAGE n of the old Sjasm.
        {"match": r"(?i:(?<=\bcode)\s+(page)\b)", "captures": {"1": {"name": "keyword.control.z80asm"}}},
    ]
    # DG bits ("-", "." and "_" are 0, any other character 1) and DH/HEX hexadecimal pairs,
    # up to the comment (documentation, DG and DH).
    data_end = r"(?=;|//|$)"
    kw[1:1] = [
        {"begin": "(?i:" + STMT + r"(dg|defg)\b)", "beginCaptures": {"1": {"name": "keyword.control.z80asm"}},
         "end": data_end, "patterns": [{"include": "#strings"},
                                       {"name": "constant.numeric.integer.binary.z80asm", "match": r"[^\s;\"']+"}]},
        {"begin": "(?i:" + STMT + r"(dh|defh|hex)\b)", "beginCaptures": {"1": {"name": "keyword.control.z80asm"}},
         "end": data_end, "patterns": [{"include": "#strings"},
                                       {"name": "constant.numeric.integer.hexadecimal.z80asm", "match": "[0-9A-Fa-f]+"}]},
    ]
    # LUA [ALLPASS|PASS1|PASS2|PASS3] ... ENDLUA: the code inside is Lua (before the
    # directives, which list LUA too).
    kw.insert(1, {
        "name": "meta.embedded.block.lua",
        "begin": "(?i:" + STMT + r"(lua)\b(?:\s+(allpass|pass[123])\b)?)",
        "beginCaptures": {"1": {"name": "keyword.control.z80asm"}, "2": {"name": "support.type.flag.z80asm"}},
        "end": "(?i:" + STMT + r"(endlua)\b)",
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
        "match": "(?i:" + STMT + r"(include)\b)(?:\s*(<[^>\n]*>))?",
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
    # A directive, with or without a leading dot (.db, .INCLUDE), where a statement starts:
    # after whitespace at the start of the line, after a label at the start of the line, after
    # ':', after dot-repeaters.
    label_first = r"(?:(?:\s*>)?([@!]?\.?" + LABEL + r"|\d+)(\+[\d*])?:?)?"
    repeaters = r"((?:" + REPEATER + r"\s+)*)"
    directive = r"(\.?" + DIRECTIVE + "|" + DOTTED_OWN + ")"
    repo["directive"] = {"patterns": [
        {"match": "(?i:^" + label_first + r"\s+" + repeaters + directive + ")",
         "captures": {"1": {"name": "entity.name.function.z80asm"}, "2": {"name": "entity.name.function.z80asm"},
                      "3": {"name": "keyword.control.repeat.z80asm"}, "4": {"name": "keyword.control.z80asm"}}},
        {"match": r"(?i:(?<=:)\s*" + repeaters + directive + ")",
         "captures": {"1": {"name": "keyword.control.repeat.z80asm"}, "2": {"name": "keyword.control.z80asm"}}},
        # The repeat count of the old Sjasm where a statement starts: "Maal8: [3] add a,a".
        {"match": "^" + label_first + r"\s+(\[[^\]\n]*\])(?=\s)",
         "captures": {"1": {"name": "entity.name.function.z80asm"}, "2": {"name": "entity.name.function.z80asm"},
                      "3": {"name": "keyword.control.repeat.z80asm"}}},
    ]}
    # #line of the C preprocessor (documentation, "#line").
    repo["line-directive"] = {"match": r"^\s*(#\s*line)\b", "captures": {"1": {"name": "keyword.control.z80asm"}}}

    top = [p["include"] for p in g["patterns"]]
    assert top == ["#comments", "#keywords", "#strings", "#mnemonics", "#numbers", "#includes", "#labels"], top
    g["patterns"] = [{"include": "#" + k} for k in ("comments", "line-directive", "end-of-source", "directive",
                                                    "label-definition", "includes", "keywords",
                                                    "strings", "mnemonics", "numbers", "labels")]
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
