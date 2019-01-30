use std::error::Error as StdError;
use std::fmt;

/// An error encountered while parsing or executing a selector.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Error { kind }
    }

    /// Create an error indicating the caller provided an empty path to search.
    pub(crate) fn empty_path() -> Self {
        Error::new(ErrorKind::EmptyPath)
    }

    /// Create an error indicating the caller provided a non-empty string that
    /// couldn't be parsed to a searchable path.
    pub(crate) fn invalid_segment(segment: String) -> Self {
        Error::new(ErrorKind::InvalidSegment(segment))
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&std::error::Error> {
        None
    }

    fn description(&self) -> &str {
        self.kind.description()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

#[derive(Debug)]
enum ErrorKind {
    /// The selector parser was passed an empty string.
    EmptyPath,
    /// The selector parser was passed a non-empty string that had
    /// an invalid part after being split by the path separator.
    InvalidSegment(String),
}

impl std::error::Error for ErrorKind {
    fn cause(&self) -> Option<&std::error::Error> {
        None
    }

    fn description(&self) -> &str {
        match self {
            ErrorKind::EmptyPath => "Empty path",
            ErrorKind::InvalidSegment(_) => "Invalid path segment",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::EmptyPath => self.description().fmt(f),
            ErrorKind::InvalidSegment(segment) => write!(
                f,
                "{}: `{}` is not an identifier",
                self.description(),
                segment
            ),
        }
    }
}
