#![windows_subsystem = "windows"]

mod windows;

use morph_core::engines::image::ImageEngine;
use morph_core::engines::pdf::PdfEngine;
use morph_core::format::Format;
use morph_core::output::OutputOptions;
use morph_core::{detect_format_from_path, ConversionRegistry};
use std::env;
use std::path::{Path, PathBuf};
use windows::{coordinate_batch, ContextMenuManager, Notifier};

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
    morph convert <files...> --to <format> [options]
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
    morph convert page1.png page2.jpg page3.webp --to pdf
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

fn parse_output_arg(output_path: &Path) -> (Option<PathBuf>, Option<String>) {
    if output_path.is_dir() {
        (Some(output_path.to_path_buf()), None)
    } else if output_path.extension().is_some() {
        let parent = output_path
            .parent()
            .map(|p| p.to_path_buf())
            .filter(|p| !p.as_os_str().is_empty());
        let file_name = output_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        (parent, file_name)
    } else {
        (Some(output_path.to_path_buf()), None)
    }
}

fn handle_convert(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: Missing input file path.");
        std::process::exit(1);
    }

    let mut raw_inputs = Vec::new();
    let mut target_ext = String::new();
    let mut quality = None;
    let mut overwrite = false;
    let mut custom_output = None;
    let mut notify = true;

    let mut i = 0;
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
            other => {
                if !other.starts_with("--") {
                    raw_inputs.push(PathBuf::from(other));
                }
            }
        }
        i += 1;
    }

    if raw_inputs.is_empty() {
        log_err!("Error: Missing input file path.");
        std::process::exit(1);
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

    // If single file input and target is PDF, check for Windows Explorer multi-select batch coordination
    let input_paths = if raw_inputs.len() == 1 && target_format == Format::Pdf {
        match coordinate_batch(&raw_inputs[0], "convert_pdf") {
            Some(batch) => batch,
            None => {
                // Sibling process in Explorer multi-select batch: file has been queued for leader.
                std::process::exit(0);
            }
        }
    } else {
        raw_inputs
    };

    let (custom_dir, custom_file) = match custom_output {
        Some(ref p) => parse_output_arg(p),
        None => (None, None),
    };

    let output_options = OutputOptions {
        custom_output_dir: custom_dir,
        overwrite,
        custom_filename: custom_file,
    };

    // If target is PDF and we have multiple files (either from CLI or Explorer batch), merge them!
    if target_format == Format::Pdf && input_paths.len() > 1 {
        log_info!(
            "Merging {} files into a single PDF document in selection order...",
            input_paths.len()
        );
        for (idx, p) in input_paths.iter().enumerate() {
            log_info!("  [{}] {}", idx + 1, p.display());
        }

        match PdfEngine::merge_all_files(&input_paths, &output_options) {
            Ok(out_path) => {
                log_info!("Success: Generated '{}'", out_path.display());
                if notify {
                    let file_name = out_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("merged.pdf");
                    Notifier::send_notification(
                        "Morph - Merge Complete",
                        &format!("Merged {} files into {}", input_paths.len(), file_name),
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
        return;
    }

    // Otherwise, process each file
    for input_path in &input_paths {
        let source_format = match detect_format_from_path(input_path) {
            Ok(f) => f,
            Err(e) => {
                log_err!(
                    "Error detecting input format for '{}': {}",
                    input_path.display(),
                    e
                );
                if notify {
                    Notifier::send_notification("Morph Conversion Failed", &format!("{}", e));
                }
                if input_paths.len() == 1 {
                    std::process::exit(1);
                }
                continue;
            }
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
                    morph_core::DocumentEngine::convert_to_pdf(input_path, &output_options)
                }
                Format::Pdf => {
                    PdfEngine::merge_all_files(std::slice::from_ref(input_path), &output_options)
                }
                _ => ImageEngine::images_to_pdf(std::slice::from_ref(input_path), &output_options),
            }
        } else {
            ImageEngine::convert_image(input_path, &target_format, quality, &output_options)
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
                log_err!("Conversion error for '{}': {}", input_path.display(), e);
                if notify {
                    Notifier::send_notification("Morph - Conversion Failed", &format!("{}", e));
                }
                if input_paths.len() == 1 {
                    std::process::exit(1);
                }
            }
        }
    }
}

fn handle_merge(args: &[String]) {
    if args.is_empty() {
        log_err!("Error: No input files provided for merge.");
        std::process::exit(1);
    }

    let mut raw_inputs = Vec::new();
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
                    raw_inputs.push(PathBuf::from(other));
                }
            }
        }
        i += 1;
    }

    if raw_inputs.is_empty() {
        log_err!("Error: No valid input files specified.");
        std::process::exit(1);
    }

    let input_paths = if raw_inputs.len() == 1 {
        match coordinate_batch(&raw_inputs[0], "merge") {
            Some(batch) => batch,
            None => std::process::exit(0),
        }
    } else {
        raw_inputs
    };

    let (custom_dir, custom_file) = match custom_output {
        Some(ref p) => parse_output_arg(p),
        None => (None, None),
    };

    let output_options = OutputOptions {
        custom_output_dir: custom_dir,
        overwrite,
        custom_filename: custom_file,
    };

    log_info!(
        "Merging {} files into single PDF in selection order...",
        input_paths.len()
    );
    for (idx, p) in input_paths.iter().enumerate() {
        log_info!("  [{}] {}", idx + 1, p.display());
    }

    let result = PdfEngine::merge_all_files(&input_paths, &output_options);

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
                    &format!("Merged {} files into {}", input_paths.len(), name),
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
    let raw_path = PathBuf::from(&args[0]);

    let file_paths = match coordinate_batch(&raw_path, "edit") {
        Some(batch) => batch,
        None => std::process::exit(0),
    };

    // Check if morph-ui executable exists beside current executable
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("morph.exe"));
    let ui_exe = current_exe.with_file_name("morph-ui.exe");

    if ui_exe.exists() {
        let mut cmd = std::process::Command::new(ui_exe);
        for p in &file_paths {
            cmd.arg(p);
        }
        let _ = cmd.spawn();
    } else {
        log_info!(
            "Morph Editor requested for {} file(s). (Popup UI will open when morph-ui is built)",
            file_paths.len()
        );
    }
}

fn handle_register() {
    match ContextMenuManager::register_current_exe() {
        Ok(_) => {
            log_info!(
                "Success: Morph context menu successfully registered in Windows File Explorer."
            );
            log_info!("Right-click any supported image (.png, .jpg, .webp, .bmp, .tiff), document (.docx, .doc, .rtf, .odt), or PDF to access Morph actions!");
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
