//! GLDF Generator from imported IFC data
//!
//! Converts ImportedLuminaire to a complete GLDF package (ZIP file).

use super::ifc_import::{ImportedGeometry, ImportedLuminaire, ImportedVariant};
use anyhow::Result;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Generate GLDF package from imported luminaire data
pub struct GldfGenerator {
    luminaire: ImportedLuminaire,
}

impl GldfGenerator {
    /// Create generator from imported luminaire
    pub fn new(luminaire: ImportedLuminaire) -> Self {
        Self { luminaire }
    }

    /// Generate GLDF as bytes (ZIP archive)
    pub fn generate(&self) -> Result<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Generate product.xml
        let product_xml = self.generate_product_xml()?;
        zip.start_file("product.xml", options)?;
        zip.write_all(product_xml.as_bytes())?;

        // Generate photometry files (LDT) with deduplication
        // Multiple light sources can share the same photometry if their distribution is identical
        // Preserve original filenames when available (from IFC roundtrip)
        let mut written_distributions: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut ldt_counter = 0;

        for variant in self.luminaire.variants.iter() {
            for ls in variant.light_sources.iter() {
                // Create a hash key from distribution data to detect duplicates
                let dist_key = if let Some(ref dist) = ls.distribution {
                    // Hash based on distribution type and first few data points
                    let sample: String = dist
                        .data
                        .iter()
                        .take(3)
                        .flat_map(|p| p.intensities.iter().take(5))
                        .map(|(a, i)| format!("{:.2}:{:.2}", a, i))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("dist:{}", sample)
                } else {
                    // No distribution - use flux as key for isotropic fallback
                    format!("iso:{:.0}", ls.luminous_flux.unwrap_or(1000.0))
                };

                // Only write if we haven't seen this distribution before
                if let std::collections::hash_map::Entry::Vacant(e) =
                    written_distributions.entry(dist_key)
                {
                    ldt_counter += 1;
                    // Use original filename if available, otherwise generate one
                    let filename = ls
                        .photometry_filename
                        .as_ref()
                        .filter(|f| f.ends_with(".ldt") || f.ends_with(".LDT"))
                        .cloned()
                        .unwrap_or_else(|| format!("photometry_{}.ldt", ldt_counter));

                    // Use raw LDT content if available (exact roundtrip), otherwise generate
                    let ldt = if let Some(ref meta) = ls.ldt_metadata {
                        if let Some(ref raw) = meta.raw_content {
                            // Exact roundtrip - use original file content
                            raw.clone()
                        } else if let Some(ref dist) = ls.distribution {
                            self.generate_eulumdat(variant, ls, dist)
                        } else {
                            self.generate_isotropic_ldt(variant, ls)
                        }
                    } else if let Some(ref dist) = ls.distribution {
                        self.generate_eulumdat(variant, ls, dist)
                    } else {
                        self.generate_isotropic_ldt(variant, ls)
                    };
                    zip.start_file(format!("ldc/{}", filename), options)?;
                    zip.write_all(ldt.as_bytes())?;
                    e.insert(filename);
                }
            }
        }

        // Generate ONE shared L3D geometry file (from first variant with geometry)
        // All variants reference this same file, similar to IFCREPRESENTATIONMAP in IFC
        if let Some((variant, geom)) = self
            .luminaire
            .variants
            .iter()
            .find_map(|v| v.geometry.as_ref().map(|g| (v, g)))
        {
            let l3d = self.generate_l3d(variant, geom)?;
            zip.start_file("geo/luminaire.l3d", options)?;
            zip.write_all(&l3d)?;
        }

        // Write embedded files (images, sensors) preserved from IFC roundtrip
        for embedded in &self.luminaire.embedded_files {
            let path = match embedded.file_type.as_str() {
                "image" => format!("image/{}", embedded.filename),
                "sensor" => format!("sensor/{}", embedded.filename),
                _ => format!("other/{}", embedded.filename),
            };
            zip.start_file(&path, options)?;
            zip.write_all(&embedded.content)?;
        }

        zip.finish()?;
        Ok(buffer.into_inner())
    }

    /// Generate product.xml content
    fn generate_product_xml(&self) -> Result<String> {
        let mut xml = String::new();

        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(r#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" "#);
        xml.push_str(
            r#"xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd">"#,
        );
        xml.push('\n');

        // Header
        xml.push_str("  <Header>\n");
        xml.push_str(&format!(
            "    <Manufacturer>{}</Manufacturer>\n",
            Self::escape_xml(self.luminaire.manufacturer.as_deref().unwrap_or("Unknown"))
        ));
        xml.push_str("    <FormatVersion major=\"1\" minor=\"0\" pre-release=\"3\" />\n");
        xml.push_str("    <CreatedWithApplication>gldf-rs IFC Import</CreatedWithApplication>\n");
        xml.push_str(&format!(
            "    <GldfCreationTimeCode>{}</GldfCreationTimeCode>\n",
            Self::current_timestamp()
        ));
        xml.push_str(&format!(
            "    <UniqueGldfId>{}</UniqueGldfId>\n",
            Self::generate_uuid()
        ));
        xml.push_str("  </Header>\n");

        // GeneralDefinitions
        xml.push_str("  <GeneralDefinitions>\n");

        // Build deduplication map for photometry files
        // Key: distribution signature, Value: (file_id, filename)
        // Preserve original filenames when available (from IFC roundtrip)
        let mut dist_to_file: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();
        let mut file_counter = 0;

        // Also build a map from (variant_idx, ls_idx) -> file_id for photometry references
        let mut ls_to_file_id: std::collections::HashMap<(usize, usize), String> =
            std::collections::HashMap::new();

        for (vi, variant) in self.luminaire.variants.iter().enumerate() {
            for (li, ls) in variant.light_sources.iter().enumerate() {
                // Create distribution signature
                let dist_key = if let Some(ref dist) = ls.distribution {
                    let sample: String = dist
                        .data
                        .iter()
                        .take(3)
                        .flat_map(|p| p.intensities.iter().take(5))
                        .map(|(a, i)| format!("{:.2}:{:.2}", a, i))
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("dist:{}", sample)
                } else {
                    format!("iso:{:.0}", ls.luminous_flux.unwrap_or(1000.0))
                };

                let file_id = if let Some((fid, _)) = dist_to_file.get(&dist_key) {
                    fid.clone()
                } else {
                    file_counter += 1;
                    let fid = format!("ldt_{}", file_counter);
                    // Use original filename if available, otherwise generate one
                    let filename = ls
                        .photometry_filename
                        .as_ref()
                        .filter(|f| f.ends_with(".ldt") || f.ends_with(".LDT"))
                        .cloned()
                        .unwrap_or_else(|| format!("photometry_{}.ldt", file_counter));
                    dist_to_file.insert(dist_key.clone(), (fid.clone(), filename));
                    fid
                };
                ls_to_file_id.insert((vi, li), file_id);
            }
        }

        // Files
        xml.push_str("    <Files>\n");
        let has_geometry = self.luminaire.variants.iter().any(|v| v.geometry.is_some());

        // Write unique LDT file references
        for (file_id, filename) in dist_to_file.values() {
            xml.push_str(&format!(
                "      <File id=\"{}\" contentType=\"ldc/eulumdat\" type=\"localFileName\">{}</File>\n",
                file_id, filename
            ));
        }

        // Single shared geometry file for all variants
        if has_geometry {
            xml.push_str("      <File id=\"geo_shared\" contentType=\"geo/l3d\" type=\"localFileName\">luminaire.l3d</File>\n");
        }

        // Embedded files (images, sensors) preserved from IFC roundtrip
        for embedded in &self.luminaire.embedded_files {
            let file_id = match embedded.file_type.as_str() {
                "image" => "productImage",
                "sensor" => "sensorFile",
                _ => continue,
            };
            xml.push_str(&format!(
                "      <File id=\"{}\" contentType=\"{}\" type=\"localFileName\">{}</File>\n",
                file_id,
                Self::escape_xml(&embedded.content_type),
                Self::escape_xml(&embedded.filename)
            ));
        }

        xml.push_str("    </Files>\n");

        // Photometries - one per unique distribution
        xml.push_str("    <Photometries>\n");
        let mut written_photometries: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut photometry_ids: Vec<(usize, usize, String)> = Vec::new();

        for (vi, variant) in self.luminaire.variants.iter().enumerate() {
            for (li, ls) in variant.light_sources.iter().enumerate() {
                let file_id = ls_to_file_id.get(&(vi, li)).unwrap();
                let phot_id = format!("phot_{}", file_id.trim_start_matches("ldt_"));
                photometry_ids.push((vi, li, phot_id.clone()));

                // Only write photometry definition once per file
                if !written_photometries.contains(file_id) {
                    xml.push_str(&format!("      <Photometry id=\"{}\">\n", phot_id));
                    xml.push_str(&format!(
                        "        <PhotometryFileReference fileId=\"{}\" />\n",
                        file_id
                    ));
                    // Add DescriptivePhotometry with efficacy if available
                    let efficacy = variant.properties.efficacy.or_else(|| {
                        // Calculate from flux and power if not directly available
                        let flux = ls.luminous_flux.or(variant.properties.luminous_flux);
                        let power = variant
                            .properties
                            .power
                            .or(variant.properties.total_wattage);
                        match (flux, power) {
                            (Some(f), Some(p)) if p > 0.0 => Some(f / p),
                            _ => None,
                        }
                    });
                    if let Some(eff) = efficacy {
                        xml.push_str("        <DescriptivePhotometry>\n");
                        xml.push_str(&format!(
                            "          <LuminousEfficacy>{:.0}</LuminousEfficacy>\n",
                            eff
                        ));
                        xml.push_str("        </DescriptivePhotometry>\n");
                    }
                    xml.push_str("      </Photometry>\n");
                    written_photometries.insert(file_id.clone());
                }
            }
        }
        xml.push_str("    </Photometries>\n");

        // Light Sources
        xml.push_str("    <LightSources>\n");
        let mut ls_counter = 1;
        let mut light_source_ids: Vec<(usize, usize, String)> = Vec::new();

        for (vi, variant) in self.luminaire.variants.iter().enumerate() {
            for (li, ls) in variant.light_sources.iter().enumerate() {
                let ls_id = format!("ls_{}", ls_counter);
                light_source_ids.push((vi, li, ls_id.clone()));

                let cct = ls
                    .color_temperature
                    .or(variant.properties.color_temperature)
                    .unwrap_or(4000.0) as i32;
                let cri = variant.properties.cri.unwrap_or(80);
                let flux = ls
                    .luminous_flux
                    .or(variant.properties.luminous_flux)
                    .unwrap_or(1000.0);
                let power = variant
                    .properties
                    .power
                    .or(variant.properties.total_wattage)
                    .unwrap_or(10.0);

                xml.push_str(&format!("      <FixedLightSource id=\"{}\">\n", ls_id));
                xml.push_str(&format!(
                    "        <Name><Locale language=\"en\">{}</Locale></Name>\n",
                    Self::escape_xml(&ls.name)
                ));
                xml.push_str(&format!(
                    "        <RatedInputPower>{:.1}</RatedInputPower>\n",
                    power
                ));
                xml.push_str(&format!(
                    "        <RatedLuminousFlux>{:.0}</RatedLuminousFlux>\n",
                    flux
                ));
                xml.push_str("        <ColorInformation>\n");
                xml.push_str(&format!(
                    "          <ColorRenderingIndex>{}</ColorRenderingIndex>\n",
                    cri
                ));
                xml.push_str(&format!(
                    "          <CorrelatedColorTemperature>{}</CorrelatedColorTemperature>\n",
                    cct
                ));
                xml.push_str("        </ColorInformation>\n");
                xml.push_str("        <LightSourceType>LED</LightSourceType>\n");
                xml.push_str("      </FixedLightSource>\n");

                ls_counter += 1;
            }
        }
        xml.push_str("    </LightSources>\n");

        // Emitters - reference photometries and light sources
        xml.push_str("    <Emitters>\n");
        let mut emitter_counter = 1;
        let mut emitter_ids: Vec<(usize, usize, String)> = Vec::new();

        for (vi, variant) in self.luminaire.variants.iter().enumerate() {
            for (li, ls) in variant.light_sources.iter().enumerate() {
                let em_id = format!("em_{}", emitter_counter);
                emitter_ids.push((vi, li, em_id.clone()));

                let file_id = ls_to_file_id.get(&(vi, li)).unwrap();
                let phot_id = format!("phot_{}", file_id.trim_start_matches("ldt_"));
                let ls_id = light_source_ids
                    .iter()
                    .find(|(v, l, _)| *v == vi && *l == li)
                    .map(|(_, _, id)| id.clone())
                    .unwrap();

                let flux = ls
                    .luminous_flux
                    .or(variant.properties.luminous_flux)
                    .unwrap_or(1000.0);

                xml.push_str(&format!("      <Emitter id=\"{}\">\n", em_id));
                xml.push_str("        <FixedLightEmitter emergencyBehaviour=\"None\">\n");
                xml.push_str(&format!(
                    "          <PhotometryReference photometryId=\"{}\" />\n",
                    phot_id
                ));
                xml.push_str(&format!(
                    "          <LightSourceReference fixedLightSourceId=\"{}\" />\n",
                    ls_id
                ));
                xml.push_str(&format!(
                    "          <RatedLuminousFlux>{:.0}</RatedLuminousFlux>\n",
                    flux
                ));
                xml.push_str("        </FixedLightEmitter>\n");
                xml.push_str("      </Emitter>\n");

                emitter_counter += 1;
            }
        }
        xml.push_str("    </Emitters>\n");

        // Geometries - reference the shared L3D file
        if has_geometry {
            xml.push_str("    <Geometries>\n");
            xml.push_str("      <ModelGeometry id=\"geom_shared\">\n");
            xml.push_str("        <GeometryFileReference fileId=\"geo_shared\" />\n");
            xml.push_str("      </ModelGeometry>\n");
            xml.push_str("    </Geometries>\n");
        }

        // Sensors - if sensor file is embedded
        let has_sensor = self
            .luminaire
            .embedded_files
            .iter()
            .any(|f| f.file_type == "sensor");
        if has_sensor {
            xml.push_str("    <Sensors>\n");
            xml.push_str("      <Sensor id=\"sensor1\">\n");
            xml.push_str("        <SensorFileReference fileId=\"sensorFile\" />\n");
            xml.push_str("      </Sensor>\n");
            xml.push_str("    </Sensors>\n");
        }

        xml.push_str("  </GeneralDefinitions>\n");

        // ProductDefinitions
        xml.push_str("  <ProductDefinitions>\n");
        xml.push_str("    <ProductMetaData>\n");
        xml.push_str(&format!(
            "      <ProductNumber><Locale language=\"en\">{}</Locale></ProductNumber>\n",
            Self::escape_xml(&self.luminaire.name)
        ));
        xml.push_str(&format!(
            "      <Name><Locale language=\"en\">{}</Locale></Name>\n",
            Self::escape_xml(&self.luminaire.name)
        ));

        // Product-level DescriptiveAttributes (IP code, safety class, median useful life)
        let da = &self.luminaire.descriptive_attributes;
        let has_electrical = da.electrical_safety_class.is_some() || da.ip_code.is_some();
        let has_ops = da.median_useful_life.is_some();

        if has_electrical || has_ops {
            xml.push_str("      <DescriptiveAttributes>\n");

            if has_electrical {
                xml.push_str("        <Electrical>\n");
                if let Some(ref sc) = da.electrical_safety_class {
                    xml.push_str(&format!(
                        "          <ElectricalSafetyClass>{}</ElectricalSafetyClass>\n",
                        Self::escape_xml(sc)
                    ));
                }
                if let Some(ref ip) = da.ip_code {
                    xml.push_str(&format!(
                        "          <IngressProtectionIPCode>{}</IngressProtectionIPCode>\n",
                        Self::escape_xml(ip)
                    ));
                }
                xml.push_str("        </Electrical>\n");
            }

            if has_ops {
                xml.push_str("        <OperationsAndMaintenance>\n");
                if let Some(ref mul) = da.median_useful_life {
                    xml.push_str("          <MedianUsefulLifeTimes>\n");
                    xml.push_str(&format!(
                        "            <MedianUsefulLife>{}</MedianUsefulLife>\n",
                        Self::escape_xml(mul)
                    ));
                    xml.push_str("          </MedianUsefulLifeTimes>\n");
                }
                xml.push_str("        </OperationsAndMaintenance>\n");
            }

            xml.push_str("      </DescriptiveAttributes>\n");
        }

        // Pictures - if image file is embedded
        let has_image = self
            .luminaire
            .embedded_files
            .iter()
            .any(|f| f.file_type == "image");
        if has_image {
            xml.push_str("      <Pictures>\n");
            xml.push_str(
                "        <Image fileId=\"productImage\" imageType=\"Product Picture\" />\n",
            );
            xml.push_str("      </Pictures>\n");
        }

        xml.push_str("    </ProductMetaData>\n");

        // Variants
        xml.push_str("    <Variants>\n");
        for (vi, variant) in self.luminaire.variants.iter().enumerate() {
            xml.push_str(&format!("      <Variant id=\"var_{}\">\n", vi + 1));
            xml.push_str(&format!(
                "        <Name><Locale language=\"en\">{}</Locale></Name>\n",
                Self::escape_xml(&variant.name)
            ));

            // Variant-level DescriptiveAttributes (only weight and rated voltage)
            // Note: IP code, safety class, and median useful life are at ProductMetaData level
            let has_electrical = variant.properties.rated_voltage.is_some();
            let has_mechanical = variant.properties.weight.is_some();

            if has_electrical || has_mechanical {
                xml.push_str("        <DescriptiveAttributes>\n");

                // Electrical (only rated voltage at variant level)
                if has_electrical {
                    xml.push_str("          <Electrical>\n");
                    if let Some(voltage) = variant.properties.rated_voltage {
                        xml.push_str(&format!(
                            "            <RatedVoltage>{:.0}</RatedVoltage>\n",
                            voltage
                        ));
                    }
                    xml.push_str("          </Electrical>\n");
                }

                // Mechanical (Weight)
                if has_mechanical {
                    xml.push_str("          <Mechanical>\n");
                    if let Some(weight) = variant.properties.weight {
                        xml.push_str(&format!("            <Weight>{:.3}</Weight>\n", weight));
                    }
                    xml.push_str("          </Mechanical>\n");
                }

                xml.push_str("        </DescriptiveAttributes>\n");
            }

            // Mountings
            if variant.properties.gldf_mounting_type.is_some()
                || variant.properties.recessed_depth.is_some()
            {
                xml.push_str("        <Mountings>\n");
                let mounting = variant
                    .properties
                    .gldf_mounting_type
                    .as_deref()
                    .unwrap_or("Ceiling");
                match mounting.to_lowercase().as_str() {
                    "ceiling" => {
                        xml.push_str("          <Ceiling>\n");
                        if let Some(depth) = variant.properties.recessed_depth {
                            xml.push_str(&format!(
                                "            <Recessed recessedDepth=\"{:.0}\" />\n",
                                depth
                            ));
                        } else {
                            xml.push_str("            <SurfaceMounted />\n");
                        }
                        xml.push_str("          </Ceiling>\n");
                    }
                    "wall" => {
                        xml.push_str("          <Wall>\n");
                        xml.push_str("            <SurfaceMounted />\n");
                        xml.push_str("          </Wall>\n");
                    }
                    "ground" => {
                        xml.push_str("          <Ground>\n");
                        xml.push_str("            <SurfaceMounted />\n");
                        xml.push_str("          </Ground>\n");
                    }
                    _ => {
                        xml.push_str("          <Ceiling>\n");
                        xml.push_str("            <SurfaceMounted />\n");
                        xml.push_str("          </Ceiling>\n");
                    }
                }
                xml.push_str("        </Mountings>\n");
            }

            // Geometry reference (shared)
            if has_geometry {
                xml.push_str("        <Geometry>\n");
                xml.push_str("          <ModelGeometryReference geometryId=\"geom_shared\" />\n");
                xml.push_str("          <EmitterReferences>\n");
                for (li, _) in variant.light_sources.iter().enumerate() {
                    let em_id = emitter_ids
                        .iter()
                        .find(|(v, l, _)| *v == vi && *l == li)
                        .map(|(_, _, id)| id.clone())
                        .unwrap_or_else(|| format!("em_{}", vi * 10 + li + 1));
                    xml.push_str(&format!(
                        "            <EmitterReference emitterId=\"{}\">\n              <EmitterObjectExternalName>LEO{}</EmitterObjectExternalName>\n            </EmitterReference>\n",
                        em_id, li
                    ));
                }
                xml.push_str("          </EmitterReferences>\n");
                xml.push_str("        </Geometry>\n");
            }

            xml.push_str("      </Variant>\n");
        }
        xml.push_str("    </Variants>\n");
        xml.push_str("  </ProductDefinitions>\n");
        xml.push_str("</Root>\n");

        Ok(xml)
    }

    /// Generate EULUMDAT file from distribution data
    ///
    /// EULUMDAT format specification - same structure as generate_isotropic_ldt
    /// Uses ldt_metadata if available to preserve original LDT file structure
    fn generate_eulumdat(
        &self,
        variant: &ImportedVariant,
        ls: &super::ifc_import::ImportedLightSource,
        dist: &super::ifc_import::ImportedDistribution,
    ) -> String {
        let mut ldt = String::new();

        // Use preserved metadata if available, otherwise calculate from distribution
        let meta = ls.ldt_metadata.as_ref();

        let flux = meta
            .and_then(|m| m.total_flux)
            .or(ls.luminous_flux)
            .or(variant.properties.luminous_flux)
            .unwrap_or(1000.0);
        let cct = ls
            .color_temperature
            .or(variant.properties.color_temperature)
            .unwrap_or(4000.0);
        let cri = variant.properties.cri.unwrap_or(80);
        let power = variant
            .properties
            .power
            .or(variant.properties.total_wattage)
            .unwrap_or(10.0);

        // Use preserved Mc/Ng if available, otherwise use distribution data size
        let num_c = meta
            .map(|m| m.num_c_planes as usize)
            .unwrap_or_else(|| dist.data.len().max(1));
        let num_gamma = meta
            .map(|m| m.num_g_angles as usize)
            .unwrap_or_else(|| dist.data.first().map(|p| p.intensities.len()).unwrap_or(19));

        // Use preserved symmetry if available
        // Isym: 0=none, 1=vertical axis, 2=C0-C180, 3=C90-C270, 4=both planes
        let isym = meta.map(|m| m.symmetry).unwrap_or_else(|| {
            // Fallback: infer symmetry from distribution data
            if dist.data.len() > 1 {
                0 // No symmetry - multiple C-planes with data
            } else {
                1 // Vertical axis symmetry - single C-plane
            }
        });

        // Use preserved Dc/Dg if available
        let dc = meta.map(|m| m.dc).unwrap_or_else(|| {
            if dist.data.len() > 1 {
                let c_angles: Vec<f64> = dist.data.iter().map(|p| p.main_angle).collect();
                let first_step = c_angles.get(1).unwrap_or(&0.0) - c_angles.first().unwrap_or(&0.0);
                let evenly_spaced = c_angles
                    .windows(2)
                    .all(|w| (w[1] - w[0] - first_step).abs() < 0.01);
                if evenly_spaced && first_step > 0.0 {
                    first_step
                } else {
                    0.0
                }
            } else {
                0.0
            }
        });

        let dg = meta.map(|m| m.dg).unwrap_or_else(|| {
            if let Some(first_plane) = dist.data.first() {
                if first_plane.intensities.len() > 1 {
                    let g_angles: Vec<f64> =
                        first_plane.intensities.iter().map(|(a, _)| *a).collect();
                    let first_step =
                        g_angles.get(1).unwrap_or(&0.0) - g_angles.first().unwrap_or(&0.0);
                    let evenly_spaced = g_angles
                        .windows(2)
                        .all(|w| (w[1] - w[0] - first_step).abs() < 0.01);
                    if evenly_spaced && first_step > 0.0 {
                        first_step
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        });

        // Use preserved DR values if available
        let dr = meta
            .and_then(|m| m.dr)
            .unwrap_or([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

        // Use preserved luminaire name if available
        let luminaire_name = meta
            .and_then(|m| m.luminaire_name.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(&variant.name);

        // Line 1: Company identification
        ldt.push_str(&format!(
            "{}\n",
            self.luminaire.manufacturer.as_deref().unwrap_or("Unknown")
        ));
        // Line 2: Ityp - Type indicator (3 = point source with any symmetry)
        ldt.push_str("3\n");
        // Line 3: Isym - Symmetry indicator
        ldt.push_str(&format!("{}\n", isym));
        // Line 4: Mc - Number of C-planes
        ldt.push_str(&format!("{}\n", num_c));
        // Line 5: Dc - Distance between C-planes (0 = from list, >0 = fixed step)
        if dc > 0.0 {
            ldt.push_str(&format!("{:.1}\n", dc));
        } else {
            ldt.push_str("0\n");
        }
        // Line 6: Ng - Number of luminous intensities in each C-plane
        ldt.push_str(&format!("{}\n", num_gamma));
        // Line 7: Dg - Distance between luminous intensities (0 = from list, >0 = fixed step)
        if dg > 0.0 {
            ldt.push_str(&format!("{:.1}\n", dg));
        } else {
            ldt.push_str("0\n");
        }
        // Line 8: Measurement report number
        ldt.push_str("1\n");
        // Line 9: Luminaire name
        ldt.push_str(&format!("{}\n", Self::escape_ldt(luminaire_name)));
        // Line 10: Luminaire number
        ldt.push_str(&format!("{}\n", Self::escape_ldt(luminaire_name)));
        // Line 11: File name
        ldt.push_str(&format!("{}.ldt\n", Self::escape_ldt(luminaire_name)));
        // Line 12: Date/user
        ldt.push_str("gldf-rs\n");
        // Line 13: Length/diameter of luminaire (mm)
        ldt.push_str("100\n");
        // Line 14: Width of luminaire (mm)
        ldt.push_str("100\n");
        // Line 15: Height of luminaire (mm)
        ldt.push_str("50\n");
        // Line 16: Length/diameter of luminous area (mm)
        ldt.push_str("80\n");
        // Line 17: Width of luminous area (mm)
        ldt.push_str("80\n");
        // Line 18: Height of luminous area C0-plane (mm)
        ldt.push_str("0\n");
        // Line 19: Height of luminous area C90-plane (mm)
        ldt.push_str("0\n");
        // Line 20: Height of luminous area C180-plane (mm)
        ldt.push_str("0\n");
        // Line 21: Height of luminous area C270-plane (mm)
        ldt.push_str("0\n");
        // Line 22: DFF - Downward flux fraction (%)
        ldt.push_str("100\n");
        // Line 23: LORL - Light output ratio luminaire (%)
        ldt.push_str("100\n");
        // Line 24: Conversion factor for luminous intensities
        ldt.push_str("1.0\n");
        // Line 25: Tilt of luminaire during measurement
        ldt.push_str("0\n");
        // Line 26: Number of standard sets of lamps
        ldt.push_str("1\n");
        // Line 26a: Number of lamps
        ldt.push_str("-1\n");
        // Line 26b: Type of lamps
        ldt.push_str(&format!("LED {}K CRI {}\n", cct as i32, cri));
        // Line 26c: Total luminous flux of lamps (lm)
        ldt.push_str(&format!("{:.0}\n", flux));
        // Line 26d: Color temperature
        ldt.push_str(&format!("{}\n", cct as i32));
        // Line 26e: Color rendering index
        ldt.push_str(&format!("{}\n", cri));
        // Line 26f: Wattage including ballast (W)
        ldt.push_str(&format!("{:.1}\n", power));

        // Direct ratios DR1-DR10 (use preserved values if available)
        for &dr_val in &dr {
            ldt.push_str(&format!("{:.5}\n", dr_val));
        }

        // C-plane angles - write all Mc angles (calculated from Dc or from data)
        // For symmetric files, Mc can be larger than dist.data.len()
        if dc > 0.0 {
            // Generate angles from Dc step
            for i in 0..num_c {
                let angle = (i as f64) * dc;
                ldt.push_str(&format!("{:.1}\n", angle));
            }
        } else {
            // Use angles from distribution data
            for plane in &dist.data {
                ldt.push_str(&format!("{:.1}\n", plane.main_angle));
            }
        }

        // Gamma angles - write all Ng angles (calculated from Dg or from data)
        if dg > 0.0 {
            // Generate angles from Dg step
            for i in 0..num_gamma {
                let angle = (i as f64) * dg;
                ldt.push_str(&format!("{:.1}\n", angle));
            }
        } else if let Some(first_plane) = dist.data.first() {
            // Use angles from distribution data
            for (angle, _) in &first_plane.intensities {
                ldt.push_str(&format!("{:.1}\n", angle));
            }
        }

        // Intensity values - write the STORED data (not expanded)
        // For symmetric files, only the first quadrant is stored
        for plane in &dist.data {
            for (_, intensity) in &plane.intensities {
                ldt.push_str(&format!("{:.2}\n", intensity));
            }
        }

        // EULUMDAT format requires CRLF line endings (Windows-style)
        ldt.replace('\n', "\r\n")
    }

    /// Generate a simple isotropic EULUMDAT file when no distribution data is available
    ///
    /// EULUMDAT format specification:
    /// Lines 1-10: Header info
    /// Lines 11-22: Luminaire dimensions and parameters (numeric)
    /// Lines 23-26: Lamp data
    /// Lines 27+: Lamp description, then angles and intensities
    fn generate_isotropic_ldt(
        &self,
        variant: &ImportedVariant,
        ls: &super::ifc_import::ImportedLightSource,
    ) -> String {
        let mut ldt = String::new();

        let flux = ls
            .luminous_flux
            .or(variant.properties.luminous_flux)
            .unwrap_or(1000.0);
        let cct = ls
            .color_temperature
            .or(variant.properties.color_temperature)
            .unwrap_or(4000.0);
        let cri = variant.properties.cri.unwrap_or(80);
        let power = variant
            .properties
            .power
            .or(variant.properties.total_wattage)
            .unwrap_or(10.0);

        // Line 1: Company identification/database/version/format identification
        ldt.push_str(&format!(
            "{}\n",
            self.luminaire.manufacturer.as_deref().unwrap_or("Unknown")
        ));
        // Line 2: Ityp - Type indicator (1=point source with symmetry about vertical axis)
        ldt.push_str("1\n");
        // Line 3: Isym - Symmetry indicator (1=symmetry about vertical axis)
        ldt.push_str("1\n");
        // Line 4: Mc - Number of C-planes between 0 and 360 degrees
        ldt.push_str("1\n");
        // Line 5: Dc - Distance between C-planes (0 = angles given in line after Dg)
        ldt.push_str("0\n");
        // Line 6: Ng - Number of luminous intensities in each C-plane
        ldt.push_str("19\n");
        // Line 7: Dg - Distance between luminous intensities per C-plane (0 = angles given)
        ldt.push_str("10\n");
        // Line 8: Measurement report number
        ldt.push_str("1\n");
        // Line 9: Luminaire name
        ldt.push_str(&format!("{}\n", Self::escape_ldt(&variant.name)));
        // Line 10: Luminaire number
        ldt.push_str(&format!("{}\n", Self::escape_ldt(&variant.name)));
        // Line 11: File name
        ldt.push_str(&format!("{}.ldt\n", Self::escape_ldt(&variant.name)));
        // Line 12: Date/user
        ldt.push_str("gldf-rs\n");
        // Line 13: Length/diameter of luminaire (mm)
        ldt.push_str("100\n");
        // Line 14: Width of luminaire (mm) (0 for circular)
        ldt.push_str("100\n");
        // Line 15: Height of luminaire (mm)
        ldt.push_str("50\n");
        // Line 16: Length/diameter of luminous area (mm)
        ldt.push_str("80\n");
        // Line 17: Width of luminous area (mm) (0 for circular)
        ldt.push_str("80\n");
        // Line 18: Height of luminous area C0-plane (mm)
        ldt.push_str("0\n");
        // Line 19: Height of luminous area C90-plane (mm)
        ldt.push_str("0\n");
        // Line 20: Height of luminous area C180-plane (mm)
        ldt.push_str("0\n");
        // Line 21: Height of luminous area C270-plane (mm)
        ldt.push_str("0\n");
        // Line 22: DFF - Downward flux fraction (%)
        ldt.push_str("100\n");
        // Line 23: LORL - Light output ratio luminaire (%)
        ldt.push_str("100\n");
        // Line 24: Conversion factor for luminous intensities
        ldt.push_str("1.0\n");
        // Line 25: Tilt of luminaire during measurement
        ldt.push_str("0\n");
        // Line 26: Number of standard sets of lamps (n)
        ldt.push_str("1\n");
        // For each lamp set (we have 1):
        // Line 26a: Number of lamps
        ldt.push_str("-1\n");
        // Line 26b: Type of lamps
        ldt.push_str(&format!("LED {}K CRI {}\n", cct as i32, cri));
        // Line 26c: Total luminous flux of lamps (lm)
        ldt.push_str(&format!("{:.0}\n", flux));
        // Line 26d: Color appearance / color temperature
        ldt.push_str(&format!("{}\n", cct as i32));
        // Line 26e: Color rendering index
        ldt.push_str(&format!("{}\n", cri));
        // Line 26f: Wattage including ballast (W)
        ldt.push_str(&format!("{:.1}\n", power));

        // Direct ratios for 10 zones (lines 27-36 in some formats, after lamp data)
        // These are ratios DR1-DR10 for room index calculation
        for _ in 0..10 {
            ldt.push_str("1.0\n");
        }

        // C-plane angles (since Dc=0, we provide angles)
        ldt.push_str("0\n");

        // Gamma angles (since Dg=10, from 0 to 180 in steps)
        for i in 0..19 {
            ldt.push_str(&format!("{}\n", i * 10));
        }

        // Intensity values for each C-plane (we have 1) and each gamma angle (19)
        // Generate a typical downlight/Lambertian distribution instead of isotropic
        // Intensity follows cos(gamma) pattern, with cutoff around 90 degrees
        // Peak intensity at gamma=0 (straight down), zero at gamma>=90
        let peak_intensity = flux / std::f64::consts::PI; // Lambertian normalization
        for i in 0..19 {
            let gamma_deg = (i * 10) as f64;
            let gamma_rad = gamma_deg * std::f64::consts::PI / 180.0;
            let intensity = if gamma_deg <= 90.0 {
                peak_intensity * gamma_rad.cos().max(0.0)
            } else {
                0.0 // No light above horizontal
            };
            ldt.push_str(&format!("{:.1}\n", intensity));
        }

        // EULUMDAT format requires CRLF line endings (Windows-style)
        ldt.replace('\n', "\r\n")
    }

    /// Generate L3D file from geometry
    fn generate_l3d(&self, variant: &ImportedVariant, geom: &ImportedGeometry) -> Result<Vec<u8>> {
        #[cfg(test)]
        eprintln!(
            "generate_l3d: {} vertices, {} triangles for variant {}",
            geom.vertices.len(),
            geom.triangles.len(),
            variant.name
        );

        let mut buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default();

        // Create structure.xml - must use GeometryReference inside Geometry element
        // Note: The filename in GeometryFileDefinition is just the base name,
        // but the actual file must be placed in a subdirectory named after the geometry ID
        // IMPORTANT: CreationTimeCode is REQUIRED by l3d_rs parser!
        let structure_xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<Luminaire xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://gldf.io/xsd/l3d/0.11.0/l3d.xsd">
  <Header>
    <Name>{}</Name>
    <CreatedWithApplication>gldf-rs IFC Import</CreatedWithApplication>
    <CreationTimeCode>{}</CreationTimeCode>
    <FormatVersion major="0" minor="11" />
  </Header>
  <GeometryDefinitions>
    <GeometryFileDefinition id="geom_1" filename="luminaire.obj" units="mm" />
  </GeometryDefinitions>
  <Structure>
    <Geometry partName="body">
      <Position x="0" y="0" z="0" />
      <Rotation x="0" y="0" z="0" />
      <GeometryReference geometryId="geom_1" />
      <LightEmittingObjects>
        <LightEmittingObject partName="LEO0">
          <Position x="0" y="0" z="0" />
          <Rotation x="0" y="0" z="0" />
          <Rectangle sizeX="0.1" sizeY="0.1" />
          <LuminousHeights c0="0" c90="0" c180="0" c270="0" />
        </LightEmittingObject>
      </LightEmittingObjects>
    </Geometry>
  </Structure>
</Luminaire>"#,
            Self::escape_xml(&variant.name),
            Self::current_timestamp()
        );

        zip.start_file("structure.xml", options)?;
        zip.write_all(structure_xml.as_bytes())?;

        // Create OBJ file in the geometry subdirectory (geom_1/luminaire.obj)
        // L3D spec requires geometry files to be in a subdirectory named after the geometry ID
        // three-d requires normals for proper rendering
        let mut obj = String::new();
        obj.push_str("# Generated by gldf-rs IFC Import\n");
        obj.push_str("# WaveFront OBJ file\n\n");
        // Reference material library (we'll create a stub MTL file)
        obj.push_str("mtllib luminaire.mtl\n\n");
        obj.push_str("g Luminaire\n\n");

        // Vertices (convert from meters to millimeters for L3D)
        for (x, y, z) in &geom.vertices {
            obj.push_str(&format!("v {} {} {}\n", x * 1000.0, y * 1000.0, z * 1000.0));
        }
        obj.push('\n');

        // Compute per-face normals and add them
        // This is needed because three-d requires normals for proper rendering
        let mut normals: Vec<(f64, f64, f64)> = Vec::new();
        for (i0, i1, i2) in &geom.triangles {
            let v0 = geom.vertices.get(*i0 as usize).unwrap_or(&(0.0, 0.0, 0.0));
            let v1 = geom.vertices.get(*i1 as usize).unwrap_or(&(0.0, 0.0, 0.0));
            let v2 = geom.vertices.get(*i2 as usize).unwrap_or(&(0.0, 0.0, 0.0));

            // Edge vectors
            let e1 = (v1.0 - v0.0, v1.1 - v0.1, v1.2 - v0.2);
            let e2 = (v2.0 - v0.0, v2.1 - v0.1, v2.2 - v0.2);

            // Cross product
            let nx = e1.1 * e2.2 - e1.2 * e2.1;
            let ny = e1.2 * e2.0 - e1.0 * e2.2;
            let nz = e1.0 * e2.1 - e1.1 * e2.0;

            // Normalize
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            let normal = if len > 1e-10 {
                (nx / len, ny / len, nz / len)
            } else {
                (0.0, 0.0, 1.0)
            };
            normals.push(normal);
        }

        // Write normals (one per face)
        for (nx, ny, nz) in &normals {
            obj.push_str(&format!("vn {} {} {}\n", nx, ny, nz));
        }
        obj.push('\n');

        // Texture coordinates - minimal set for three-d compatibility
        obj.push_str("vt 0.0 0.0\n");
        obj.push('\n');

        obj.push_str("usemtl default\n");

        // Faces with vertex/texture/normal indices (OBJ uses 1-based indices)
        // Format: f v1/vt1/vn1 v2/vt1/vn1 v3/vt1/vn1
        // Each face uses its own normal (face_idx + 1) and the single texture coord (1)
        for (face_idx, (i0, i1, i2)) in geom.triangles.iter().enumerate() {
            let n = face_idx + 1;
            obj.push_str(&format!(
                "f {}/1/{} {}/1/{} {}/1/{}\n",
                i0 + 1,
                n,
                i1 + 1,
                n,
                i2 + 1,
                n
            ));
        }

        // The OBJ file must be placed in geom_1/ subdirectory to match the geometry definition
        zip.start_file("geom_1/luminaire.obj", options)?;
        zip.write_all(obj.as_bytes())?;

        zip.finish()?;
        Ok(buffer.into_inner())
    }

    /// Escape XML special characters
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Escape for EULUMDAT (max 78 chars, no special chars)
    fn escape_ldt(s: &str) -> String {
        let s = s
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .collect::<String>();
        if s.len() > 78 {
            s[..78].to_string()
        } else {
            s
        }
    }

    /// Get current timestamp in ISO format
    fn current_timestamp() -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use chrono::Utc;
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }

        #[cfg(target_arch = "wasm32")]
        {
            "2024-01-01T00:00:00Z".to_string()
        }
    }

    /// Generate a UUID
    fn generate_uuid() -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                (nanos >> 96) as u32,
                ((nanos >> 80) & 0xFFFF) as u16,
                ((nanos >> 64) & 0xFFFF) as u16,
                ((nanos >> 48) & 0xFFFF) as u16,
                (nanos & 0xFFFFFFFFFFFF) as u64
            )
        }

        #[cfg(target_arch = "wasm32")]
        {
            "00000000-0000-0000-0000-000000000000".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ifc::ifc_import::{ImportedLightSource, ImportedProperties};

    #[test]
    fn test_generate_basic_gldf() {
        let luminaire = ImportedLuminaire {
            name: "Test Luminaire".to_string(),
            description: Some("Test description".to_string()),
            manufacturer: Some("Test Corp".to_string()),
            model_reference: Some("TEST-001".to_string()),
            variants: vec![ImportedVariant {
                name: "Standard".to_string(),
                description: None,
                gldf_variant_id: Some("var1".to_string()),
                fixture_type: Some("POINTSOURCE".to_string()),
                light_sources: vec![ImportedLightSource {
                    name: "LED".to_string(),
                    color_temperature: Some(4000.0),
                    luminous_flux: Some(1000.0),
                    emission_source: Some("LED".to_string()),
                    distribution: None,
                    photometry_filename: None,
                    ldt_metadata: None,
                }],
                photometry: None,
                geometry: None,
                properties: ImportedProperties {
                    number_of_sources: Some(1),
                    total_wattage: Some(10.0),
                    mounting_type: Some("CEILING".to_string()),
                    ..Default::default()
                },
            }],
            descriptive_attributes: Default::default(),
            embedded_files: Vec::new(),
        };

        let generator = GldfGenerator::new(luminaire);
        let gldf_bytes = generator.generate().unwrap();

        // Verify it's a valid ZIP
        assert!(!gldf_bytes.is_empty());
        assert!(gldf_bytes[0..2] == [0x50, 0x4B]); // PK signature

        // Extract and verify product.xml
        let cursor = Cursor::new(gldf_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut product_xml = String::new();
        use std::io::Read;
        archive
            .by_name("product.xml")
            .unwrap()
            .read_to_string(&mut product_xml)
            .unwrap();

        assert!(product_xml.contains("Test Luminaire"));
        assert!(product_xml.contains("Test Corp"));
        // Variant IDs are generated as var_1, var_2, etc.
        assert!(product_xml.contains("var_1"));
    }
}
