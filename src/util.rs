/// Parse a static string to a path.
///
/// # Panics
/// This function will panic if `src` is not a valid path.
pub(crate) fn syn_path(src: &'static str) -> syn::Path {
    syn::parse_str(src).unwrap()
}