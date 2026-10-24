use crate::error::{MorphError, Result};
use crate::output::{OutputManager, OutputOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DocumentEngine;

struct TempPathGuard {
    path: PathBuf,
    keep: bool,
}

impl TempPathGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        if !self.keep && self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl DocumentEngine {
    /// Checks if Microsoft Word is available via Windows COM / App Paths
    pub fn is_msword_available() -> bool {
        #[cfg(windows)]
        {
            use winreg::enums::HKEY_LOCAL_MACHINE;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\winword.exe")
                .is_ok()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// Checks if LibreOffice headless binary is installed
    pub fn is_libreoffice_available() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
            PathBuf::from("soffice"),
        ];

        for path in &candidates {
            if path.exists() {
                return Some(path.clone());
            }
        }
        None
    }

    /// Converts a document (DOCX, DOC, ODT, RTF) to PDF with maximum fidelity.
    pub fn convert_to_pdf(input_path: &Path, output_options: &OutputOptions) -> Result<PathBuf> {
        let destination = OutputManager::resolve_destination(input_path, "pdf", output_options)?;
        let parent_dir = destination.parent().unwrap_or_else(|| Path::new("."));

        let temp_filename = format!(
            ".morph-tmp-{}.pdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let staging_path = parent_dir.join(temp_filename);
        let mut guard = TempPathGuard::new(staging_path.clone());

        let abs_input = if let Ok(canon) = input_path.canonicalize() {
            let clean = canon
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_string();
            PathBuf::from(clean)
        } else {
            input_path.to_path_buf()
        };

        if Self::is_msword_available() {
            Self::convert_with_msword(&abs_input, &staging_path)?;
        } else if let Some(soffice_path) = Self::is_libreoffice_available() {
            Self::convert_with_libreoffice(&soffice_path, &abs_input, &staging_path)?;
        } else {
            return Err(MorphError::UnsupportedFormat(
                "Document conversion requires Microsoft Word or LibreOffice to be installed locally."
                    .to_string(),
            ));
        }

        // Validate staging output
        if !staging_path.exists() {
            return Err(MorphError::ValidationFailed(format!(
                "Document engine failed to produce output for '{}'",
                destination.display()
            )));
        }

        let metadata = fs::metadata(&staging_path)?;
        if metadata.len() == 0 {
            return Err(MorphError::ValidationFailed(format!(
                "Output file for '{}' is empty (0 bytes)",
                destination.display()
            )));
        }

        if destination.exists() {
            fs::remove_file(&destination)?;
        }

        fs::rename(&staging_path, &destination).map_err(|e| {
            MorphError::IoError(std::io::Error::other(format!(
                "Failed to finalize output to '{}': {}",
                destination.display(),
                e
            )))
        })?;

        guard.keep = true;
        Ok(destination)
    }

    #[cfg(windows)]
    fn convert_with_msword(input_path: &Path, staging_output: &Path) -> Result<()> {
        let in_str = input_path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\'', "''");
        let out_str = staging_output
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\'', "''");

        // PowerShell COM script to convert DOCX -> PDF with full formatting fidelity
        let ps_script = format!(
            r#"$ErrorActionPreference = 'Stop';
$word = New-Object -ComObject Word.Application;
$word.Visible = $false;
$word.DisplayAlerts = 0;
try {{
    $doc = $word.Documents.Open('{in_str}');
    if ($null -eq $doc) {{
        throw "Could not open document '{in_str}'";
    }}
    $doc.ExportAsFixedFormat('{out_str}', 17);
    $doc.Close([ref]$false);
}} finally {{
    $word.Quit();
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($word) | Out-Null;
}}
"#
        );

        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("powershell.exe");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &ps_script,
            ])
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(MorphError::PdfError(format!(
                "Microsoft Word conversion failed: {}",
                err.trim()
            )));
        }

        Ok(())
    }

    #[cfg(not(windows))]
    fn convert_with_msword(_input_path: &Path, _staging_output: &Path) -> Result<()> {
        Err(MorphError::UnsupportedFormat(
            "Microsoft Word COM automation is only available on Windows".into(),
        ))
    }

    fn convert_with_libreoffice(
        soffice_path: &Path,
        input_path: &Path,
        staging_output: &Path,
    ) -> Result<()> {
        let temp_dir = staging_output.parent().unwrap_or_else(|| Path::new("."));
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new(soffice_path);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .args([
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                &temp_dir.to_string_lossy(),
                &input_path.to_string_lossy(),
            ])
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(MorphError::PdfError(format!(
                "LibreOffice conversion failed: {}",
                err.trim()
            )));
        }

        // LibreOffice writes to temp_dir/<stem>.pdf, so copy to staging_output if different
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc");
        let lo_pdf = temp_dir.join(format!("{}.pdf", stem));
        if lo_pdf.exists() && lo_pdf != staging_output {
            std::fs::copy(&lo_pdf, staging_output)?;
            let _ = std::fs::remove_file(lo_pdf);
        }

        Ok(())
    }
}
