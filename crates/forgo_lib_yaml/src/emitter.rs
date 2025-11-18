// crates/forgo_lib_yaml/src/emitter.rs
use crate::ast::{BlockStyle, Elem, Error, Node, Scalar};
use crate::emitter_helpers::{
    emit_indent, emit_key, emit_scalar_with_ctx, is_block_scalar_candidate,
};

pub struct Emitter;

// ---------- test-only tracing ----------
#[cfg(test)]
macro_rules! dbg_emitln {
    ($($t:tt)*) => { eprintln!($($t)*); }
}
#[cfg(not(test))]
macro_rules! dbg_emitln {
    ($($t:tt)*) => {};
}
// ---------------------------------------

impl Emitter {
    pub fn emit_elem(elem: &Elem, indent: usize, out: &mut String) -> Result<(), Error> {
        // leading comments
        for c in &elem.meta.leading_comments {
            emit_indent(indent, out);
            out.push('#');
            if !c.is_empty() {
                out.push(' ');
                out.push_str(c);
            }
            out.push('\n');
        }
        match &elem.node {
            Node::Map(entries) => {
                // Empty maps emit as flow style
                if entries.is_empty() {
                    out.push_str("{}");
                    return Ok(());
                }
                for (k, v) in entries {
                    // emit leading comments for the value `v`
                    for c in &v.meta.leading_comments {
                        emit_indent(indent, out);
                        out.push('#');
                        if !c.is_empty() {
                            out.push(' ');
                            out.push_str(c);
                        }
                        out.push('\n');
                    }

                    emit_indent(indent, out);
                    emit_key(k, out);
                    out.push(':');

                    match &v.node {
                        // ---- BLOCK SCALAR (must be before generic scalar) ----
                        // Emit: "key: &anc |#cmt\n  <body>\n"
                        // Anchor appears before the header symbol per YAML emission norms.
                        // NOTE: Don't use block scalar if prefer_quoted is set (for escape sequences)
                        Node::Scalar(Scalar::Str(s))
                            if !v.meta.prefer_quoted
                                && (v.meta.prefer_block
                                    || v.meta.block_style.is_some()
                                    || is_block_scalar_candidate(s)) =>
                        {
                            dbg_emitln!(
                                "[emit] MAP key={:?} BLOCK scalar: anchor={:?} style={:?} prefer_block={} trailing_cmt={:?}",
                                k,
                                v.meta.anchor,
                                v.meta.block_style,
                                v.meta.prefer_block,
                                v.meta.trailing_comment
                            );

                            // value-side tag and anchor come *before* the header symbol
                            out.push(' '); // space after ':'

                            if let Some(t) = &v.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }

                            if let Some(a) = &v.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }

                            match v.meta.block_style {
                                Some(BlockStyle::Literal(Some(c))) => {
                                    out.push('|');
                                    out.push(c);
                                }
                                Some(BlockStyle::Folded(Some(c))) => {
                                    out.push('>');
                                    out.push(c);
                                }
                                Some(BlockStyle::Literal(None)) => out.push('|'),
                                Some(BlockStyle::Folded(None)) => {
                                    // Canonicalize: folded without chomp indicator -> literal
                                    // (content already folded during parse)
                                    out.push('|')
                                }
                                None => out.push('|'),
                            }

                            // header trailing comment (e.g. "# hdr")
                            Self::emit_trailing_comment(&v.meta, out);
                            out.push('\n');

                            // body (indented two from the key line)
                            dbg_emitln!("[emit]   body start (indent={})", indent + 2);
                            Self::emit_block_body(s, indent + 2, out);
                            dbg_emitln!("[emit]   body end");
                        }
                        // ---- ALIAS VALUE ----
                        Node::Alias(name) => {
                            out.push(' ');
                            // Tag before anchor before alias: key: !tag &anc *ref
                            if let Some(t) = &v.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &v.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            out.push('*');
                            out.push_str(name);
                            Self::emit_trailing_comment(&v.meta, out);
                            out.push('\n');
                        }
                        Node::Seq(items) => {
                            // Empty sequences emit flow style on same line
                            if items.is_empty() {
                                out.push(' ');
                                if let Some(t) = &v.meta.tag {
                                    out.push_str(t);
                                    out.push(' ');
                                }
                                if let Some(a) = &v.meta.anchor {
                                    out.push('&');
                                    out.push_str(a);
                                    out.push(' ');
                                }
                                out.push_str("[]");
                                Self::emit_trailing_comment(&v.meta, out);
                                out.push('\n');
                            } else {
                                // Non-empty sequences use block style
                                // Tag and anchor on header line for nested container
                                if let Some(t) = &v.meta.tag {
                                    out.push(' ');
                                    out.push_str(t);
                                }
                                if let Some(a) = &v.meta.anchor {
                                    out.push(' ');
                                    out.push('&');
                                    out.push_str(a);
                                }
                                Self::emit_trailing_comment(&v.meta, out);
                                out.push('\n');
                                Self::emit_elem(v, indent + 2, out)?;
                            }
                        }
                        Node::Map(entries) => {
                            // Empty maps emit flow style on same line
                            if entries.is_empty() {
                                out.push(' ');
                                if let Some(t) = &v.meta.tag {
                                    out.push_str(t);
                                    out.push(' ');
                                }
                                if let Some(a) = &v.meta.anchor {
                                    out.push('&');
                                    out.push_str(a);
                                    out.push(' ');
                                }
                                out.push_str("{}");
                                Self::emit_trailing_comment(&v.meta, out);
                                out.push('\n');
                            } else {
                                // Non-empty maps use block style
                                if let Some(t) = &v.meta.tag {
                                    out.push(' ');
                                    out.push_str(t);
                                }
                                if let Some(a) = &v.meta.anchor {
                                    out.push(' ');
                                    out.push('&');
                                    out.push_str(a);
                                }
                                Self::emit_trailing_comment(&v.meta, out);
                                out.push('\n');
                                Self::emit_elem(v, indent + 2, out)?;
                            }
                        }
                        // GENERIC SCALAR — this must be last among scalar-like cases
                        Node::Scalar(sc) => {
                            out.push(' ');
                            // Tag and anchor on a generic scalar value (e.g., key: !tag &anc 42)
                            if let Some(t) = &v.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &v.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            let force = matches!(sc, Scalar::Str(_))
                                && (v.meta.prefer_quoted
                                    || (v.meta.trailing_comment.is_some()
                                        && !v.meta.leading_comments.is_empty()));
                            emit_scalar_with_ctx(sc, force, /*in_seq_item=*/ false, out);
                            Self::emit_trailing_comment(&v.meta, out);
                            out.push('\n');
                        }
                    }
                }
                Ok(())
            }
            Node::Seq(items) => {
                // Empty sequences emit as flow style
                if items.is_empty() {
                    out.push_str("[]");
                    return Ok(());
                }
                for it in items {
                    // emit leading comments for this item `it`
                    for c in &it.meta.leading_comments {
                        emit_indent(indent, out);
                        out.push('#');
                        if !c.is_empty() {
                            out.push(' ');
                            out.push_str(c);
                        }
                        out.push('\n');
                    }

                    match &it.node {
                        // BLOCK SCALAR (seq item) — before generic scalar
                        // NOTE: Don't use block scalar if prefer_quoted is set (for escape sequences)
                        Node::Scalar(Scalar::Str(s))
                            if !it.meta.prefer_quoted
                                && (it.meta.prefer_block
                                    || it.meta.block_style.is_some()
                                    || is_block_scalar_candidate(s)) =>
                        {
                            emit_indent(indent, out);
                            out.push_str("- ");
                            // tag and anchor on item
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }

                            match it.meta.block_style {
                                Some(BlockStyle::Literal(Some(c))) => {
                                    out.push('|');
                                    out.push(c);
                                }
                                Some(BlockStyle::Folded(Some(c))) => {
                                    out.push('>');
                                    out.push(c);
                                }
                                _ => out.push('|'),
                            }

                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                            Self::emit_block_body(s, indent + 2, out);
                        }
                        Node::Alias(name) => {
                            emit_indent(indent, out);
                            out.push_str("- ");
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            out.push('*');
                            out.push_str(name);
                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                        }
                        Node::Scalar(sc) => {
                            emit_indent(indent, out);
                            out.push_str("- ");
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            let force = matches!(sc, Scalar::Str(_)) && it.meta.prefer_quoted;
                            emit_scalar_with_ctx(sc, force, /*in_seq_item=*/ true, out);

                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                        }
                        Node::Seq(items) if items.is_empty() => {
                            // Empty sequence as flow style on same line
                            emit_indent(indent, out);
                            out.push_str("- ");
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            out.push_str("[]");
                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                        }
                        Node::Map(entries) if entries.is_empty() => {
                            // Empty map as flow style on same line
                            emit_indent(indent, out);
                            out.push_str("- ");
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                                out.push(' ');
                            }
                            out.push_str("{}");
                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                        }
                        _ => {
                            emit_indent(indent, out);
                            out.push_str("- ");
                            if let Some(t) = &it.meta.tag {
                                out.push_str(t);
                                out.push(' ');
                            }
                            if let Some(a) = &it.meta.anchor {
                                out.push('&');
                                out.push_str(a);
                            }
                            Self::emit_trailing_comment(&it.meta, out);
                            out.push('\n');
                            Self::emit_elem(it, indent + 2, out)?;
                        }
                    }
                }
                Ok(())
            }
            Node::Scalar(Scalar::Str(s))
                if !elem.meta.prefer_quoted
                    && (is_block_scalar_candidate(s) || elem.meta.prefer_block) =>
            {
                dbg_emitln!(
                    "[emit] TOP-LEVEL BLOCK scalar: anchor={:?} style={:?} prefer_block={} trailing_cmt={:?}",
                    elem.meta.anchor,
                    elem.meta.block_style,
                    elem.meta.prefer_block,
                    elem.meta.trailing_comment
                );

                // top-level block scalar
                emit_indent(indent, out);

                // >>> FIX: include anchor before the header if present
                if let Some(a) = &elem.meta.anchor {
                    out.push('&');
                    out.push_str(a);
                    out.push(' ');
                }

                match elem.meta.block_style {
                    Some(BlockStyle::Literal(Some(c))) => {
                        out.push('|');
                        out.push(c);
                    }
                    Some(BlockStyle::Folded(Some(c))) => {
                        out.push('>');
                        out.push(c);
                    }
                    _ => out.push('|'),
                }

                Self::emit_trailing_comment(&elem.meta, out);
                out.push('\n');
                dbg_emitln!("[emit]   body start (indent={})", indent + 2);
                Self::emit_block_body(s, indent + 2, out);
                dbg_emitln!("[emit]   body end");
                Ok(())
            }
            Node::Scalar(sc) => {
                emit_indent(indent, out);
                // Top-level scalar with possible anchor
                if let Some(a) = &elem.meta.anchor {
                    out.push('&');
                    out.push_str(a);
                    out.push(' ');
                }
                let force = matches!(sc, Scalar::Str(_)) && elem.meta.prefer_quoted;
                emit_scalar_with_ctx(sc, force, /*in_seq_item=*/ false, out);
                Self::emit_trailing_comment(&elem.meta, out);
                out.push('\n');
                Ok(())
            }
            Node::Alias(name) => {
                emit_indent(indent, out);
                // Top-level alias can carry an anchor too (rare but valid)
                if let Some(a) = &elem.meta.anchor {
                    out.push('&');
                    out.push_str(a);
                    out.push(' ');
                }
                out.push('*');
                out.push_str(name);
                Self::emit_trailing_comment(&elem.meta, out);
                out.push('\n');
                Ok(())
            }
        }
    }

    fn emit_trailing_comment(meta: &crate::ast::Meta, out: &mut String) {
        if let Some(tc) = &meta.trailing_comment {
            out.push(' ');
            out.push('#');
            out.push(' ');
            out.push_str(tc);
        }
    }

    fn emit_block_body(s: &str, indent: usize, out: &mut String) {
        // Emit each logical line, preserving its content (including leading spaces),
        // but stripping trailing whitespace, and always add exactly one newline per line.
        // If the body is empty, emit a single blank indented line.
        if s.is_empty() {
            emit_indent(indent, out);
            out.push('\n');
            return;
        }

        // We normalize only line endings here; we do NOT trim leading spaces.
        // But we DO trim trailing spaces (YAML spec requirement).
        // Keep a possible final empty line if present in `s`.
        let mut start = 0usize;
        while start <= s.len() {
            let rest = &s[start..];
            if rest.is_empty() {
                break;
            }
            if let Some(pos) = rest.find('\n') {
                let (line, adv) = (&rest[..pos], pos + 1);
                emit_indent(indent, out);
                // Strip trailing whitespace from line
                out.push_str(line.trim_end());
                out.push('\n');
                dbg_emitln!("[emit]     line: {:?}", line);
                start += adv;
            } else {
                emit_indent(indent, out);
                // Strip trailing whitespace from final line
                out.push_str(rest.trim_end());
                out.push('\n');
                dbg_emitln!("[emit]     line: {:?}", rest);
                break;
            }
        }
    }
}
