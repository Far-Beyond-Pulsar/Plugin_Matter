//! Plugin registration and metadata

use gpui::App;
use std::path::PathBuf;
use std::sync::Arc;

/// Matter texture editor plugin
pub struct MatterPlugin {
    // Plugin state will go here
}

impl MatterPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

// For now we're not implementing the full plugin trait
// We'll first test the panel standalone
