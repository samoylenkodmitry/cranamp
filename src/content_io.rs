use anyhow::{Context, Result};
use std::io::{Cursor, Read};

pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 16 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_DECODED_BYTES: u64 = 64 * 1024 * 1024;

pub async fn read_content(
    content: &cranpose_services::ContentHandle,
    limit: usize,
) -> Result<Vec<u8>> {
    anyhow::ensure!(
        content.metadata().len.unwrap_or(0) <= limit as u64,
        "File exceeds the {} MiB import limit",
        limit / 1024 / 1024
    );
    let reader = content.open().await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = reader.read_chunk().await? {
        anyhow::ensure!(
            chunk.len() <= limit.saturating_sub(bytes.len()),
            "File exceeds the {} MiB import limit",
            limit / 1024 / 1024
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_document(path: impl AsRef<std::path::Path>) -> Result<Vec<u8>> {
    read_limited(std::fs::File::open(path)?, MAX_DOCUMENT_BYTES)
}

fn read_limited(reader: impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() <= limit,
        "File exceeds the {} MiB import limit",
        limit / 1024 / 1024
    );
    Ok(bytes)
}

pub fn open_archive(bytes: &[u8]) -> Result<zip::ZipArchive<Cursor<&[u8]>>> {
    anyhow::ensure!(bytes.len() <= MAX_DOCUMENT_BYTES, "Archive exceeds 64 MiB");
    let end = (bytes.len().saturating_sub(65_557)..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            bytes[i..].starts_with(b"PK\x05\x06")
                && i + 22 + u16::from_le_bytes([bytes[i + 20], bytes[i + 21]]) as usize
                    == bytes.len()
        })
        .context("Archive has no valid ZIP directory")?;
    let count = u16::from_le_bytes([bytes[end + 10], bytes[end + 11]]);
    let directory_bytes = u32::from_le_bytes(bytes[end + 12..end + 16].try_into()?);
    anyhow::ensure!(
        count <= 2048 && directory_bytes <= 2 * 1024 * 1024,
        "Archive directory is too large"
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    anyhow::ensure!(archive.len() <= 2048, "Archive has too many entries");
    let mut total = 0u64;
    let mut names = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        anyhow::ensure!(
            entry.size() <= MAX_ENTRY_BYTES as u64,
            "Archive entry exceeds 16 MiB: {}",
            entry.name()
        );
        total = total
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        anyhow::ensure!(
            total <= MAX_EXPANDED_BYTES,
            "Expanded archive exceeds 128 MiB"
        );
        anyhow::ensure!(
            names.insert(entry.name().replace('\\', "/").to_ascii_lowercase()),
            "Duplicate archive entry: {}",
            entry.name()
        );
    }
    Ok(archive)
}

pub fn read_entry(entry: impl Read) -> Result<Vec<u8>> {
    read_limited(entry, MAX_ENTRY_BYTES)
}

fn image_reader(bytes: &[u8]) -> Result<image::ImageReader<Cursor<&[u8]>>> {
    Ok(image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?)
}

fn decoded_size(bytes: &[u8]) -> Result<u64> {
    let (width, height) = image_reader(bytes)?.into_dimensions()?;
    let size = u64::from(width) * u64::from(height) * 4;
    anyhow::ensure!(
        width <= 4096 && height <= 4096 && size <= MAX_ENTRY_BYTES as u64,
        "Image exceeds 4096 pixels per side or 16 MiB decoded"
    );
    Ok(size)
}

#[derive(Default)]
pub struct ImageBudget(u64);
impl ImageBudget {
    pub fn include(&mut self, bytes: &[u8]) -> Result<()> {
        self.0 += decoded_size(bytes)?;
        anyhow::ensure!(self.0 <= MAX_DECODED_BYTES, "Images exceed 64 MiB decoded");
        Ok(())
    }
    pub fn include_decoded(&mut self, image: &image::RgbaImage) -> Result<()> {
        self.0 += image.as_raw().len() as u64;
        anyhow::ensure!(self.0 <= MAX_DECODED_BYTES, "Images exceed 64 MiB decoded");
        Ok(())
    }
}

pub fn decode_image(bytes: &[u8]) -> Result<image::RgbaImage> {
    decoded_size(bytes)?;
    let mut reader = image_reader(bytes)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(MAX_ENTRY_BYTES as u64);
    reader.limits(limits);
    Ok(reader.decode()?.to_rgba8())
}

#[cfg(test)]
#[path = "../test/unit/content_io.rs"]
mod tests;
