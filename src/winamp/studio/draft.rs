//! Where a hosted editor keeps its work: the recoverable draft it saves when
//! it is left, and the skin it hands back to the player.
//!
//! There used to be a second editor here, a touch layout of its own, and every
//! capability the engine grew had to be added to it and to the desktop one or
//! it did not exist on the two platforms with no MCP to ask for it with
//! instead. There is one editor now, in `mod.rs`, laid out for whatever
//! surface it is given; what is left here was never about layout.
use super::*;

pub(crate) fn new_mobile_document(path: Option<&str>) -> anyhow::Result<SharedDocument> {
    let mut doc = initial_document(path)?;
    doc.view.panel = "canvas".into();
    doc.view.zoom = 2;
    let shared = SharedDocument(Arc::new(Mutex::new(doc)));
    Ok(shared)
}

/// Name the applied skin keeps in the player's library, on disk or in the
/// browser's scoped storage.
const APPLIED: &str = "Studio edited.wsz";

// Draft and applied-skin storage. Native keeps real files; the browser has no
// filesystem, so both go through the same scoped preferences the player already
// uses for its skin library.
#[cfg(not(target_arch = "wasm32"))]
fn draft_path() -> std::path::PathBuf {
    super::super::app_config_dir().join("skin-studio/draft.cstudio")
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn store_draft(bytes: &[u8], previous: bool) -> anyhow::Result<()> {
    let path = draft_path();
    let path = if previous {
        path.with_file_name("previous.cstudio")
    } else {
        path
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("cstudio.tmp");
    std::fs::write(&temp, bytes)?;
    std::fs::rename(temp, path)?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load_draft() -> anyhow::Result<Vec<u8>> {
    Ok(std::fs::read(draft_path())?)
}

#[cfg(target_arch = "wasm32")]
const DRAFT_KEY: &str = "cranamp.studio.draft.v1";
#[cfg(target_arch = "wasm32")]
pub(super) fn store_draft(bytes: &[u8], previous: bool) -> anyhow::Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let key = if previous {
        format!("{DRAFT_KEY}.previous")
    } else {
        DRAFT_KEY.to_string()
    };
    cranpose_services::preferences()
        .set(&key, &STANDARD.encode(bytes))
        .map_err(|e| anyhow::anyhow!("Browser storage is full or blocked: {e}"))
}
#[cfg(target_arch = "wasm32")]
pub(super) fn load_draft() -> anyhow::Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = cranpose_services::preferences()
        .get(DRAFT_KEY)
        .ok_or_else(|| anyhow::anyhow!("No saved draft in this browser yet"))?;
    Ok(STANDARD.decode(encoded)?)
}

/// Persist the edited skin where the player can find it again, returning the
/// bytes to apply now and the path (or browser key) to remember.
pub(super) fn publish(shared: &SharedDocument) -> anyhow::Result<(Vec<u8>, String)> {
    let mut doc = shared.lock().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = super::super::skins_library_dir().join(APPLIED);
        doc.export(&path)?;
        Ok((doc.archive()?, path.to_string_lossy().into_owned()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        doc.finish_stroke();
        let bytes = doc.archive()?;
        // Validate before writing, exactly as the native export does: a broken
        // archive must not replace the entry the player already has.
        super::super::skin::load_skin(&bytes)?;
        let key = super::super::browser_skins::save(
            cranpose_services::preferences().as_ref(),
            APPLIED,
            &bytes,
        )
        .map_err(anyhow::Error::msg)?;
        doc.path = Some(key.clone());
        doc.mark_project_saved(format!("Applied {APPLIED}"));
        Ok((bytes, key))
    }
}

pub(super) fn save_draft(shared: &SharedDocument) -> anyhow::Result<()> {
    let mut doc = shared.lock().unwrap();
    let bytes = doc.project_bytes()?;
    store_draft(&bytes, false)?;
    doc.mark_project_saved("Saved draft".into());
    Ok(())
}
