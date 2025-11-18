// crates/forgo_lib_yaml/src/parser.rs
use crate::ast::{BlockStyle, Doc, Elem, Error, Meta, Node, Scalar};
use crate::lexer::{Lexer, StrTok, Tok};
use crate::parser_helpers::{
    apply_chomping, classify_scalar, classify_text_as_scalar, fold_text,
    strip_space_before_colon,
};
use crate::util::is_plain_scalar_char;

/// Tracks whether a parsed value is inline (on same line as key) or block (nested structure)
#[derive(Debug, Clone, Copy, PartialEq)]
enum ValueType {
    /// Value is inline on the same line (e.g., "key: value")
    Inline,
    /// Value is a nested block structure (e.g., "key:\n  - item")
    Block,
}

pub struct Parser<'a> {
    lx: Lexer<'a>,
    look: Tok,
    /// Accumulates full-line comments to attach to the next produced Elem.
    pending_leading_comments: Vec<String>,
    /// Tag handles defined by %TAG directives for the current document
    tag_handles: Vec<(String, String)>,
    /// Anchors defined in the current document (for per-document scope isolation)
    document_anchors: std::collections::HashSet<String>,
}

impl<'a> Parser<'a> {
    fn sees_indented_doc_marker(&mut self) -> bool {
        // Build the full visible line starting with the current token's raw text,
        // then append the remainder of the physical line.
        let mut lx2 = self.lx.clone();
        let mut line = String::new();

        // 1) Prepend the current token's raw textual form.
        match &self.look {
            Tok::Dash => {
                // "- " means list item; never a doc marker.
                return false;
            }
            Tok::Minus => {
                // A single '-' without following space (part of "----", "x-1", etc.)
                line.push('-');
            }
            Tok::Str(StrTok {
                text,
                quoted: false,
            }) => {
                // If the first token is a plain word (could be '---' or '...'),
                // include its text so we see the true start of the line.
                line.push_str(text);
            }
            Tok::DocStart | Tok::DocEnd => {
                // Shouldn’t happen at nonzero indent (lexer only emits at indent 0),
                // but treat as marker if we do see it.
                return true;
            }
            // Other tokens don't start a doc marker; leave 'line' empty here.
            _ => {}
        }

        // 2) Append the rest of this physical line from the cloned lexer.
        while let Some(ch) = lx2.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            line.push(ch);
            lx2.bump();
        }

        // 3) Normalize and decide.
        // Never treat list items as markers (“- ---”, “- ...”).
        let t0 = line.trim_start_matches(|c| c == ' ' || c == '\t');
        if t0.starts_with("- ") {
            return false;
        }

        // Accept exactly '---' or '...' optionally followed by ws and/or an inline comment.
        let (is_marker, rest) = if t0 == "---" || t0.starts_with("--- ") || t0.starts_with("---\t")
        {
            (true, &t0[3..])
        } else if t0 == "..." || t0.starts_with("... ") || t0.starts_with("...\t") {
            (true, &t0[3..])
        } else {
            (false, "")
        };

        if !is_marker {
            return false;
        }
        let rest = rest.trim_start_matches(|c| c == ' ' || c == '\t');
        rest.is_empty() || rest.starts_with('#')
    }

    pub fn new(s: &'a str) -> Self {
        let mut lx = Lexer::new(s);
        let look = lx.next_token();
        Self {
            lx,
            look,
            pending_leading_comments: Vec::new(),
            tag_handles: Vec::new(),
            document_anchors: std::collections::HashSet::new(),
        }
    }
    fn bump(&mut self) {
        self.look = self.lx.next_token();
    }

    /// Canonicalize a key for duplicate detection.
    /// Quoted and unquoted representations of the same string should be equal.
    fn canonicalize_key(key: &str) -> String {
        // For YAML 1.2, canonical form removes quotes and normalizes whitespace
        // For now, we just remove surrounding quotes
        key.trim_matches('"').trim_matches('\'').to_string()
    }

    /// Serialize a complex key (flow sequence or mapping) to a string for use as a map key.
    /// This creates a canonical string representation of the structure.
    fn serialize_complex_key_to_string(&self, node: &Node) -> Result<String, Error> {
        match node {
            Node::Seq(items) => {
                let mut parts = Vec::new();
                for item in items {
                    let s = self.node_to_string(&item.node)?;
                    parts.push(s);
                }
                Ok(format!("[{}]", parts.join(", ")))
            }
            Node::Map(entries) => {
                let mut parts = Vec::new();
                for (k, v) in entries {
                    let v_str = self.node_to_string(&v.node)?;
                    parts.push(format!("{}: {}", k, v_str));
                }
                Ok(format!("{{{}}}", parts.join(", ")))
            }
            _ => self.node_to_string(node),
        }
    }

    /// Convert a node to its string representation.
    fn node_to_string(&self, node: &Node) -> Result<String, Error> {
        match node {
            Node::Scalar(s) => {
                // Convert Scalar to string
                let s_str = match s {
                    Scalar::Str(s) => s.clone(),
                    Scalar::Bool(b) => b.to_string(),
                    Scalar::Num { text, .. } => text.clone(),
                    Scalar::Null => "null".to_string(),
                };
                Ok(s_str)
            }
            Node::Seq(items) => {
                let mut parts = Vec::new();
                for item in items {
                    parts.push(self.node_to_string(&item.node)?);
                }
                Ok(format!("[{}]", parts.join(", ")))
            }
            Node::Map(entries) => {
                let mut parts = Vec::new();
                for (k, v) in entries {
                    let v_str = self.node_to_string(&v.node)?;
                    parts.push(format!("{}: {}", k, v_str));
                }
                Ok(format!("{{{}}}", parts.join(", ")))
            }
            Node::Alias(name) => Ok(format!("*{}", name)),
        }
    }

    /// Check if current token is an error and convert to parse error
    fn check_error(&self) -> Result<(), Error> {
        if let Tok::Error(msg) = &self.look {
            return Err(Error::Parse(msg.clone()));
        }
        Ok(())
    }

    // /// Register an anchor in the current document scope
    // #[allow(dead_code)]
    // fn register_anchor(&mut self, anchor: &str) {
    //     self.document_anchors.insert(anchor.to_string());
    // }

    /// Collect all anchors from an element tree into the document scope
    fn collect_document_anchors(&mut self, elem: &Elem) {
        if let Some(ref anchor) = elem.meta.anchor {
            self.document_anchors.insert(anchor.clone());
        }

        match &elem.node {
            Node::Map(entries) => {
                for (_, v) in entries {
                    self.collect_document_anchors(v);
                }
            }
            Node::Seq(items) => {
                for item in items {
                    self.collect_document_anchors(item);
                }
            }
            _ => {}
        }
    }

    /// Check that there are no additional documents after the current position.
    /// Used by from_str() to ensure single-document input.
    // #[allow(dead_code)]
    // pub fn check_no_trailing_documents(&mut self) -> Result<(), Error> {
    //     // Skip any trailing whitespace/comments
    //     while matches!(self.look, Tok::Newline | Tok::Comment(_)) {
    //         self.bump();
    //     }

    //     // If we see another DocStart, that means there's a second document
    //     if matches!(self.look, Tok::DocStart) {
    //         return Err(Error::Parse(
    //             "input contains multiple documents; use from_stream() for multi-document input".into()
    //         ));
    //     }

    //     Ok(())
    // }
    pub fn parse_stream(&mut self) -> Result<Vec<Doc>, Error> {
        let mut docs = Vec::new();
        loop {
            // skip inter-doc whitespace/comments
            while matches!(self.look, Tok::Newline | Tok::Comment(_)) {
                self.bump();
                self.check_error()?; // Check for lexer errors
            }

            self.check_error()?; // Check for lexer errors
            match self.look {
                Tok::Eof => break,

                // ignore stray '...' between docs (legal stream separator)
                Tok::DocEnd => {
                    self.bump();
                    continue;
                }

                // For anything else (including Tok::DocStart), parse a document.
                _ => {
                    let d = self.parse_document()?;
                    docs.push(d);
                    // loop will clean up to next doc or EOF
                }
            }
        }
        Ok(docs)
    }

    pub fn parse_document(&mut self) -> Result<Doc, Error> {
        // 1) First, consume *all* leading blank lines (LF/CRLF/CR).
        let mut had_leading_blank = false;
        while matches!(self.look, Tok::Newline) {
            had_leading_blank = true;
            self.bump();
        }

        // 2) Parse directives and doc start if present.
        let mut yaml_version: Option<(u8, u8)> = None;
        self.tag_handles.clear(); // Clear any tag handles from previous document
        self.document_anchors.clear(); // Clear anchors from previous document (per-document scope)
        let mut explicit_start = false;
        let mut explicit_end = false; // if this doc is terminated by '...'

        // Read possible `%YAML` / `%TAG` lines that appear before content or `---`
        'dir_loop: loop {
            // Eat blank lines between directives (allowed by spec)
            while matches!(self.look, Tok::Newline) {
                self.bump();
            }

            // Check if we have a non-zero indent
            let indent_level = if let Tok::Indent(n) = self.look {
                n
            } else {
                0
            };

            // Peek the remainder of the current physical line (without consuming tokens).
            // If the current look is Indent(0), the lexer's char index is already after spaces,
            // so peeking here sees the first non-space char of this line.
            let mut lx2 = self.lx.clone();
            let mut buf = String::new();
            while let Some(ch) = lx2.peek() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
                buf.push(ch);
                lx2.bump();
            }
            let line = buf.trim();

            // Directives cannot be indented
            if (line.starts_with("%YAML ") || line.starts_with("%TAG ")) && indent_level > 0 {
                return Err(Error::Parse("directives cannot be indented".into()));
            }

            // Not a directive? Stop scanning; DO NOT consume the Indent(0) token.
            if !(line.starts_with("%YAML ") || line.starts_with("%TAG ")) {
                break 'dir_loop;
            }

            // It is a directive; now consume tokens up to EOL in the *main* lexer,
            // which will include a leading Indent(0) token if present.
            while !matches!(self.look, Tok::Newline | Tok::Eof) {
                self.bump();
            }
            if matches!(self.look, Tok::Newline) {
                self.bump();
            }

            // Parse out the directive we just consumed, using `line`.
            if line.starts_with("%YAML ") {
                // Check for duplicate YAML directive
                if yaml_version.is_some() {
                    return Err(Error::Parse("duplicate %YAML directive".into()));
                }
                if let Some(ver) = line.split_whitespace().nth(1) {
                    let mut it = ver.split('.');
                    if let (Some(maj), Some(min)) = (it.next(), it.next()) {
                        if let (Ok(ma), Ok(mi)) = (maj.parse::<u8>(), min.parse::<u8>()) {
                            yaml_version = Some((ma, mi));
                        } else {
                            return Err(Error::Parse("invalid %YAML version".into()));
                        }
                    }
                }
            } else {
                // %TAG !h! prefix
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let handle = parts[1];
                    // Validate handle format: must start and end with '!'
                    if !handle.starts_with('!') || !handle.ends_with('!') {
                        return Err(Error::Parse(format!("invalid tag handle format: {}", handle)));
                    }
                    // Check for duplicate tag handle
                    if self.tag_handles.iter().any(|(h, _)| h == handle) {
                        return Err(Error::Parse(format!("duplicate tag handle {}", handle)));
                    }
                    self.tag_handles.push((parts[1].to_string(), parts[2].to_string()));
                } else {
                    return Err(Error::Parse("invalid %TAG directive".into()));
                }
            }

            // Loop to allow more directives
        }

        // 3) First, gather any leading comments before document start marker
        while let Tok::Comment(c) = &self.look {
            self.pending_leading_comments.push(c.clone());
            self.bump();
            while matches!(self.look, Tok::Newline) {
                self.bump();
            }
        }

        // Optionally a `---` after directives/comments
        if matches!(self.look, Tok::DocStart) {
            explicit_start = true;
            self.bump();
            // After ---, we can only have:
            // 1. Newline (content on next line)
            // 2. Pipe/Gt (block scalar on same line)
            // 3. EOF
            // 4. Comment (TODO: handle this)
            // Anything else is invalid (e.g., "--- key: value" is not allowed)
            match &self.look {
                Tok::Newline => {
                    self.bump();
                }
                Tok::Pipe | Tok::Gt | Tok::Eof => {
                    // These are allowed after --- on same line
                }
                _ => {
                    return Err(Error::Parse(
                        "document start marker '---' must be followed by newline or block scalar indicator".into()
                    ));
                }
            }
        }

        // Still in parse_document(), right after the version check above.
        if explicit_start {
            // Peek the current line without consuming tokens.
            let mut lx2 = self.lx.clone();
            let mut buf = String::new();
            while let Some(ch) = lx2.peek() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
                buf.push(ch);
                lx2.bump();
            }
            let line = buf.trim();
            if line.starts_with("%YAML ") || line.starts_with("%TAG ") {
                return Err(Error::Parse(
                    "directives (%YAML/%TAG) must appear before document start '---'".into(),
                ));
            }
        }

        // After optional '---' handling, still in parse_document():
        if let Some((maj, min)) = yaml_version {
            if (maj, min) != (1, 2) {
                return Err(Error::Parse(
                    "unsupported %YAML version; only 1.2 is supported".into(),
                ));
            }
        }

        // 4) Parse the root node
        // Check for empty document: if we see DocStart, DocEnd, or Eof immediately, the document is empty
        let root = if matches!(self.look, Tok::DocStart | Tok::DocEnd | Tok::Eof) {
            // Empty document - use empty map as root
            Elem::new(Node::Map(vec![]))
        } else {
            self.parse_elem_at_indent(0)?
        };

        // 5) End marker `...` optionally terminates the document
        //    (Lexer yields Tok::DocEnd at line start; eat it and any trailing newline)
        //    If not present, we'll just consume to EoF or next doc's start.
        if matches!(self.look, Tok::Newline) {
            self.bump();
        }
        if matches!(self.look, Tok::DocEnd) {
            explicit_end = true;
            self.bump();
            // Document end marker must be followed by newline, EOF, or comment only
            match &self.look {
                Tok::Newline => {
                    self.bump();
                }
                Tok::Eof | Tok::Comment(_) => {
                    // These are allowed after ... on same line
                }
                _ => {
                    return Err(Error::Parse(
                        "document end marker '...' must be followed by newline or end of file".into()
                    ));
                }
            }
        }
        // For stream parsing, leave the remaining tokens (which may include another DocStart) to caller.

        // 6) Validate that we don't have unexpected tokens after the document
        // Skip trailing newlines and comments
        while matches!(self.look, Tok::Newline | Tok::Comment(_)) {
            self.bump();
        }

        // Now we should only see EOF or DocStart (for stream parsing)
        match &self.look {
            Tok::Eof => {
                // OK - end of single document
            }
            Tok::DocStart => {
                // Another document follows - this is only valid in stream parsing
                // For single document parsing (from_str), this should be an error
                // But since parse_stream() uses this method, we accept it here
                // The caller (from_str vs from_stream) handles this appropriately
            }
            Tok::RBracket => {
                return Err(Error::Parse("unexpected ']' after document".into()));
            }
            Tok::RBrace => {
                return Err(Error::Parse("unexpected '}' after document".into()));
            }
            Tok::Error(msg) => {
                return Err(Error::Parse(msg.clone()));
            }
            _ => {
                // Allow other tokens for now (might be part of multi-doc stream)
            }
        }

        // Collect all anchors from the parsed document into document scope
        self.collect_document_anchors(&root);


        // Validate that all aliases reference anchors defined in THIS document only
        Self::check_aliases(&root, &self.document_anchors)?;

        Ok(Doc {
            root,
            had_leading_blank,
            saw_carriage_return: self.lx.saw_cr, // propagate
            yaml_version,
            tag_handles: self.tag_handles.clone(),
            explicit_start,
            explicit_end,
        })
    }

    fn check_aliases(elem: &Elem, anchors: &std::collections::HashSet<String>) -> Result<(), Error> {
        match &elem.node {
            Node::Alias(name) => {
                if !anchors.contains(name) {
                    return Err(Error::Parse(format!("undefined alias: *{}", name)));
                }
                Ok(())
            }
            Node::Map(entries) => {
                for (_, v) in entries {
                    Self::check_aliases(v, anchors)?;
                }
                Ok(())
            }
            Node::Seq(items) => {
                for item in items {
                    Self::check_aliases(item, anchors)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn parse_elem_at_indent(&mut self, base: usize) -> Result<Elem, Error> {
        // When we produce an Elem, we must attach .meta.leading_comments from buffer and clear it.
        let mut elem = match self.look.clone() {
            Tok::TabIndent => {
                return Err(Error::Parse("tabs are not allowed for indentation".into()));
            }
            Tok::Indent(n) if n >= base => self.parse_block(base)?,
            Tok::LBracket => {
                let node = self.parse_inline_array_node()?;
                Elem::new(node)
            }
            Tok::LBrace => {
                let node = self.parse_inline_map_node()?;
                Elem::new(node)
            }
            Tok::Str(s) => {
                self.bump();
                let mut el = Elem::new(Node::Scalar(classify_scalar(&s)));
                el.meta.prefer_quoted = s.quoted; // remember original quoting
                el
            }
            Tok::Star => {
                // alias at root position
                self.bump();
                let name = self.parse_alias_name()?;
                Elem::new(Node::Alias(name))
            }
            Tok::Newline => {
                self.bump();
                return self.parse_elem_at_indent(base);
            }
            Tok::Comment(c) => {
                // just in case a comment leaked here
                self.pending_leading_comments.push(c);
                self.bump();
                return self.parse_elem_at_indent(base);
            }
            Tok::Comma => {
                self.bump();
                return self.parse_elem_at_indent(base);
            }
            Tok::DocStart | Tok::DocEnd => {
                return Err(Error::Parse("document marker not allowed here".into()));
            }
            Tok::Pipe | Tok::Gt => {
                // Block scalar at document root (e.g., after `---` on same line)
                self.parse_block_scalar(base)?
            }
            Tok::Tag(tag_text) => {
                // Tag before value: parse tag, then recursively parse the value
                self.bump();
                let mut elem = self.parse_elem_at_indent(base)?;
                elem.meta.tag = Some(tag_text);
                return Ok(elem);
            }
            Tok::Eof => Elem::new(Node::Map(vec![])),
            other => {
                return Err(Error::Parse(format!(
                    "unexpected token at indent {base}: {:?}",
                    other
                )));
            }
        };
        if !self.pending_leading_comments.is_empty() {
            elem.meta.leading_comments = std::mem::take(&mut self.pending_leading_comments);
        }
        Ok(elem)
    }

    fn parse_block(&mut self, base: usize) -> Result<Elem, Error> {
        // Parse either a Map or Seq block.
        let mut entries: Vec<(String, Elem)> = vec![];
        let mut items: Vec<Elem> = vec![];
        let (mut saw_map, mut saw_seq) = (false, false);
        // Track canonical keys to detect duplicates
        let mut canonical_keys: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        // Track last entry's indent and value type for ill-formed structure detection
        let mut last_entry_indent: Option<usize> = None;
        let mut last_value_type: Option<ValueType> = None;
        // Capture leading comments that appear before any content (belong to the container itself)
        // Only take pending comments for top-level blocks (base == 0), as nested blocks should
        // have their leading comments attached to the first child
        let mut block_leading_comments: Vec<String> = if base == 0 {
            std::mem::take(&mut self.pending_leading_comments)
        } else {
            vec![]
        };

        'block: loop {
            match self.look.clone() {
                Tok::TabIndent => {
                    return Err(Error::Parse("tabs are not allowed for indentation".into()));
                }
                Tok::Indent(n) if n < base => break, // out of this block
                Tok::Indent(n) => {
                    self.bump(); // consume Indent
                    // capture any full-line comments before this item
                    // Note: we only reach here if n >= base, so comments are at the right level
                    while let Tok::Comment(c) = &self.look {
                        // If we haven't seen any content yet, these comments belong to the block itself
                        if !saw_map && !saw_seq {
                            block_leading_comments.push(c.clone());
                        } else {
                            self.pending_leading_comments.push(c.clone());
                        }
                        self.bump();
                    }

                    // Single, deduped guard: if this line begins with a *standalone* '---'/'...' at
                    // nonzero indent, reject it. This compensates for the lexer emitting marker
                    // tokens only at indent 0 (see `sees_indented_doc_marker()` docs).
                    if base > 0 && self.sees_indented_doc_marker() {
                        return Err(Error::Parse(
                            "document marker not allowed inside block".into(),
                        ));
                    }

                    match self.look.clone() {
                        Tok::Dash => {
                            // `Tok::Dash` means the lexer saw `- ` (dash+space). It's definitely a list item.
                            // Check for ill-formed: sequence after inline map value at greater indent
                            if let (Some(last_indent), Some(ValueType::Inline)) = (last_entry_indent, last_value_type) {
                                if saw_map && !saw_seq && n > last_indent {
                                    return Err(Error::Parse(
                                        "improper indentation: sequence cannot follow an inline map value at deeper indentation".into()
                                    ));
                                }
                            }
                            saw_seq = true;
                            self.bump(); // consume Tok::Dash

                            let mut child = self.parse_line_value_or_nested(n + 2)?;
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments =
                                    std::mem::take(&mut self.pending_leading_comments);
                            }
                            items.push(child);
                        }

                        Tok::Str(StrTok {
                            text: key,
                            quoted: quoted_key,
                        }) => {
                            // Check for directives appearing after content has started
                            if key == "%YAML" || key == "%TAG" {
                                return Err(Error::Parse(
                                    "directives must appear before document content".into(),
                                ));
                            }

                            // Could be "key:" or a standalone scalar line.
                            self.bump();

                            if matches!(self.look, Tok::Colon) {
                                // Map entry: key:
                                self.bump(); // ':'
                                saw_map = true;
                                let (mut value, vtype) = self.parse_value_after_colon(n + 2)?;
                                if !self.pending_leading_comments.is_empty() {
                                    value.meta.leading_comments =
                                        std::mem::take(&mut self.pending_leading_comments);
                                }

                                // Check for duplicate keys using canonical comparison
                                let canonical_key = Self::canonicalize_key(&key);
                                if let Some(&idx) = canonical_keys.get(&canonical_key) {
                                    // Duplicate key - replace with last value (last wins)
                                    entries[idx] = (key, value);
                                } else {
                                    // New key - add to map and track index
                                    canonical_keys.insert(canonical_key, entries.len());
                                    entries.push((key, value));
                                }

                                // Track this entry's indent and value type for ill-formed structure detection
                                last_entry_indent = Some(n);
                                last_value_type = Some(vtype);
                            } else {
                                // Not followed by ':'. If this is the first thing at this block level,
                                // and we're exactly at the base indent, and the token was QUOTED,
                                // interpret it as a top-level (or block-root) scalar doc, not a seq item.
                                if n == base && !saw_map && !saw_seq {
                                    let mut elem =
                                        Elem::new(Node::Scalar(classify_scalar(&StrTok {
                                            text: key,
                                            quoted: quoted_key, // preserve
                                        })));
                                    // Attach block-level comments first, then any pending comments
                                    if !block_leading_comments.is_empty() {
                                        elem.meta.leading_comments = block_leading_comments.clone();
                                    }
                                    if !self.pending_leading_comments.is_empty() {
                                        elem.meta.leading_comments.extend(std::mem::take(&mut self.pending_leading_comments));
                                    }
                                    // Remember original quoting; unquoted will stay unquoted.
                                    elem.meta.prefer_quoted = quoted_key;
                                    self.consume_trailing_comment_into(&mut elem.meta);
                                    return Ok(elem);
                                }

                                // Otherwise, treat it as a seq item (fallback).
                                saw_seq = true;
                                let mut child =
                                    Elem::new(Node::Scalar(classify_scalar(&StrTok {
                                        text: key,
                                        quoted: quoted_key, // preserve
                                    })));
                                if !self.pending_leading_comments.is_empty() {
                                    child.meta.leading_comments =
                                        std::mem::take(&mut self.pending_leading_comments);
                                }
                                self.consume_trailing_comment_into(&mut child.meta);
                                items.push(child);
                            }
                        }
                        Tok::LBracket => {
                            // Flow sequence at block level - treat as root if first thing
                            let mut child = Elem::new(self.parse_inline_array_node()?);
                            // Attach block-level comments first, then any pending comments
                            if !block_leading_comments.is_empty() {
                                child.meta.leading_comments = block_leading_comments.clone();
                            }
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments.extend(std::mem::take(&mut self.pending_leading_comments));
                            }
                            // If this is the first thing at this block level, it's the block's value
                            if n == base && !saw_map && !saw_seq {
                                self.consume_trailing_comment_into(&mut child.meta);
                                return Ok(child);
                            }
                            // Otherwise we're in a sequence context
                            self.consume_trailing_comment_into(&mut child.meta);
                            saw_seq = true;
                            items.push(child);
                        }
                        Tok::LBrace => {
                            // Flow mapping at block level - treat as root if first thing
                            let mut child = Elem::new(self.parse_inline_map_node()?);
                            // Attach block-level comments first, then any pending comments
                            if !block_leading_comments.is_empty() {
                                child.meta.leading_comments = block_leading_comments.clone();
                            }
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments.extend(std::mem::take(&mut self.pending_leading_comments));
                            }
                            // If this is the first thing at this block level, it's the block's value
                            if n == base && !saw_map && !saw_seq {
                                self.consume_trailing_comment_into(&mut child.meta);
                                return Ok(child);
                            }
                            // Otherwise we're in a sequence context
                            self.consume_trailing_comment_into(&mut child.meta);
                            saw_seq = true;
                            items.push(child);
                        }
                        Tok::Star => {
                            saw_seq = true;
                            self.bump();
                            let name = self.parse_alias_name()?;
                            let mut child = Elem::new(Node::Alias(name));
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments =
                                    std::mem::take(&mut self.pending_leading_comments);
                            }
                            self.consume_trailing_comment_into(&mut child.meta);
                            items.push(child);
                        }
                        Tok::Pipe | Tok::Gt => {
                            // Block scalar header at this indent
                            let mut child = self.parse_block_scalar(n)?;
                            // Attach block-level comments first, then any pending comments
                            if !block_leading_comments.is_empty() {
                                child.meta.leading_comments = block_leading_comments.clone();
                            }
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments.extend(std::mem::take(&mut self.pending_leading_comments));
                            }
                            // If this is the first thing at this block level, it's the block's value.
                            if n == base && !saw_map && !saw_seq {
                                self.consume_trailing_comment_into(&mut child.meta);
                                return Ok(child);
                            }
                            // Otherwise we're in a sequence context — keep as seq item.
                            self.consume_trailing_comment_into(&mut child.meta);
                            saw_seq = true;
                            items.push(child);
                        }
                        Tok::Newline => {
                            self.bump();
                        }
                        Tok::Comma => {
                            self.bump(); /* tolerate stray commas between items */
                        }
                        Tok::DocStart | Tok::DocEnd => {
                            // If a doc marker shows up while we’re parsing a block:
                            // - at top level (base == 0): end this document's content and let caller handle the marker
                            // - nested: it's illegal
                            if base == 0 {
                                break 'block; // do NOT consume; let parse_document/parse_stream see it
                            } else {
                                return Err(Error::Parse(
                                    "document marker not allowed inside block".into(),
                                ));
                            }
                        }
                        // This one existed but warned; change `c` -> `_c`
                        Tok::Comment(_c) => {
                            self.pending_leading_comments.push(_c.to_string());
                            self.bump();
                        }
                        other => {
                            // Fallback: collect a single-line plain scalar.
                            let s =
                                self.parse_inline_scalar_line(/* glue_after_colon = */ true);
                            let mut child = Elem::new(Node::Scalar(Scalar::Str(s)));
                            // Attach block-level comments first, then any pending comments
                            if !block_leading_comments.is_empty() {
                                child.meta.leading_comments = block_leading_comments.clone();
                            }
                            if !self.pending_leading_comments.is_empty() {
                                child.meta.leading_comments.extend(std::mem::take(&mut self.pending_leading_comments));
                            }
                            self.consume_trailing_comment_into(&mut child.meta);

                            // NEW: if this is the FIRST thing at this block level, interpret it as
                            // the block's scalar *value* (not a seq item).
                            if n == base && !saw_map && !saw_seq {
                                return Ok(child);
                            }

                            // Otherwise we're already in a sequence context — keep treating as seq item.
                            let _ = other;
                            saw_seq = true;
                            items.push(child);
                        }
                    }
                }
                Tok::Newline => {
                    self.bump();
                    continue;
                } // skip blank lines inside block
                Tok::Comment(c) => {
                    // full-line comment between entries
                    self.pending_leading_comments.push(c);
                    self.bump();

                    // For nested blocks, peek ahead to decide if we should continue or break
                    if base > 0 {
                        // Look at the next token after this comment (and any newlines)
                        loop {
                            match &self.look {
                                Tok::Newline => {
                                    self.bump();
                                    continue;
                                }
                                Tok::Indent(n) if *n < base => {
                                    // The next content is at a shallower indent - this comment belongs to outer block
                                    // Clear it from pending and break
                                    self.pending_leading_comments.pop();
                                    break 'block;
                                }
                                _ => {
                                    // Either an Indent at our level/deeper, or some other token
                                    break;
                                }
                            }
                        }
                    }
                    continue;
                }
                Tok::DocStart | Tok::DocEnd => {
                    // Marker at beginning of a physical line (no Indent token for this line yet).
                    // If we’re nested, treat it as end-of-this-block and let the caller (outer level)
                    // handle the document marker. Do NOT consume it here.
                    break 'block;
                }
                _ => break, // anything else ends this block
            }
        }

        let mut result = if saw_map && !saw_seq {
            Elem::new(Node::Map(entries))
        } else if saw_seq && !saw_map {
            Elem::new(Node::Seq(items))
        } else if saw_map {
            Elem::new(Node::Map(entries))
        } else {
            Elem::new(Node::Seq(items))
        };

        // Attach comments that appeared before any content to the block itself
        if !block_leading_comments.is_empty() {
            result.meta.leading_comments = block_leading_comments;
        }

        Ok(result)
    }

    fn parse_value_after_colon(&mut self, nested_indent: usize) -> Result<(Elem, ValueType), Error> {
        // Check for lexer errors first
        if let Tok::Error(msg) = &self.look {
            return Err(Error::Parse(msg.clone()));
        }

        // Track whether the value is inline or block
        let value_type = ValueType::Inline; // Default to inline, update if we see block content

        // Handle scalars, inline arrays, block scalars, nested blocks, anchors, and tags.
        // Optional tags and anchors can precede any node.
        let mut meta = Meta::default();

        // Parse optional tag: !!str, !tag, !handle!suffix, or !<verbatim>
        if let Tok::Tag(tag_text) = &self.look {
            // Validate tag handle if it's a named handle (e.g., !handle!suffix)
            // Built-in handles: ! (primary) and !! (secondary)
            if tag_text.starts_with("!!") {
                // Secondary tag handle - always valid (maps to tag:yaml.org,2002:)
            } else if tag_text.starts_with("!<") && tag_text.ends_with('>') {
                // Verbatim tag - always valid
            } else if tag_text.len() > 1 && tag_text[1..].contains('!') {
                // Named handle like !handle!suffix
                // Extract the handle part (everything from first ! to second !)
                if let Some(end_pos) = tag_text[1..].find('!') {
                    let handle = &tag_text[..=end_pos + 1]; // Include both ! characters
                    // Check if this handle is defined
                    if !self.tag_handles.iter().any(|(h, _)| h == handle) {
                        return Err(Error::Parse(format!("undefined tag handle: {}", handle)));
                    }
                }
            }
            // Primary tag handle ! is always valid
            // Store and consume the tag
            meta.tag = Some(tag_text.clone());
            self.bump();
        }

        // collect inline anchors if present: &name (multiple allowed; we keep the last)
        if matches!(self.look, Tok::Amp) {
            while let Tok::Amp = self.look {
                self.bump();
                if let Tok::Str(StrTok { text, .. }) = &self.look {
                    meta.anchor = Some(text.clone());
                    self.bump();
                } else {
                    return Err(Error::Parse("expected anchor name after '&'".into()));
                }
            }
        }

        let mut elem = match self.look.clone() {
            Tok::DocStart | Tok::DocEnd => {
                return Err(Error::Parse(
                    "document marker not allowed as a value".into(),
                ));
            }
            Tok::Str(s) => {
                self.bump();
                // Start with the first token, then (if unquoted) append the rest of the line.
                let mut text = s.text.clone();
                if !s.quoted {
                    let rest = self.parse_inline_scalar_line(/* glue_after_colon = */ false);
                    if !rest.is_empty() {
                        let r = rest.trim_start();
                        let needs_space = !text.is_empty()
                            && !(r.starts_with(':') || r.starts_with('-') || r.starts_with('+'));
                        if needs_space {
                            text.push(' ');
                        }
                        text.push_str(&rest);
                    }
                }

                // normalize *once* right before classifying
                strip_space_before_colon(&mut text);

                let scalar = classify_text_as_scalar(&text, s.quoted);
                let mut e = Elem {
                    node: Node::Scalar(scalar),
                    meta,
                };
                e.meta.prefer_quoted = s.quoted;
                e
            }
            Tok::LBracket => {
                let node = self.parse_inline_array_node()?;
                Elem { node, meta }
            }
            Tok::LBrace => {
                let node = self.parse_inline_map_node()?;
                Elem { node, meta }
            }
            Tok::RBracket => {
                return Err(Error::Parse("unexpected ']' without matching '['".into()));
            }
            Tok::RBrace => {
                return Err(Error::Parse("unexpected '}' without matching '{'".into()));
            }
            Tok::Pipe | Tok::Gt => {
                // Block scalar content must be indented relative to the key line, not nested content
                let mut e = self.parse_block_scalar(nested_indent.saturating_sub(2))?;
                // If we saw tag/anchor before the block scalar header, preserve them on the element.
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                return Ok((e, ValueType::Block)); // Block scalar
            }
            Tok::Star => {
                self.bump();
                let name = self.parse_alias_name()?;
                Elem {
                    node: Node::Alias(name),
                    meta,
                }
            }
            Tok::Comment(c) => {
                // Trailing comment after colon - consume it and check for nested content
                meta.trailing_comment = Some(c.trim().to_string());
                self.bump(); // consume the comment
                // Now check what follows - should be Newline or EOF
                match self.look.clone() {
                    Tok::Newline => {
                        // Fall through to Newline handler
                        self.bump();
                        loop {
                            match self.look.clone() {
                                Tok::Indent(n) if n >= nested_indent => {
                                    if self.sees_indented_doc_marker() {
                                        return Err(Error::Parse(
                                            "document marker not allowed inside block".into(),
                                        ));
                                    }
                                    let mut child = self.parse_elem_at_indent(nested_indent)?;
                                    if meta.tag.is_some() && child.meta.tag.is_none() {
                                        child.meta.tag = meta.tag.take();
                                    }
                                    if meta.anchor.is_some() && child.meta.anchor.is_none() {
                                        child.meta.anchor = meta.anchor.take();
                                    }
                                    if meta.trailing_comment.is_some() && child.meta.trailing_comment.is_none() {
                                        child.meta.trailing_comment = meta.trailing_comment.take();
                                    }
                                    return Ok((child, ValueType::Block)); // Nested block content
                                }
                                Tok::Comment(c) => {
                                    self.pending_leading_comments.push(c);
                                    self.bump();
                                }
                                Tok::Newline => {
                                    self.bump();
                                }
                                Tok::DocStart | Tok::DocEnd => {
                                    return Err(Error::Parse(
                                        "document marker not allowed inside block".into(),
                                    ));
                                }
                                _ => break,
                            }
                        }
                        // Empty scalar after newline with no nested content
                        return Ok((Elem {
                            node: Node::Scalar(Scalar::Str(String::new())),
                            meta,
                        }, ValueType::Inline));
                    }
                    _ => {
                        // Comment at end of file or before other token
                        return Ok((Elem {
                            node: Node::Scalar(Scalar::Str(String::new())),
                            meta,
                        }, ValueType::Inline));
                    }
                }
            }
            Tok::Newline => {
                self.bump();
                loop {
                    match self.look.clone() {
                        Tok::Indent(n) if n >= nested_indent => {
                            // DO NOT self.bump() here

                            // Same strict guard as parse_block():
                            if self.sees_indented_doc_marker() {
                                return Err(Error::Parse(
                                    "document marker not allowed inside block".into(),
                                ));
                            }

                            let mut child = self.parse_elem_at_indent(nested_indent)?;
                            if meta.tag.is_some() && child.meta.tag.is_none() {
                                child.meta.tag = meta.tag.take();
                            }
                            if meta.anchor.is_some() && child.meta.anchor.is_none() {
                                child.meta.anchor = meta.anchor.take();
                            }
                            return Ok((child, ValueType::Block)); // Nested block content
                        }
                        Tok::Comment(c) => {
                            self.pending_leading_comments.push(c);
                            self.bump();
                        }
                        Tok::Newline => {
                            self.bump();
                        }
                        Tok::DocStart | Tok::DocEnd => {
                            return Err(Error::Parse(
                                "document marker not allowed inside block".into(),
                            ));
                        }
                        _ => break,
                    }
                }
                // Empty scalar after newline with no nested content
                return Ok((Elem {
                    node: Node::Scalar(Scalar::Str(String::new())),
                    meta,
                }, ValueType::Inline));
            }

            Tok::Eof => Elem {
                node: Node::Scalar(Scalar::Str(String::new())),
                meta,
            },
            Tok::Comma => {
                // treat as end-of-value separator on the same line
                self.bump();
                let mut e = Elem::new(Node::Scalar(Scalar::Str(String::new())));
                self.consume_trailing_comment_into(&mut e.meta);
                return Ok((e, ValueType::Inline));
            }
            Tok::Plus | Tok::Minus => {
                // Value begins with an explicit sign; collect the rest of the line then classify.
                let sign = if matches!(self.look, Tok::Plus) {
                    '+'
                } else {
                    '-'
                };
                self.bump();
                let mut text = String::new();
                text.push(sign);
                let rest = self.parse_inline_scalar_line(/* glue_after_colon = */ true);
                if !rest.is_empty() {
                    // parse_inline_scalar_line already suppresses the space after signs,
                    // so just append.
                    text.push_str(&rest);
                }
                let scalar = classify_text_as_scalar(&text, false);
                Elem {
                    node: Node::Scalar(scalar),
                    meta,
                }
            }
            _ => {
                let s = self.parse_inline_scalar_line(/* glue_after_colon = */ false);
                Elem {
                    node: Node::Scalar(Scalar::Str(s)),
                    meta,
                }
            }
        };

        self.consume_trailing_comment_into(&mut elem.meta);
        // All inline values (scalars, flow collections) return Inline type
        Ok((elem, value_type))
    }

    fn parse_inline_array_node(&mut self) -> Result<Node, Error> {
        // LBracket already current
        self.bump();
        let mut items: Vec<Elem> = vec![];
        let mut current_item: Option<Elem> = None;
        let mut prev_tok = Tok::LBracket;
        let mut current_item_strings = 0; // Track strings in current item
        let mut has_comma = false; // Track if we've seen any commas

        loop {
            match self.look.clone() {
                Tok::DocStart | Tok::DocEnd => {
                    return Err(Error::Parse(
                        "document marker not allowed inside flow sequence".into(),
                    ));
                }
                Tok::TabIndent => {
                    return Err(Error::Parse("tabs are not allowed for indentation".into()));
                }
                Tok::Eof => return Err(Error::Parse("unterminated flow sequence".into())),
                Tok::RBracket => {
                    // Check for trailing comma before closing bracket
                    if matches!(prev_tok, Tok::Comma) {
                        return Err(Error::Parse("trailing comma in flow sequence".into()));
                    }
                    self.bump();
                    // Add final item if any
                    if let Some(item) = current_item.take() {
                        // Check if this item was multi-word without commas
                        if current_item_strings > 1 && !has_comma {
                            return Err(Error::Parse(
                                "flow sequence items must be separated by commas".into(),
                            ));
                        }
                        items.push(item);
                    }
                    break;
                }
                Tok::Comma => {
                    // Add current item to list
                    if let Some(item) = current_item.take() {
                        items.push(item);
                    }
                    has_comma = true; // Mark that we've seen a comma
                    current_item_strings = 0; // Reset for next item
                    self.bump();
                    prev_tok = Tok::Comma;
                }
                Tok::Str(StrTok { text, quoted }) => {
                    self.bump();
                    current_item_strings += 1;

                    // Handle consecutive strings: they form a multi-word scalar
                    if let Some(ref mut item) = current_item {
                        // Append to existing item if previous was also a string
                        if matches!(prev_tok, Tok::Str(_)) {
                            if let Node::Scalar(Scalar::Str(ref mut s)) = item.node {
                                s.push(' ');
                                s.push_str(&text);
                                item.meta.prefer_quoted |= quoted;
                            }
                        } else {
                            // Previous was not a string - this is a new item without comma separator
                            return Err(Error::Parse(
                                "flow sequence items must be separated by commas".into(),
                            ));
                        }
                    } else {
                        // Create new scalar item
                        let scalar = classify_text_as_scalar(&text, quoted);
                        let mut elem = Elem::new(Node::Scalar(scalar));
                        elem.meta.prefer_quoted = quoted;
                        current_item = Some(elem);
                    }
                    prev_tok = Tok::Str(StrTok { text, quoted });
                }
                Tok::LBracket => {
                    // Nested flow sequence
                    let node = self.parse_inline_array_node()?;
                    current_item = Some(Elem::new(node));
                    prev_tok = Tok::RBracket; // Simulate consumed collection
                }
                Tok::LBrace => {
                    // Nested flow mapping
                    let node = self.parse_inline_map_node()?;
                    current_item = Some(Elem::new(node));
                    prev_tok = Tok::RBrace; // Simulate consumed collection
                }
                Tok::RBrace => {
                    return Err(Error::Parse("unexpected '}' in flow sequence".into()));
                }
                Tok::Newline => {
                    self.bump();
                }
                Tok::Indent(_) => {
                    self.bump();
                    // Per YAML 1.2.2 Section 8.2.3: "block styles are not allowed inside flow collections"
                    // Check if block sequence indicator follows indentation
                    if matches!(self.look, Tok::Dash) {
                        return Err(Error::Parse(
                            "block styles are not allowed inside flow collections (YAML 1.2.2 §8.2.3)".into()
                        ));
                    }
                }
                Tok::Comment(_) => {
                    self.bump();
                }
                Tok::Error(msg) => {
                    return Err(Error::Parse(msg));
                }
                Tok::Plus | Tok::Minus => {
                    // Handle explicit sign on numbers like +5 or -3
                    let sign = if matches!(self.look, Tok::Plus) {
                        '+'
                    } else {
                        '-'
                    };
                    self.bump();

                    // Expect a number after the sign
                    match self.look.clone() {
                        Tok::Str(StrTok { text, quoted }) if !quoted => {
                            self.bump();
                            current_item_strings += 1;

                            // Combine sign with number
                            let mut signed_text = String::new();
                            signed_text.push(sign);
                            signed_text.push_str(&text);

                            let scalar = classify_text_as_scalar(&signed_text, false);
                            current_item = Some(Elem::new(Node::Scalar(scalar)));
                            prev_tok = Tok::Str(StrTok { text: signed_text, quoted: false });
                        }
                        _ => {
                            // Sign without number following - treat as plain text
                            let mut text = String::new();
                            text.push(sign);
                            current_item = Some(Elem::new(Node::Scalar(Scalar::Str(text))));
                        }
                    }
                }
                _ => {
                    // Other tokens - skip for now
                    self.bump();
                }
            }
        }

        Ok(Node::Seq(items))
    }

    fn parse_inline_map_node(&mut self) -> Result<Node, Error> {
        // LBrace already current
        self.bump();
        let mut entries: Vec<(String, Elem)> = vec![];
        let mut current_key = String::new();
        let mut current_val: Option<Elem> = None;
        let mut in_value = false;
        let mut has_complex_key = false; // Track if current key is complex (preceded by ?)
        let mut prev_tok = Tok::LBrace;

        loop {
            match self.look.clone() {
                Tok::DocStart | Tok::DocEnd => {
                    return Err(Error::Parse(
                        "document marker not allowed inside flow mapping".into(),
                    ));
                }
                Tok::TabIndent => {
                    return Err(Error::Parse("tabs are not allowed for indentation".into()));
                }
                Tok::Eof => return Err(Error::Parse("unterminated flow mapping".into())),
                Tok::RBrace => {
                    // Check for trailing comma before closing brace
                    if matches!(prev_tok, Tok::Comma) {
                        return Err(Error::Parse("trailing comma in flow mapping".into()));
                    }
                    self.bump();
                    // Process final key-value pair if any
                    let key = current_key.trim().to_string();
                    if !key.is_empty() {
                        if !in_value {
                            return Err(Error::Parse(
                                "flow mapping key without value (missing colon)".into(),
                            ));
                        }
                        // Check for duplicate key
                        if entries.iter().any(|(k, _)| k == &key) {
                            return Err(Error::Parse(format!("duplicate key in flow mapping: '{}'", key)));
                        }
                        let val = current_val.take().unwrap_or_else(|| Elem::new(Node::Scalar(Scalar::Str(String::new()))));
                        entries.push((key, val));
                    }
                    break;
                }
                Tok::Colon => {
                    if in_value && current_val.is_some() {
                        // We already have a value - colon not allowed here
                        return Err(Error::Parse("unexpected colon in flow mapping value".into()));
                    }
                    // Transition from key to value
                    // Empty keys are allowed in YAML 1.2.2
                    in_value = true;
                    self.bump();
                    prev_tok = Tok::Colon;
                }
                Tok::Comma => {
                    if !in_value {
                        return Err(Error::Parse(
                            "flow mapping key without value (missing colon before comma)".into(),
                        ));
                    }
                    let key = current_key.trim().to_string();
                    // Empty keys are allowed in YAML 1.2.2
                    // Check for duplicate key
                    if entries.iter().any(|(k, _)| k == &key) {
                        return Err(Error::Parse(format!("duplicate key in flow mapping: '{}'", key)));
                    }
                    let val = current_val.take().unwrap_or_else(|| Elem::new(Node::Scalar(Scalar::Str(String::new()))));
                    entries.push((key, val));
                    current_key.clear();
                    in_value = false;
                    has_complex_key = false; // Reset for next entry
                    self.bump();
                    prev_tok = Tok::Comma;
                }
                Tok::Str(StrTok { text, quoted }) => {
                    self.bump();

                    if in_value {
                        // Handle consecutive strings in values
                        if let Some(ref mut val) = current_val {
                            // Append if previous was also a string (multi-word scalar)
                            if matches!(prev_tok, Tok::Str(_)) {
                                if let Node::Scalar(Scalar::Str(ref mut s)) = val.node {
                                    s.push(' ');
                                    s.push_str(&text);
                                    val.meta.prefer_quoted |= quoted;
                                }
                            } else {
                                // Previous was not a string - missing comma
                                return Err(Error::Parse(
                                    "flow mapping values must be separated by commas".into(),
                                ));
                            }
                        } else {
                            // Create new scalar value
                            let scalar = classify_text_as_scalar(&text, quoted);
                            let mut elem = Elem::new(Node::Scalar(scalar));
                            elem.meta.prefer_quoted = quoted;
                            current_val = Some(elem);
                        }
                    } else {
                        // Key accumulation - consecutive strings are part of the same key
                        if !current_key.is_empty() {
                            current_key.push(' ');
                        }
                        current_key.push_str(&text);
                    }
                    prev_tok = Tok::Str(StrTok { text, quoted });
                }
                Tok::Question => {
                    // Complex key marker - the next token is a complex key (flow sequence or mapping)
                    if in_value {
                        return Err(Error::Parse("unexpected '?' in flow mapping value".into()));
                    }
                    has_complex_key = true;
                    self.bump();
                    prev_tok = Tok::Question;
                }
                Tok::LBracket => {
                    // Nested flow sequence
                    if !in_value && !has_complex_key {
                        return Err(Error::Parse("flow sequence not allowed as map key (use '?' for complex keys)".into()));
                    }
                    let node = self.parse_inline_array_node()?;
                    if in_value {
                        current_val = Some(Elem::new(node));
                    } else {
                        // Complex key - serialize it to a string
                        current_key = self.serialize_complex_key_to_string(&node)?;
                        has_complex_key = false;
                    }
                    prev_tok = Tok::RBracket; // Simulate that we consumed a collection
                }
                Tok::LBrace => {
                    // Nested flow mapping
                    if !in_value && !has_complex_key {
                        return Err(Error::Parse("flow mapping not allowed as map key (use '?' for complex keys)".into()));
                    }
                    let node = self.parse_inline_map_node()?;
                    if in_value {
                        current_val = Some(Elem::new(node));
                    } else {
                        // Complex key - serialize it to a string
                        current_key = self.serialize_complex_key_to_string(&node)?;
                        has_complex_key = false;
                    }
                    prev_tok = Tok::RBrace; // Simulate that we consumed a collection
                }
                Tok::RBracket => {
                    return Err(Error::Parse("unexpected ']' in flow mapping".into()));
                }
                Tok::Newline => {
                    self.bump();
                }
                Tok::Indent(_) => {
                    self.bump();
                    // Per YAML 1.2.2 Section 8.2.3: "block styles are not allowed inside flow collections"
                    // Check if block sequence indicator follows indentation in value position
                    if in_value && matches!(self.look, Tok::Dash) {
                        return Err(Error::Parse(
                            "block styles are not allowed inside flow collections (YAML 1.2.2 §8.2.3)".into()
                        ));
                    }
                }
                Tok::Comment(_) => {
                    self.bump();
                }
                Tok::Error(msg) => {
                    return Err(Error::Parse(msg));
                }
                _ => {
                    // Other tokens - skip for now
                    self.bump();
                }
            }
        }

        Ok(Node::Map(entries))
    }

    fn parse_block_scalar(&mut self, parent_indent: usize) -> Result<Elem, Error> {
        // Current token is '|' or '>'
        let literal = matches!(self.look, Tok::Pipe);
        self.bump();

        // Parse indicators in ANY order: one of { '+', '-' } and one of { '1'..'9' }.
        let mut chomp: Option<char> = None;
        let mut indent_indicator: Option<usize> = None;

        loop {
            match self.look.clone() {
                Tok::Plus | Tok::Minus => {
                    if chomp.is_some() {
                        return Err(Error::Parse("duplicate chomping indicator".into()));
                    }
                    chomp = Some(match self.look {
                        Tok::Plus => '+',
                        _ => '-',
                    });
                    self.bump();
                }
                Tok::Str(StrTok {
                    text,
                    quoted: false,
                }) => {
                    // Handle multi-character strings like "2-" or "-2" by parsing char by char
                    let mut chars = text.chars().peekable();
                    let mut consumed_any = false;

                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            if ch == '0' {
                                return Err(Error::Parse("indent indicator 0 is invalid".into()));
                            }
                            if indent_indicator.is_some() {
                                return Err(Error::Parse("duplicate indent indicator".into()));
                            }
                            indent_indicator = Some((ch as u8 - b'0') as usize);
                            chars.next();
                            consumed_any = true;
                        } else if ch == '+' || ch == '-' {
                            if chomp.is_some() {
                                return Err(Error::Parse("duplicate chomping indicator".into()));
                            }
                            chomp = Some(ch);
                            chars.next();
                            consumed_any = true;
                        } else {
                            break;
                        }
                    }

                    if consumed_any {
                        self.bump();
                    }

                    // If we didn't consume any characters from this token, break
                    if !consumed_any {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Consume to EOL (allow trailing comment)
        let mut trailing_meta = Meta::default();
        self.consume_trailing_comment_into(&mut trailing_meta);

        // // Now read following lines with indent > parent_indent
        // let mut lines: Vec<String> = Vec::new();

        // // We consider subsequent Newline/Indent/Str/etc.; a block scalar body lines are
        // // those with Indent > parent_indent. Empty lines are included.
        // loop {
        //     match self.look.clone() {
        //         Tok::Indent(n) if n >= parent_indent => {
        //             self.bump();
        //             // Preserve relative indentation (beyond parent)
        //             let rel = n.saturating_sub(parent_indent);
        //             let (line, _had_nl) = self.collect_line_as_text();
        //             let mut buf = String::new();
        //             for _ in 0..rel {
        //                 buf.push(' ');
        //             }
        //             buf.push_str(&line);
        //             lines.push(buf);
        //         }
        //         Tok::Newline => {
        //             self.bump();
        //             // Blank line is part of the block scalar
        //             lines.push(String::new());
        //         }
        //         Tok::Comment(_) => {
        //             // Full-line comment counts as empty line in block scalar
        //             self.bump();
        //             lines.push(String::new());
        //         }
        //         _ => break,
        //     }
        // }
        // Collect lines with absolute indentation.
        // With indent indicator k: only admit lines with indent >= parent_indent + k
        // Without indicator: admit lines with indent > parent_indent
        let mut lines_with_indent: Vec<(usize, String)> = Vec::new();
        let mut detected_content_indent: Option<usize> = None;

        // Skip leading blank lines before first content line
        while matches!(self.look, Tok::Newline | Tok::Comment(_)) {
            self.bump();
        }

        loop {
            match self.look.clone() {
                Tok::Indent(n) => {
                    // With explicit indent indicator k: lines must have indent >= parent_indent + k
                    if let Some(k) = indent_indicator {
                        let min_acceptable = parent_indent + k;
                        if n < min_acceptable {
                            // Under-indented line: treat as empty and continue
                            self.bump();
                            let (_line, _had_nl) = self.collect_line_as_text();
                            lines_with_indent.push((0, String::new()));
                            continue;
                        }
                    } else {
                        // Without indicator: detect content indent from first non-empty line
                        // and require subsequent lines to maintain that indent (or be more indented)
                        if let Some(content_indent) = detected_content_indent {
                            // We've seen content - require at least that indentation
                            if n < content_indent {
                                break;
                            }
                        } else {
                            // First potential content line
                            // For top-level (parent_indent == 0), allow content at indent 0
                            // For nested contexts, content must be indented more than parent
                            if parent_indent > 0 && n <= parent_indent {
                                // Nested context: can't be content if at or below parent level
                                break;
                            }
                            // For top-level or properly indented: this could be content
                        }
                    }

                    self.bump();
                    let (line, _had_nl) = self.collect_line_as_text();

                    // Track first non-empty line's indent for auto-detection
                    if detected_content_indent.is_none() && !line.trim().is_empty() {
                        detected_content_indent = Some(n);
                    }

                    lines_with_indent.push((n, line));
                }
                Tok::Newline => {
                    self.bump();
                    // blank line is part of block (stored with indent 0)
                    lines_with_indent.push((0, String::new()));
                }
                Tok::Comment(_) => {
                    // Full-line comment inside the scalar body: treat as blank
                    self.bump();
                    lines_with_indent.push((0, String::new()));
                }
                _ => break,
            }
        }

        // let body = if lines.is_empty() {
        //     String::new()
        // } else if literal {
        //     apply_chomping(lines.join("\n"), chomp)
        // } else {
        //     let folded = fold_text(lines);
        //     apply_chomping(folded, chomp)
        // };
        // Determine content base indentation:
        // - With indicator k: base = parent_indent + k
        // - Without indicator: base = minimum indent of non-empty content lines
        let base_indent = if let Some(k) = indent_indicator {
            parent_indent + k
        } else {
            lines_with_indent
                .iter()
                .filter(|(_, text)| !text.is_empty())
                .map(|(indent, _)| *indent)
                .min()
                .unwrap_or(parent_indent + 1)
        };

        // Build dedented lines: each line's relative indent = (actual_indent - base_indent), clamped to 0
        let mut dedented: Vec<String> = Vec::with_capacity(lines_with_indent.len());
        for (n, text) in lines_with_indent {
            if text.is_empty() {
                dedented.push(String::new());
                continue;
            }

            // Relative indentation: how much beyond base this line is
            let rel = if n >= base_indent {
                n - base_indent
            } else {
                // Line is less indented than base - treat as base level (no extra indent)
                0
            };

            let mut buf = String::with_capacity(rel + text.len());
            for _ in 0..rel {
                buf.push(' ');
            }
            buf.push_str(&text);
            dedented.push(buf);
        }

        let body = if dedented.is_empty() {
            String::new()
        } else if literal {
            // join() doesn't add trailing newline, so we must add it
            // before chomping, so chomping indicators work correctly
            let mut text = dedented.join("\n");
            text.push('\n');
            apply_chomping(text, chomp)
        } else {
            // fold_text() doesn't add trailing newline either
            let mut folded = fold_text(dedented);
            folded.push('\n');
            apply_chomping(folded, chomp)
        };

        let mut elem = Elem::new(Node::Scalar(Scalar::Str(body)));
        // preserve any trailing comment from the header line
        elem.meta.trailing_comment = trailing_meta.trailing_comment;

        // CRUCIAL: remember that this scalar came from a block scalar header
        elem.meta.prefer_block = true;

        // Preserve original header style and chomp indicator.
        // The emitter will decide whether to honor it (when chomp is Some)
        // or canonicalize (when chomp is None).
        elem.meta.block_style = Some(if literal {
            BlockStyle::Literal(chomp)
        } else {
            BlockStyle::Folded(chomp)
        });

        Ok(elem)
    }

    fn parse_alias_name(&mut self) -> Result<String, Error> {
        if let Tok::Str(StrTok {
            text,
            quoted: false,
        }) = &self.look
        {
            let name = text.clone();
            self.bump();
            Ok(name)
        } else {
            Err(Error::Parse("expected alias name after '*'".into()))
        }
    }

    fn parse_line_value_or_nested(&mut self, nested_indent: usize) -> Result<Elem, Error> {
        // Check for lexer errors first
        if let Tok::Error(msg) = &self.look {
            return Err(Error::Parse(msg.clone()));
        }

        // Allow optional leading tag and/or anchor(s) on the sequence item value.
        let mut meta = Meta::default();

        // Parse tag if present (e.g., "- !str value")
        if let Tok::Tag(tag_text) = &self.look {
            meta.tag = Some(tag_text.clone());
            self.bump();
        }

        // Parse anchor(s) if present
        if matches!(self.look, Tok::Amp) {
            while let Tok::Amp = self.look {
                self.bump();
                if let Tok::Str(StrTok { text, .. }) = &self.look {
                    meta.anchor = Some(text.clone());
                    self.bump();
                } else {
                    return Err(Error::Parse("expected anchor name after '&'".into()));
                }
            }
        }

        match self.look.clone() {
            Tok::DocStart | Tok::DocEnd => {
                // A document marker cannot serve as an item’s value; this is inside a block.
                return Err(Error::Parse(
                    "document marker not allowed inside block".into(),
                ));
            }
            Tok::Str(s) => {
                self.bump();

                // If the next token is a colon, decide whether this is a map entry
                // or a glued plain scalar "k:v". It’s a map entry only if the ':'
                // is followed by whitespace/EOL/comment, not another plain char.
                if matches!(self.look, Tok::Colon) {
                    // Clone the lexer state that is already positioned *after* ':'
                    let lx2 = self.lx.clone();
                    // DON'T bump here; we are already after ':'
                    let after = lx2.peek();

                    let colon_is_separator = match after {
                        None => true,
                        Some(' ' | '\t' | '\n' | '\r' | '#') => true,
                        // If next visible char is a plain-scalar char, this is glued "k:v" -> scalar
                        Some(c) => !is_plain_scalar_char(c),
                    };

                    if colon_is_separator {
                        // Map item inside a sequence: "- k: <value>"
                        self.bump(); // consume ':'
                        let (mut value, _vtype) = self.parse_value_after_colon(nested_indent)?;
                        if !self.pending_leading_comments.is_empty() {
                            value.meta.leading_comments =
                                std::mem::take(&mut self.pending_leading_comments);
                        }
                        let key = s.text.clone();
                        return Ok(Elem::new(Node::Map(vec![(key, value)])));
                    }
                    // else fall through to scalar handling (glued "k:v")
                }

                // Otherwise: inline scalar on the same line (existing behavior)
                let mut text = s.text.clone();
                if !s.quoted {
                    let rest = self.parse_inline_scalar_line(/* glue_after_colon = */ true);
                    if !rest.is_empty() {
                        let r = rest.trim_start();
                        let needs_space = !text.is_empty()
                            && !(r.starts_with(':') || r.starts_with('-') || r.starts_with('+'));
                        if needs_space {
                            text.push(' ');
                        }
                        text.push_str(&rest);
                    }
                }

                // normalize
                strip_space_before_colon(&mut text);

                let scalar = classify_text_as_scalar(&text, s.quoted);
                let mut e = Elem::new(Node::Scalar(scalar));
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                e.meta.prefer_quoted = s.quoted;

                self.consume_trailing_comment_into(&mut e.meta);
                return Ok(e);
            }
            Tok::LBracket => {
                let node = self.parse_inline_array_node()?;
                let mut e = Elem::new(node);
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                self.consume_trailing_comment_into(&mut e.meta);
                Ok(e)
            }
            Tok::LBrace => {
                let node = self.parse_inline_map_node()?;
                let mut e = Elem::new(node);
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                self.consume_trailing_comment_into(&mut e.meta);
                Ok(e)
            }
            Tok::Pipe | Tok::Gt => {
                // parse_block_scalar already handles trailing comments and sets prefer_block
                // Block scalar content must be indented relative to the item line, not nested content
                let mut e = self.parse_block_scalar(nested_indent.saturating_sub(2))?;
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                Ok(e)
            }
            Tok::Star => {
                self.bump();
                let name = self.parse_alias_name()?;
                let mut e = Elem::new(Node::Alias(name));
                if e.meta.tag.is_none() {
                    e.meta.tag = meta.tag.take();
                }
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                self.consume_trailing_comment_into(&mut e.meta);
                Ok(e)
            }
            Tok::Newline => {
                self.bump();
                match self.look {
                    Tok::Indent(n) if n >= nested_indent => {
                        self.parse_elem_at_indent(nested_indent)
                    }
                    Tok::DocStart | Tok::DocEnd => {
                        return Err(Error::Parse(
                            "document marker not allowed inside block".into(),
                        ));
                    }
                    _ => Ok(Elem::new(Node::Seq(vec![]))),
                }
            }
            Tok::Eof => Ok(Elem::new(Node::Seq(vec![]))),
            Tok::Comma => {
                self.bump();
                let mut e = Elem::new(Node::Scalar(Scalar::Str(String::new())));
                self.consume_trailing_comment_into(&mut e.meta);
                return Ok(e);
            }
            Tok::Plus | Tok::Minus => {
                // Item begins with a sign; collect and classify so numbers stay typed.
                let sign = if matches!(self.look, Tok::Plus) {
                    '+'
                } else {
                    '-'
                };
                self.bump();
                let mut text = String::new();
                text.push(sign);
                let rest = self.parse_inline_scalar_line(/* glue_after_colon = */ true);
                if !rest.is_empty() {
                    text.push_str(&rest);
                }
                let scalar = classify_text_as_scalar(&text, false);
                let mut e = Elem::new(Node::Scalar(scalar));
                self.consume_trailing_comment_into(&mut e.meta);
                Ok(e)
            }
            Tok::Error(msg) => {
                return Err(Error::Parse(msg));
            }
            Tok::RBrace | _ => {
                // For now, treat remaining tokens as part of scalar text
                let s = self.parse_inline_scalar_line(/* glue_after_colon = */ false);
                let mut e = Elem::new(Node::Scalar(Scalar::Str(s)));
                if e.meta.anchor.is_none() {
                    e.meta.anchor = meta.anchor.take();
                }
                self.consume_trailing_comment_into(&mut e.meta);
                Ok(e)
            }
        }
    }

    fn parse_inline_scalar_line(&mut self, glue_after_colon: bool) -> String {
        let mut s = String::new();
        loop {
            match self.look.clone() {
                Tok::DocStart | Tok::DocEnd => break,
                Tok::TabIndent => {
                    // Should be unreachable here because we’re mid-line and TabIndent is only
                    // produced at BOL. Treat it as an end-of-value to keep type signature simple.
                    break;
                }
                Tok::Str(StrTok { text, .. }) => {
                    // Insert a space between adjacent words unless we’re
                    // immediately after ':' (and glue_after_colon is true),
                    // or immediately after '+' / '-'.
                    if !s.is_empty() {
                        if s.ends_with(':') {
                            if !glue_after_colon {
                                s.push(' ');
                            }
                        } else if !s.ends_with('-') && !s.ends_with('+') {
                            s.push(' ');
                        }
                    }
                    s.push_str(&text);
                    self.bump();
                }
                Tok::Colon => {
                    self.bump();
                    // Always glue ':' to the next token; the Str arm decides whether
                    // to add a space based on glue_after_colon.
                    s.push(':');
                }
                Tok::Comment(_) => {
                    /* stop before inline comment */
                    break;
                }
                Tok::Comma | Tok::Newline | Tok::Eof | Tok::RBracket | Tok::Question => break,
                Tok::Dash => {
                    self.bump();
                    // Inside a scalar line, '-' is part of the token (x-1, foo-bar).
                    // Do not force a space.
                    s.push('-');
                }
                Tok::Indent(_) | Tok::LBracket => break,
                Tok::Pipe | Tok::Gt | Tok::Plus | Tok::Minus | Tok::Amp | Tok::Star => {
                    // treat as plain char for this fallback
                    let ch = match self.look {
                        Tok::Pipe => '|',
                        Tok::Gt => '>',
                        Tok::Plus => '+',
                        Tok::Minus => '-',
                        Tok::Amp => '&',
                        Tok::Star => '*',
                        _ => unreachable!(),
                    };
                    self.bump();
                    // Do not insert a space before signs; we may join them to following digits.
                    if !s.is_empty() && !matches!(ch, '+' | '-') {
                        s.push(' ');
                    }
                    s.push(ch);
                }
                Tok::LBrace | Tok::RBrace => {
                    // Flow mapping delimiters should end the scalar line
                    break;
                }
                Tok::Tag(_) => {
                    // Tag appears before a value, not inside it
                    break;
                }
                Tok::Error(_) => {
                    // Lexer error should stop parsing
                    break;
                }
            }
        }
        s
    }

    fn consume_trailing_comment_into(&mut self, meta: &mut Meta) {
        // Consume any spaces then optional trailing comment token then optional newline.
        match self.look.clone() {
            Tok::Comment(c) => {
                self.bump();
                meta.trailing_comment = Some(c.trim().to_string());
            }
            _ => {}
        }
        if matches!(self.look, Tok::Newline) {
            self.bump();
        }
    }

    /// Collect the rest of the current line as raw text from the lexer.
    /// This is used for block scalars where we want literal content including apostrophes.
    fn collect_raw_line_from_lexer(&mut self) -> String {
        let mut result = String::new();
        loop {
            match self.lx.peek() {
                Some('\n') | Some('\r') => break,
                None => break,
                Some(ch) => {
                    result.push(ch);
                    self.lx.bump();
                }
            }
        }
        // After reading the raw line, update self.look to the newline/EOF
        self.look = self.lx.next_token();
        result
    }

    fn collect_line_as_text(&mut self) -> (String, bool) {
        let mut s = String::new();
        loop {
            match self.look.clone() {
                Tok::DocStart | Tok::DocEnd => {
                    return (s, true);
                }
                Tok::TabIndent => {
                    self.bump();
                    return (s, true);
                }
                Tok::Newline => {
                    self.bump();
                    return (s, true);
                }
                Tok::Eof => return (s, false),
                Tok::Comment(_) => {
                    self.bump();
                }
                Tok::Str(StrTok { text, .. }) => {
                    if !s.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(&text);
                    self.bump();
                }
                Tok::Colon => {
                    self.bump();
                    s.push(':');
                }
                Tok::Comma => {
                    self.bump();
                    s.push(',');
                }
                Tok::LBracket => {
                    self.bump();
                    s.push('[');
                }
                Tok::RBracket => {
                    self.bump();
                    s.push(']');
                }
                Tok::Pipe => {
                    self.bump();
                    s.push('|');
                }
                Tok::Gt => {
                    self.bump();
                    s.push('>');
                }
                Tok::Plus => {
                    self.bump();
                    s.push('+');
                }
                Tok::Minus => {
                    self.bump();
                    s.push('-');
                }
                Tok::Dash => {
                    self.bump();
                    if !s.ends_with(' ') {
                        s.push(' ');
                    }
                    s.push('-');
                }
                Tok::Indent(_) => {
                    self.bump();
                    s.push(' ');
                }
                Tok::Amp => {
                    self.bump();
                    s.push('&');
                }
                Tok::Star => {
                    self.bump();
                    s.push('*');
                }
                Tok::Question => {
                    self.bump();
                    s.push('?');
                }
                Tok::Tag(tag_text) => {
                    // Tags can appear in scalar text (if not parsed as metadata)
                    self.bump();
                    if !s.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(&tag_text);
                }
                Tok::LBrace => {
                    self.bump();
                    s.push('{');
                }
                Tok::RBrace => {
                    self.bump();
                    s.push('}');
                }
                Tok::Error(_) => {
                    // When we hit a tokenization error (like unterminated quote from apostrophe),
                    // switch to raw collection to get the literal content
                    let raw = self.collect_raw_line_from_lexer();
                    if !s.is_empty() && !raw.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(&raw);
                    return (s, true);
                }
            }
        }
    }
}
