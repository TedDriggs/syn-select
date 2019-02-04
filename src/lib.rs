//! Library to get a specific element by path in Rust code.
//!
//! # Usage
//! ```rust,edition2018
//! let file: syn::File = syn::parse_str(
//!     r#"
//!     mod a {
//!         mod b {
//!             trait C {
//!                 fn d(self) {}
//!                 fn f() {}
//!             }
//!         }
//!     }"#).unwrap();
//! let results = syn_select::select("a::b::C::d", &file).unwrap();
//! assert_eq!(results.len(), 1);
//! ```

use syn::Item;

mod error;
mod search;
mod selector;
mod util;

pub use error::Error;
pub use selector::Selector;

/// Parse a path, then search a file for all results that exactly match the specified
/// path.
///
/// # Returns
/// This function can find multiple items if:
///
/// 1. There is a module and a function of the same name
/// 2. The same path is declared multiple times, differing by config flags
pub fn select(path: &str, file: &syn::File) -> Result<Vec<Item>, Error> {
    Ok(Selector::try_from(path)?.apply_to(file))
}

#[cfg(test)]
mod tests {
    use syn::Item;

    use super::{select, util};

    fn sample() -> syn::File {
        syn::parse_str(
            "mod a {
            mod b {
                trait C {
                    fn d() {
                        struct E;
                    }
                    fn f(self) {
                        struct E;
                    }
                }
            }
            fn b() {}
        }",
        )
        .unwrap()
    }

    fn sample_with_cfg() -> syn::File {
        syn::parse_str(
            r#"
            /// Outer doc
            #[cfg(feature = "g")]
            mod imp {
                /// Documentation
                #[serde(skip)]
                #[cfg(feature = "h")]
                pub struct H(u8);
            }
            #[cfg(not(feature = "g"))]
            mod imp {
                pub struct H(u16);
            }"#,
        )
        .unwrap()
    }

    fn search_sample(path: &str) -> Vec<syn::Item> {
        select(path, &sample()).unwrap()
    }

    fn ident(ident: &str) -> syn::Ident {
        syn::parse_str::<syn::Ident>(ident).unwrap()
    }

    #[test]
    fn example_1() {
        let result = search_sample("a::b::C");
        assert_eq!(result.len(), 1);
        if let Item::Trait(item) = &result[0] {
            assert_eq!(item.ident, ident("C"));
        } else {
            panic!("Result was wrong type {:?}", &result[0]);
        }
    }

    #[test]
    fn example_2() {
        let result = search_sample("a::b::C::d::E");
        assert_eq!(result.len(), 1);
        if let Item::Struct(item) = &result[0] {
            assert_eq!(item.ident, ident("E"));
        } else {
            panic!("Result was wrong type {:?}", &result[0]);
        }
    }

    /// If I query for "a::b::C::f" I should get the trait C filtered down to only function f.
    /// The trait needs to be included because fn f(self) {} by itself is not a valid top-level
    /// Item.
    #[test]
    fn example_3() {
        let result = search_sample("a::b::C::f");
        assert_eq!(result.len(), 1);
        if let Item::Trait(item) = &result[0] {
            assert_eq!(item.items.len(), 1);
            if let syn::TraitItem::Method(item) = &item.items[0] {
                assert_eq!(item.sig.ident, ident("f"));
            }
        }
    }

    #[test]
    fn example_4() {
        let result = search_sample("a::b");
        assert_eq!(result.len(), 2);
    }

    /// Test that `cfg` attributes are intelligently added to search results, and
    /// that attribute order is idiomatic.
    #[test]
    fn example_5() {
        let result = select("imp::H", &sample_with_cfg()).unwrap();
        assert_eq!(result.len(), 2);
        if let Item::Struct(item) = &result[0] {
            assert_eq!(item.attrs.len(), 4);
            assert_eq!(item.attrs[0].path, util::syn_path("doc"));
            assert_eq!(item.attrs[1].path, util::syn_path("cfg"));
            assert_eq!(item.attrs[2].path, util::syn_path("serde"));
            assert_eq!(item.attrs[3].path, util::syn_path("cfg"));
        } else {
            panic!("First result should be struct");
        }

        if let Item::Struct(item) = &result[1] {
            assert_eq!(item.attrs.len(), 1);
            assert_eq!(item.attrs[0].path, util::syn_path("cfg"));
        } else {
            panic!("Second result should be struct");
        }
    }

    #[test]
    fn example_6() {
        let result = search_sample("a::b::C::_::E");
        assert_eq!(result.len(), 2);
    }
}
