//! The selector for a given search, with its trait implementations.

use crate::search::Search;
use crate::Error;
use std::fmt;
use std::str::FromStr;
use syn::{Ident, Item};

/// The path provided by the user to search for.
/// This is stricter than a `syn::Path`, as we don't allow generics or UFCS constructs.
pub(crate) struct Selector {
    segments: Vec<Ident>,
}

impl Selector {
    /// Use this selector to search a file, returning the list of items that match the selector.
    pub fn search(&self, file: &syn::File) -> Vec<Item> {
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
