//! Layout preprocessor for Mesh language.
//! Converts indentation-based blocks into explicit brace-delimited blocks (`{ ... }`),
//! while properly handling:
//! 1. Multi-line indented blocks introduced by `:` at end of line.
//! 2. Indented closures nested inside arrays (`[...]`), closing the block before the `,`.
//! 3. Single-line inline bodies after `:` or `=>`.
//! 4. Already explicit brace blocks (`fn main(): { ... }`).
//! 5. Distinguishing type annotations (like `x: Int` or `(x: Int, y: Int)`) from block colons.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTokKind {
    Atom,
    StringLit,
    FloatLit,
    IntLit,
    Name,
    Arrow,     // ->
    FatArrow,  // =>
    Colon,     // :
    Comma,     // ,
    Semicolon, // ;
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Dot,       // .
    Op,        // operators
    Other,
}

#[derive(Debug, Clone)]
pub struct LayoutTok {
    pub kind: LayoutTokKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    bd: usize,
    colon_indent: usize,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct OpenBlock {
    indent: usize,
    bd: usize,
}

fn lex_line(line: &str) -> Vec<LayoutTok> {
    let mut toks = Vec::new();
    let bytes = line.as_bytes();
    let n = bytes.len();
    let mut i = 0;

    while i < n {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\r' {
            i += 1;
            continue;
        }

        // Line comments: # or //
        if b == b'#' || (b == b'/' && i + 1 < n && bytes[i + 1] == b'/') {
            break;
        }

        // String literals: "..." or '...'
        if b == b'"' || b == b'\'' {
            let quote = b;
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            toks.push(LayoutTok {
                kind: LayoutTokKind::StringLit,
                text: line[start..i].to_string(),
            });
            continue;
        }

        // Atom: :[a-zA-Z_]\w*
        if b == b':' && i + 1 < n && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
            let start = i;
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            toks.push(LayoutTok {
                kind: LayoutTokKind::Atom,
                text: line[start..i].to_string(),
            });
            continue;
        }

        // Arrow: ->
        if b == b'-' && i + 1 < n && bytes[i + 1] == b'>' {
            toks.push(LayoutTok {
                kind: LayoutTokKind::Arrow,
                text: "->".to_string(),
            });
            i += 2;
            continue;
        }

        // FatArrow: =>
        if b == b'=' && i + 1 < n && bytes[i + 1] == b'>' {
            toks.push(LayoutTok {
                kind: LayoutTokKind::FatArrow,
                text: "=>".to_string(),
            });
            i += 2;
            continue;
        }

        // Single punctuation
        match b {
            b':' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::Colon,
                    text: ":".to_string(),
                });
                i += 1;
                continue;
            }
            b',' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::Comma,
                    text: ",".to_string(),
                });
                i += 1;
                continue;
            }
            b';' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::Semicolon,
                    text: ";".to_string(),
                });
                i += 1;
                continue;
            }
            b'(' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::LParen,
                    text: "(".to_string(),
                });
                i += 1;
                continue;
            }
            b')' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::RParen,
                    text: ")".to_string(),
                });
                i += 1;
                continue;
            }
            b'[' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::LBracket,
                    text: "[".to_string(),
                });
                i += 1;
                continue;
            }
            b']' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::RBracket,
                    text: "]".to_string(),
                });
                i += 1;
                continue;
            }
            b'{' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::LBrace,
                    text: "{".to_string(),
                });
                i += 1;
                continue;
            }
            b'}' => {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::RBrace,
                    text: "}".to_string(),
                });
                i += 1;
                continue;
            }
            b'.' => {
                if i + 1 < n && bytes[i + 1] == b'.' {
                    if i + 2 < n && bytes[i + 2] == b'=' {
                        toks.push(LayoutTok {
                            kind: LayoutTokKind::Op,
                            text: "..=".to_string(),
                        });
                        i += 3;
                    } else {
                        toks.push(LayoutTok {
                            kind: LayoutTokKind::Op,
                            text: "..".to_string(),
                        });
                        i += 2;
                    }
                } else {
                    toks.push(LayoutTok {
                        kind: LayoutTokKind::Dot,
                        text: ".".to_string(),
                    });
                    i += 1;
                }
                continue;
            }
            _ => {}
        }

        // Numbers: Int or Float
        if b.is_ascii_digit() {
            let start = i;
            while i < n && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < n && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
                i += 1; // skip '.'
                while i < n && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                toks.push(LayoutTok {
                    kind: LayoutTokKind::FloatLit,
                    text: line[start..i].to_string(),
                });
            } else {
                toks.push(LayoutTok {
                    kind: LayoutTokKind::IntLit,
                    text: line[start..i].to_string(),
                });
            }
            continue;
        }

        // Identifiers / keywords: [a-zA-Z_]\w*
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            toks.push(LayoutTok {
                kind: LayoutTokKind::Name,
                text: line[start..i].to_string(),
            });
            continue;
        }

        // Multi-char operators: ==, !=, <=, >=, etc.
        if b"=!<>+-*/%^&|".contains(&b) {
            let start = i;
            while i < n && b"=!<>+-*/%^&|".contains(&bytes[i]) {
                i += 1;
            }
            toks.push(LayoutTok {
                kind: LayoutTokKind::Op,
                text: line[start..i].to_string(),
            });
            continue;
        }

        // Fallback for single char
        toks.push(LayoutTok {
            kind: LayoutTokKind::Other,
            text: line[i..i + 1].to_string(),
        });
        i += 1;
    }

    toks
}

fn ends_with_terminator(out: &[String]) -> bool {
    if let Some(last) = out.last() {
        last == "{" || last == ";" || last == "," || last == ":" || last == "->" || last == "=>"
    } else {
        true
    }
}

fn dedent(
    leading: usize,
    indent_stack: &mut Vec<usize>,
    open_blocks: &mut Vec<OpenBlock>,
    out: &mut Vec<String>,
    pending: Option<&mut Vec<Pending>>,
) {
    let mut closed = 0;
    while *indent_stack.last().unwrap_or(&0) > leading {
        indent_stack.pop();
        if !open_blocks.is_empty() {
            open_blocks.pop();
            if !ends_with_terminator(out) {
                out.push(";".to_string());
            }
            out.push("}".to_string());
            closed += 1;
        }
    }
    if closed > 0 {
        out.push(";".to_string());
    }
    if let Some(pending) = pending {
        let top_now = *indent_stack.last().unwrap_or(&0);
        let mut i = 0;
        while i < pending.len() {
            if pending[i].colon_indent >= leading && leading <= top_now {
                pending.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

/// Convert indented Mesh source into brace-delimited source code.
pub fn layout(src: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut bracket_depth: usize = 0;
    let mut pending: Vec<Pending> = Vec::new();
    let mut open_blocks: Vec<OpenBlock> = Vec::new();

    for raw_line in src.lines() {
        let expanded = raw_line.replace('\t', "    ");
        let stripped = expanded.trim_start();

        // Skip blank lines or comments
        if stripped.is_empty() || stripped.starts_with('#') || stripped.starts_with("//") {
            continue;
        }

        let leading = expanded.len() - stripped.len();

        // ── Open pending blocks / dedent ──────────────────────────────
        if !pending.is_empty() {
            let mut opened = false;
            let mut i = 0;
            while i < pending.len() {
                let p = pending[i];
                if leading > p.colon_indent && bracket_depth == p.bd {
                    indent_stack.push(leading);
                    open_blocks.push(OpenBlock {
                        indent: leading,
                        bd: p.bd,
                    });
                    out.push("{".to_string());
                    pending.remove(i);
                    opened = true;
                } else {
                    i += 1;
                }
            }
            if !opened && bracket_depth == 0 {
                dedent(
                    leading,
                    &mut indent_stack,
                    &mut open_blocks,
                    &mut out,
                    Some(&mut pending),
                );
            }
        } else if bracket_depth == 0 {
            dedent(leading, &mut indent_stack, &mut open_blocks, &mut out, None);
        }

        // ── Tokenize line ─────────────────────────────────────────────
        let pending_before = pending.len();
        let line_toks = lex_line(stripped);

        let mut paren_nesting: usize = 0;
        let mut tok_idx = 0;
        while tok_idx < line_toks.len() {
            let tok = &line_toks[tok_idx];

            match tok.kind {
                LayoutTokKind::LParen => {
                    paren_nesting += 1;
                    bracket_depth += 1;
                    out.push(tok.text.clone());
                }
                LayoutTokKind::LBracket => {
                    bracket_depth += 1;
                    out.push(tok.text.clone());
                }
                LayoutTokKind::LBrace => {
                    out.push(tok.text.clone());
                }
                LayoutTokKind::RParen => {
                    paren_nesting = paren_nesting.saturating_sub(1);
                    while let Some(top) = open_blocks.last() {
                        if top.bd >= bracket_depth {
                            if indent_stack.len() > 1 {
                                indent_stack.pop();
                            }
                            open_blocks.pop();
                            if !ends_with_terminator(&out) {
                                out.push(";".to_string());
                            }
                            out.push("}".to_string());
                        } else {
                            break;
                        }
                    }
                    bracket_depth = bracket_depth.saturating_sub(1);
                    out.push(tok.text.clone());
                }
                LayoutTokKind::RBracket => {
                    while let Some(top) = open_blocks.last() {
                        if top.bd >= bracket_depth {
                            if indent_stack.len() > 1 {
                                indent_stack.pop();
                            }
                            open_blocks.pop();
                            if !ends_with_terminator(&out) {
                                out.push(";".to_string());
                            }
                            out.push("}".to_string());
                        } else {
                            break;
                        }
                    }
                    bracket_depth = bracket_depth.saturating_sub(1);
                    out.push(tok.text.clone());
                }
                LayoutTokKind::RBrace => {
                    out.push(tok.text.clone());
                }
                LayoutTokKind::Comma => {
                    // Close open blocks whose bd >= bracket_depth
                    // (e.g. nested closures inside arrays: [ fn(x): return x, fn(x): ... ])
                    while let Some(top) = open_blocks.last() {
                        if top.bd >= bracket_depth {
                            if indent_stack.len() > 1 {
                                indent_stack.pop();
                            }
                            open_blocks.pop();
                            if !ends_with_terminator(&out) {
                                out.push(";".to_string());
                            }
                            out.push("}".to_string());
                        } else {
                            break;
                        }
                    }
                    out.push(",".to_string());
                }
                LayoutTokKind::Colon | LayoutTokKind::FatArrow => {
                    let remaining = &line_toks[tok_idx + 1..];

                    // Check if colon is inside parens (like (x: Int, y: Float)) -> parameter type annotation!
                    let is_param_type_ann = paren_nesting > 0 && tok.kind == LayoutTokKind::Colon;
                    let is_local_type_ann = tok.kind == LayoutTokKind::Colon
                        && tok_idx == 2
                        && matches!(
                            line_toks.first().map(|t| t.text.as_str()),
                            Some("let" | "const")
                        )
                        && line_toks
                            .get(1)
                            .is_some_and(|t| t.kind == LayoutTokKind::Name);

                    if is_param_type_ann || is_local_type_ann {
                        out.push(tok.text.clone());
                    } else if !remaining.is_empty() {
                        if remaining[0].kind == LayoutTokKind::LBrace {
                            // Explicit brace: fn foo(): { ... }
                            out.push(tok.text.clone());
                        } else if tok.kind == LayoutTokKind::Colon
                            && tok_idx > 0
                            && (line_toks[tok_idx - 1].kind == LayoutTokKind::Name)
                            && (remaining.len() == 1
                                || (remaining.len() > 1
                                    && remaining[1].kind != LayoutTokKind::Colon))
                            && (line_toks[0].text != "if"
                                && line_toks[0].text != "else"
                                && line_toks[0].text != "while"
                                && line_toks[0].text != "fn"
                                && line_toks[0].text != "for")
                            && remaining.iter().all(|t| {
                                t.kind != LayoutTokKind::Op && t.text != "=" && t.text != "return"
                            })
                        {
                            // Field declaration in struct (e.g. `x: Int`): not a block!
                            out.push(tok.text.clone());
                        } else {
                            // Same-line inline body (e.g. fn(x): x + 1 or fn(x) => x + 1 or if c: return 1)
                            let mut body_toks: Vec<LayoutTok> = Vec::new();
                            let mut suffix_toks: Vec<LayoutTok> = Vec::new();
                            let inner_bd = bracket_depth;
                            let mut scan_bd = bracket_depth;

                            for rt in remaining {
                                match rt.kind {
                                    LayoutTokKind::LParen
                                    | LayoutTokKind::LBracket
                                    | LayoutTokKind::LBrace => {
                                        scan_bd += 1;
                                        if suffix_toks.is_empty() {
                                            body_toks.push(rt.clone());
                                        } else {
                                            suffix_toks.push(rt.clone());
                                        }
                                    }
                                    LayoutTokKind::RParen
                                    | LayoutTokKind::RBracket
                                    | LayoutTokKind::RBrace => {
                                        scan_bd = scan_bd.saturating_sub(1);
                                        if scan_bd < inner_bd {
                                            suffix_toks.push(rt.clone());
                                        } else if suffix_toks.is_empty() {
                                            body_toks.push(rt.clone());
                                        } else {
                                            suffix_toks.push(rt.clone());
                                        }
                                    }
                                    LayoutTokKind::Comma if scan_bd == inner_bd => {
                                        suffix_toks.push(rt.clone());
                                    }
                                    _ => {
                                        if suffix_toks.is_empty() {
                                            body_toks.push(rt.clone());
                                        } else {
                                            suffix_toks.push(rt.clone());
                                        }
                                    }
                                }
                            }

                            out.push(tok.text.clone());
                            out.push("{".to_string());
                            for bt in &body_toks {
                                out.push(bt.text.clone());
                            }
                            if !body_toks.is_empty() && !ends_with_terminator(&out) {
                                out.push(";".to_string());
                            }
                            out.push("}".to_string());

                            for st in suffix_toks {
                                match st.kind {
                                    LayoutTokKind::LParen
                                    | LayoutTokKind::LBracket
                                    | LayoutTokKind::LBrace => {
                                        bracket_depth += 1;
                                        out.push(st.text);
                                    }
                                    LayoutTokKind::RParen
                                    | LayoutTokKind::RBracket
                                    | LayoutTokKind::RBrace => {
                                        while let Some(top) = open_blocks.last() {
                                            if top.bd >= bracket_depth {
                                                if indent_stack.len() > 1 {
                                                    indent_stack.pop();
                                                }
                                                open_blocks.pop();
                                                if !ends_with_terminator(&out) {
                                                    out.push(";".to_string());
                                                }
                                                out.push("}".to_string());
                                                out.push(";".to_string());
                                            } else {
                                                break;
                                            }
                                        }
                                        bracket_depth = bracket_depth.saturating_sub(1);
                                        out.push(st.text);
                                    }
                                    LayoutTokKind::Comma => {
                                        while let Some(top) = open_blocks.last() {
                                            if top.bd >= bracket_depth {
                                                if indent_stack.len() > 1 {
                                                    indent_stack.pop();
                                                }
                                                open_blocks.pop();
                                                if !ends_with_terminator(&out) {
                                                    out.push(";".to_string());
                                                }
                                                out.push("}".to_string());
                                            } else {
                                                break;
                                            }
                                        }
                                        out.push(",".to_string());
                                    }
                                    _ => {
                                        out.push(st.text);
                                    }
                                }
                            }
                            break; // Done with line tokens
                        }
                    } else {
                        // Colon at end of line -> introduces indented block!
                        pending.push(Pending {
                            bd: bracket_depth,
                            colon_indent: leading,
                        });
                        out.push(tok.text.clone());
                    }
                }
                _ => {
                    out.push(tok.text.clone());
                }
            }
            tok_idx += 1;
        }

        let innermost_bd = open_blocks.last().map(|ob| ob.bd).unwrap_or(0);
        if bracket_depth == innermost_bd && pending.len() == pending_before {
            if !ends_with_terminator(&out) {
                out.push(";".to_string());
            }
        }
    }

    // ── Close all remaining blocks ────────────────────────────────────
    while indent_stack.len() > 1 {
        indent_stack.pop();
        if !open_blocks.is_empty() {
            open_blocks.pop();
        }
        if !ends_with_terminator(&out) {
            out.push(";".to_string());
        }
        out.push("}".to_string());
        out.push(";".to_string());
    }

    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_simple_function() {
        let src = r#"
fn main():
    const a = 42
    let b = a * 3
    println("${b}")
"#;
        let laid_out = layout(src);
        assert!(laid_out.contains("fn main ( ) : {"));
        assert!(laid_out.contains("const a = 42 ;"));
        assert!(laid_out.contains("} ;"));
    }

    #[test]
    fn test_layout_preserves_local_type_annotation() {
        let src = "const nguess: Float = 1.0";
        let laid_out = layout(src);
        assert!(laid_out.contains("const nguess : Float = 1.0"));
        assert!(!laid_out.contains("Float {"));
    }

    #[test]
    fn test_layout_indented_closures_in_array() {
        let src = r#"
let funcs = [
    fn(x):
        let y = x + 1
        return y,
    fn(x):
        return x * 2,
]
"#;
        let laid_out = layout(src);
        // Both closures must be wrapped in { ... } and separated by ','
        assert!(laid_out.contains("fn ( x ) : { let y = x + 1 ; return y ; } ,"));
        assert!(laid_out.contains("fn ( x ) : { return x * 2 ; } ,"));
    }

    #[test]
    fn test_layout_inline_closure() {
        let src = "let inc = fn(x) => x + 1";
        let laid_out = layout(src);
        assert!(laid_out.contains("=> { x + 1 ; }"));
    }

    #[test]
    fn test_layout_explicit_braces_preserved() {
        let src = r#"
fn main(): {
    const a = 42
    return a
}
"#;
        let laid_out = layout(src);
        println!("LAID OUT: {}", laid_out);
        assert!(laid_out.contains("fn main ( ) : { const a = 42 ; return a ; }"));
    }
}
