//! The plugin's example documents: a read-only `examples/` folder in the
//! plugin's virtual folder (`plugins/<plugin>/examples/`), served from files
//! embedded in the binary (`examples_assets/`). Each `.pif` bundle is a valid
//! Pulsar Image Format document (a folder holding `manifest.json`).

use std::sync::Arc;

use engine_fs::{EmbeddedFile, FileProvider, FolderSource, InGame, MemoryFolder, PluginFolder};

macro_rules! example {
    ($name:literal) => {
        EmbeddedFile {
            path: concat!("examples/", $name),
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples_assets/",
                $name
            )),
        }
    };
}

/// Every embedded file, by its path inside the plugin's folder.
pub(crate) static EXAMPLES: &[EmbeddedFile] = &[
    example!("README.md"),
    example!("BlankCanvas.pif/manifest.json"),
    example!("LayerStack.pif/manifest.json"),
    example!("LinearMask.pif/manifest.json"),
    example!("VectorBadge.pif/manifest.json"),
];

fn open(folder: PluginFolder) -> Arc<dyn FolderSource> {
    Arc::new(MemoryFolder::with_files(folder, EXAMPLES))
}

/// Registers the plugin's example files.
pub(crate) fn build(p: &mut pulsar_plugins::PluginBuilder) {
    p.add(FileProvider {
        origin: p.origin(),
        open,
        in_game: InGame::Snapshot,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulsar_image_format::model::Layer;
    use pulsar_image_format::PifAssetManager;

    fn bundles() -> Vec<&'static str> {
        EXAMPLES
            .iter()
            .filter_map(|file| file.path.strip_suffix("/manifest.json"))
            .collect()
    }

    /// Writes the embedded bundle to a temporary directory so the PIF
    /// library, the plugin's own loader, opens it from disk.
    fn open_bundle(bundle: &str) -> PifAssetManager {
        let name = bundle.trim_start_matches("examples/");
        let root = std::env::temp_dir()
            .join(format!("matter_examples_{}", std::process::id()))
            .join(name);
        std::fs::create_dir_all(root.join(".raster")).unwrap();
        for file in EXAMPLES.iter().filter(|f| f.path.starts_with(bundle)) {
            let relative = file.path[bundle.len()..].trim_start_matches('/');
            std::fs::write(root.join(relative), file.bytes).unwrap();
        }
        PifAssetManager::open(&root).unwrap_or_else(|e| panic!("{bundle}: {e}"))
    }

    #[test]
    fn every_bundle_opens_with_the_pif_loader() {
        let bundles = bundles();
        assert!(bundles.len() >= 4, "several varied documents");
        for bundle in bundles {
            let manager = open_bundle(bundle);
            let manifest = manager.manifest();
            assert_eq!(manifest.pif_version, "1.0", "{bundle}");
            assert!(manifest.canvas.width > 0 && manifest.canvas.height > 0);
            assert!(!manifest.layers.is_empty(), "{bundle} has a layer");
            for layer in &manifest.layers {
                assert!((0.0..=1.0).contains(&layer.opacity()), "{bundle}");
                if let Layer::Raster {
                    tile_size, tiles, ..
                } = layer
                {
                    assert!(*tile_size > 0);
                    // The bundles hold no tiles, so every tile map is empty.
                    assert!(tiles.is_empty(), "{bundle}");
                }
            }
        }
    }

    #[test]
    fn the_examples_are_varied_and_listed_in_the_readme() {
        let readme = std::str::from_utf8(
            EXAMPLES
                .iter()
                .find(|f| f.path == "examples/README.md")
                .unwrap()
                .bytes,
        )
        .unwrap();
        let mut kinds = std::collections::HashSet::new();
        for bundle in bundles() {
            let name = bundle.trim_start_matches("examples/");
            assert!(readme.contains(name), "README lists {name}");
            for layer in &open_bundle(bundle).manifest().layers {
                kinds.insert(matches!(layer, Layer::Vector { .. }));
            }
        }
        assert_eq!(kinds.len(), 2, "both raster and vector layers appear");
    }
}
