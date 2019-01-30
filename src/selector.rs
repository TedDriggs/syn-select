//! The selector for a given search, with its trait implementations.

use crate::search::Search;
use crate::Error;
use std::fmt;
use std::str::FromStr;
use syn::{Ident, Item};

/// The path provided by the user to search for.
///
/// Not all Rust paths are valid selectors; UFCS and generics are not supported.
#[derive(Clone)]
pub struct Selector {
    segments: Vec<Ident>,
}

impl Selector {
    /// Create a new `Selector` by parsing the passed-in string.
    ///
    /// # Usage
    /// ```rust,edition2018
    /// use syn_select::Selector;
    /// let selector = Selector::try_from("hello::world").unwrap();
    /// assert_eq!(format!("{:?}", selector), "hello::world".to_owned());
    /// ```
    pub fn try_from(s: impl AsRef<str>) -> Result<Self, Error> {
        s.as_ref().parse()
    }

    /// Use this selector to search a file, returning the list of items that match the selector.
    pub fn apply_to(&self, file: &syn::File) -> Vec<Item> {
        let mut search = Search::new(self);
        search.search_file(file);
        search.results
    }

    pub(crate) fn part(&self, index: usize) -> &Ident {
        &self.segments[index]
    }

    pub(crate) fn len(&self) -> usize {
        self.segments.len()
    }
}

impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.segments[0])?;
        for segment in self.segments.iter().skip(1) {
            write!(f, "::{}", segment)?;
        }

        Ok(())
    }
}

impl FromStr for Selector {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut segments = Vec::new();

        if input.trim() == "" {
            return Err(Error::empty_path());
        }

        for segment in input.split("::") {
            match syn::parse_str(segment) {
                Ok(ident) => segments.push(ident),
                Err(_) => return Err(Error::invalid_path()),
            }
        }

        Ok(Selector { segments })
    }
}
