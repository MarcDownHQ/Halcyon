//! Halcyon: YouTube channels, playlists and lives for Nightglass, as a WebAssembly component.
//!
//! Everything but [`wasm`] is plain Rust that builds and is tested natively; `wasm` adapts the host imports and
//! exports of `wit/provider.wit` to it.

pub mod model;
pub mod provider;

#[cfg(target_family = "wasm")]
mod wasm;
