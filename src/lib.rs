//! Plugin_Matter - Professional Texture & Material Editor
//!
//! A modular texture painting and procedural material generation plugin for Pulsar Engine.

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
