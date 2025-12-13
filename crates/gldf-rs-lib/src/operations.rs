//! CRUD operations for GldfProduct.
//!
//! This module provides methods for creating, reading, updating, and deleting
//! elements within a GLDF product structure.

use crate::gldf::general_definitions::files::File;
use crate::gldf::general_definitions::geometries::{Geometries, ModelGeometry, SimpleGeometry};
use crate::gldf::general_definitions::lightsources::{
    ChangeableLightSource, Emitter, Emitters, FixedLightSource, LightSources,
};
use crate::gldf::general_definitions::photometries::{Photometries, Photometry};
use crate::gldf::product_definitions::{ProductMetaData, Variant, Variants};
use crate::gldf::GldfProduct;
use anyhow::{anyhow, Result};
use std::collections::HashSet;

impl GldfProduct {
    // ==================== ID Generation ====================

    /// Generates a unique ID with the given prefix.
    ///
    /// Scans all existing IDs in the product and generates a new one
    /// that doesn't conflict.
    pub fn generate_unique_id(&self, prefix: &str) -> String {
        let existing_ids = self.get_all_ids();
        let mut counter = 1;
        loop {
            let candidate = format!("{}_{}", prefix, counter);
            if !existing_ids.contains(&candidate) {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Gets all IDs used in the product.
    pub fn get_all_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();

        // File IDs
        for file in &self.general_definitions.files.file {
            ids.insert(file.id.clone());
        }

        // Variant IDs
        if let Some(ref variants) = self.product_definitions.variants {
            for variant in &variants.variant {
                ids.insert(variant.id.clone());
            }
        }

        // Photometry IDs
        if let Some(ref photometries) = self.general_definitions.photometries {
            for photometry in &photometries.photometry {
                ids.insert(photometry.id.clone());
            }
        }

        // Geometry IDs
        if let Some(ref geometries) = self.general_definitions.geometries {
            for geom in &geometries.simple_geometry {
                ids.insert(geom.id.clone());
            }
            for geom in &geometries.model_geometry {
                ids.insert(geom.id.clone());
            }
        }

        // Light source IDs
        if let Some(ref light_sources) = self.general_definitions.light_sources {
            for source in &light_sources.fixed_light_source {
                ids.insert(source.id.clone());
            }
            for source in &light_sources.changeable_light_source {
                ids.insert(source.id.clone());
            }
        }

        // Emitter IDs
        if let Some(ref emitters) = self.general_definitions.emitters {
            for emitter in &emitters.emitter {
                ids.insert(emitter.id.clone());
            }
        }

        ids
    }

    /// Gets all file IDs that are referenced by other elements.
    pub fn get_referenced_file_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();

        // Photometry file references
        if let Some(ref photometries) = self.general_definitions.photometries {
            for photometry in &photometries.photometry {
                if let Some(ref file_ref) = photometry.photometry_file_reference {
                    ids.insert(file_ref.file_id.clone());
                }
            }
        }

        // Geometry file references
        if let Some(ref geometries) = self.general_definitions.geometries {
            for geom in &geometries.model_geometry {
                for file_ref in &geom.geometry_file_reference {
                    ids.insert(file_ref.file_id.clone());
                }
            }
        }

        ids
    }

    // ==================== File Operations ====================

    /// Adds a new file definition.
    ///
    /// # Errors
    /// Returns an error if a file with the same ID already exists.
    pub fn add_file(&mut self, file: File) -> Result<()> {
        // Check for duplicate ID
        if self.general_definitions.files.file.iter().any(|f| f.id == file.id) {
            return Err(anyhow!("File with ID '{}' already exists", file.id));
        }
        self.general_definitions.files.file.push(file);
        Ok(())
    }

    /// Updates an existing file definition.
    ///
    /// # Errors
    /// Returns an error if no file with the given ID exists.
    pub fn update_file(&mut self, id: &str, file: File) -> Result<()> {
        let pos = self
            .general_definitions
            .files
            .file
            .iter()
            .position(|f| f.id == id)
            .ok_or_else(|| anyhow!("File with ID '{}' not found", id))?;

        self.general_definitions.files.file[pos] = file;
        Ok(())
    }

    /// Removes a file definition by ID.
    ///
    /// # Errors
    /// Returns an error if no file with the given ID exists.
    pub fn remove_file(&mut self, id: &str) -> Result<File> {
        let pos = self
            .general_definitions
            .files
            .file
            .iter()
            .position(|f| f.id == id)
            .ok_or_else(|| anyhow!("File with ID '{}' not found", id))?;

        Ok(self.general_definitions.files.file.remove(pos))
    }

    /// Gets a file definition by ID.
    pub fn get_file(&self, id: &str) -> Option<&File> {
        self.general_definitions.files.file.iter().find(|f| f.id == id)
    }

    /// Gets a mutable reference to a file definition by ID.
    pub fn get_file_mut(&mut self, id: &str) -> Option<&mut File> {
        self.general_definitions.files.file.iter_mut().find(|f| f.id == id)
    }

    // ==================== Variant Operations ====================

    /// Adds a new variant.
    ///
    /// # Errors
    /// Returns an error if a variant with the same ID already exists.
    pub fn add_variant(&mut self, variant: Variant) -> Result<()> {
        // Ensure variants container exists
        if self.product_definitions.variants.is_none() {
            self.product_definitions.variants = Some(Variants::default());
        }

        let variants = self.product_definitions.variants.as_mut().unwrap();

        // Check for duplicate ID
        if variants.variant.iter().any(|v| v.id == variant.id) {
            return Err(anyhow!("Variant with ID '{}' already exists", variant.id));
        }

        variants.variant.push(variant);
        Ok(())
    }

    /// Updates an existing variant.
    ///
    /// # Errors
    /// Returns an error if no variant with the given ID exists.
    pub fn update_variant(&mut self, id: &str, variant: Variant) -> Result<()> {
        let variants = self
            .product_definitions
            .variants
            .as_mut()
            .ok_or_else(|| anyhow!("No variants defined"))?;

        let pos = variants
            .variant
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| anyhow!("Variant with ID '{}' not found", id))?;

        variants.variant[pos] = variant;
        Ok(())
    }

    /// Removes a variant by ID.
    ///
    /// # Errors
    /// Returns an error if no variant with the given ID exists.
    pub fn remove_variant(&mut self, id: &str) -> Result<Variant> {
        let variants = self
            .product_definitions
            .variants
            .as_mut()
            .ok_or_else(|| anyhow!("No variants defined"))?;

        let pos = variants
            .variant
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| anyhow!("Variant with ID '{}' not found", id))?;

        Ok(variants.variant.remove(pos))
    }

    /// Gets a variant by ID.
    pub fn get_variant(&self, id: &str) -> Option<&Variant> {
        self.product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.iter().find(|var| var.id == id))
    }

    /// Gets a mutable reference to a variant by ID.
    pub fn get_variant_mut(&mut self, id: &str) -> Option<&mut Variant> {
        self.product_definitions
            .variants
            .as_mut()
            .and_then(|v| v.variant.iter_mut().find(|var| var.id == id))
    }

    // ==================== Photometry Operations ====================

    /// Adds a new photometry definition.
    ///
    /// # Errors
    /// Returns an error if a photometry with the same ID already exists.
    pub fn add_photometry(&mut self, photometry: Photometry) -> Result<()> {
        // Ensure photometries container exists
        if self.general_definitions.photometries.is_none() {
            self.general_definitions.photometries = Some(Photometries::default());
        }

        let photometries = self.general_definitions.photometries.as_mut().unwrap();

        // Check for duplicate ID
        if photometries.photometry.iter().any(|p| p.id == photometry.id) {
            return Err(anyhow!("Photometry with ID '{}' already exists", photometry.id));
        }

        photometries.photometry.push(photometry);
        Ok(())
    }

    /// Updates an existing photometry definition.
    ///
    /// # Errors
    /// Returns an error if no photometry with the given ID exists.
    pub fn update_photometry(&mut self, id: &str, photometry: Photometry) -> Result<()> {
        let photometries = self
            .general_definitions
            .photometries
            .as_mut()
            .ok_or_else(|| anyhow!("No photometries defined"))?;

        let pos = photometries
            .photometry
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow!("Photometry with ID '{}' not found", id))?;

        photometries.photometry[pos] = photometry;
        Ok(())
    }

    /// Removes a photometry definition by ID.
    ///
    /// # Errors
    /// Returns an error if no photometry with the given ID exists.
    pub fn remove_photometry(&mut self, id: &str) -> Result<Photometry> {
        let photometries = self
            .general_definitions
            .photometries
            .as_mut()
            .ok_or_else(|| anyhow!("No photometries defined"))?;

        let pos = photometries
            .photometry
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow!("Photometry with ID '{}' not found", id))?;

        Ok(photometries.photometry.remove(pos))
    }

    /// Gets a photometry by ID.
    pub fn get_photometry(&self, id: &str) -> Option<&Photometry> {
        self.general_definitions
            .photometries
            .as_ref()
            .and_then(|p| p.photometry.iter().find(|phot| phot.id == id))
    }

    // ==================== Geometry Operations ====================

    /// Adds a new simple geometry.
    ///
    /// # Errors
    /// Returns an error if a geometry with the same ID already exists.
    pub fn add_simple_geometry(&mut self, geometry: SimpleGeometry) -> Result<()> {
        // Ensure geometries container exists
        if self.general_definitions.geometries.is_none() {
            self.general_definitions.geometries = Some(Geometries::default());
        }

        let geometries = self.general_definitions.geometries.as_mut().unwrap();

        // Check for duplicate ID across both simple and model geometries
        if geometries.simple_geometry.iter().any(|g| g.id == geometry.id)
            || geometries.model_geometry.iter().any(|g| g.id == geometry.id)
        {
            return Err(anyhow!("Geometry with ID '{}' already exists", geometry.id));
        }

        geometries.simple_geometry.push(geometry);
        Ok(())
    }

    /// Adds a new model geometry.
    ///
    /// # Errors
    /// Returns an error if a geometry with the same ID already exists.
    pub fn add_model_geometry(&mut self, geometry: ModelGeometry) -> Result<()> {
        // Ensure geometries container exists
        if self.general_definitions.geometries.is_none() {
            self.general_definitions.geometries = Some(Geometries::default());
        }

        let geometries = self.general_definitions.geometries.as_mut().unwrap();

        // Check for duplicate ID across both simple and model geometries
        if geometries.simple_geometry.iter().any(|g| g.id == geometry.id)
            || geometries.model_geometry.iter().any(|g| g.id == geometry.id)
        {
            return Err(anyhow!("Geometry with ID '{}' already exists", geometry.id));
        }

        geometries.model_geometry.push(geometry);
        Ok(())
    }

    /// Removes a simple geometry by ID.
    ///
    /// # Errors
    /// Returns an error if no simple geometry with the given ID exists.
    pub fn remove_simple_geometry(&mut self, id: &str) -> Result<SimpleGeometry> {
        let geometries = self
            .general_definitions
            .geometries
            .as_mut()
            .ok_or_else(|| anyhow!("No geometries defined"))?;

        let pos = geometries
            .simple_geometry
            .iter()
            .position(|g| g.id == id)
            .ok_or_else(|| anyhow!("Simple geometry with ID '{}' not found", id))?;

        Ok(geometries.simple_geometry.remove(pos))
    }

    /// Removes a model geometry by ID.
    ///
    /// # Errors
    /// Returns an error if no model geometry with the given ID exists.
    pub fn remove_model_geometry(&mut self, id: &str) -> Result<ModelGeometry> {
        let geometries = self
            .general_definitions
            .geometries
            .as_mut()
            .ok_or_else(|| anyhow!("No geometries defined"))?;

        let pos = geometries
            .model_geometry
            .iter()
            .position(|g| g.id == id)
            .ok_or_else(|| anyhow!("Model geometry with ID '{}' not found", id))?;

        Ok(geometries.model_geometry.remove(pos))
    }

    /// Gets a simple geometry by ID.
    pub fn get_simple_geometry(&self, id: &str) -> Option<&SimpleGeometry> {
        self.general_definitions
            .geometries
            .as_ref()
            .and_then(|g| g.simple_geometry.iter().find(|geom| geom.id == id))
    }

    /// Gets a model geometry by ID.
    pub fn get_model_geometry(&self, id: &str) -> Option<&ModelGeometry> {
        self.general_definitions
            .geometries
            .as_ref()
            .and_then(|g| g.model_geometry.iter().find(|geom| geom.id == id))
    }

    // ==================== Light Source Operations ====================

    /// Adds a new fixed light source.
    ///
    /// # Errors
    /// Returns an error if a light source with the same ID already exists.
    pub fn add_fixed_light_source(&mut self, source: FixedLightSource) -> Result<()> {
        // Ensure light sources container exists
        if self.general_definitions.light_sources.is_none() {
            self.general_definitions.light_sources = Some(LightSources::default());
        }

        let light_sources = self.general_definitions.light_sources.as_mut().unwrap();

        // Check for duplicate ID
        if light_sources.fixed_light_source.iter().any(|s| s.id == source.id)
            || light_sources.changeable_light_source.iter().any(|s| s.id == source.id)
        {
            return Err(anyhow!("Light source with ID '{}' already exists", source.id));
        }

        light_sources.fixed_light_source.push(source);
        Ok(())
    }

    /// Adds a new changeable light source.
    ///
    /// # Errors
    /// Returns an error if a light source with the same ID already exists.
    pub fn add_changeable_light_source(&mut self, source: ChangeableLightSource) -> Result<()> {
        // Ensure light sources container exists
        if self.general_definitions.light_sources.is_none() {
            self.general_definitions.light_sources = Some(LightSources::default());
        }

        let light_sources = self.general_definitions.light_sources.as_mut().unwrap();

        // Check for duplicate ID
        if light_sources.fixed_light_source.iter().any(|s| s.id == source.id)
            || light_sources.changeable_light_source.iter().any(|s| s.id == source.id)
        {
            return Err(anyhow!("Light source with ID '{}' already exists", source.id));
        }

        light_sources.changeable_light_source.push(source);
        Ok(())
    }

    /// Removes a fixed light source by ID.
    ///
    /// # Errors
    /// Returns an error if no fixed light source with the given ID exists.
    pub fn remove_fixed_light_source(&mut self, id: &str) -> Result<FixedLightSource> {
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;

        let pos = light_sources
            .fixed_light_source
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| anyhow!("Fixed light source with ID '{}' not found", id))?;

        Ok(light_sources.fixed_light_source.remove(pos))
    }

    /// Removes a changeable light source by ID.
    ///
    /// # Errors
    /// Returns an error if no changeable light source with the given ID exists.
    pub fn remove_changeable_light_source(&mut self, id: &str) -> Result<ChangeableLightSource> {
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;

        let pos = light_sources
            .changeable_light_source
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| anyhow!("Changeable light source with ID '{}' not found", id))?;

        Ok(light_sources.changeable_light_source.remove(pos))
    }

    /// Gets a fixed light source by ID.
    pub fn get_fixed_light_source(&self, id: &str) -> Option<&FixedLightSource> {
        self.general_definitions
            .light_sources
            .as_ref()
            .and_then(|ls| ls.fixed_light_source.iter().find(|s| s.id == id))
    }

    /// Gets a changeable light source by ID.
    pub fn get_changeable_light_source(&self, id: &str) -> Option<&ChangeableLightSource> {
        self.general_definitions
            .light_sources
            .as_ref()
            .and_then(|ls| ls.changeable_light_source.iter().find(|s| s.id == id))
    }

    // ==================== Emitter Operations ====================

    /// Adds a new emitter.
    ///
    /// # Errors
    /// Returns an error if an emitter with the same ID already exists.
    pub fn add_emitter(&mut self, emitter: Emitter) -> Result<()> {
        // Ensure emitters container exists
        if self.general_definitions.emitters.is_none() {
            self.general_definitions.emitters = Some(Emitters::default());
        }

        let emitters = self.general_definitions.emitters.as_mut().unwrap();

        // Check for duplicate ID
        if emitters.emitter.iter().any(|e| e.id == emitter.id) {
            return Err(anyhow!("Emitter with ID '{}' already exists", emitter.id));
        }

        emitters.emitter.push(emitter);
        Ok(())
    }

    /// Removes an emitter by ID.
    ///
    /// # Errors
    /// Returns an error if no emitter with the given ID exists.
    pub fn remove_emitter(&mut self, id: &str) -> Result<Emitter> {
        let emitters = self
            .general_definitions
            .emitters
            .as_mut()
            .ok_or_else(|| anyhow!("No emitters defined"))?;

        let pos = emitters
            .emitter
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| anyhow!("Emitter with ID '{}' not found", id))?;

        Ok(emitters.emitter.remove(pos))
    }

    /// Gets an emitter by ID.
    pub fn get_emitter(&self, id: &str) -> Option<&Emitter> {
        self.general_definitions
            .emitters
            .as_ref()
            .and_then(|e| e.emitter.iter().find(|em| em.id == id))
    }

    /// Gets a mutable reference to an emitter by ID.
    pub fn get_emitter_mut(&mut self, id: &str) -> Option<&mut Emitter> {
        self.general_definitions
            .emitters
            .as_mut()
            .and_then(|e| e.emitter.iter_mut().find(|em| em.id == id))
    }

    // ==================== Product Metadata Operations ====================

    /// Sets the product metadata.
    pub fn set_product_metadata(&mut self, meta: ProductMetaData) {
        self.product_definitions.product_meta_data = Some(meta);
    }

    /// Gets the product metadata.
    pub fn get_product_metadata(&self) -> Option<&ProductMetaData> {
        self.product_definitions.product_meta_data.as_ref()
    }

    /// Gets a mutable reference to the product metadata.
    pub fn get_product_metadata_mut(&mut self) -> Option<&mut ProductMetaData> {
        self.product_definitions.product_meta_data.as_mut()
    }

    /// Ensures product metadata exists, creating default if needed.
    pub fn ensure_product_metadata(&mut self) -> &mut ProductMetaData {
        if self.product_definitions.product_meta_data.is_none() {
            self.product_definitions.product_meta_data = Some(ProductMetaData::default());
        }
        self.product_definitions.product_meta_data.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_id() {
        let product = GldfProduct::default();
        let id1 = product.generate_unique_id("file");
        assert_eq!(id1, "file_1");

        let mut product2 = GldfProduct::default();
        product2.add_file(File {
            id: "file_1".to_string(),
            content_type: "ldc/eulumdat".to_string(),
            type_attr: "localFileName".to_string(),
            file_name: "test.ldt".to_string(),
            language: String::new(),
        }).unwrap();

        let id2 = product2.generate_unique_id("file");
        assert_eq!(id2, "file_2");
    }

    #[test]
    fn test_file_operations() {
        let mut product = GldfProduct::default();

        // Add file
        let file = File {
            id: "test_file".to_string(),
            content_type: "ldc/eulumdat".to_string(),
            type_attr: "localFileName".to_string(),
            file_name: "test.ldt".to_string(),
            language: String::new(),
        };
        assert!(product.add_file(file.clone()).is_ok());

        // Duplicate add should fail
        assert!(product.add_file(file.clone()).is_err());

        // Get file
        assert!(product.get_file("test_file").is_some());
        assert!(product.get_file("nonexistent").is_none());

        // Update file
        let mut updated_file = file.clone();
        updated_file.file_name = "updated.ldt".to_string();
        assert!(product.update_file("test_file", updated_file).is_ok());
        assert_eq!(product.get_file("test_file").unwrap().file_name, "updated.ldt");

        // Remove file
        assert!(product.remove_file("test_file").is_ok());
        assert!(product.get_file("test_file").is_none());
    }

    #[test]
    fn test_variant_operations() {
        let mut product = GldfProduct::default();

        // Add variant
        let variant = Variant {
            id: "variant_1".to_string(),
            ..Default::default()
        };
        assert!(product.add_variant(variant.clone()).is_ok());

        // Duplicate add should fail
        assert!(product.add_variant(variant.clone()).is_err());

        // Get variant
        assert!(product.get_variant("variant_1").is_some());

        // Remove variant
        assert!(product.remove_variant("variant_1").is_ok());
        assert!(product.get_variant("variant_1").is_none());
    }

    #[test]
    fn test_light_source_operations() {
        let mut product = GldfProduct::default();

        // Add fixed light source
        let source = FixedLightSource {
            id: "source_1".to_string(),
            ..Default::default()
        };
        assert!(product.add_fixed_light_source(source.clone()).is_ok());

        // Duplicate should fail
        assert!(product.add_fixed_light_source(source).is_err());

        // Get source
        assert!(product.get_fixed_light_source("source_1").is_some());

        // Remove source
        assert!(product.remove_fixed_light_source("source_1").is_ok());
        assert!(product.get_fixed_light_source("source_1").is_none());
    }
}
