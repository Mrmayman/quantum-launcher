//! A shim library that wraps reqwest.
//! Seriously, this is the entire library:
//!
//! ```no_run
//! pub use reqwest::*;
//! ```
//!
//! This crate, `ql_reqwest`, is present so I can have
//! fine-grained control over reqwest's build process across
//! platforms, and correctly tweak the TLS implementation and
//! other crate features.

pub use reqwest::*;
