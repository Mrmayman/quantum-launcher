//! A shim library for
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher)
//! that wraps reqwest. Seriously, this is the entire library:
//!
//! ```no_run
//! pub use reqwest::*;
//! ```
//!
//! This crate, `ql_reqwest`, is present so I can have
//! fine-grained control over reqwest's build process across
//! platforms, and correctly tweak the TLS implementation and
//! other crate features.
//!
//! # Not recommended to use this in your own projects!

pub use reqwest::*;
