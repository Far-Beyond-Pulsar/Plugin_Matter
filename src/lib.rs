//! Plugin_Matter - Professional Texture & Material Editor
//!
//! A modular texture painting and procedural material generation plugin for Pulsar Engine.

rust_i18n::i18n!("locales", fallback = "en");

pub mod brush_engine;
mod canvas;
mod panel;
mod panels;
mod plugin;
pub mod state;
mod tools;
mod ui;

pub use panel::MatterEditorPanel;
pub use plugin::MatterPlugin;

// This plugin's descriptor (`pulsar-plugin.toml`): linking the crate installs it.
pulsar_plugins::installed!("pulsar-plugin.toml", build);

/// Everything the plugin registers (the editor provider).
pub fn build(p: &mut pulsar_plugins::PluginBuilder) {
    builtin_provider::build(p);
    examples::build(p);
}

/// The plugin's editor provider, registered by `build` (origin-gated).
mod builtin_provider;

/// The plugin's example documents, served as `examples/` in its folder.
mod examples;
