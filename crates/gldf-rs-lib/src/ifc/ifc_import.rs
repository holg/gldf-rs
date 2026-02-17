//! IFC to GLDF Import
//!
//! Converts IFC luminaire data to GLDF format.
//! Extracts:
//! - IFCLIGHTFIXTURETYPE → GLDF Variants
//! - Property sets → GLDF attributes
//! - IFCLIGHTSOURCEGONIOMETRIC → Light sources
//! - IFCLIGHTINTENSITYDISTRIBUTION → EULUMDAT photometry
//! - IFCSHAPEREPRESENTATION → L3D geometry

use super::step_parser::{StepParser, StepValue};
use anyhow::{anyhow, Result};

/// Extracted luminaire data from IFC
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedLuminaire {
    /// Product name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Model/article number
    pub model_reference: Option<String>,
    /// Variants (one per IFCLIGHTFIXTURETYPE)
    pub variants: Vec<ImportedVariant>,
    /// Product-level descriptive attributes (IP code, safety class, etc.)
    pub descriptive_attributes: ProductDescriptiveAttributes,
    /// Embedded files (images, sensors, etc.) for roundtrip preservation
    pub embedded_files: Vec<EmbeddedFile>,
}

/// Product-level descriptive attributes (shared across all variants)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProductDescriptiveAttributes {
    /// Electrical safety class (I, II, III)
    pub electrical_safety_class: Option<String>,
    /// IP code (e.g., IP20, IP65)
    pub ip_code: Option<String>,
    /// Median useful life (e.g., "L80B50 70000h 25°C")
    pub median_useful_life: Option<String>,
}

/// Embedded file from GLDF (image, sensor, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedFile {
    /// File type ("image", "sensor", etc.)
    pub file_type: String,
    /// Original filename
    pub filename: String,
    /// Content type (MIME type, e.g., "image/png", "sensor/sensldt")
    pub content_type: String,
    /// File content as bytes
    pub content: Vec<u8>,
}

/// Extracted variant data
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedVariant {
    /// Variant name (from type name)
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// GLDF variant ID (if present from Pset_LuminaireVariant)
    pub gldf_variant_id: Option<String>,
    /// Light fixture type enum
    pub fixture_type: Option<String>,
    /// Light sources in this variant
    pub light_sources: Vec<ImportedLightSource>,
    /// Photometry data (if embedded)
    pub photometry: Option<ImportedPhotometry>,
    /// Geometry mesh data
    pub geometry: Option<ImportedGeometry>,
    /// Properties
    pub properties: ImportedProperties,
}

/// Extracted light source data
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedLightSource {
    /// Source name
    pub name: String,
    /// Correlated color temperature (K)
    pub color_temperature: Option<f64>,
    /// Luminous flux (lm)
    pub luminous_flux: Option<f64>,
    /// Emission source type (LED, FLUORESCENT, etc.)
    pub emission_source: Option<String>,
    /// Light intensity distribution reference
    pub distribution: Option<ImportedDistribution>,
    /// Original photometry filename (e.g., "narrow.ldt") - preserved through IFC roundtrip
    pub photometry_filename: Option<String>,
    /// Original LDT metadata preserved through IFC roundtrip
    pub ldt_metadata: Option<LdtMetadata>,
}

/// LDT file metadata for roundtrip preservation
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LdtMetadata {
    /// Symmetry type (0=none, 1=vertical axis, 2=C0-C180, 3=C90-C270, 4=both planes)
    pub symmetry: i32,
    /// Number of C-planes in original file
    pub num_c_planes: i32,
    /// Number of gamma angles in original file
    pub num_g_angles: i32,
    /// C-plane step (Dc)
    pub dc: f64,
    /// Gamma step (Dg)
    pub dg: f64,
    /// Total luminous flux from LDT header
    pub total_flux: Option<f64>,
    /// Direct ratios (DR) values
    pub dr: Option<[f64; 10]>,
    /// Luminaire name from LDT
    pub luminaire_name: Option<String>,
    /// Manufacturer from LDT
    pub manufacturer: Option<String>,
    /// Complete raw LDT content (for exact roundtrip)
    pub raw_content: Option<String>,
}

/// Light intensity distribution data
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedDistribution {
    /// Distribution type (TYPE_A, TYPE_B, TYPE_C)
    pub distribution_type: String,
    /// Distribution data: (main_angle, secondary_angles, intensities)
    pub data: Vec<DistributionPlane>,
}

/// Single plane of distribution data (C-plane for Type C)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DistributionPlane {
    /// Main plane angle (C-angle for Type C)
    pub main_angle: f64,
    /// Intensities at secondary angles (gamma angles)
    pub intensities: Vec<(f64, f64)>, // (angle, intensity)
}

/// Extracted photometry in EULUMDAT-compatible format
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedPhotometry {
    /// Luminaire name
    pub name: String,
    /// Total luminous flux
    pub luminous_flux: f64,
    /// Number of C-planes
    pub num_c_planes: usize,
    /// Number of gamma angles per plane
    pub num_gamma_angles: usize,
    /// C-plane angles
    pub c_angles: Vec<f64>,
    /// Gamma angles
    pub gamma_angles: Vec<f64>,
    /// Intensity values [c_plane][gamma]
    pub intensities: Vec<Vec<f64>>,
}

/// Extracted mesh geometry
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedGeometry {
    /// Vertex positions (x, y, z) in meters
    pub vertices: Vec<(f64, f64, f64)>,
    /// Triangle indices (0-based)
    pub triangles: Vec<(u32, u32, u32)>,
}

/// Extracted properties from property sets
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportedProperties {
    /// Number of light sources
    pub number_of_sources: Option<i32>,
    /// Total wattage (W)
    pub total_wattage: Option<f64>,
    /// Mounting type (CEILING, WALL, etc.)
    pub mounting_type: Option<String>,
    /// Color temperature (K)
    pub color_temperature: Option<f64>,
    /// Luminous flux (lm)
    pub luminous_flux: Option<f64>,
    /// Power (W)
    pub power: Option<f64>,
    /// Efficacy (lm/W)
    pub efficacy: Option<f64>,
    /// Color rendering index
    pub cri: Option<i32>,
    /// Weight (kg)
    pub weight: Option<f64>,
    /// IP code (IEC 60529)
    pub ip_code: Option<String>,
    /// IK code (IEC 62262) - mechanical impact protection
    pub ik_code: Option<String>,
    /// Rated voltage (V)
    pub rated_voltage: Option<f64>,
    /// Maximum rated voltage (V) - for voltage range
    pub rated_voltage_max: Option<f64>,
    /// Rated current (A)
    pub rated_current: Option<f64>,
    /// Power factor (cos φ)
    pub power_factor: Option<f64>,
    /// Nominal frequency min (Hz)
    pub nominal_frequency_min: Option<f64>,
    /// Nominal frequency max (Hz)
    pub nominal_frequency_max: Option<f64>,
    /// Number of electrical poles
    pub number_of_poles: Option<i32>,
    /// Has protective earth connection
    pub has_protective_earth: Option<bool>,
    /// Insulation standard class (CLASS0, CLASSI, CLASSII, CLASSIII)
    pub insulation_standard_class: Option<String>,
    /// Heat dissipation (W)
    pub heat_dissipation: Option<f64>,
    /// Nominal power consumption (W)
    pub nominal_power_consumption: Option<f64>,
    /// Number of power supply ports
    pub number_of_power_supply_ports: Option<i32>,
    /// Electrical safety class (I, II, III) - legacy/GLDF format
    pub electrical_safety_class: Option<String>,
    /// Median useful life (e.g., "L80B50 70000h 25°C")
    pub median_useful_life: Option<String>,
    /// GLDF mounting type (Ceiling, Wall, Ground, etc.)
    pub gldf_mounting_type: Option<String>,
    /// Recessed depth (mm)
    pub recessed_depth: Option<f64>,
    /// Maintenance factor (0.0-1.0)
    pub maintenance_factor: Option<f64>,
}

/// IFC to GLDF importer
pub struct IfcImporter {
    parser: StepParser,
}

impl IfcImporter {
    /// Create importer from IFC file content
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> Result<Self> {
        let parser = StepParser::parse(content)?;
        Ok(Self { parser })
    }

    /// Import all luminaires from the IFC file
    pub fn import(&self) -> Result<ImportedLuminaire> {
        let mut luminaire = ImportedLuminaire::default();

        // Find all light fixture types
        let fixture_types = self.parser.find_by_type("IFCLIGHTFIXTURETYPE");

        if fixture_types.is_empty() {
            return Err(anyhow!("No IFCLIGHTFIXTURETYPE found in IFC file"));
        }

        // Extract manufacturer info from first type
        if let Some(first_type) = fixture_types.first() {
            let params = first_type.get_params();

            // Name is param[2]
            if let Some(name) = params.get(2).and_then(|v| v.as_string()) {
                // Split variant suffix if present (e.g., "Product - Variant")
                if let Some(pos) = name.find(" - ") {
                    luminaire.name = name[..pos].to_string();
                } else {
                    luminaire.name = name.to_string();
                }
            }

            // Description is param[3]
            if let Some(desc) = params.get(3).and_then(|v| v.as_string()) {
                luminaire.description = Some(desc.to_string());
            }

            // Get manufacturer from property set
            if let Some(pset) =
                self.find_property_set(first_type.id, "Pset_ManufacturerTypeInformation")
            {
                if let Some(mfr) = self.get_property_value(&pset, "Manufacturer") {
                    luminaire.manufacturer = mfr.as_string().map(|s| s.to_string());
                }
                if let Some(model) = self.get_property_value(&pset, "ModelReference") {
                    luminaire.model_reference = model.as_string().map(|s| s.to_string());
                }
            }
        }

        // Extract each fixture type as a variant
        for fixture_type in &fixture_types {
            let variant = self.extract_variant(fixture_type)?;
            luminaire.variants.push(variant);
        }

        // Extract product-level descriptive attributes from first variant
        // (IP code, safety class, etc. are product-level in GLDF, not per-variant)
        if let Some(first_variant) = luminaire.variants.first() {
            luminaire.descriptive_attributes.electrical_safety_class =
                first_variant.properties.electrical_safety_class.clone();
            luminaire.descriptive_attributes.ip_code = first_variant.properties.ip_code.clone();
            luminaire.descriptive_attributes.median_useful_life =
                first_variant.properties.median_useful_life.clone();
        }

        // Extract embedded files (images, sensors) from Pset_GLDF_EmbeddedFile
        luminaire.embedded_files = self.extract_embedded_files();

        Ok(luminaire)
    }

    /// Extract embedded GLDF files (images, sensors) from IFC property sets
    fn extract_embedded_files(&self) -> Vec<EmbeddedFile> {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let mut files = Vec::new();

        // Find all Pset_GLDF_EmbeddedFile property sets
        for entity in self.parser.find_by_type("IFCPROPERTYSET") {
            let params = entity.get_params();
            if let Some(name) = params.get(2).and_then(|v| v.as_string()) {
                if name == "Pset_GLDF_EmbeddedFile" {
                    // Extract properties from this pset
                    let mut file_type = None;
                    let mut filename = None;
                    let mut content_type = None;
                    let mut content_b64 = None;

                    if let Some(StepValue::List(props)) = params.get(4) {
                        for prop_ref in props {
                            if let StepValue::Ref(prop_id) = prop_ref {
                                if let Some(prop_entity) = self.parser.get(*prop_id) {
                                    let prop_params = prop_entity.get_params();
                                    if let Some(prop_name) =
                                        prop_params.first().and_then(|v: &StepValue| v.as_string())
                                    {
                                        let value = prop_params
                                            .get(2)
                                            .and_then(|v: &StepValue| v.as_string());
                                        match prop_name {
                                            "GLDF_FileType" => file_type = value.map(String::from),
                                            "GLDF_Filename" => filename = value.map(String::from),
                                            "GLDF_ContentType" => {
                                                content_type = value.map(String::from)
                                            }
                                            "GLDF_FileContent" => {
                                                content_b64 = value.map(String::from)
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // If we have all required fields, decode and add the file
                    if let (Some(ft), Some(fn_), Some(ct), Some(b64)) =
                        (file_type, filename, content_type, content_b64)
                    {
                        if let Ok(content) = STANDARD.decode(&b64) {
                            files.push(EmbeddedFile {
                                file_type: ft,
                                filename: fn_,
                                content_type: ct,
                                content,
                            });
                        }
                    }
                }
            }
        }

        files
    }

    /// Extract variant data from IFCLIGHTFIXTURETYPE
    fn extract_variant(
        &self,
        fixture_type: &super::step_parser::StepEntity,
    ) -> Result<ImportedVariant> {
        let mut variant = ImportedVariant::default();
        let params = fixture_type.get_params();

        // Name (param[2])
        if let Some(name) = params.get(2).and_then(|v| v.as_string()) {
            variant.name = name.to_string();
        }

        // Description (param[3])
        if let Some(desc) = params.get(3).and_then(|v| v.as_string()) {
            variant.description = Some(desc.to_string());
        }

        // Predefined type (param[9])
        if let Some(ptype) = params.get(9).and_then(|v| v.as_enum()) {
            variant.fixture_type = Some(ptype.to_string());
        }

        // Extract properties from Pset_LightFixtureTypeCommon
        if let Some(pset) = self.find_property_set(fixture_type.id, "Pset_LightFixtureTypeCommon") {
            if let Some(v) = self.get_property_value(&pset, "NumberOfSources") {
                variant.properties.number_of_sources = v.as_integer().map(|i| i as i32);
            }
            if let Some(v) = self.get_property_value(&pset, "TotalWattage") {
                variant.properties.total_wattage = v.as_real();
            }
            if let Some(v) = self.get_property_value(&pset, "LightFixtureMountingType") {
                variant.properties.mounting_type = v.as_string().map(|s| s.to_string());
            }
        }

        // Extract variant-specific properties from Pset_LuminaireVariant
        if let Some(pset) = self.find_property_set(fixture_type.id, "Pset_LuminaireVariant") {
            if let Some(v) = self.get_property_value(&pset, "GLDF_VariantId") {
                variant.gldf_variant_id = v.as_string().map(|s| s.to_string());
            }
            if let Some(v) = self.get_property_value(&pset, "ColorTemperature") {
                variant.properties.color_temperature = v.as_real();
            }
            if let Some(v) = self.get_property_value(&pset, "LuminousFlux") {
                variant.properties.luminous_flux = v.as_real();
            }
            if let Some(v) = self.get_property_value(&pset, "Power") {
                variant.properties.power = v.as_real();
            }
            if let Some(v) = self.get_property_value(&pset, "Efficacy") {
                variant.properties.efficacy = v.as_real();
            }
            if let Some(v) = self.get_property_value(&pset, "ColorRenderingIndex") {
                variant.properties.cri = v.as_integer().map(|i| i as i32);
            }
            if let Some(v) = self.get_property_value(&pset, "Weight") {
                variant.properties.weight = v.as_real();
            }
        }

        // Extract electrical properties from Pset_ElectricalDeviceCommon (Electrical MVD)
        if let Some(pset) = self.find_property_set(fixture_type.id, "Pset_ElectricalDeviceCommon") {
            // IP_Code (IEC 60529)
            if let Some(v) = self.get_property_value(&pset, "IP_Code") {
                variant.properties.ip_code = v.as_string().map(|s| s.to_string());
            }
            // Legacy IPCode support
            if variant.properties.ip_code.is_none() {
                if let Some(v) = self.get_property_value(&pset, "IPCode") {
                    variant.properties.ip_code = v.as_string().map(|s| s.to_string());
                }
            }
            // IK_Code (IEC 62262)
            if let Some(v) = self.get_property_value(&pset, "IK_Code") {
                variant.properties.ik_code = v.as_string().map(|s| s.to_string());
            }
            // RatedVoltage
            if let Some(v) = self.get_property_value(&pset, "RatedVoltage") {
                variant.properties.rated_voltage = v.as_real();
                // TODO: Handle bounded values for voltage range
            }
            // RatedCurrent
            if let Some(v) = self.get_property_value(&pset, "RatedCurrent") {
                variant.properties.rated_current = v.as_real();
            }
            // PowerFactor
            if let Some(v) = self.get_property_value(&pset, "PowerFactor") {
                variant.properties.power_factor = v.as_real();
            }
            // NominalFrequencyRange - bounded value
            if let Some(v) = self.get_property_value(&pset, "NominalFrequencyRange") {
                // For bounded values, try to extract both min and max
                if let Some(freq) = v.as_real() {
                    variant.properties.nominal_frequency_min = Some(freq);
                    variant.properties.nominal_frequency_max = Some(freq);
                }
            }
            // NumberOfPoles
            if let Some(v) = self.get_property_value(&pset, "NumberOfPoles") {
                variant.properties.number_of_poles = v.as_integer().map(|i| i as i32);
            }
            // HasProtectiveEarth
            if let Some(v) = self.get_property_value(&pset, "HasProtectiveEarth") {
                variant.properties.has_protective_earth = v.as_boolean();
            }
            // InsulationStandardClass - can be enum or string
            if let Some(v) = self.get_property_value(&pset, "InsulationStandardClass") {
                variant.properties.insulation_standard_class = v
                    .as_enum()
                    .map(|s| s.to_string())
                    .or_else(|| v.as_string().map(|s| s.to_string()));
            }
            // HeatDissipation
            if let Some(v) = self.get_property_value(&pset, "HeatDissipation") {
                variant.properties.heat_dissipation = v.as_real();
            }
            // Power
            if let Some(v) = self.get_property_value(&pset, "Power") {
                // Don't overwrite power from variant pset
                if variant.properties.power.is_none() {
                    variant.properties.power = v.as_real();
                }
            }
            // NominalPowerConsumption
            if let Some(v) = self.get_property_value(&pset, "NominalPowerConsumption") {
                variant.properties.nominal_power_consumption = v.as_real();
            }
            // NumberOfPowerSupplyPorts
            if let Some(v) = self.get_property_value(&pset, "NumberOfPowerSupplyPorts") {
                variant.properties.number_of_power_supply_ports = v.as_integer().map(|i| i as i32);
            }
        }

        // Extract from Pset_LightFixtureTypeCommon
        if let Some(pset) = self.find_property_set(fixture_type.id, "Pset_LightFixtureTypeCommon") {
            if let Some(v) = self.get_property_value(&pset, "MaintenanceFactor") {
                variant.properties.maintenance_factor = v.as_real();
            }
            // These might override values from basic pset
            if variant.properties.number_of_sources.is_none() {
                if let Some(v) = self.get_property_value(&pset, "NumberOfSources") {
                    variant.properties.number_of_sources = v.as_integer().map(|i| i as i32);
                }
            }
            if variant.properties.total_wattage.is_none() {
                if let Some(v) = self.get_property_value(&pset, "TotalWattage") {
                    variant.properties.total_wattage = v.as_real();
                }
            }
        }

        // Extract GLDF descriptive attributes from ALL Pset_GLDF_DescriptiveAttributes property sets
        for pset in self.find_all_property_sets(fixture_type.id, "Pset_GLDF_DescriptiveAttributes")
        {
            if let Some(v) = self.get_property_value(&pset, "ElectricalSafetyClass") {
                variant.properties.electrical_safety_class = v.as_string().map(|s| s.to_string());
            }
            if let Some(v) = self.get_property_value(&pset, "MedianUsefulLife") {
                variant.properties.median_useful_life = v.as_string().map(|s| s.to_string());
            }
            if let Some(v) = self.get_property_value(&pset, "GLDF_MountingType") {
                variant.properties.gldf_mounting_type = v.as_string().map(|s| s.to_string());
            }
            if let Some(v) = self.get_property_value(&pset, "GLDF_RecessedDepth") {
                variant.properties.recessed_depth = v.as_real();
            }
        }

        // Extract photometry filenames from Pset_GLDF_PhotometryFiles (for roundtrip preservation)
        let photometry_filenames = self.extract_photometry_filenames(fixture_type.id);

        // Extract LDT metadata from Pset_GLDF_LDTMetadata (for roundtrip preservation)
        let ldt_metadata_map = self.extract_ldt_metadata(fixture_type.id);

        // Extract raw LDT content from Pset_GLDF_LDTRawContent (for exact roundtrip)
        let ldt_raw_content_map = self.extract_ldt_raw_content(fixture_type.id);

        // Find associated light sources via IFCRELASSIGNSTOGROUP
        let light_sources = self.find_light_sources_for_fixture(fixture_type.id);
        for (idx, ls_id) in light_sources.iter().enumerate() {
            if let Some(mut ls) = self.extract_light_source(*ls_id) {
                // Associate preserved photometry filename with this light source
                if let Some(filename) = photometry_filenames.get(idx) {
                    ls.photometry_filename = Some(filename.clone());
                }
                // Associate preserved LDT metadata with this light source
                // First try raw content, then fall back to metadata
                if let Some((filename, content)) = ldt_raw_content_map.get(&(idx + 1)) {
                    let mut meta = ldt_metadata_map
                        .get(&(idx + 1))
                        .cloned()
                        .unwrap_or_default();
                    meta.raw_content = Some(content.clone());
                    // Override filename if we have it from raw content
                    ls.photometry_filename = Some(filename.clone());
                    ls.ldt_metadata = Some(meta);
                } else if let Some(meta) = ldt_metadata_map.get(&(idx + 1)) {
                    ls.ldt_metadata = Some(meta.clone());
                }
                variant.light_sources.push(ls);
            }
        }

        // Find geometry from fixture occurrence
        if let Some(occurrence_id) = self.find_fixture_occurrence(fixture_type.id) {
            if let Some(geom) = self.extract_geometry(occurrence_id) {
                variant.geometry = Some(geom);
            }
        }

        Ok(variant)
    }

    /// Extract photometry filenames from Pset_GLDF_PhotometryFiles
    fn extract_photometry_filenames(&self, element_id: u64) -> Vec<String> {
        let mut filenames = Vec::new();

        if let Some(pset_id) = self.find_property_set(element_id, "Pset_GLDF_PhotometryFiles") {
            // Get all PhotometryFile_N properties (1, 2, 3, etc.)
            for i in 1..=100 {
                let prop_name = format!("PhotometryFile_{}", i);
                if let Some(value) = self.get_property_value(&pset_id, &prop_name) {
                    if let Some(filename) = value.as_string() {
                        filenames.push(filename.to_string());
                    }
                } else {
                    break; // No more properties
                }
            }
        }

        filenames
    }

    /// Extract LDT metadata from all Pset_GLDF_LDTMetadata property sets
    fn extract_ldt_metadata(
        &self,
        element_id: u64,
    ) -> std::collections::HashMap<usize, LdtMetadata> {
        use std::collections::HashMap;
        let mut metadata_map: HashMap<usize, LdtMetadata> = HashMap::new();

        // Find ALL Pset_GLDF_LDTMetadata property sets for this element
        for pset_id in self.find_all_property_sets(element_id, "Pset_GLDF_LDTMetadata") {
            // Each property set contains metadata for one LDT file
            // Try to find the index from properties like LDT_1_Index, LDT_2_Index, etc.
            for i in 1..=100 {
                let idx_name = format!("LDT_{}_Index", i);
                // Check if this index exists in this pset
                if self.get_property_value(&pset_id, &idx_name).is_none() {
                    continue; // Try next index
                }

                let mut meta = LdtMetadata::default();

                // Symmetry
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_Symmetry", i)) {
                    meta.symmetry = v.as_integer().unwrap_or(0) as i32;
                }

                // Number of C-planes
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_NumCPlanes", i))
                {
                    meta.num_c_planes = v.as_integer().unwrap_or(1) as i32;
                }

                // Number of gamma angles
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_NumGAngles", i))
                {
                    meta.num_g_angles = v.as_integer().unwrap_or(19) as i32;
                }

                // C-plane distance (Dc)
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_Dc", i)) {
                    meta.dc = v.as_real().unwrap_or(0.0);
                }

                // Gamma distance (Dg)
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_Dg", i)) {
                    meta.dg = v.as_real().unwrap_or(0.0);
                }

                // Total flux
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_TotalFlux", i))
                {
                    meta.total_flux = Some(v.as_real().unwrap_or(1000.0));
                }

                // DR values (stored as comma-separated string)
                if let Some(v) = self.get_property_value(&pset_id, &format!("LDT_{}_DR", i)) {
                    if let Some(dr_str) = v.as_string() {
                        let dr_vals: Vec<f64> = dr_str
                            .split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect();
                        if dr_vals.len() == 10 {
                            let mut dr_arr = [1.0; 10];
                            for (j, val) in dr_vals.iter().enumerate() {
                                dr_arr[j] = *val;
                            }
                            meta.dr = Some(dr_arr);
                        }
                    }
                }

                // Luminaire name
                if let Some(v) =
                    self.get_property_value(&pset_id, &format!("LDT_{}_LuminaireName", i))
                {
                    meta.luminaire_name = v.as_string().map(|s| s.to_string());
                }

                metadata_map.insert(i, meta);
            }
        }

        metadata_map
    }

    /// Extract raw LDT content from Pset_GLDF_LDTRawContent
    fn extract_ldt_raw_content(
        &self,
        element_id: u64,
    ) -> std::collections::HashMap<usize, (String, String)> {
        use std::collections::HashMap;
        let mut content_map: HashMap<usize, (String, String)> = HashMap::new();

        // Find ALL Pset_GLDF_LDTRawContent property sets for this element
        for pset_id in self.find_all_property_sets(element_id, "Pset_GLDF_LDTRawContent") {
            // Each property set contains LDT_N_Filename and LDT_N_Content
            for i in 1..=100 {
                let filename_prop = format!("LDT_{}_Filename", i);
                let content_prop = format!("LDT_{}_Content", i);

                let filename = self
                    .get_property_value(&pset_id, &filename_prop)
                    .and_then(|v| v.as_string().map(|s| s.to_string()));
                let content_b64 = self
                    .get_property_value(&pset_id, &content_prop)
                    .and_then(|v| v.as_string().map(|s| s.to_string()));

                if let (Some(f), Some(c)) = (filename, content_b64) {
                    // Decode base64 content
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    if let Ok(decoded) = STANDARD.decode(&c) {
                        if let Ok(content) = String::from_utf8(decoded) {
                            content_map.insert(i, (f, content));
                        }
                    }
                }
            }
        }

        content_map
    }

    /// Find ALL property sets with a given name for an element
    fn find_all_property_sets(&self, element_id: u64, pset_name: &str) -> Vec<u64> {
        let mut psets = Vec::new();

        // Look for IFCRELDEFINESBYPROPERTIES that references our element
        for entity in self.parser.find_by_type("IFCRELDEFINESBYPROPERTIES") {
            let params = entity.get_params();

            // RelatedObjects is param[4] (list)
            let related_to_element = params
                .get(4)
                .and_then(|v| v.as_list())
                .is_some_and(|list| list.iter().any(|item| item.as_ref() == Some(element_id)));

            if !related_to_element {
                continue;
            }

            // RelatingPropertyDefinition is param[5] (ref)
            if let Some(pset_ref) = params.get(5).and_then(|v| v.as_ref()) {
                if let Some(pset) = self.parser.get(pset_ref) {
                    if pset.entity_type == "IFCPROPERTYSET" {
                        let pset_params = pset.get_params();
                        // Name is param[2]
                        if let Some(name) = pset_params.get(2).and_then(|v| v.as_string()) {
                            if name == pset_name {
                                psets.push(pset_ref);
                            }
                        }
                    }
                }
            }
        }

        psets
    }

    /// Find property set for an element
    fn find_property_set(&self, element_id: u64, pset_name: &str) -> Option<u64> {
        // Look for IFCRELDEFINESBYPROPERTIES that references our element
        for entity in self.parser.find_by_type("IFCRELDEFINESBYPROPERTIES") {
            let params = entity.get_params();

            // RelatedObjects is param[4] (list of element refs)
            if let Some(related) = params.get(4).and_then(|v| v.as_list()) {
                let has_element = related.iter().any(|v| v.as_ref() == Some(element_id));

                if has_element {
                    // RelatingPropertyDefinition is param[5]
                    if let Some(pset_ref) = params.get(5).and_then(|v| v.as_ref()) {
                        if let Some(pset) = self.parser.get(pset_ref) {
                            if pset.entity_type == "IFCPROPERTYSET" {
                                let pset_params = pset.get_params();
                                // Name is param[2]
                                if let Some(name) = pset_params.get(2).and_then(|v| v.as_string()) {
                                    if name == pset_name {
                                        return Some(pset_ref);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Get property value from property set
    fn get_property_value(&self, pset_id: &u64, property_name: &str) -> Option<StepValue> {
        let pset = self.parser.get(*pset_id)?;
        let params = pset.get_params();

        // HasProperties is param[4] (list of property refs)
        let properties = params.get(4)?.as_list()?;

        for prop_ref in properties {
            if let Some(prop_id) = prop_ref.as_ref() {
                if let Some(prop) = self.parser.get(prop_id) {
                    if prop.entity_type == "IFCPROPERTYSINGLEVALUE" {
                        let prop_params = prop.get_params();
                        // Name is param[0]
                        if let Some(name) = prop_params.first().and_then(|v| v.as_string()) {
                            if name == property_name {
                                // NominalValue is param[2]
                                return prop_params.get(2).cloned();
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Find light sources associated with a fixture type via IFCRELASSIGNSTOGROUP
    fn find_light_sources_for_fixture(&self, fixture_type_id: u64) -> Vec<u64> {
        let mut sources = Vec::new();

        // First find fixture occurrences of this type
        let occurrences = self.find_occurrences_of_type(fixture_type_id);

        for occ_id in occurrences {
            // Find IFCRELASSIGNSTOGROUP that has this occurrence as RelatingGroup
            for entity in self.parser.find_by_type("IFCRELASSIGNSTOGROUP") {
                let params = entity.get_params();

                // RelatingGroup is param[6]
                if params.get(6).and_then(|v| v.as_ref()) == Some(occ_id) {
                    // RelatedObjects is param[5] (list)
                    if let Some(related) = params.get(5).and_then(|v| v.as_list()) {
                        for item in related {
                            if let Some(id) = item.as_ref() {
                                if let Some(e) = self.parser.get(id) {
                                    if e.entity_type == "IFCLIGHTSOURCEGONIOMETRIC"
                                        || e.entity_type == "IFCLIGHTSOURCESPOT"
                                        || e.entity_type == "IFCLIGHTSOURCEPOSITIONAL"
                                    {
                                        sources.push(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Only fall back to finding ALL light sources if no grouped sources were found
        // This handles legacy IFC files without proper grouping
        if sources.is_empty() {
            for entity in self.parser.find_by_type("IFCLIGHTSOURCEGONIOMETRIC") {
                sources.push(entity.id);
            }
        }

        sources
    }

    /// Find fixture occurrences (IFCLIGHTFIXTURE) of a given type
    fn find_occurrences_of_type(&self, type_id: u64) -> Vec<u64> {
        let mut occurrences = Vec::new();

        // Find IFCRELDEFINESBYTYPE
        for entity in self.parser.find_by_type("IFCRELDEFINESBYTYPE") {
            let params = entity.get_params();

            // RelatingType is param[5]
            if params.get(5).and_then(|v| v.as_ref()) == Some(type_id) {
                // RelatedObjects is param[4] (list)
                if let Some(related) = params.get(4).and_then(|v| v.as_list()) {
                    for item in related {
                        if let Some(id) = item.as_ref() {
                            occurrences.push(id);
                        }
                    }
                }
            }
        }

        occurrences
    }

    /// Find a fixture occurrence for a type
    fn find_fixture_occurrence(&self, type_id: u64) -> Option<u64> {
        self.find_occurrences_of_type(type_id).first().copied()
    }

    /// Extract light source data
    fn extract_light_source(&self, light_source_id: u64) -> Option<ImportedLightSource> {
        let entity = self.parser.get(light_source_id)?;
        let params = entity.get_params();

        let mut source = ImportedLightSource::default();

        // IFCLIGHTSOURCEGONIOMETRIC params:
        // 0: Name, 1: Description, 2: LightColour, 3: AmbientIntensity,
        // 4: Position, 5: ColourAppearance, 6: ColourTemperature, 7: LuminousFlux,
        // 8: LightEmissionSource, 9: LightDistributionDataSource

        // Name (param[0])
        if let Some(name) = params.first().and_then(|v| v.as_string()) {
            source.name = name.to_string();
        }

        // ColorTemperature (param[6])
        if let Some(cct) = params.get(6).and_then(|v| v.as_real()) {
            source.color_temperature = Some(cct);
        }

        // LuminousFlux (param[7])
        if let Some(flux) = params.get(7).and_then(|v| v.as_real()) {
            source.luminous_flux = Some(flux);
        }

        // LightEmissionSource (param[8])
        if let Some(emission) = params.get(8).and_then(|v| v.as_enum()) {
            source.emission_source = Some(emission.to_string());
        }

        // LightDistributionDataSource (param[9])
        if let Some(dist_ref) = params.get(9).and_then(|v| v.as_ref()) {
            // First check if it's an IFCEXTERNALREFERENCE (contains filename)
            if let Some(ext_ref) = self.parser.get(dist_ref) {
                if ext_ref.entity_type == "IFCEXTERNALREFERENCE" {
                    let ext_params = ext_ref.get_params();
                    // Location is param[0] - contains the file path
                    if let Some(location) = ext_params.first().and_then(|v| v.as_string()) {
                        // Extract just the filename from the path
                        let filename = location.rsplit('/').next().unwrap_or(location);
                        source.photometry_filename = Some(filename.to_string());
                    }
                }
            }
            // Then try to extract distribution data
            source.distribution = self.extract_distribution(dist_ref);
        }

        Some(source)
    }

    /// Extract light distribution data
    fn extract_distribution(&self, dist_id: u64) -> Option<ImportedDistribution> {
        let entity = self.parser.get(dist_id)?;

        // Could be IFCLIGHTINTENSITYDISTRIBUTION or IFCEXTERNALREFERENCE
        if entity.entity_type == "IFCLIGHTINTENSITYDISTRIBUTION" {
            let params = entity.get_params();
            let mut dist = ImportedDistribution::default();

            // LightDistributionCurve (param[0]) - enum
            if let Some(curve_type) = params.first().and_then(|v| v.as_enum()) {
                dist.distribution_type = curve_type.to_string();
            }

            // DistributionData (param[1]) - list of IFCLIGHTDISTRIBUTIONDATA refs
            if let Some(data_list) = params.get(1).and_then(|v| v.as_list()) {
                for data_ref in data_list {
                    if let Some(data_id) = data_ref.as_ref() {
                        if let Some(plane) = self.extract_distribution_plane(data_id) {
                            dist.data.push(plane);
                        }
                    }
                }
            }

            return Some(dist);
        }

        None
    }

    /// Extract a single distribution plane
    fn extract_distribution_plane(&self, data_id: u64) -> Option<DistributionPlane> {
        let entity = self.parser.get(data_id)?;
        if entity.entity_type != "IFCLIGHTDISTRIBUTIONDATA" {
            return None;
        }

        let params = entity.get_params();
        let mut plane = DistributionPlane::default();

        // MainPlaneAngle (param[0])
        if let Some(main) = params.first().and_then(|v| v.as_real()) {
            plane.main_angle = main;
        }

        // SecondaryPlaneAngle (param[1]) - list
        // LuminousIntensity (param[2]) - list
        if let Some(secondary_list) = params.get(1).and_then(|v| v.as_list()) {
            let intensity_list = params.get(2).and_then(|v| v.as_list());

            for (i, angle_val) in secondary_list.iter().enumerate() {
                if let Some(angle) = angle_val.as_real() {
                    let intensity = intensity_list
                        .and_then(|l| l.get(i))
                        .and_then(|v| v.as_real())
                        .unwrap_or(0.0);
                    plane.intensities.push((angle, intensity));
                }
            }
        }

        Some(plane)
    }

    /// Extract geometry from fixture occurrence
    fn extract_geometry(&self, occurrence_id: u64) -> Option<ImportedGeometry> {
        let entity = self.parser.get(occurrence_id)?;
        let params = entity.get_params();

        // Representation is param[6] for IFCLIGHTFIXTURE
        let rep_ref = params.get(6).and_then(|v| v.as_ref())?;
        let rep = self.parser.get(rep_ref)?;

        if rep.entity_type != "IFCPRODUCTDEFINITIONSHAPE" {
            return None;
        }

        let rep_params = rep.get_params();
        // Representations is param[2] (list)
        let reps = rep_params.get(2)?.as_list()?;

        for rep_item in reps {
            if let Some(shape_rep_id) = rep_item.as_ref() {
                if let Some(geom) = self.extract_shape_representation(shape_rep_id) {
                    return Some(geom);
                }
            }
        }

        None
    }

    /// Extract geometry from IFCSHAPEREPRESENTATION
    fn extract_shape_representation(&self, rep_id: u64) -> Option<ImportedGeometry> {
        let rep = self.parser.get(rep_id)?;
        if rep.entity_type != "IFCSHAPEREPRESENTATION" {
            return None;
        }

        let params = rep.get_params();
        // RepresentationType is param[2]
        let rep_type = params.get(2).and_then(|v| v.as_string())?;

        // Items is param[3] (list)
        let items = params.get(3)?.as_list()?;

        // Handle different representation types
        match rep_type {
            "Tessellation" => {
                // Look for IFCTRIANGULATEDFACESET
                for item in items {
                    if let Some(item_id) = item.as_ref() {
                        if let Some(geom) = self.extract_triangulated_faceset(item_id) {
                            return Some(geom);
                        }
                    }
                }
            }
            "MappedRepresentation" => {
                // Look for IFCMAPPEDITEM and follow to the actual geometry
                for item in items {
                    if let Some(item_id) = item.as_ref() {
                        if let Some(geom) = self.extract_mapped_item(item_id) {
                            return Some(geom);
                        }
                    }
                }
            }
            "SweptSolid" => {
                // IFCEXTRUDEDAREASOLID - create simple box mesh
                for item in items {
                    if let Some(item_id) = item.as_ref() {
                        if let Some(geom) = self.extract_extruded_solid(item_id) {
                            return Some(geom);
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Extract geometry from IFCMAPPEDITEM
    fn extract_mapped_item(&self, item_id: u64) -> Option<ImportedGeometry> {
        let entity = self.parser.get(item_id)?;
        if entity.entity_type != "IFCMAPPEDITEM" {
            return None;
        }

        let params = entity.get_params();
        // MappingSource is param[0] - ref to IFCREPRESENTATIONMAP
        let map_ref = params.first().and_then(|v| v.as_ref())?;
        let rep_map = self.parser.get(map_ref)?;

        if rep_map.entity_type != "IFCREPRESENTATIONMAP" {
            return None;
        }

        let map_params = rep_map.get_params();
        // MappedRepresentation is param[1] - ref to IFCSHAPEREPRESENTATION
        let shape_rep_ref = map_params.get(1).and_then(|v| v.as_ref())?;

        // Recursively extract from the mapped shape representation
        self.extract_shape_representation(shape_rep_ref)
    }

    /// Extract mesh from IFCTRIANGULATEDFACESET
    fn extract_triangulated_faceset(&self, faceset_id: u64) -> Option<ImportedGeometry> {
        let entity = self.parser.get(faceset_id)?;
        if entity.entity_type != "IFCTRIANGULATEDFACESET" {
            return None;
        }

        let params = entity.get_params();
        let mut geom = ImportedGeometry::default();

        // Coordinates is param[0] - ref to IFCCARTESIANPOINTLIST3D
        if let Some(coords_ref) = params.first().and_then(|v| v.as_ref()) {
            if let Some(coords) = self.parser.get(coords_ref) {
                if coords.entity_type == "IFCCARTESIANPOINTLIST3D" {
                    let coords_params = coords.get_params();
                    // CoordList is param[0] - nested list of coordinates
                    if let Some(coord_list) = coords_params.first().and_then(|v| v.as_list()) {
                        for point in coord_list {
                            if let Some(point_coords) = point.as_list() {
                                let x = point_coords
                                    .first()
                                    .and_then(|v| v.as_real())
                                    .unwrap_or(0.0);
                                let y =
                                    point_coords.get(1).and_then(|v| v.as_real()).unwrap_or(0.0);
                                let z =
                                    point_coords.get(2).and_then(|v| v.as_real()).unwrap_or(0.0);
                                geom.vertices.push((x, y, z));
                            }
                        }
                    }
                }
            }
        }

        // CoordIndex is param[3] - list of triangle indices (1-based in IFC)
        if let Some(index_list) = params.get(3).and_then(|v| v.as_list()) {
            for tri in index_list {
                if let Some(indices) = tri.as_list() {
                    let i0 = indices.first().and_then(|v| v.as_integer()).unwrap_or(1) as u32 - 1;
                    let i1 = indices.get(1).and_then(|v| v.as_integer()).unwrap_or(1) as u32 - 1;
                    let i2 = indices.get(2).and_then(|v| v.as_integer()).unwrap_or(1) as u32 - 1;
                    geom.triangles.push((i0, i1, i2));
                }
            }
        }

        if geom.vertices.is_empty() || geom.triangles.is_empty() {
            return None;
        }

        Some(geom)
    }

    /// Extract mesh from IFCEXTRUDEDAREASOLID (create simple box)
    fn extract_extruded_solid(&self, solid_id: u64) -> Option<ImportedGeometry> {
        let entity = self.parser.get(solid_id)?;
        if entity.entity_type != "IFCEXTRUDEDAREASOLID" {
            return None;
        }

        let params = entity.get_params();

        // SweptArea is param[0] - ref to profile
        let profile_ref = params.first().and_then(|v| v.as_ref())?;
        let profile = self.parser.get(profile_ref)?;

        // Depth is param[3]
        let depth = params.get(3).and_then(|v| v.as_real()).unwrap_or(0.1);

        // Handle IFCRECTANGLEPROFILEDEF
        if profile.entity_type == "IFCRECTANGLEPROFILEDEF" {
            let profile_params = profile.get_params();
            let width = profile_params
                .get(3)
                .and_then(|v| v.as_real())
                .unwrap_or(0.3);
            let height = profile_params
                .get(4)
                .and_then(|v| v.as_real())
                .unwrap_or(0.3);

            return Some(self.create_box_mesh(width, height, depth));
        }

        None
    }

    /// Create a simple box mesh
    fn create_box_mesh(&self, width: f64, height: f64, depth: f64) -> ImportedGeometry {
        let hw = width / 2.0;
        let hh = height / 2.0;

        let vertices = vec![
            // Bottom face
            (-hw, -hh, 0.0),
            (hw, -hh, 0.0),
            (hw, hh, 0.0),
            (-hw, hh, 0.0),
            // Top face
            (-hw, -hh, depth),
            (hw, -hh, depth),
            (hw, hh, depth),
            (-hw, hh, depth),
        ];

        let triangles = vec![
            // Bottom
            (0, 2, 1),
            (0, 3, 2),
            // Top
            (4, 5, 6),
            (4, 6, 7),
            // Front
            (0, 1, 5),
            (0, 5, 4),
            // Back
            (2, 3, 7),
            (2, 7, 6),
            // Left
            (0, 4, 7),
            (0, 7, 3),
            // Right
            (1, 2, 6),
            (1, 6, 5),
        ];

        ImportedGeometry {
            vertices,
            triangles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_basic() {
        let content = r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('GLDF to IFC Export'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'gldf-rs','gldf-rs','');
FILE_SCHEMA(('IFC4'));
ENDSEC;

DATA;
#1=IFCPERSON($,$,'',$,$,$,$,$);
#2=IFCORGANIZATION($,'Test Corp','GLDF Export',$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1.0','Test','Test');
#5=IFCOWNERHISTORY(#3,#4,.READWRITE.,.ADDED.,$,$,$,0);
#6=IFCLIGHTFIXTURETYPE('guid1',#5,'LED Panel - 4000K','Test fixture',$,$,$,$,$,.POINTSOURCE.);
#7=IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('Test Corp'),$);
#8=IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('LED-001'),$);
#9=IFCPROPERTYSET('guid2',#5,'Pset_ManufacturerTypeInformation',$,(#7,#8));
#10=IFCRELDEFINESBYPROPERTIES('guid3',#5,$,$,(#6),#9);
ENDSEC;

END-ISO-10303-21;
"#;

        let importer = IfcImporter::from_str(content).unwrap();
        let luminaire = importer.import().unwrap();

        assert_eq!(luminaire.name, "LED Panel");
        assert_eq!(luminaire.manufacturer, Some("Test Corp".to_string()));
        assert_eq!(luminaire.model_reference, Some("LED-001".to_string()));
        assert_eq!(luminaire.variants.len(), 1);
        assert_eq!(luminaire.variants[0].name, "LED Panel - 4000K");
    }
}
