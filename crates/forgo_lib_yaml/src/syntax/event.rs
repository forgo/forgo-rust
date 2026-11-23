//! Parser events, positions and scalar styles.

use std::borrow::Cow;

/// A position in the input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mark {
    /// Byte offset into the input.
    pub index: usize,
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based column, counted in characters.
    pub col: usize,
}

/// A half-open source range `start..end`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: Mark,
    pub end: Mark,
}

impl Span {
    pub fn new(start: Mark, end: Mark) -> Self {
        Span { start, end }
    }
}

/// How a scalar was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

/// The chomping indicator of a block scalar header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Chomping {
    /// `-`: drop the final line break and trailing empty lines.
    Strip,
    /// No indicator: keep the final line break only.
    #[default]
    Clip,
    /// `+`: keep the final line break and trailing empty lines.
    Keep,
}

/// The indicators written in a literal or folded block scalar header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockHeader {
    pub chomping: Chomping,
    /// The explicit indentation indicator (1-9), if written.
    pub indent: Option<u8>,
}

/// A node tag as written in the source, plus its resolved form.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag<'a> {
    /// `!`, `!!` or `!name!`; empty for a verbatim tag `!<…>`.
    pub handle: Cow<'a, str>,
    /// The suffix as written (percent-escapes kept). For a verbatim tag,
    /// the text between `<` and `>`. Empty for the non-specific tag `!`.
    pub suffix: Cow<'a, str>,
    /// The tag after handle resolution and percent-decoding, such as
    /// `tag:yaml.org,2002:str`. The non-specific tag resolves to `!`.
    pub resolved: Cow<'a, str>,
}

impl Tag<'_> {
    /// Is this the non-specific tag `!`?
    pub fn is_non_specific(&self) -> bool {
        self.resolved == "!"
    }

    pub fn into_owned(self) -> Tag<'static> {
        Tag {
            handle: Cow::Owned(self.handle.into_owned()),
            suffix: Cow::Owned(self.suffix.into_owned()),
            resolved: Cow::Owned(self.resolved.into_owned()),
        }
    }
}

/// One parse event. Values borrow from the input where possible.
#[derive(Clone, Debug, PartialEq)]
pub enum Event<'a> {
    StreamStart,
    StreamEnd,
    DocumentStart {
        /// The document began with `---`.
        explicit: bool,
        /// The `%YAML` directive, if any.
        version: Option<(u32, u32)>,
        /// `%TAG` directives declared for this document, as (handle, prefix).
        tags: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    },
    DocumentEnd {
        /// The document ended with `...`.
        explicit: bool,
    },
    SequenceStart {
        anchor: Option<Cow<'a, str>>,
        tag: Option<Tag<'a>>,
        flow: bool,
    },
    SequenceEnd,
    MappingStart {
        anchor: Option<Cow<'a, str>>,
        tag: Option<Tag<'a>>,
        flow: bool,
    },
    MappingEnd,
    Scalar {
        anchor: Option<Cow<'a, str>>,
        tag: Option<Tag<'a>>,
        value: Cow<'a, str>,
        style: ScalarStyle,
        /// Header indicators of a literal or folded scalar.
        block: Option<BlockHeader>,
    },
    Alias {
        name: Cow<'a, str>,
    },
}

fn own(c: Cow<'_, str>) -> Cow<'static, str> {
    Cow::Owned(c.into_owned())
}

impl Event<'_> {
    /// Detach the event from the input it borrows from.
    pub fn into_owned(self) -> Event<'static> {
        match self {
            Event::StreamStart => Event::StreamStart,
            Event::StreamEnd => Event::StreamEnd,
            Event::DocumentStart {
                explicit,
                version,
                tags,
            } => Event::DocumentStart {
                explicit,
                version,
                tags: tags.into_iter().map(|(h, p)| (own(h), own(p))).collect(),
            },
            Event::DocumentEnd { explicit } => Event::DocumentEnd { explicit },
            Event::SequenceStart { anchor, tag, flow } => Event::SequenceStart {
                anchor: anchor.map(own),
                tag: tag.map(Tag::into_owned),
                flow,
            },
            Event::SequenceEnd => Event::SequenceEnd,
            Event::MappingStart { anchor, tag, flow } => Event::MappingStart {
                anchor: anchor.map(own),
                tag: tag.map(Tag::into_owned),
                flow,
            },
            Event::MappingEnd => Event::MappingEnd,
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
                block,
            } => Event::Scalar {
                anchor: anchor.map(own),
                tag: tag.map(Tag::into_owned),
                value: own(value),
                style,
                block,
            },
            Event::Alias { name } => Event::Alias { name: own(name) },
        }
    }
}
