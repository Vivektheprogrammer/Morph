use crate::error::{MorphError, Result};
use crate::output::{OutputManager, OutputOptions};
use lopdf::{Document, Object};
use std::path::{Path, PathBuf};

pub struct PdfEngine;

impl PdfEngine {
    /// Losslessly merges multiple PDF files into a single combined PDF document.
    pub fn merge_pdfs(input_paths: &[PathBuf], output_options: &OutputOptions) -> Result<PathBuf> {
        if input_paths.is_empty() {
            return Err(MorphError::InvalidInput(
                "No input PDF files provided for merge".into(),
            ));
        }

        let destination =
            OutputManager::resolve_destination(&input_paths[0], "pdf", output_options)?;

        // If only 1 PDF is provided, copy/finalize directly
        if input_paths.len() == 1 {
            let staging = OutputManager::create_staging_file(
                destination.parent().unwrap_or_else(|| Path::new(".")),
            )?;
            std::fs::copy(&input_paths[0], staging.path())?;
            return OutputManager::finalize_atomic(staging, &destination);
        }

        let mut max_id = 1;
        let mut documents = Vec::new();

        for p in input_paths {
            let mut doc = Document::load(p).map_err(|e| {
                MorphError::PdfError(format!("Failed to load PDF '{}': {}", p.display(), e))
            })?;
            doc.renumber_objects_with(max_id);
            max_id = doc.max_id + 1;
            documents.push(doc);
        }

        let mut merged_doc = Document::with_version("1.7");
        let mut pages_obj_ids = Vec::new();

        for doc in &documents {
            let pages = doc.get_pages();
            for (_page_num, object_id) in pages {
                pages_obj_ids.push(object_id);
            }

            for (object_id, object) in &doc.objects {
                // Copy all objects except the root Catalog and Pages tree
                if let Ok(dict) = object.as_dict() {
                    let type_name = dict.get(b"Type").and_then(|o| o.as_name()).unwrap_or(b"");
                    if type_name == b"Catalog" || type_name == b"Pages" {
                        continue;
                    }
                }
                merged_doc.objects.insert(*object_id, object.clone());
            }
        }

        // Create new Pages dictionary
        let pages_id = (max_id, 0);
        max_id += 1;
        let catalog_id = (max_id, 0);

        let count = pages_obj_ids.len() as i64;
        let kids_array = Object::Array(
            pages_obj_ids
                .iter()
                .map(|&id| Object::Reference(id))
                .collect(),
        );

        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Count", Object::Integer(count));
        pages_dict.set("Kids", kids_array);

        merged_doc
            .objects
            .insert(pages_id, Object::Dictionary(pages_dict));

        // Update parent reference in all pages to the new Pages dictionary
        for &page_id in &pages_obj_ids {
            if let Ok(page_obj) = merged_doc.get_object_mut(page_id) {
                if let Ok(page_dict) = page_obj.as_dict_mut() {
                    page_dict.set("Parent", Object::Reference(pages_id));
                }
            }
        }

        // Create Catalog dictionary
        let mut catalog_dict = lopdf::Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages_id));

        merged_doc
            .objects
            .insert(catalog_id, Object::Dictionary(catalog_dict));
        merged_doc
            .trailer
            .set("Root", Object::Reference(catalog_id));
        merged_doc.max_id = max_id;

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        merged_doc
            .save(staging.path())
            .map_err(|e| MorphError::PdfError(format!("Failed to save merged PDF: {}", e)))?;

        OutputManager::finalize_atomic(staging, &destination)
    }

    /// Rotate specific pages by degrees (90, 180, 270) losslessly.
    pub fn rotate_pages(
        input_path: &Path,
        pages_to_rotate: &[u32],
        degrees: i32,
        output_options: &OutputOptions,
    ) -> Result<PathBuf> {
        let destination = OutputManager::resolve_destination(input_path, "pdf", output_options)?;

        let mut doc = Document::load(input_path).map_err(|e| {
            MorphError::PdfError(format!(
                "Failed to load PDF '{}': {}",
                input_path.display(),
                e
            ))
        })?;

        let pages = doc.get_pages();
        let rotate_all = pages_to_rotate.is_empty();

        for (&page_num, &page_id) in &pages {
            if rotate_all || pages_to_rotate.contains(&page_num) {
                if let Ok(page_obj) = doc.get_object_mut(page_id) {
                    if let Ok(page_dict) = page_obj.as_dict_mut() {
                        let current_rot = page_dict
                            .get(b"Rotate")
                            .and_then(|r| r.as_i64())
                            .unwrap_or(0);
                        let mut new_rot = (current_rot + degrees as i64) % 360;
                        if new_rot < 0 {
                            new_rot += 360;
                        }
                        page_dict.set("Rotate", Object::Integer(new_rot));
                    }
                }
            }
        }

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        doc.save(staging.path())
            .map_err(|e| MorphError::PdfError(format!("Failed to save rotated PDF: {}", e)))?;

        OutputManager::finalize_atomic(staging, &destination)
    }

    /// Delete specific 1-based page numbers from a PDF document.
    pub fn delete_pages(
        input_path: &Path,
        pages_to_delete: &[u32],
        output_options: &OutputOptions,
    ) -> Result<PathBuf> {
        let destination = OutputManager::resolve_destination(input_path, "pdf", output_options)?;

        let mut doc = Document::load(input_path).map_err(|e| {
            MorphError::PdfError(format!(
                "Failed to load PDF '{}': {}",
                input_path.display(),
                e
            ))
        })?;

        doc.delete_pages(pages_to_delete);

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        doc.save(staging.path()).map_err(|e| {
            MorphError::PdfError(format!("Failed to save PDF after page deletion: {}", e))
        })?;

        OutputManager::finalize_atomic(staging, &destination)
    }

    /// Reorder or extract pages according to a sequence of 1-based page numbers.
    pub fn reorder_pages(
        input_path: &Path,
        new_order: &[u32],
        output_options: &OutputOptions,
    ) -> Result<PathBuf> {
        if new_order.is_empty() {
            return Err(MorphError::InvalidInput(
                "Page order cannot be empty".into(),
            ));
        }

        let destination = OutputManager::resolve_destination(input_path, "pdf", output_options)?;

        let doc = Document::load(input_path).map_err(|e| {
            MorphError::PdfError(format!(
                "Failed to load PDF '{}': {}",
                input_path.display(),
                e
            ))
        })?;

        let existing_pages = doc.get_pages();
        let mut ordered_ids = Vec::with_capacity(new_order.len());

        for &page_num in new_order {
            if let Some(&id) = existing_pages.get(&page_num) {
                ordered_ids.push(id);
            } else {
                return Err(MorphError::PdfError(format!(
                    "Requested page {} does not exist in PDF '{}' (total pages: {})",
                    page_num,
                    input_path.display(),
                    existing_pages.len()
                )));
            }
        }

        let mut new_doc = doc.clone();

        // Update Pages tree kids to match new order
        if let Some(first_page_id) = new_doc.get_pages().values().next().copied() {
            // Find parent Pages object
            if let Ok(page_obj) = new_doc.get_object(first_page_id) {
                if let Ok(parent_ref) = page_obj
                    .as_dict()
                    .and_then(|d| d.get(b"Parent"))
                    .and_then(|p| p.as_reference())
                {
                    if let Ok(parent_obj) = new_doc.get_object_mut(parent_ref) {
                        if let Ok(parent_dict) = parent_obj.as_dict_mut() {
                            parent_dict.set(
                                "Kids",
                                Object::Array(
                                    ordered_ids
                                        .iter()
                                        .map(|&id| Object::Reference(id))
                                        .collect(),
                                ),
                            );
                            parent_dict.set("Count", Object::Integer(ordered_ids.len() as i64));
                        }
                    }
                }
            }
        }

        let staging = OutputManager::create_staging_file(
            destination.parent().unwrap_or_else(|| Path::new(".")),
        )?;

        new_doc
            .save(staging.path())
            .map_err(|e| MorphError::PdfError(format!("Failed to save reordered PDF: {}", e)))?;

        OutputManager::finalize_atomic(staging, &destination)
    }

    /// Returns the total page count of a PDF document.
    pub fn page_count(path: &Path) -> Result<usize> {
        let doc = Document::load(path).map_err(|e| {
            MorphError::PdfError(format!("Failed to load PDF '{}': {}", path.display(), e))
        })?;
        Ok(doc.get_pages().len())
    }
}
