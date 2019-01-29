# syn-select

[![Build Status](https://travis-ci.org/TedDriggs/syn-select.svg?branch=master)](https://travis-ci.org/TedDriggs/syn-select)
[![Latest Version](https://img.shields.io/crates/v/syn-select.svg)](https://crates.io/crates/syn-select)
[![Documentation](https://docs.rs/syn-select/badge.svg)](https://docs.rs/syn-select)

Lightweight path selector for searching Rust code.

```rust
mod a {
    mod b {
        trait C {
            fn d(self) {}

            fn f() {}
        }
    }
}

fn main() {
    let src_file = syn::parse_str(include_str!("./rs")).unwrap();

    // This will print out the trait `C`, limited to only function `d`.
    dbg!(syn_select::select("a::b::C::d", &src_file).unwrap());
}
```
