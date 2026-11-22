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
mod ui;

pub use panel::MatterEditorPanel;
pub use plugin::MatterPlugin;

// This plugin's descriptor (`pulsar-plugin.toml`): linking the crate installs it.
pulsar_plugins::installed!("pulsar-plugin.toml", build);

/// Everything the plugin registers (the editor provider).
pub fn build(p: &mut pulsar_plugins::PluginBuilder) {
    // The icon the editor shows beside this plugin (Extensions window).
    p.add(pulsar_plugins::PluginIcon {
        origin: p.origin(),
        format: pulsar_plugins::IconFormat::Png,
        bytes: include_bytes!("../icon.png"),
    });
    builtin_provider::build(p);
    examples::build(p);
}

/// The plugin's editor provider, registered by `build` (origin-gated).
mod builtin_provider;

/// The plugin's example documents, served as `examples/` in its folder.
mod examples;

// The plugin as a library of its own, loaded by the editor from `plugins/editor`
// (`cargo build -p <this package> --release --features dll`).
#[cfg(feature = "dll")]
plugin_editor_api::export_library_plugin!();
