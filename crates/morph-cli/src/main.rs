#![windows_subsystem = "windows"]

mod windows;

use morph_core::engines::image::ImageEngine;
use morph_core::engines::pdf::PdfEngine;
use morph_core::format::Format;
use morph_core::output::OutputOptions;
use morph_core::{detect_format_from_path, ConversionRegistry};
use std::env;
use std::path::PathBuf;
use windows::{ContextMenuManager, Notifier};

macro_rules! log_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(windows)]
        {
            use std::io::Write;
            if let Ok(mut conout) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                let _ = writeln!(conout, "{}", msg);
            }
        }
    }};
}

macro_rules! log_err {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        #[cfg(windows)]
        {
            use std::io::Write;
            if let Ok(mut conout) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                let _ = writeln!(conout, "{}", msg);
            }
        }
    }};
}

fn print_help() {
    log_info!(
        r#"Morph - Lightweight, Offline-First Universal File Utility (v{})

USAGE:
    morph convert <file> --to <format> [options]
    morph merge <files...> [--output <path>] [options]
    morph pdf rotate <file> --degrees <90|180|270> [--pages <1,2,...>]
    morph pdf delete <file> --pages <1,2,...>
    morph edit <file>
    morph register
    morph unregister
    morph status

OPTIONS:
    --to <format>          Target format (pdf, png, jpg, webp, bmp, tiff)
    --quality <1-100>      Output quality for lossy formats (default: 85)
    --output <path>        Custom output directory or filename
    --overwrite            Overwrite destination if it already exists
    --no-notify            Suppress native Windows toast notifications

EXAMPLES:
    morph convert document.png --to pdf
    morph convert document.docx --to pdf
    morph convert photo.png --to jpg --quality 90
    morph merge page1.png page2.jpg page3.png --output combined.pdf
    morph pdf rotate scan.pdf --degrees 90
    morph register
"#,
        morph_core::version()
    );
}

fn main() {
    #[cfg(windows)]
    unsafe {
        // If run from a console/terminal (PowerShell or cmd), attach to it so CLI output works.
        // If launched by File Explorer, this fails silently with NO console window appearing!
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "convert" => handle_convert(&args[2..]),
        "merge" => handle_merge(&args[2..]),
        "pdf" => handle_pdf(&args[2..]),
        "edit" => handle_edit(&args[2..]),
        "register" => handle_register(),
        "unregister" => handle_unregister(),
        "status" => handle_status(),
        "help" | "--help" | "-h" => print_help(),
        other => {
            log_err!("Unknown command: '{}'. Run 'morph help' for usage.", other);
            std::process::exit(1);
        }
    }
}

fn handle_convert(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: Missing input file path.");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[0]);
    let mut target_ext = String::new();
    let mut quality = None;
    let mut overwrite = false;
    let mut custom_output = None;
    let mut notify = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--to" => {
                if i + 1 < args.len() {
                    target_ext = args[i + 1].clone();
                    i += 1;
                }
            }
            "--quality" => {
                if i + 1 < args.len() {
                    quality = args[i + 1].parse::<u8>().ok();
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    custom_output = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--overwrite" => overwrite = true,
            "--no-notify" => notify = false,
            _ => {}
        }
        i += 1;
    }

    if target_ext.is_empty() {
        log_err!("Error: Target format must be specified with --to <format>.");
        std::process::exit(1);
    }

    let target_format = match Format::from_extension(&target_ext) {
        Some(f) => f,
        None => {
            log_err!("Error: Unsupported target format '{}'.", target_ext);
            std::process::exit(1);
        }
    };

    let source_format = match detect_format_from_path(&input_path) {
        Ok(f) => f,
        Err(e) => {
            log_err!("Error detecting input format: {}", e);
            if notify {
                Notifier::send_notification("Morph Conversion Failed", &format!("{}", e));
            }
            std::process::exit(1);
        }
    };

    let output_options = OutputOptions {
        custom_output_dir: custom_output,
        overwrite,
        custom_filename: None,
    };

    log_info!(
        "Converting '{}' ({}) -> {}...",
        input_path.display(),
        source_format,
        target_format
    );

    let result = if target_format == Format::Pdf {
        match source_format {
            Format::Docx | Format::Doc | Format::Odt | Format::Rtf | Format::Txt => {
                morph_core::DocumentEngine::convert_to_pdf(&input_path, &output_options)
            }
            _ => ImageEngine::images_to_pdf(std::slice::from_ref(&input_path), &output_options),
        }
    } else {
        ImageEngine::convert_image(&input_path, &target_format, quality, &output_options)
    };

    match result {
        Ok(out_path) => {
            log_info!("Success: Generated '{}'", out_path.display());
            if notify {
                let file_name = out_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("output");
                Notifier::send_notification(
                    "Morph - Conversion Complete",
                    &format!("Saved to {}", file_name),
                );
            }
        }
        Err(e) => {
            log_err!("Conversion error: {}", e);
            if notify {
                Notifier::send_notification("Morph - Conversion Failed", &format!("{}", e));
            }
            std::process::exit(1);
        }
    }
}

fn handle_merge(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: No input files provided for merge.");
        std::process::exit(1);
    }

    let mut input_paths = Vec::new();
    let mut custom_output = None;
    let mut overwrite = false;
    let mut notify = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                if i + 1 < args.len() {
                    custom_output = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--overwrite" => overwrite = true,
            "--no-notify" => notify = false,
            other => {
                if !other.starts_with("--") {
                    input_paths.push(PathBuf::from(other));
                }
            }
        }
        i += 1;
    }

    if input_paths.is_empty() {
        log_err!("Error: No valid input files specified.");
        std::process::exit(1);
    }

    let output_options = OutputOptions {
        custom_output_dir: None,
        overwrite,
        custom_filename: custom_output
            .as_ref()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string()),
    };

    // Determine whether merging images or PDFs
    let first_fmt = match detect_format_from_path(&input_paths[0]) {
        Ok(f) => f,
        Err(e) => {
            log_err!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = if first_fmt == Format::Pdf {
        log_info!("Merging {} PDF documents...", input_paths.len());
        PdfEngine::merge_pdfs(&input_paths, &output_options)
    } else {
        log_info!("Combining {} images into PDF...", input_paths.len());
        ImageEngine::images_to_pdf(&input_paths, &output_options)
    };

    match result {
        Ok(out_path) => {
            log_info!("Success: Created '{}'", out_path.display());
            if notify {
                let name = out_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("merged.pdf");
                Notifier::send_notification(
                    "Morph - Merge Complete",
                    &format!("Saved to {}", name),
                );
            }
        }
        Err(e) => {
            log_err!("Merge error: {}", e);
            if notify {
                Notifier::send_notification("Morph - Merge Failed", &format!("{}", e));
            }
            std::process::exit(1);
        }
    }
}

fn handle_pdf(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: Missing PDF subcommand. Use 'morph pdf rotate' or 'morph pdf delete'.");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "rotate" => {
            if args.len() < 2 {
                log_err!("Error: Missing PDF file path for rotation.");
                std::process::exit(1);
            }
            let file_path = PathBuf::from(&args[1]);
            let mut degrees = 90;
            let mut pages = Vec::new();
            let mut overwrite = false;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--degrees" => {
                        if i + 1 < args.len() {
                            degrees = args[i + 1].parse::<i32>().unwrap_or(90);
                            i += 1;
                        }
                    }
                    "--pages" => {
                        if i + 1 < args.len() {
                            pages = args[i + 1]
                                .split(',')
                                .filter_map(|s| s.trim().parse::<u32>().ok())
                                .collect();
                            i += 1;
                        }
                    }
                    "--overwrite" => overwrite = true,
                    _ => {}
                }
                i += 1;
            }

            let options = OutputOptions {
                overwrite,
                ..Default::default()
            };

            match PdfEngine::rotate_pages(&file_path, &pages, degrees, &options) {
                Ok(out) => {
                    log_info!("Success: Rotated PDF saved as '{}'", out.display());
                    Notifier::send_notification(
                        "Morph - PDF Rotated",
                        "Pages successfully rotated",
                    );
                }
                Err(e) => {
                    log_err!("Rotation error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "delete" => {
            if args.len() < 2 {
                log_err!("Error: Missing PDF file path.");
                std::process::exit(1);
            }
            let file_path = PathBuf::from(&args[1]);
            let mut pages = Vec::new();
            let mut overwrite = false;

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--pages" => {
                        if i + 1 < args.len() {
                            pages = args[i + 1]
                                .split(',')
                                .filter_map(|s| s.trim().parse::<u32>().ok())
                                .collect();
                            i += 1;
                        }
                    }
                    "--overwrite" => overwrite = true,
                    _ => {}
                }
                i += 1;
            }

            if pages.is_empty() {
                log_err!("Error: Specify pages to delete with --pages <1,2,...>");
                std::process::exit(1);
            }

            let options = OutputOptions {
                overwrite,
                ..Default::default()
            };

            match PdfEngine::delete_pages(&file_path, &pages, &options) {
                Ok(out) => {
                    log_info!(
                        "Success: Saved PDF after deleting pages as '{}'",
                        out.display()
                    );
                    Notifier::send_notification(
                        "Morph - Pages Deleted",
                        "Pages successfully removed",
                    );
                }
                Err(e) => {
                    log_err!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        other => {
            log_err!("Unknown PDF command '{}'", other);
            std::process::exit(1);
        }
    }
}

fn handle_edit(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: Missing file path for editor.");
        std::process::exit(1);
    }
    let file_path = PathBuf::from(&args[0]);

    // Check if morph-ui executable exists beside current executable
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("morph.exe"));
    let ui_exe = current_exe.with_file_name("morph-ui.exe");

    if ui_exe.exists() {
        let _ = std::process::Command::new(ui_exe).arg(&file_path).spawn();
    } else {
        log_info!(
            "Morph Editor requested for '{}'. (Popup UI will open when morph-ui is built)",
            file_path.display()
        );
    }
}

fn handle_register() {
    match ContextMenuManager::register_current_exe() {
        Ok(_) => {
            log_info!(
                "Success: Morph context menu successfully registered in Windows File Explorer."
            );
            log_info!("Right-click any supported image (.png, .jpg, .webp, .bmp, .tiff) or PDF to access Morph actions!");
        }
        Err(e) => {
            log_err!("Failed to register context menu: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_unregister() {
    match ContextMenuManager::unregister() {
        Ok(_) => {
            log_info!(
                "Success: Morph context menu successfully removed from Windows File Explorer."
            );
        }
        Err(e) => {
            log_err!("Failed to unregister context menu: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_status() {
    log_info!("Morph Desktop Utility v{}", morph_core::version());
    log_info!("Offline capability: 100% Fully Offline (Zero Network / Cloud Dependency)");
    log_info!(
        "Context Menu Integration: {}",
        if ContextMenuManager::is_registered() {
            "Registered (Active in Windows File Explorer)"
        } else {
            "Not Registered (Run 'morph register' to enable)"
        }
    );

    let reg = ConversionRegistry::new();
    log_info!(
        "\nRegistered Conversion Capabilities ({} operations):",
        reg.operations().len()
    );
    for op in reg.operations() {
        log_info!(
            " - [{}] {} -> {:?}",
            op.name, op.description, op.target_format
        );
    }
}
