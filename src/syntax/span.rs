/// A custom span type, because `logos::Span` is not `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span<'src> {
    pub source: &'src str,
    pub start: usize,
    pub end: usize,
}

impl<'src> Span<'src> {
    pub fn from_range(source: &'src str, r: std::ops::Range<impl Into<usize>>) -> Self {
        Self {
            source,
            start: r.start.into(),
            end: r.end.into(),
        }
    }

    pub fn from_eoi(source: &'src str, pos: impl Into<usize>) -> Self {
        let pos = pos.into();
        Self {
            source,
            start: pos,
            end: pos,
        }
    }
}

impl<'src> ariadne::Span for Span<'src> {
    type SourceId = &'src str;

    fn source(&self) -> &Self::SourceId {
        &self.source
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
