//! L3D to LDT Mapping Helper
//!
//! This module provides utilities to extract the relationship between
//! L3D geometry files and LDT photometry files from GLDF structure.

use crate::gldf::GldfProduct;
use crate::FileBufGldf;

/// Represents a mapping between an L3D geometry file and its associated LDT photometry file
#[derive(Debug, Clone)]
pub struct L3dLdtMapping {
    /// The file ID of the L3D geometry file
    pub l3d_file_id: String,
    /// The file name of the L3D file
    pub l3d_file_name: Option<String>,
    /// The file ID of the associated LDT/IES photometry file
    pub ldt_file_id: Option<String>,
    /// The file name of the LDT/IES file
    pub ldt_file_name: Option<String>,
    /// The variant ID this mapping comes from
    pub variant_id: String,
    /// The emitter ID
    pub emitter_id: Option<String>,
}

/// Per-emitter data for rendering with correct intensity and color
#[derive(Debug, Clone, Default)]
pub struct EmitterRenderData {
    /// The LEO (Light Emitting Object) name from L3D structure.xml
    pub leo_name: String,
    /// The emitter ID from GLDF
    pub emitter_id: String,
    /// Rated luminous flux in lumens (lm)
    pub luminous_flux: Option<i32>,
    /// Correlated color temperature in Kelvin (K)
    pub color_temperature: Option<i32>,
    /// LDT file ID for photometry data
    pub ldt_file_id: Option<String>,
    /// LDT file name
    pub ldt_file_name: Option<String>,
    /// Emergency behavior (e.g., "Combined", "EmergencyOnly")
    pub emergency_behavior: Option<String>,
}

/// Mounting type for luminaire positioning
#[derive(Debug, Clone, PartialEq)]
pub enum MountingType {
    /// Ceiling mounted (recessed, surface, or pendant)
    Ceiling,
    /// Wall mounted
    Wall,
    /// Ground mounted (pole, free-standing)
    Ground,
    /// Working plane (table/desk)
    WorkingPlane,
}

/// Mounting information extracted from GLDF variant
#[derive(Debug, Clone, Default)]
pub struct MountingInfo {
    /// Primary mounting type
    pub mounting_type: Option<MountingType>,
    /// Recessed depth in mm (for ceiling/wall recessed)
    pub recessed_depth_mm: Option<i32>,
    /// Pendant length in mm (for ceiling pendant)
    pub pendant_length_mm: Option<f64>,
    /// Mounting height in mm (for wall mounted)
    pub mounting_height_mm: Option<i32>,
    /// Pole height in mm (for ground pole-top/pole-integrated)
    pub pole_height_mm: Option<i32>,
    /// Whether surface mounted
    pub is_surface_mounted: bool,
    /// Whether free-standing
    pub is_free_standing: bool,
}

impl MountingInfo {
    /// Get the Y offset in meters for positioning in 3D scene
    /// Returns (y_position, is_from_ceiling)
    pub fn get_position_offset(&self, room_height: f32) -> (f32, bool) {
        match self.mounting_type {
            Some(MountingType::Ceiling) => {
                if let Some(pendant_mm) = self.pendant_length_mm {
                    // Pendant: hang down from ceiling
                    (room_height - (pendant_mm as f32 / 1000.0), true)
                } else if self.recessed_depth_mm.is_some() {
                    // Recessed: flush with ceiling
                    (room_height, true)
                } else {
                    // Surface mounted: slightly below ceiling
                    (room_height - 0.05, true)
                }
            }
            Some(MountingType::Wall) => {
                // Wall: use mounting height or default to 2m
                let height_m = self.mounting_height_mm.unwrap_or(2000) as f32 / 1000.0;
                (height_m, false)
            }
            Some(MountingType::Ground) => {
                if let Some(pole_mm) = self.pole_height_mm {
                    // Pole mounted: at pole height
                    (pole_mm as f32 / 1000.0, false)
                } else if self.is_free_standing {
                    // Free-standing on ground
                    (0.0, false)
                } else {
                    // Default ground level
                    (0.0, false)
                }
            }
            Some(MountingType::WorkingPlane) => {
                // Working plane: typical desk height ~0.75m
                (0.75, false)
            }
            None => {
                // Default: assume ceiling mounted
                (room_height - 0.1, true)
            }
        }
    }
}

/// All emitter data for a variant
#[derive(Debug, Clone, Default)]
pub struct VariantEmitterData {
    /// The variant ID
    pub variant_id: String,
    /// L3D file ID
    pub l3d_file_id: Option<String>,
    /// L3D file name
    pub l3d_file_name: Option<String>,
    /// Per-emitter render data
    pub emitters: Vec<EmitterRenderData>,
    /// Mounting information for positioning
    pub mounting: MountingInfo,
}

/// Extract L3D to LDT mappings from a GLDF product
///
/// This function traverses the GLDF structure to find all L3D geometry files
/// and their associated LDT photometry files through the variant/emitter chain.
///
/// The mapping chain is:
/// ```text
/// Variant → Geometry → ModelGeometryReference (geometry_id)
///     → ModelGeometry → GeometryFileReference (file_id) → L3D file
///
/// Variant → Geometry → ModelGeometryReference → EmitterReference (emitter_id)
///     → Emitter → FixedLightEmitter/ChangeableLightEmitter
///     → PhotometryReference (photometry_id)
///     → Photometry → PhotometryFileReference (file_id) → LDT file
/// ```
pub fn get_l3d_ldt_mappings(gldf: &GldfProduct) -> Vec<L3dLdtMapping> {
    let mut mappings = Vec::new();

    // Get variants
    let Some(variants) = &gldf.product_definitions.variants else {
        return mappings;
    };

    // Get general definitions for lookups
    let general_defs = &gldf.general_definitions;

    // Build lookup maps
    let geometries = general_defs.geometries.as_ref();
    let emitters = general_defs.emitters.as_ref();
    let photometries = general_defs.photometries.as_ref();
    let files = &general_defs.files.file;

    // Helper to find file name by file_id
    let get_file_name = |file_id: &str| -> Option<String> {
        files
            .iter()
            .find(|f| f.id == file_id)
            .map(|f| f.file_name.clone())
    };

    // Helper to find L3D file_id from geometry_id
    let get_l3d_file_id = |geometry_id: &str| -> Option<String> {
        geometries.and_then(|g| {
            g.model_geometry
                .iter()
                .find(|mg| mg.id == geometry_id)
                .and_then(|mg| mg.geometry_file_reference.first())
                .map(|gfr| gfr.file_id.clone())
        })
    };

    // Helper to find photometry_id from emitter_id
    let get_photometry_id = |emitter_id: &str| -> Option<String> {
        emitters.and_then(|e| {
            e.emitter
                .iter()
                .find(|em| em.id == emitter_id)
                .and_then(|em| {
                    // Check fixed light emitters first
                    em.fixed_light_emitter
                        .first()
                        .map(|fle| fle.photometry_reference.photometry_id.clone())
                        .or_else(|| {
                            // Then check changeable light emitters
                            em.changeable_light_emitter
                                .first()
                                .map(|cle| cle.photometry_reference.photometry_id.clone())
                        })
                })
        })
    };

    // Helper to find LDT file_id from photometry_id
    let get_ldt_file_id = |photometry_id: &str| -> Option<String> {
        photometries.and_then(|p| {
            p.photometry
                .iter()
                .find(|ph| ph.id == photometry_id)
                .and_then(|ph| ph.photometry_file_reference.as_ref())
                .map(|pfr| pfr.file_id.clone())
        })
    };

    // Iterate through variants
    for variant in &variants.variant {
        // Check for geometry in variant
        if let Some(geometry) = &variant.geometry {
            // Handle model geometry reference
            if let Some(model_geo_ref) = &geometry.model_geometry_reference {
                let geometry_id = &model_geo_ref.geometry_id;

                // Get L3D file info
                let l3d_file_id = get_l3d_file_id(geometry_id);
                let l3d_file_name = l3d_file_id.as_ref().and_then(|id| get_file_name(id));

                // Get LDT file info from emitter references
                let mut ldt_file_id = None;
                let mut ldt_file_name = None;
                let mut emitter_id = None;

                for emitter_ref in &model_geo_ref.emitter_reference {
                    emitter_id = Some(emitter_ref.emitter_id.clone());

                    if let Some(photometry_id) = get_photometry_id(&emitter_ref.emitter_id) {
                        if let Some(file_id) = get_ldt_file_id(&photometry_id) {
                            ldt_file_name = get_file_name(&file_id);
                            ldt_file_id = Some(file_id);
                            break; // Use first found
                        }
                    }
                }

                if let Some(l3d_id) = l3d_file_id {
                    mappings.push(L3dLdtMapping {
                        l3d_file_id: l3d_id,
                        l3d_file_name,
                        ldt_file_id,
                        ldt_file_name,
                        variant_id: variant.id.clone(),
                        emitter_id,
                    });
                }
            }

            // Handle simple geometry reference (has emitter_id directly)
            if let Some(simple_geo_ref) = &geometry.simple_geometry_reference {
                let emitter_id = &simple_geo_ref.emitter_id;

                // Get LDT file info
                if let Some(photometry_id) = get_photometry_id(emitter_id) {
                    if let Some(ldt_file_id) = get_ldt_file_id(&photometry_id) {
                        let ldt_file_name = get_file_name(&ldt_file_id);

                        // Simple geometry doesn't have L3D, but we record the LDT mapping
                        mappings.push(L3dLdtMapping {
                            l3d_file_id: simple_geo_ref.geometry_id.clone(),
                            l3d_file_name: None, // Simple geometry, no L3D file
                            ldt_file_id: Some(ldt_file_id),
                            ldt_file_name,
                            variant_id: variant.id.clone(),
                            emitter_id: Some(emitter_id.clone()),
                        });
                    }
                }
            }
        }
    }

    mappings
}

/// Get all L3D files from a GLDF with their associated LDT data
pub fn get_l3d_files_with_ldt(gldf: &FileBufGldf) -> Vec<L3dWithLdt> {
    let mappings = get_l3d_ldt_mappings(&gldf.gldf);
    let mut results = Vec::new();

    for mapping in mappings {
        // Find L3D content
        let l3d_content = gldf
            .files
            .iter()
            .find(|f| {
                f.file_id.as_ref() == Some(&mapping.l3d_file_id)
                    || f.name.as_ref() == mapping.l3d_file_name.as_ref()
            })
            .and_then(|f| f.content.clone());

        // Find LDT content
        let ldt_content = mapping.ldt_file_id.as_ref().and_then(|ldt_id| {
            gldf.files
                .iter()
                .find(|f| {
                    f.file_id.as_ref() == Some(ldt_id)
                        || f.name.as_ref() == mapping.ldt_file_name.as_ref()
                })
                .and_then(|f| f.content.clone())
        });

        if l3d_content.is_some() {
            results.push(L3dWithLdt {
                l3d_file_name: mapping
                    .l3d_file_name
                    .unwrap_or_else(|| mapping.l3d_file_id.clone()),
                l3d_content,
                ldt_file_name: mapping.ldt_file_name,
                ldt_content,
                variant_id: mapping.variant_id,
            });
        }
    }

    results
}

/// L3D file with its associated LDT content
#[derive(Debug, Clone)]
pub struct L3dWithLdt {
    /// The L3D file name
    pub l3d_file_name: String,
    /// The L3D file content (binary)
    pub l3d_content: Option<Vec<u8>>,
    /// The associated LDT file name
    pub ldt_file_name: Option<String>,
    /// The LDT file content (text)
    pub ldt_content: Option<Vec<u8>>,
    /// The variant ID this comes from
    pub variant_id: String,
}

impl L3dWithLdt {
    /// Get the LDT content as a string (for parsing externally)
    pub fn ldt_as_string(&self) -> Option<String> {
        self.ldt_content
            .as_ref()
            .and_then(|content| std::str::from_utf8(content).ok().map(|s| s.to_string()))
    }

    /// Check if this mapping has both L3D and LDT content
    pub fn has_both(&self) -> bool {
        self.l3d_content.is_some() && self.ldt_content.is_some()
    }

    /// Check if this mapping has L3D content
    pub fn has_l3d(&self) -> bool {
        self.l3d_content.is_some()
    }

    /// Check if this mapping has LDT content
    pub fn has_ldt(&self) -> bool {
        self.ldt_content.is_some()
    }
}

/// Find the first L3D file with LDT from a GLDF
pub fn get_first_l3d_with_ldt(gldf: &FileBufGldf) -> Option<L3dWithLdt> {
    get_l3d_files_with_ldt(gldf).into_iter().next()
}

/// Get per-emitter render data for a specific variant
///
/// This extracts the complete rendering information for each emitter in a variant:
/// - LEO name (Light Emitting Object name from L3D)
/// - Luminous flux in lumens
/// - Color temperature in Kelvin
/// - Associated photometry file
///
/// # Example
///
/// ```ignore
/// let emitter_data = get_variant_emitter_data(&gldf, "variant_1");
/// for emitter in &emitter_data.emitters {
///     println!("{}: {} lm, {} K", emitter.leo_name,
///         emitter.luminous_flux.unwrap_or(0),
///         emitter.color_temperature.unwrap_or(3000));
/// }
/// ```
pub fn get_variant_emitter_data(gldf: &GldfProduct, variant_id: &str) -> VariantEmitterData {
    let mut result = VariantEmitterData {
        variant_id: variant_id.to_string(),
        ..Default::default()
    };

    // Get variants
    let Some(variants) = &gldf.product_definitions.variants else {
        return result;
    };

    // Find the specific variant
    let Some(variant) = variants.variant.iter().find(|v| v.id == variant_id) else {
        return result;
    };

    // Get general definitions for lookups
    let general_defs = &gldf.general_definitions;
    let geometries = general_defs.geometries.as_ref();
    let emitters = general_defs.emitters.as_ref();
    let light_sources = general_defs.light_sources.as_ref();
    let photometries = general_defs.photometries.as_ref();
    let files = &general_defs.files.file;

    // Helper to find file name by file_id
    let get_file_name = |file_id: &str| -> Option<String> {
        files
            .iter()
            .find(|f| f.id == file_id)
            .map(|f| f.file_name.clone())
    };

    // Helper to find L3D file_id from geometry_id
    let get_l3d_file_id = |geometry_id: &str| -> Option<String> {
        geometries.and_then(|g| {
            g.model_geometry
                .iter()
                .find(|mg| mg.id == geometry_id)
                .and_then(|mg| mg.geometry_file_reference.first())
                .map(|gfr| gfr.file_id.clone())
        })
    };

    // Helper to find photometry file info from emitter_id
    let get_photometry_info = |emitter_id: &str| -> (Option<String>, Option<String>) {
        let photometry_id = emitters.and_then(|e| {
            e.emitter
                .iter()
                .find(|em| em.id == emitter_id)
                .and_then(|em| {
                    em.fixed_light_emitter
                        .first()
                        .map(|fle| fle.photometry_reference.photometry_id.clone())
                        .or_else(|| {
                            em.changeable_light_emitter
                                .first()
                                .map(|cle| cle.photometry_reference.photometry_id.clone())
                        })
                })
        });

        if let Some(ref photo_id) = photometry_id {
            let ldt_file_id = photometries.and_then(|p| {
                p.photometry
                    .iter()
                    .find(|ph| ph.id == *photo_id)
                    .and_then(|ph| ph.photometry_file_reference.as_ref())
                    .map(|pfr| pfr.file_id.clone())
            });
            let ldt_file_name = ldt_file_id.as_ref().and_then(|id| get_file_name(id));
            (ldt_file_id, ldt_file_name)
        } else {
            (None, None)
        }
    };

    // Helper to get emitter details (flux, color temp, emergency behavior)
    let get_emitter_details = |emitter_id: &str| -> (Option<i32>, Option<i32>, Option<String>) {
        let emitter = emitters.and_then(|e| e.emitter.iter().find(|em| em.id == emitter_id));

        let Some(emitter) = emitter else {
            return (None, None, None);
        };

        // Get from fixed light emitter
        if let Some(fle) = emitter.fixed_light_emitter.first() {
            let flux = fle.rated_luminous_flux;
            let emergency = fle.emergency_behaviour.clone();

            // Get color temperature from referenced light source
            let color_temp = fle
                .light_source_reference
                .fixed_light_source_id
                .as_ref()
                .and_then(|ls_id| {
                    light_sources.and_then(|lss| {
                        lss.fixed_light_source
                            .iter()
                            .find(|ls| ls.id == *ls_id)
                            .and_then(|ls| ls.color_information.as_ref())
                            .and_then(|ci| ci.correlated_color_temperature)
                    })
                });

            return (flux, color_temp, emergency);
        }

        // Get from changeable light emitter
        if let Some(cle) = emitter.changeable_light_emitter.first() {
            let emergency = cle.emergency_behaviour.clone();
            // Changeable emitters don't have rated_luminous_flux directly
            return (None, None, emergency);
        }

        (None, None, None)
    };

    // Process the variant's geometry
    if let Some(geometry) = &variant.geometry {
        if let Some(model_geo_ref) = &geometry.model_geometry_reference {
            let geometry_id = &model_geo_ref.geometry_id;

            // Get L3D file info
            result.l3d_file_id = get_l3d_file_id(geometry_id);
            result.l3d_file_name = result.l3d_file_id.as_ref().and_then(|id| get_file_name(id));

            // Process each emitter reference
            for emitter_ref in &model_geo_ref.emitter_reference {
                let (ldt_file_id, ldt_file_name) = get_photometry_info(&emitter_ref.emitter_id);
                let (flux, color_temp, emergency) = get_emitter_details(&emitter_ref.emitter_id);

                result.emitters.push(EmitterRenderData {
                    leo_name: emitter_ref.emitter_object_external_name.clone(),
                    emitter_id: emitter_ref.emitter_id.clone(),
                    luminous_flux: flux,
                    color_temperature: color_temp,
                    ldt_file_id,
                    ldt_file_name,
                    emergency_behavior: emergency,
                });
            }
        }
    }

    // Extract mounting information from variant
    result.mounting = extract_mounting_info(variant);

    result
}

/// Extract mounting information from a GLDF variant
fn extract_mounting_info(variant: &crate::gldf::product_definitions::Variant) -> MountingInfo {
    let mut info = MountingInfo::default();

    let Some(mountings) = &variant.mountings else {
        return info;
    };

    // Check ceiling mounting
    if let Some(ceiling) = &mountings.ceiling {
        info.mounting_type = Some(MountingType::Ceiling);

        // Check for recessed
        if let Some(recessed) = &ceiling.recessed {
            info.recessed_depth_mm = Some(recessed.recessed_depth);
        }

        // Check for pendant
        if let Some(pendant) = &ceiling.pendant {
            info.pendant_length_mm = Some(pendant.pendant_length);
        }

        // Check for surface mounted
        if ceiling.surface_mounted.is_some() {
            info.is_surface_mounted = true;
        }
    }

    // Check wall mounting (takes precedence if both exist - unusual)
    if let Some(wall) = &mountings.wall {
        info.mounting_type = Some(MountingType::Wall);
        info.mounting_height_mm = Some(wall.mounting_height);

        if let Some(recessed) = &wall.recessed {
            info.recessed_depth_mm = Some(recessed.recessed_depth);
        }

        if wall.surface_mounted.is_some() {
            info.is_surface_mounted = true;
        }
    }

    // Check ground mounting
    if let Some(ground) = &mountings.ground {
        info.mounting_type = Some(MountingType::Ground);

        // Pole top
        if let Some(pole_top) = &ground.pole_top {
            info.pole_height_mm = pole_top.get_pole_height();
        }

        // Pole integrated
        if let Some(pole_integrated) = &ground.pole_integrated {
            if info.pole_height_mm.is_none() {
                info.pole_height_mm = pole_integrated.get_pole_height();
            }
        }

        // Free standing
        if ground.free_standing.is_some() {
            info.is_free_standing = true;
        }

        // Surface mounted on ground
        if ground.surface_mounted.is_some() {
            info.is_surface_mounted = true;
        }

        // Recessed in ground
        if let Some(recessed) = &ground.recessed {
            info.recessed_depth_mm = Some(recessed.recessed_depth);
        }
    }

    // Check working plane
    if let Some(working_plane) = &mountings.working_plane {
        info.mounting_type = Some(MountingType::WorkingPlane);

        if working_plane.free_standing.is_some() {
            info.is_free_standing = true;
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_creation() {
        // Test would require a sample GLDF file
        let mapping = L3dLdtMapping {
            l3d_file_id: "geo_001".to_string(),
            l3d_file_name: Some("luminaire.l3d".to_string()),
            ldt_file_id: Some("photo_001".to_string()),
            ldt_file_name: Some("luminaire.ldt".to_string()),
            variant_id: "variant_1".to_string(),
            emitter_id: Some("emitter_1".to_string()),
        };

        assert_eq!(mapping.l3d_file_id, "geo_001");
        assert_eq!(mapping.ldt_file_id, Some("photo_001".to_string()));
    }
}
