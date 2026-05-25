use std::path::Path;

use cortex_storage::manifest::StorageManifest;

pub fn validate(root: &str) -> Result<String, String> {
    let manifest = load(root)?;
    Ok(format!(
        "ok generation={} checkpoint_seq={} live_segments={} retired_segments={}",
        manifest.generation,
        manifest.checkpoint_seq,
        manifest.live_segments.len(),
        manifest.retired_segments.len()
    ))
}

pub fn dump(root: &str) -> Result<String, String> {
    let manifest = load(root)?;
    let mut lines = vec![format!(
        "generation={} checkpoint_seq={} live_segments={} retired_segments={}",
        manifest.generation,
        manifest.checkpoint_seq,
        manifest.live_segments.len(),
        manifest.retired_segments.len()
    )];
    for segment in manifest.live_segments {
        lines.push(format!(
            "live id={} generation={} checkpoint_seq={} cell_count={}",
            segment.id, segment.generation, segment.checkpoint_seq, segment.cell_count
        ));
    }
    for segment in manifest.retired_segments {
        lines.push(format!(
            "retired id={} generation={} checkpoint_seq={} cell_count={}",
            segment.id, segment.generation, segment.checkpoint_seq, segment.cell_count
        ));
    }
    Ok(lines.join("\n"))
}

fn load(root: &str) -> Result<StorageManifest, String> {
    StorageManifest::load(Path::new(root).join("manifest.acm")).map_err(|error| error.to_string())
}
