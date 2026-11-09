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

    /// Resolves the intended destination path for merging multiple files into one.
    ///
    /// By default:
    /// - Determines a merged base name:
    ///   - If sources share a common prefix (e.g. "adhar f" and "adhar b" -> "adhar"),
    ///     uses `<common_prefix>_merged.<ext>`.
    ///   - Otherwise uses `<first_source_stem>_merged.<ext>`.
    ///   - If `custom_filename` is provided, honors that.
    /// - Performs collision avoidance if `<name>.<ext>` already exists: `<name> (1).<ext>`, etc.
    pub fn resolve_merge_destination(
        sources: &[PathBuf],
        target_extension: &str,
        options: &OutputOptions,
    ) -> Result<PathBuf> {
        if sources.is_empty() {
            return Err(MorphError::InvalidInput(
                "No source files provided to resolve merge destination".into(),
            ));
        }

        let parent_dir = if let Some(ref dir) = options.custom_output_dir {
            dir.as_path()
        } else {
            sources[0].parent().unwrap_or_else(|| Path::new("."))
        };

        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }

        let clean_ext = target_extension.trim_start_matches('.');

        let stem = if let Some(ref custom_name) = options.custom_filename {
            Path::new(custom_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("merged")
                .to_string()
        } else {
            let stems: Vec<String> = sources
                .iter()
                .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
                .collect();

            let common_prefix = if stems.len() > 1 {
                find_common_stem_prefix(&stems)
            } else {
                None
            };

            if let Some(prefix) = common_prefix {
                if prefix.to_lowercase().ends_with("_merged") || prefix.to_lowercase() == "merged" {
                    prefix
                } else {
                    format!("{}_merged", prefix)
                }
            } else {
                let first_stem = stems.first().map(|s| s.as_str()).unwrap_or("merged");
                if first_stem.to_lowercase().ends_with("_merged") || first_stem.to_lowercase() == "merged" {
                    first_stem.to_string()
                } else {
                    format!("{}_merged", first_stem)
                }
            }
        };

        let initial_name = format!("{}.{}", stem, clean_ext);
        let initial_path = parent_dir.join(&initial_name);

        if options.overwrite || !initial_path.exists() {
            return Ok(initial_path);
        }

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

    /// Creates a temporary file with a specific extension in the destination directory
    pub fn create_staging_file_with_ext(
        destination_dir: &Path,
        extension: &str,
    ) -> Result<NamedTempFile> {
        if !destination_dir.exists() {
            fs::create_dir_all(destination_dir)?;
        }
        let clean_ext = extension.trim_start_matches('.');
        let suffix = format!(".{}", clean_ext);
        let temp = tempfile::Builder::new()
            .prefix(".morph-tmp-")
            .suffix(&suffix)
            .tempfile_in(destination_dir)?;
        Ok(temp)
    }

    /// Creates a temporary file in the destination directory to ensure
    /// atomic renaming on the same filesystem/drive volume.
    pub fn create_staging_file(destination_dir: &Path) -> Result<NamedTempFile> {
        Self::create_staging_file_with_ext(destination_dir, "tmp")
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

fn find_common_stem_prefix(stems: &[String]) -> Option<String> {
    if stems.len() < 2 {
        return None;
    }
    let first = &stems[0];
    let mut prefix_len = 0;
    for (i, c) in first.char_indices() {
        if stems.iter().all(|s| s.starts_with(&first[..i + c.len_utf8()])) {
            prefix_len = i + c.len_utf8();
        } else {
            break;
        }
    }
    if prefix_len == 0 {
        return None;
    }
    let raw = &first[..prefix_len];
    let trimmed = raw.trim_end_matches(|c: char| c.is_whitespace() || c == '_' || c == '-');
    if trimmed.chars().count() >= 2 {
        Some(trimmed.to_string())
    } else {
        None
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

    #[test]
    fn test_resolve_merge_destination() {
        let temp = tempdir().unwrap();
        let f1 = temp.path().join("adhar f.jpg");
        let f2 = temp.path().join("adhar b.jpg");
        fs::write(&f1, b"1").unwrap();
        fs::write(&f2, b"2").unwrap();

        let options = OutputOptions::default();

        // Should resolve to adhar_merged.pdf
        let dest = OutputManager::resolve_merge_destination(&[f1.clone(), f2.clone()], "pdf", &options).unwrap();
        assert_eq!(dest, temp.path().join("adhar_merged.pdf"));

        // If adhar_merged.pdf exists, should resolve to adhar_merged (1).pdf
        fs::write(&dest, b"existing").unwrap();
        let dest2 = OutputManager::resolve_merge_destination(&[f1.clone(), f2.clone()], "pdf", &options).unwrap();
        assert_eq!(dest2, temp.path().join("adhar_merged (1).pdf"));

        // Files without common prefix: should use first file stem + _merged
        let a = temp.path().join("report.docx");
        let b = temp.path().join("notes.txt");
        let dest_no_prefix = OutputManager::resolve_merge_destination(&[a, b], "pdf", &options).unwrap();
        assert_eq!(dest_no_prefix, temp.path().join("report_merged.pdf"));
    }
}
