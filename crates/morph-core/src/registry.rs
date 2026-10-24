use crate::format::Format;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionOperation {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source_formats: Vec<Format>,
    pub target_format: Format,
    pub offline_capable: bool,
    pub instant_capable: bool,
    pub requires_editor: bool,
    pub lossless: bool,
}

pub struct ConversionRegistry {
    operations: Vec<ConversionOperation>,
}

impl Default for ConversionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversionRegistry {
    pub fn new() -> Self {
        let ops = vec![
            // 1. Image -> PDF (Direct Instant Action)
            ConversionOperation {
                id: "img_to_pdf",
                name: "Convert to PDF",
                description: "Convert image directly to a clean PDF page",
                source_formats: vec![
                    Format::Png,
                    Format::Jpeg,
                    Format::Webp,
                    Format::Bmp,
                    Format::Tiff,
                    Format::Gif,
                    Format::Ico,
                ],
                target_format: Format::Pdf,
                offline_capable: true,
                instant_capable: true,
                requires_editor: false,
                lossless: true,
            },
            // 2. Image Cross-Conversions
            ConversionOperation {
                id: "img_to_png",
                name: "Convert to PNG",
                description: "Convert image to PNG format",
                source_formats: vec![
                    Format::Jpeg,
                    Format::Webp,
                    Format::Bmp,
                    Format::Tiff,
                    Format::Gif,
                    Format::Ico,
                ],
                target_format: Format::Png,
                offline_capable: true,
                instant_capable: true,
                requires_editor: false,
                lossless: true,
            },
            ConversionOperation {
                id: "img_to_jpg",
                name: "Convert to JPG",
                description: "Convert image to JPG format",
                source_formats: vec![
                    Format::Png,
                    Format::Webp,
                    Format::Bmp,
                    Format::Tiff,
                    Format::Gif,
                ],
                target_format: Format::Jpeg,
                offline_capable: true,
                instant_capable: true,
                requires_editor: false,
                lossless: false,
            },
            ConversionOperation {
                id: "img_to_webp",
                name: "Convert to WebP",
                description: "Convert image to WebP format",
                source_formats: vec![
                    Format::Png,
                    Format::Jpeg,
                    Format::Bmp,
                    Format::Tiff,
                    Format::Gif,
                ],
                target_format: Format::Webp,
                offline_capable: true,
                instant_capable: true,
                requires_editor: false,
                lossless: false,
            },
            // 3. Multi-Image to PDF (Popup Editor or Direct Batch)
            ConversionOperation {
                id: "multi_img_to_pdf",
                name: "Combine into PDF",
                description: "Combine multiple images into a single multi-page PDF document",
                source_formats: vec![
                    Format::Png,
                    Format::Jpeg,
                    Format::Webp,
                    Format::Bmp,
                    Format::Tiff,
                ],
                target_format: Format::Pdf,
                offline_capable: true,
                instant_capable: true,
                requires_editor: true,
                lossless: true,
            },
            // 4. Structural PDF Manipulation
            ConversionOperation {
                id: "pdf_merge",
                name: "Merge PDFs",
                description: "Combine multiple PDF documents into one without rasterization",
                source_formats: vec![Format::Pdf],
                target_format: Format::Pdf,
                offline_capable: true,
                instant_capable: true,
                requires_editor: true,
                lossless: true,
            },
            ConversionOperation {
                id: "pdf_edit_pages",
                name: "Edit PDF Pages",
                description: "Rearrange, rotate, delete, or extract pages from a PDF",
                source_formats: vec![Format::Pdf],
                target_format: Format::Pdf,
                offline_capable: true,
                instant_capable: false,
                requires_editor: true,
                lossless: true,
            },
        ];

        Self { operations: ops }
    }

    pub fn operations(&self) -> &[ConversionOperation] {
        &self.operations
    }

    pub fn find_instant_operations(&self, source: &Format) -> Vec<&ConversionOperation> {
        self.operations
            .iter()
            .filter(|op| op.instant_capable && op.source_formats.contains(source))
            .collect()
    }

    pub fn find_editor_operations(&self, source: &Format) -> Vec<&ConversionOperation> {
        self.operations
            .iter()
            .filter(|op| op.requires_editor && op.source_formats.contains(source))
            .collect()
    }

    pub fn can_convert(&self, from: &Format, to: &Format) -> bool {
        self.operations
            .iter()
            .any(|op| op.source_formats.contains(from) && &op.target_format == to)
    }

    pub fn get_operation(&self, id: &str) -> Option<&ConversionOperation> {
        self.operations.iter().find(|op| op.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lookups() {
        let registry = ConversionRegistry::new();
        assert!(registry.can_convert(&Format::Png, &Format::Pdf));
        assert!(registry.can_convert(&Format::Jpeg, &Format::Png));
        assert!(registry.can_convert(&Format::Png, &Format::Jpeg));
        assert!(registry.can_convert(&Format::Pdf, &Format::Pdf));

        let instant_png = registry.find_instant_operations(&Format::Png);
        assert!(!instant_png.is_empty());
        assert!(instant_png.iter().any(|op| op.target_format == Format::Pdf));
    }
}
