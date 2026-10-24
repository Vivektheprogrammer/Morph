use crate::error::{MorphError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    pub custom_output_dir: Option<PathBuf>,
    pub overwrite: bool,
    pub custom_filename: Option<String>,
}

pub struct OutputManager;

impl OutputManager {
    /// Resolves the intended destination path.
    ///
    /// By default:
    /// - Saves beside the source file (in the same directory).
    /// - If the target file already exists and `overwrite` is false,
    ///   applies collision avoidance: `name (1).ext`, `name (2).ext`, etc.
    pub fn resolve_destination(
        source: &Path,
        target_extension: &str,
        options: &OutputOptions,
    ) -> Result<PathBuf> {
        let parent_dir = if let Some(ref dir) = options.custom_output_dir {
            dir.as_path()
        } else {
            source.parent().unwrap_or_else(|| Path::new("."))
        };

        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }

        let clean_ext = target_extension.trim_start_matches('.');
        let stem = if let Some(ref custom_name) = options.custom_filename {
            Path::new(custom_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output")
        } else {
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output")
        };

        let initial_name = format!("{}.{}", stem, clean_ext);
        let initial_path = parent_dir.join(&initial_name);

        if options.overwrite || !initial_path.exists() {
            return Ok(initial_path);
        }

        // Apply collision naming: stem (1).ext, stem (2).ext, ...
        let mut counter = 1;
        loop {
            let collision_name = format!("{} ({}).{}", stem, counter, clean_ext);
            let candidate_path = parent_dir.join(&collision_name);
            if !candidate_path.exists() {
                return Ok(candidate_path);
            }
            counter += 1;
            if counter > 10000 {
                return Err(MorphError::FileAlreadyExists(initial_path));
            }
        }
    }

    /// Creates a temporary file in the destination directory to ensure
    /// atomic renaming on the same filesystem/drive volume.
    pub fn create_staging_file(destination_dir: &Path) -> Result<NamedTempFile> {
        if !destination_dir.exists() {
            fs::create_dir_all(destination_dir)?;
        }
        let temp = tempfile::Builder::new()
            .prefix(".morph-tmp-")
            .tempfile_in(destination_dir)?;
        Ok(temp)
    }

    /// Validates the staging file and atomically moves it to the final destination.
    pub fn finalize_atomic(staging: NamedTempFile, destination: &Path) -> Result<PathBuf> {
        let metadata = staging.as_file().metadata()?;
        if metadata.len() == 0 {
            return Err(MorphError::ValidationFailed(format!(
                "Output file for '{}' is empty (0 bytes)",
                destination.display()
            )));
        }

        // On Windows, if destination already exists, persist() might fail if not removed first when overwrite is true.
        if destination.exists() {
            fs::remove_file(destination)?;
        }

        staging.persist(destination).map_err(|e| {
            MorphError::IoError(std::io::Error::other(format!(
                "Failed to persist output file to '{}': {}",
                destination.display(),
                e
            )))
        })?;

        Ok(destination.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_collision_resolution() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("photo.jpg");
        fs::write(&file_path, b"dummy").unwrap();

        let options = OutputOptions::default();

        // Target: photo.pdf (doesn't exist yet)
        let dest1 = OutputManager::resolve_destination(&file_path, "pdf", &options).unwrap();
        assert_eq!(dest1, temp.path().join("photo.pdf"));

        // Simulate photo.pdf exists
        fs::write(&dest1, b"pdf1").unwrap();

        // Target: should now resolve to photo (1).pdf
        let dest2 = OutputManager::resolve_destination(&file_path, "pdf", &options).unwrap();
        assert_eq!(dest2, temp.path().join("photo (1).pdf"));

        // Simulate photo (1).pdf exists
        fs::write(&dest2, b"pdf2").unwrap();

        // Target: should now resolve to photo (2).pdf
        let dest3 = OutputManager::resolve_destination(&file_path, "pdf", &options).unwrap();
        assert_eq!(dest3, temp.path().join("photo (2).pdf"));
    }

    #[test]
    fn test_atomic_finalize() {
        let temp = tempdir().unwrap();
        let dest = temp.path().join("output.pdf");

        let mut staging = OutputManager::create_staging_file(temp.path()).unwrap();
        use std::io::Write;
        staging.write_all(b"%PDF-1.7\nvalid\n%%EOF").unwrap();

        let final_path = OutputManager::finalize_atomic(staging, &dest).unwrap();
        assert_eq!(final_path, dest);
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"%PDF-1.7\nvalid\n%%EOF");
    }

    #[test]
    fn test_empty_output_validation_fails() {
        let temp = tempdir().unwrap();
        let dest = temp.path().join("empty.pdf");

        let staging = OutputManager::create_staging_file(temp.path()).unwrap();
        // Do not write anything (0 bytes)

        let result = OutputManager::finalize_atomic(staging, &dest);
        assert!(result.is_err());
        assert!(!dest.exists());
    }
}
