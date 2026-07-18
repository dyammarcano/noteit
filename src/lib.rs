//! noteit — a CLI that captures notes bound to the git repository (or path)
//! you run it in. This library crate exposes the modules the `noteit` binary
//! composes; see `cli` for the command surface and `store` for persistence.

pub mod cli;
pub mod context;
pub mod plugin;
pub mod render;
pub mod repoid;
pub mod store;
