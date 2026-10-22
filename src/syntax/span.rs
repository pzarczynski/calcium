#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

/// A custom span type, because `logos::Span` is not `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub fileid: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn from_range(fileid: FileId, r: std::ops::Range<impl Into<usize>>) -> Self {
        Self {
            fileid,
            start: r.start.into(),
            end: r.end.into(),
        }
    }

    pub fn from_eoi(fileid: FileId, pos: impl Into<usize>) -> Self {
        let pos = pos.into();
        Self {
            fileid,
            start: pos,
            end: pos,
        }
    }
}

impl ariadne::Span for Span {
    type SourceId = FileId;

    fn source(&self) -> &Self::SourceId {
        &self.fileid
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
