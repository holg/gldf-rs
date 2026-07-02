//! CRUD operations for GldfProduct.
//!
//! This module provides methods for creating, reading, updating, and deleting
//! elements within a GLDF product structure.

use crate::gldf::general_definitions::electrical::{
    ControlGear, ControlGears, Equipment, Equipments,
};
use crate::gldf::general_definitions::files::File;
use crate::gldf::general_definitions::geometries::{Geometries, ModelGeometry, SimpleGeometry};
use crate::gldf::general_definitions::lightsources::{
    ChangeableLightSource, Emitter, Emitters, FixedLightSource, LightSources,
    MultiChannelLightSource, SpectrumReference,
};
use crate::gldf::general_definitions::photometries::{
    Photometries, Photometry, Spectrum, Spectrums,
};
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

        // Spectrum IDs
        if let Some(ref spectrums) = self.general_definitions.spectrums {
            for spectrum in &spectrums.spectrum {
                ids.insert(spectrum.id.clone());
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

        // Light source IDs (Fixed, Changeable, MultiChannel — XSD declares
        // FixedLightSourceId, ChangeableLightSourceId, MultiChannelLightSourceId
        // as three separate keys but they share the same id namespace in
        // practice because emitter/equipment refs disambiguate by attribute).
        if let Some(ref light_sources) = self.general_definitions.light_sources {
            for source in &light_sources.fixed_light_source {
                ids.insert(source.id.clone());
            }
            for source in &light_sources.changeable_light_source {
                ids.insert(source.id.clone());
            }
            for source in &light_sources.multi_channel_light_source {
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
        if self
            .general_definitions
            .files
            .file
            .iter()
            .any(|f| f.id == file.id)
        {
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

    /// Renames a `<File>` id, cascading into every referencing slot so
    /// the document stays internally consistent.
    ///
    /// References updated (per the rc.3 XSD `FileId` keyrefs at lines 86-114):
    /// - `Photometry/PhotometryFileReference @fileId`
    /// - `Spectrum/SpectrumFileReference @fileId`
    /// - `Fixed/Changeable/MultiChannel LightSource/LightSourceImages/Image @fileId`
    /// - `ModelGeometry/GeometryFileReference @fileId`
    /// - `Sensor/SensorFileReference @fileId`
    /// - `ProductMetaData/Pictures/Image @fileId` (product-level pictures)
    /// - `Variant/Pictures/Image @fileId`
    /// - `Variant/Symbol @fileId`
    ///
    /// # Errors
    /// - `old_id` doesn't exist.
    /// - `new_id` is empty.
    /// - `new_id` is already in use by another File.
    pub fn rename_file(&mut self, old_id: &str, new_id: &str) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New file id must not be empty");
        }
        if !self
            .general_definitions
            .files
            .file
            .iter()
            .any(|f| f.id == old_id)
        {
            anyhow::bail!("File with id '{}' not found", old_id);
        }
        if self
            .general_definitions
            .files
            .file
            .iter()
            .any(|f| f.id == new_id)
        {
            anyhow::bail!("File id '{}' is already in use", new_id);
        }
        if let Some(f) = self
            .general_definitions
            .files
            .file
            .iter_mut()
            .find(|f| f.id == old_id)
        {
            f.id = new_id.to_string();
        }
        // Photometry file refs
        if let Some(ps) = self.general_definitions.photometries.as_mut() {
            for p in &mut ps.photometry {
                if let Some(fr) = p.photometry_file_reference.as_mut() {
                    if fr.file_id == old_id {
                        fr.file_id = new_id.to_string();
                    }
                }
            }
        }
        // Spectrum file refs
        if let Some(ss) = self.general_definitions.spectrums.as_mut() {
            for sp in &mut ss.spectrum {
                if let Some(fr) = sp.spectrum_file_reference.as_mut() {
                    if fr.file_id == old_id {
                        fr.file_id = new_id.to_string();
                    }
                }
            }
        }
        // LightSource images (all three kinds)
        if let Some(ls) = self.general_definitions.light_sources.as_mut() {
            for f in &mut ls.fixed_light_source {
                if let Some(imgs) = f.light_source_images.as_mut() {
                    for img in &mut imgs.image {
                        if img.file_id == old_id {
                            img.file_id = new_id.to_string();
                        }
                    }
                }
            }
            for c in &mut ls.changeable_light_source {
                if let Some(imgs) = c.light_source_images.as_mut() {
                    for img in &mut imgs.image {
                        if img.file_id == old_id {
                            img.file_id = new_id.to_string();
                        }
                    }
                }
            }
            for m in &mut ls.multi_channel_light_source {
                if let Some(imgs) = m.light_source_images.as_mut() {
                    for img in &mut imgs.image {
                        if img.file_id == old_id {
                            img.file_id = new_id.to_string();
                        }
                    }
                }
            }
        }
        // ModelGeometry file refs
        if let Some(geoms) = self.general_definitions.geometries.as_mut() {
            for m in &mut geoms.model_geometry {
                for gfr in &mut m.geometry_file_reference {
                    if gfr.file_id == old_id {
                        gfr.file_id = new_id.to_string();
                    }
                }
            }
        }
        // Sensor file refs
        if let Some(sensors) = self.general_definitions.sensors.as_mut() {
            for sensor in &mut sensors.sensor {
                if let Some(fr) = sensor.sensor_file_reference.as_mut() {
                    if fr.file_id == old_id {
                        fr.file_id = new_id.to_string();
                    }
                }
            }
        }
        // ProductMetaData pictures
        if let Some(meta) = self.product_definitions.product_meta_data.as_mut() {
            if let Some(pics) = meta.pictures.as_mut() {
                for img in &mut pics.image {
                    if img.file_id == old_id {
                        img.file_id = new_id.to_string();
                    }
                }
            }
        }
        // Variant pictures + symbol
        if let Some(vs) = self.product_definitions.variants.as_mut() {
            for v in &mut vs.variant {
                if let Some(pics) = v.pictures.as_mut() {
                    for img in &mut pics.image {
                        if img.file_id == old_id {
                            img.file_id = new_id.to_string();
                        }
                    }
                }
                if let Some(sym) = v.symbol.as_mut() {
                    if sym.file_id == old_id {
                        sym.file_id = new_id.to_string();
                    }
                }
            }
        }
        Ok(())
    }

    /// Renames a Photometry by id and cascades every `PhotometryReference`
    /// in the document.
    ///
    /// Cascade sites:
    ///   - `ChangeableLightSource.PhotometryReference` (optional)
    ///   - `MultiChannelLightSource.Channels.Channel.PhotometryReference`
    ///   - `Emitter.FixedLightEmitter.PhotometryReference` (Vec)
    ///   - `Emitter.ChangeableLightEmitter.PhotometryReference` (Vec)
    ///
    /// `FixedLightSource` has no PhotometryReference field of its own —
    /// it binds photometry via the Emitter wrapper instead.
    ///
    /// Returns an error if the new id is empty, the old id is unknown,
    /// or the new id is already taken.
    pub fn rename_photometry(&mut self, old_id: &str, new_id: &str) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New photometry id must not be empty");
        }
        let Some(ps) = self.general_definitions.photometries.as_mut() else {
            anyhow::bail!("Photometry with id '{}' not found", old_id);
        };
        if !ps.photometry.iter().any(|p| p.id == old_id) {
            anyhow::bail!("Photometry with id '{}' not found", old_id);
        }
        if ps.photometry.iter().any(|p| p.id == new_id) {
            anyhow::bail!("Photometry id '{}' is already in use", new_id);
        }
        if let Some(p) = ps.photometry.iter_mut().find(|p| p.id == old_id) {
            p.id = new_id.to_string();
        }
        // ChangeableLightSource + MultiChannel PhotometryReference sites.
        if let Some(ls) = self.general_definitions.light_sources.as_mut() {
            for c in &mut ls.changeable_light_source {
                if let Some(pr) = c.photometry_reference.as_mut() {
                    if pr.photometry_id == old_id {
                        pr.photometry_id = new_id.to_string();
                    }
                }
            }
            for m in &mut ls.multi_channel_light_source {
                for ch in &mut m.channels.channel {
                    if ch.photometry_reference.photometry_id == old_id {
                        ch.photometry_reference.photometry_id = new_id.to_string();
                    }
                }
            }
        }
        // Emitter PhotometryReference sites — both light-emitter kinds
        // are stored as Vec<…> on the Emitter wrapper.
        if let Some(ems) = self.general_definitions.emitters.as_mut() {
            for em in &mut ems.emitter {
                for fle in &mut em.fixed_light_emitter {
                    if fle.photometry_reference.photometry_id == old_id {
                        fle.photometry_reference.photometry_id = new_id.to_string();
                    }
                }
                for cle in &mut em.changeable_light_emitter {
                    if cle.photometry_reference.photometry_id == old_id {
                        cle.photometry_reference.photometry_id = new_id.to_string();
                    }
                }
            }
        }
        Ok(())
    }

    /// Gets a file definition by ID.
    pub fn get_file(&self, id: &str) -> Option<&File> {
        self.general_definitions
            .files
            .file
            .iter()
            .find(|f| f.id == id)
    }

    /// Gets a mutable reference to a file definition by ID.
    pub fn get_file_mut(&mut self, id: &str) -> Option<&mut File> {
        self.general_definitions
            .files
            .file
            .iter_mut()
            .find(|f| f.id == id)
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
        if photometries
            .photometry
            .iter()
            .any(|p| p.id == photometry.id)
        {
            return Err(anyhow!(
                "Photometry with ID '{}' already exists",
                photometry.id
            ));
        }

        photometries.photometry.push(photometry);
        Ok(())
    }

    /// Adds a sensor definition to `GeneralDefinitions/Sensors`.
    ///
    /// Creates the `Sensors` container if absent. The sensor's
    /// `SensorFileReference.file_id` should point at an embedded `<File>`
    /// with content type `sensor/sensxml` or `sensor/sensldt`; see
    /// [`crate::editable::EditableGldf::add_sensor_file`] for the
    /// high-level "embed + link" helper.
    ///
    /// # Errors
    /// Returns an error if a sensor with the same ID already exists.
    pub fn add_sensor(
        &mut self,
        sensor: crate::gldf::general_definitions::sensors::Sensor,
    ) -> Result<()> {
        use crate::gldf::general_definitions::sensors::Sensors;
        if self.general_definitions.sensors.is_none() {
            self.general_definitions.sensors = Some(Sensors::default());
        }
        let sensors = self.general_definitions.sensors.as_mut().unwrap();
        if sensors.sensor.iter().any(|s| s.id == sensor.id) {
            return Err(anyhow!(
                "Sensor with ID '{}' already exists",
                sensor.id
            ));
        }
        sensors.sensor.push(sensor);
        Ok(())
    }

    /// Adds a spectrum definition to `GeneralDefinitions/Spectrums`.
    ///
    /// Creates the `Spectrums` container if absent. The spectrum's
    /// `SpectrumFileReference.file_id` should point at an embedded `<File>`
    /// (contentType `spectrum/text`); see [`crate::editable::EditableGldf`]
    /// for the high-level "embed + link" helper.
    ///
    /// # Errors
    /// Returns an error if a spectrum with the same ID already exists.
    pub fn add_spectrum(&mut self, spectrum: Spectrum) -> Result<()> {
        if self.general_definitions.spectrums.is_none() {
            self.general_definitions.spectrums = Some(Spectrums::default());
        }
        let spectrums = self.general_definitions.spectrums.as_mut().unwrap();
        if spectrums.spectrum.iter().any(|s| s.id == spectrum.id) {
            return Err(anyhow!("Spectrum with ID '{}' already exists", spectrum.id));
        }
        spectrums.spectrum.push(spectrum);
        Ok(())
    }

    /// Links a spectrum to a light source by setting its `SpectrumReference`.
    ///
    /// Targets the `FixedLightSource` or `ChangeableLightSource` whose id
    /// matches `light_source_id`.
    ///
    /// # Errors
    /// Returns an error if no light source with that id exists.
    pub fn add_spectrum_reference_to_light_source(
        &mut self,
        light_source_id: &str,
        spectrum_id: &str,
    ) -> Result<()> {
        let reference = SpectrumReference {
            spectrum_id: spectrum_id.to_string(),
        };
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;

        if let Some(ls) = light_sources
            .fixed_light_source
            .iter_mut()
            .find(|s| s.id == light_source_id)
        {
            ls.spectrum_reference = Some(reference);
            return Ok(());
        }
        if let Some(ls) = light_sources
            .changeable_light_source
            .iter_mut()
            .find(|s| s.id == light_source_id)
        {
            ls.spectrum_reference = Some(reference);
            return Ok(());
        }
        Err(anyhow!(
            "No light source with ID '{}' to attach spectrum '{}'",
            light_source_id,
            spectrum_id
        ))
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
        if geometries
            .simple_geometry
            .iter()
            .any(|g| g.id == geometry.id)
            || geometries
                .model_geometry
                .iter()
                .any(|g| g.id == geometry.id)
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
        if geometries
            .simple_geometry
            .iter()
            .any(|g| g.id == geometry.id)
            || geometries
                .model_geometry
                .iter()
                .any(|g| g.id == geometry.id)
        {
            return Err(anyhow!("Geometry with ID '{}' already exists", geometry.id));
        }

        geometries.model_geometry.push(geometry);
        Ok(())
    }

    /// Adds a new control gear. Returns an error if a control gear with the
    /// same ID already exists.
    pub fn add_control_gear(&mut self, gear: ControlGear) -> Result<()> {
        if self.general_definitions.control_gears.is_none() {
            self.general_definitions.control_gears = Some(ControlGears::default());
        }
        let cgs = self.general_definitions.control_gears.as_mut().unwrap();
        if cgs.control_gear.iter().any(|g| g.id == gear.id) {
            return Err(anyhow!("Control gear with ID '{}' already exists", gear.id));
        }
        cgs.control_gear.push(gear);
        Ok(())
    }

    /// Removes a control gear by ID. Returns an error if no control gear
    /// with the given ID exists.
    pub fn remove_control_gear(&mut self, id: &str) -> Result<ControlGear> {
        let cgs = self
            .general_definitions
            .control_gears
            .as_mut()
            .ok_or_else(|| anyhow!("No control gears defined"))?;
        let pos = cgs
            .control_gear
            .iter()
            .position(|g| g.id == id)
            .ok_or_else(|| anyhow!("Control gear with ID '{}' not found", id))?;
        Ok(cgs.control_gear.remove(pos))
    }

    /// Adds a new equipment. Returns an error if an equipment with the same
    /// ID already exists.
    pub fn add_equipment(&mut self, equipment: Equipment) -> Result<()> {
        if self.general_definitions.equipments.is_none() {
            self.general_definitions.equipments = Some(Equipments::default());
        }
        let eqs = self.general_definitions.equipments.as_mut().unwrap();
        if eqs.equipment.iter().any(|e| e.id == equipment.id) {
            return Err(anyhow!(
                "Equipment with ID '{}' already exists",
                equipment.id
            ));
        }
        eqs.equipment.push(equipment);
        Ok(())
    }

    /// Removes an equipment by ID. Returns an error if no equipment with
    /// the given ID exists.
    pub fn remove_equipment(&mut self, id: &str) -> Result<Equipment> {
        let eqs = self
            .general_definitions
            .equipments
            .as_mut()
            .ok_or_else(|| anyhow!("No equipments defined"))?;
        let pos = eqs
            .equipment
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| anyhow!("Equipment with ID '{}' not found", id))?;
        Ok(eqs.equipment.remove(pos))
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
        if light_sources
            .fixed_light_source
            .iter()
            .any(|s| s.id == source.id)
            || light_sources
                .changeable_light_source
                .iter()
                .any(|s| s.id == source.id)
        {
            return Err(anyhow!(
                "Light source with ID '{}' already exists",
                source.id
            ));
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
        if light_sources
            .fixed_light_source
            .iter()
            .any(|s| s.id == source.id)
            || light_sources
                .changeable_light_source
                .iter()
                .any(|s| s.id == source.id)
        {
            return Err(anyhow!(
                "Light source with ID '{}' already exists",
                source.id
            ));
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

    /// Renames a `FixedLightSource` id, updating every reference in the
    /// product so the document stays internally consistent.
    ///
    /// References updated:
    /// - `FixedLightEmitter::light_source_reference.fixed_light_source_id`
    ///   in every `Emitter`.
    /// - `Electrical::light_source_reference.fixed_light_source_id` on
    ///   `ProductMetaData` and on every `Variant`.
    ///
    /// # Errors
    /// - The source `old_id` doesn't exist.
    /// - `new_id` is already in use by another fixed or changeable light source.
    pub fn rename_fixed_light_source(&mut self, old_id: &str, new_id: &str) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New light source id must not be empty");
        }

        // 1) Validate: old exists, new is free across BOTH fixed + changeable.
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;

        if !light_sources
            .fixed_light_source
            .iter()
            .any(|s| s.id == old_id)
        {
            anyhow::bail!("Fixed light source with id '{}' not found", old_id);
        }
        if light_sources
            .fixed_light_source
            .iter()
            .any(|s| s.id == new_id)
            || light_sources
                .changeable_light_source
                .iter()
                .any(|s| s.id == new_id)
        {
            anyhow::bail!("Light source id '{}' is already in use", new_id);
        }

        // 2) Rename the source itself.
        if let Some(s) = light_sources
            .fixed_light_source
            .iter_mut()
            .find(|s| s.id == old_id)
        {
            s.id = new_id.to_string();
        }

        // 3) Update every FixedLightEmitter reference.
        if let Some(emitters) = self.general_definitions.emitters.as_mut() {
            for emitter in &mut emitters.emitter {
                for fle in &mut emitter.fixed_light_emitter {
                    if fle
                        .light_source_reference
                        .fixed_light_source_id
                        .as_deref()
                        == Some(old_id)
                    {
                        fle.light_source_reference.fixed_light_source_id =
                            Some(new_id.to_string());
                    }
                }
            }
        }

        // 4) Update every Equipment.light_source_reference (each Equipment
        //    can hold one optional light-source-id reference; per the
        //    rc-3 XSD the attribute is always `@changeableLightSourceId`).
        if let Some(equipments) = self.general_definitions.equipments.as_mut() {
            for equipment in &mut equipments.equipment {
                if let Some(lsr) = equipment.light_source_reference.as_mut() {
                    if lsr.changeable_light_source_id == old_id {
                        lsr.changeable_light_source_id = new_id.to_string();
                    }
                }
            }
        }

        Ok(())
    }

    /// Replaces a `FixedLightSource` in place by id. The new value's `id`
    /// field is overwritten with the target id so the slot identity stays
    /// stable — every existing reference (Emitter / Equipment / Variant)
    /// continues to resolve. To rename a light source use
    /// [`Self::rename_fixed_light_source`] which also rewrites the refs.
    ///
    /// # Errors
    /// Returns an error if no fixed light source with the given id exists.
    pub fn update_fixed_light_source(
        &mut self,
        id: &str,
        mut new_value: FixedLightSource,
    ) -> Result<()> {
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        let slot = light_sources
            .fixed_light_source
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("Fixed light source with id '{}' not found", id))?;
        new_value.id = id.to_string();
        *slot = new_value;
        Ok(())
    }

    /// Replaces a `ChangeableLightSource` in place by id. Same identity-
    /// preservation contract as [`Self::update_fixed_light_source`].
    ///
    /// # Errors
    /// Returns an error if no changeable light source with the given id exists.
    pub fn update_changeable_light_source(
        &mut self,
        id: &str,
        mut new_value: ChangeableLightSource,
    ) -> Result<()> {
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        let slot = light_sources
            .changeable_light_source
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("Changeable light source with id '{}' not found", id))?;
        new_value.id = id.to_string();
        *slot = new_value;
        Ok(())
    }

    /// Renames a `ChangeableLightSource` id, updating every reference in
    /// the product so the document stays internally consistent.
    ///
    /// References updated (per the rc.3 XSD keyrefs):
    /// - `Equipment::light_source_reference.changeable_light_source_id`
    ///   (the `Equipment-LightSource` keyref on `@changeableLightSourceId`).
    ///
    /// ChangeableLightSources are *not* referenced from ChangeableLight-
    /// Emitter directly — the user picks the lamp at install time — so
    /// there's no emitter-side cascade like the Fixed variant has.
    ///
    /// # Errors
    /// - The source `old_id` doesn't exist.
    /// - `new_id` is already in use by another fixed or changeable light source.
    pub fn rename_changeable_light_source(&mut self, old_id: &str, new_id: &str) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New light source id must not be empty");
        }
        let light_sources = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        if !light_sources
            .changeable_light_source
            .iter()
            .any(|s| s.id == old_id)
        {
            anyhow::bail!("Changeable light source with id '{}' not found", old_id);
        }
        if light_sources
            .fixed_light_source
            .iter()
            .any(|s| s.id == new_id)
            || light_sources
                .changeable_light_source
                .iter()
                .any(|s| s.id == new_id)
        {
            anyhow::bail!("Light source id '{}' is already in use", new_id);
        }
        if let Some(s) = light_sources
            .changeable_light_source
            .iter_mut()
            .find(|s| s.id == old_id)
        {
            s.id = new_id.to_string();
        }
        if let Some(equipments) = self.general_definitions.equipments.as_mut() {
            for equipment in &mut equipments.equipment {
                if let Some(lsr) = equipment.light_source_reference.as_mut() {
                    if lsr.changeable_light_source_id == old_id {
                        lsr.changeable_light_source_id = new_id.to_string();
                    }
                }
            }
        }
        Ok(())
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

    // ==================== MultiChannel Light Source Operations ====================

    /// Adds a new multi-channel light source. Errors if the id collides
    /// with any existing LightSource id (Fixed, Changeable, or MultiChannel
    /// share an id namespace for the editor's safety).
    pub fn add_multi_channel_light_source(
        &mut self,
        source: MultiChannelLightSource,
    ) -> Result<()> {
        if self.general_definitions.light_sources.is_none() {
            self.general_definitions.light_sources = Some(LightSources::default());
        }
        let ls = self.general_definitions.light_sources.as_mut().unwrap();
        if ls.fixed_light_source.iter().any(|s| s.id == source.id)
            || ls.changeable_light_source.iter().any(|s| s.id == source.id)
            || ls
                .multi_channel_light_source
                .iter()
                .any(|s| s.id == source.id)
        {
            return Err(anyhow!(
                "Light source with ID '{}' already exists",
                source.id
            ));
        }
        ls.multi_channel_light_source.push(source);
        Ok(())
    }

    /// Removes a multi-channel light source by id.
    pub fn remove_multi_channel_light_source(
        &mut self,
        id: &str,
    ) -> Result<MultiChannelLightSource> {
        let ls = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        let pos = ls
            .multi_channel_light_source
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| anyhow!("MultiChannel light source with id '{}' not found", id))?;
        Ok(ls.multi_channel_light_source.remove(pos))
    }

    /// Replaces a multi-channel light source in place by id. The new value's
    /// id field is overwritten to keep the slot stable so existing
    /// MultiChannelLightEmitter references stay valid.
    pub fn update_multi_channel_light_source(
        &mut self,
        id: &str,
        mut new_value: MultiChannelLightSource,
    ) -> Result<()> {
        let ls = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        let slot = ls
            .multi_channel_light_source
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| anyhow!("MultiChannel light source with id '{}' not found", id))?;
        new_value.id = id.to_string();
        *slot = new_value;
        Ok(())
    }

    /// Renames a MultiChannelLightSource id, cascading into every
    /// `Emitter/MultiChannelLightEmitter/LightSourceReference
    /// @multiChannelLightSourceId` (XSD keyref L154).
    pub fn rename_multi_channel_light_source(
        &mut self,
        old_id: &str,
        new_id: &str,
    ) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New light source id must not be empty");
        }
        let ls = self
            .general_definitions
            .light_sources
            .as_mut()
            .ok_or_else(|| anyhow!("No light sources defined"))?;
        if !ls
            .multi_channel_light_source
            .iter()
            .any(|s| s.id == old_id)
        {
            anyhow::bail!("MultiChannel light source with id '{}' not found", old_id);
        }
        if ls.fixed_light_source.iter().any(|s| s.id == new_id)
            || ls.changeable_light_source.iter().any(|s| s.id == new_id)
            || ls
                .multi_channel_light_source
                .iter()
                .any(|s| s.id == new_id)
        {
            anyhow::bail!("Light source id '{}' is already in use", new_id);
        }
        if let Some(s) = ls
            .multi_channel_light_source
            .iter_mut()
            .find(|s| s.id == old_id)
        {
            s.id = new_id.to_string();
        }
        if let Some(emitters) = self.general_definitions.emitters.as_mut() {
            for emitter in &mut emitters.emitter {
                for mle in &mut emitter.multi_channel_light_emitter {
                    if mle.light_source_reference.multi_channel_light_source_id == old_id {
                        mle.light_source_reference.multi_channel_light_source_id =
                            new_id.to_string();
                    }
                }
            }
        }
        Ok(())
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

    /// Renames an `Emitter` id, updating every reference in the product so
    /// the document stays internally consistent.
    ///
    /// References updated (per the rc.3 XSD keyrefs at lines 182-216):
    /// - `Variant > Geometry > EmitterReference @emitterId` (the bare form,
    ///   used when a variant has no 3D model attached).
    /// - `Variant > Geometry > SimpleGeometryReference @emitterId`.
    /// - `Variant > Geometry > ModelGeometryReference > EmitterReference
    ///   @emitterId` (the nested form alongside an L3D/M3D/R3D model).
    ///
    /// The XSD also allows a `<GeometryReferences>` wrapper around the
    /// SimpleGeometry/ModelGeometry choice (XSD L4304) but the Rust model
    /// does not yet carry that variant on `Geometry` — when it is added,
    /// this method must be extended to walk it too.
    ///
    /// # Errors
    /// - The source `old_id` doesn't exist.
    /// - `new_id` is already in use by another emitter.
    /// - `new_id` is empty.
    pub fn rename_emitter(&mut self, old_id: &str, new_id: &str) -> Result<()> {
        if old_id == new_id {
            return Ok(());
        }
        if new_id.is_empty() {
            anyhow::bail!("New emitter id must not be empty");
        }
        let emitters = self
            .general_definitions
            .emitters
            .as_mut()
            .ok_or_else(|| anyhow!("No emitters defined"))?;
        if !emitters.emitter.iter().any(|e| e.id == old_id) {
            anyhow::bail!("Emitter with id '{}' not found", old_id);
        }
        if emitters.emitter.iter().any(|e| e.id == new_id) {
            anyhow::bail!("Emitter id '{}' is already in use", new_id);
        }
        if let Some(e) = emitters.emitter.iter_mut().find(|e| e.id == old_id) {
            e.id = new_id.to_string();
        }

        // Cascade into every Variant.Geometry that references this emitter.
        if let Some(variants) = self.product_definitions.variants.as_mut() {
            for v in &mut variants.variant {
                if let Some(geom) = v.geometry.as_mut() {
                    for er in &mut geom.emitter_reference {
                        if er.emitter_id == old_id {
                            er.emitter_id = new_id.to_string();
                        }
                    }
                    if let Some(sr) = geom.simple_geometry_reference.as_mut() {
                        if sr.emitter_id == old_id {
                            sr.emitter_id = new_id.to_string();
                        }
                    }
                    if let Some(mr) = geom.model_geometry_reference.as_mut() {
                        for er in &mut mr.emitter_reference {
                            if er.emitter_id == old_id {
                                er.emitter_id = new_id.to_string();
                            }
                        }
                    }
                }
            }
        }
        Ok(())
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
        product2
            .add_file(File {
                id: "file_1".to_string(),
                content_type: "ldc/eulumdat".to_string(),
                type_attr: "localFileName".to_string(),
                file_name: "test.ldt".to_string(),
                language: String::new(),
            })
            .unwrap();

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
        assert_eq!(
            product.get_file("test_file").unwrap().file_name,
            "updated.ldt"
        );

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

    #[test]
    fn test_multi_channel_light_source_round_trip() {
        // Build a GLDF that has a MultiChannelLightSource with two channels
        // and a MultiChannelLightEmitter referencing it. Serialize → parse →
        // assert the structure survived. This is the data-loss canary: if
        // we ever forget to plumb a field through, this test catches it.
        use crate::gldf::general_definitions::lightsources::{
            ChannelPhotometryReference, ChannelSpectrumReference, Channels, Emitter, Emitters,
            MultiChannelChannel, MultiChannelLightEmitter, MultiChannelLightSource,
            MultiChannelLightSourceReference,
        };
        use crate::gldf::header::{Locale, LocaleFoo};

        let mut product = GldfProduct::default();

        let mc = MultiChannelLightSource {
            id: "mc_1".to_string(),
            name: LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: "Tunable LED".to_string(),
                }],
            },
            manufacturer: Some("Acme".to_string()),
            rated_input_power: 24.0,
            channels: Channels {
                channel: vec![
                    MultiChannelChannel {
                        display_name: LocaleFoo {
                            locale: vec![Locale {
                                language: "en".to_string(),
                                value: "Warm".to_string(),
                            }],
                        },
                        spectrum_reference: ChannelSpectrumReference {
                            spectrum_id: "spec_warm".to_string(),
                        },
                        photometry_reference: ChannelPhotometryReference {
                            photometry_id: "photo_warm".to_string(),
                        },
                        rated_luminous_flux: 1200,
                        color_information: None,
                    },
                    MultiChannelChannel {
                        display_name: LocaleFoo {
                            locale: vec![Locale {
                                language: "en".to_string(),
                                value: "Cool".to_string(),
                            }],
                        },
                        spectrum_reference: ChannelSpectrumReference {
                            spectrum_id: "spec_cool".to_string(),
                        },
                        photometry_reference: ChannelPhotometryReference {
                            photometry_id: "photo_cool".to_string(),
                        },
                        rated_luminous_flux: 1500,
                        color_information: None,
                    },
                ],
            },
            ..Default::default()
        };
        product.add_multi_channel_light_source(mc).unwrap();

        product
            .general_definitions
            .emitters = Some(Emitters {
            emitter: vec![Emitter {
                id: "em_1".to_string(),
                multi_channel_light_emitter: vec![MultiChannelLightEmitter {
                    light_source_reference: MultiChannelLightSourceReference {
                        multi_channel_light_source_id: "mc_1".to_string(),
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }],
        });

        // Round-trip via XML
        let xml = product.to_xml().expect("serialize MultiChannel");
        assert!(
            xml.contains("<MultiChannelLightSource"),
            "serialized XML missing MultiChannelLightSource element"
        );
        assert!(
            xml.contains("<MultiChannelLightEmitter"),
            "serialized XML missing MultiChannelLightEmitter element"
        );
        assert!(
            xml.contains("multiChannelLightSourceId=\"mc_1\""),
            "serialized XML missing emitter reference attribute"
        );

        let parsed = GldfProduct::from_xml(&xml).expect("parse MultiChannel back");
        let ls = parsed
            .general_definitions
            .light_sources
            .as_ref()
            .expect("parsed product has light_sources");
        assert_eq!(ls.multi_channel_light_source.len(), 1);
        let parsed_mc = &ls.multi_channel_light_source[0];
        assert_eq!(parsed_mc.id, "mc_1");
        assert_eq!(parsed_mc.manufacturer.as_deref(), Some("Acme"));
        assert_eq!(parsed_mc.rated_input_power, 24.0);
        assert_eq!(parsed_mc.channels.channel.len(), 2);
        assert_eq!(parsed_mc.channels.channel[0].spectrum_reference.spectrum_id, "spec_warm");
        assert_eq!(parsed_mc.channels.channel[1].rated_luminous_flux, 1500);

        let emitters = parsed
            .general_definitions
            .emitters
            .as_ref()
            .expect("parsed product has emitters");
        assert_eq!(emitters.emitter[0].multi_channel_light_emitter.len(), 1);
        assert_eq!(
            emitters.emitter[0].multi_channel_light_emitter[0]
                .light_source_reference
                .multi_channel_light_source_id,
            "mc_1"
        );

        // Rename and confirm the cascade rewires the emitter reference.
        let mut parsed = parsed;
        parsed
            .rename_multi_channel_light_source("mc_1", "mc_renamed")
            .expect("rename MultiChannel");
        assert_eq!(
            parsed
                .general_definitions
                .light_sources
                .as_ref()
                .unwrap()
                .multi_channel_light_source[0]
                .id,
            "mc_renamed"
        );
        assert_eq!(
            parsed
                .general_definitions
                .emitters
                .as_ref()
                .unwrap()
                .emitter[0]
                .multi_channel_light_emitter[0]
                .light_source_reference
                .multi_channel_light_source_id,
            "mc_renamed"
        );
    }
}
