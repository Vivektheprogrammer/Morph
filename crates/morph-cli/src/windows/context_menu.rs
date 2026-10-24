use winreg::enums::*;
use winreg::RegKey;

const IMAGE_EXTENSIONS: &[&str] = &[".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tiff"];
const DOCUMENT_EXTENSIONS: &[&str] = &[".docx", ".doc", ".rtf", ".odt"];

pub struct ContextMenuManager;

impl ContextMenuManager {
    /// Registers Morph context menu cascading menus in HKCU\Software\Classes\SystemFileAssociations
    pub fn register_current_exe() -> std::io::Result<()> {
        let current_exe = std::env::current_exe()?;
        let exe_str = current_exe.to_str().unwrap_or("morph.exe");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 1. Register for Image Extensions
        for &ext in IMAGE_EXTENSIONS {
            let assoc_path = format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\Morph",
                ext
            );
            let (morph_key, _) = hkcu.create_subkey(&assoc_path)?;
            morph_key.set_value("MUIVerb", &"Morph")?;
            morph_key.set_value("SubCommands", &"")?;
            morph_key.set_value("Icon", &format!("{},0", exe_str))?;

            // Submenu: Convert to PDF
            let (pdf_key, _) = morph_key.create_subkey(r"shell\ConvertToPdf")?;
            pdf_key.set_value("MUIVerb", &"Convert to PDF")?;
            let (cmd_pdf, _) = pdf_key.create_subkey("command")?;
            cmd_pdf.set_value("", &format!(r#""{}" convert "%1" --to pdf"#, exe_str))?;

            // Submenu: Convert to PNG (if not already PNG)
            if ext != ".png" {
                let (png_key, _) = morph_key.create_subkey(r"shell\ConvertToPng")?;
                png_key.set_value("MUIVerb", &"Convert to PNG")?;
                let (cmd_png, _) = png_key.create_subkey("command")?;
                cmd_png.set_value("", &format!(r#""{}" convert "%1" --to png"#, exe_str))?;
            }

            // Submenu: Convert to JPG (if not already JPG/JPEG)
            if ext != ".jpg" && ext != ".jpeg" {
                let (jpg_key, _) = morph_key.create_subkey(r"shell\ConvertToJpg")?;
                jpg_key.set_value("MUIVerb", &"Convert to JPG")?;
                let (cmd_jpg, _) = jpg_key.create_subkey("command")?;
                cmd_jpg.set_value("", &format!(r#""{}" convert "%1" --to jpg"#, exe_str))?;
            }

            // Submenu: Convert to WebP (if not already WebP)
            if ext != ".webp" {
                let (webp_key, _) = morph_key.create_subkey(r"shell\ConvertToWebp")?;
                webp_key.set_value("MUIVerb", &"Convert to WebP")?;
                let (cmd_webp, _) = webp_key.create_subkey("command")?;
                cmd_webp.set_value("", &format!(r#""{}" convert "%1" --to webp"#, exe_str))?;
            }

            // Submenu: Open in Morph Editor
            let (edit_key, _) = morph_key.create_subkey(r"shell\OpenEditor")?;
            edit_key.set_value("MUIVerb", &"Open in Morph Editor...")?;
            let (cmd_edit, _) = edit_key.create_subkey("command")?;
            cmd_edit.set_value("", &format!(r#""{}" edit "%1""#, exe_str))?;
        }

        // 2. Register for Document Extensions (Word / Office)
        for &ext in DOCUMENT_EXTENSIONS {
            let assoc_path = format!(
                r"Software\Classes\SystemFileAssociations\{}\shell\Morph",
                ext
            );
            let (morph_key, _) = hkcu.create_subkey(&assoc_path)?;
            morph_key.set_value("MUIVerb", &"Morph")?;
            morph_key.set_value("SubCommands", &"")?;
            morph_key.set_value("Icon", &format!("{},0", exe_str))?;

            // Submenu: Convert to PDF
            let (pdf_key, _) = morph_key.create_subkey(r"shell\ConvertToPdf")?;
            pdf_key.set_value("MUIVerb", &"Convert to PDF")?;
            let (cmd_pdf, _) = pdf_key.create_subkey("command")?;
            cmd_pdf.set_value("", &format!(r#""{}" convert "%1" --to pdf"#, exe_str))?;
        }

        // 3. Register for PDF files
        {
            let pdf_assoc_path = r"Software\Classes\SystemFileAssociations\.pdf\shell\Morph";
            let (morph_pdf_key, _) = hkcu.create_subkey(pdf_assoc_path)?;
            morph_pdf_key.set_value("MUIVerb", &"Morph")?;
            morph_pdf_key.set_value("SubCommands", &"")?;
            morph_pdf_key.set_value("Icon", &format!("{},0", exe_str))?;

            // Submenu: Edit Pages...
            let (edit_key, _) = morph_pdf_key.create_subkey(r"shell\EditPages")?;
            edit_key.set_value("MUIVerb", &"Edit PDF Pages...")?;
            let (cmd_edit, _) = edit_key.create_subkey("command")?;
            cmd_edit.set_value("", &format!(r#""{}" edit "%1""#, exe_str))?;

            // Submenu: Rotate 90° Clockwise
            let (rot_key, _) = morph_pdf_key.create_subkey(r"shell\Rotate90")?;
            rot_key.set_value("MUIVerb", &"Rotate 90° Clockwise")?;
            let (cmd_rot, _) = rot_key.create_subkey("command")?;
            cmd_rot.set_value(
                "",
                &format!(r#""{}" pdf rotate "%1" --degrees 90"#, exe_str),
            )?;
        }

        Ok(())
    }

    /// Completely removes all Morph context menu entries from HKCU\Software\Classes\SystemFileAssociations
    pub fn unregister() -> std::io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        for &ext in IMAGE_EXTENSIONS {
            let assoc_path = format!(r"Software\Classes\SystemFileAssociations\{}\shell", ext);
            if let Ok(shell_key) = hkcu.open_subkey_with_flags(&assoc_path, KEY_ALL_ACCESS) {
                let _ = shell_key.delete_subkey_all("Morph");
            }
        }

        for &ext in DOCUMENT_EXTENSIONS {
            let assoc_path = format!(r"Software\Classes\SystemFileAssociations\{}\shell", ext);
            if let Ok(shell_key) = hkcu.open_subkey_with_flags(&assoc_path, KEY_ALL_ACCESS) {
                let _ = shell_key.delete_subkey_all("Morph");
            }
        }

        let pdf_assoc_path = r"Software\Classes\SystemFileAssociations\.pdf\shell";
        if let Ok(shell_key) = hkcu.open_subkey_with_flags(pdf_assoc_path, KEY_ALL_ACCESS) {
            let _ = shell_key.delete_subkey_all("Morph");
        }

        Ok(())
    }

    /// Checks whether Morph is currently registered for .png
    pub fn is_registered() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Classes\SystemFileAssociations\.png\shell\Morph";
        hkcu.open_subkey(path).is_ok()
    }
}
