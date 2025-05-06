//! # A crate for dealing with mods
//!
//! This crate provides a way to manage mods for Quantum Launcher.
//!
//! # Features
//! ## Modrinth
//! - Install mods
//! - Uninstall mods
//! - Update mods
//! - Search for mods
//! ## Loaders: Install and uninstall:
//! - Fabric
//! - Forge
//! - Optifine
//! - Quilt
//! - Paper

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_precision_loss)]

/// Installers and Uninstallers for loaders (Fabric/Forge/Optifine/Quilt/Paper).
pub mod loaders;
mod presets;
mod rate_limiter;
/// Mod manager integrated with Modrinth.
pub mod store;
pub use presets::PresetJson;
