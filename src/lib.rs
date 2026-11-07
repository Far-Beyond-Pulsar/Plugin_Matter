//! Plugin_Matter - Professional Texture & Material Editor
//!
//! A modular texture painting and procedural material generation plugin for Pulsar Engine.

rust_i18n::i18n!("locales", fallback = "en");

mod plugin;
mod panel;
mod panels;
pub mod state;
mod canvas;
mod tools;
mod ui;
pub mod brush_engine;

pub use plugin::MatterPlugin;
pub use panel::MatterEditorPanel;

// This plugin's descriptor (`pulsar-plugin.toml`): linking the crate installs it.
pulsar_plugins::installed!("pulsar-plugin.toml");

/// The plugin's editor provider, submitted to the editor (origin-gated).
mod builtin_provider;
