//! IFC STEP file writer
//!
//! Generates IFC files in STEP Physical File Format (ISO 10303-21)

use super::types::{EntityRef, LightEmissionSourceEnum, LightFixtureTypeEnum, OptionalRef};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use chrono::{DateTime, Utc};

/// IFC STEP file writer
///
/// Builds a valid IFC file with the minimum structure needed for light fixtures.
pub struct StepWriter {
    schema: String,
    next_id: u64,
    entities: Vec<String>,
    // Track entity IDs for relationships
    entity_map: HashMap<String, EntityRef>,
}

impl StepWriter {
    /// Create a new STEP writer for the given IFC schema
    pub fn new(schema: &str) -> Self {
        Self {
            schema: schema.to_string(),
            next_id: 1,
            entities: Vec::new(),
            entity_map: HashMap::new(),
        }
    }

    /// Get next entity ID and increment counter
    fn next_id(&mut self) -> EntityRef {
        let id = EntityRef::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add an entity line
    fn add_entity(&mut self, entity: String) -> EntityRef {
        let id = self.next_id();
        self.entities.push(format!("{}={}", id, entity));
        id
    }

    /// Generate a compressed GUID (22 chars, IFC style)
    fn generate_guid() -> String {
        // Get a pseudo-random seed - use different methods for native vs WASM
        #[cfg(not(target_arch = "wasm32"))]
        let seed = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        };

        #[cfg(target_arch = "wasm32")]
        let seed = {
            // Use a simple counter + random-ish value for WASM
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // Mix in some "randomness" from the address of a stack variable
            let stack_addr = &count as *const _ as u128;
            (count as u128).wrapping_mul(0x517cc1b727220a95) ^ stack_addr
        };

        // IFC uses base64-like encoding for GUIDs
        const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
        let mut result = String::new();
        let mut n = seed;
        for _ in 0..22 {
            result.push(CHARS[(n % 64) as usize] as char);
            n /= 64;
        }
        result.chars().rev().collect()
    }

    /// Escape a string for STEP format
    fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "''")
    }

    // =========================================================================
    // Core IFC entities
    // =========================================================================

    /// Add IfcOwnerHistory
    pub fn add_owner_history(&mut self, organization_name: &str) -> EntityRef {
        // Person
        let person = self.add_entity("IFCPERSON($,$,'',$,$,$,$,$)".to_string());

        // Organization
        let org = self.add_entity(format!(
            "IFCORGANIZATION($,'{}','GLDF Export',$,$)",
            Self::escape_string(organization_name)
        ));

        // PersonAndOrganization
        let person_org = self.add_entity(format!("IFCPERSONANDORGANIZATION({},{},$)", person, org));

        // Application
        let app = self.add_entity(format!(
            "IFCAPPLICATION({},'0.3.3','gldf-rs','gldf-rs')",
            org
        ));

        // OwnerHistory
        self.add_entity(format!(
            "IFCOWNERHISTORY({},{},{},{},$,$,$,{})",
            person_org,
            app,
            ".READWRITE.",
            ".ADDED.",
            Self::current_timestamp()
        ))
    }

    /// Add IfcProject
    pub fn add_project(&mut self, name: &str, owner_history: EntityRef) -> EntityRef {
        // Units
        let si_length = self.add_entity("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)".to_string());
        let si_area = self.add_entity("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)".to_string());
        let si_volume = self.add_entity("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)".to_string());
        let si_angle = self.add_entity("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)".to_string());
        let si_lumen = self.add_entity("IFCSIUNIT(*,.LUMINOUSFLUXUNIT.,$,.LUMEN.)".to_string());
        let si_power = self.add_entity("IFCSIUNIT(*,.POWERUNIT.,$,.WATT.)".to_string());

        let unit_assignment = self.add_entity(format!(
            "IFCUNITASSIGNMENT(({},{},{},{},{},{}))",
            si_length, si_area, si_volume, si_angle, si_lumen, si_power
        ));

        // Geometric context
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,1.))".to_string());
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let placement = self.add_entity(format!(
            "IFCAXIS2PLACEMENT3D({},{},{})",
            origin, axis, ref_dir
        ));

        let context_3d = self.add_entity(format!(
            "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.0E-5,{},{})",
            placement,
            OptionalRef::None
        ));

        // Project
        let project = self.add_entity(format!(
            "IFCPROJECT('{}',{},'{}','GLDF to IFC Export',$,$,$,({}),{})",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            context_3d,
            unit_assignment
        ));

        self.entity_map.insert("project".to_string(), project);
        self.entity_map.insert("context_3d".to_string(), context_3d);
        project
    }

    /// Add IfcSite
    pub fn add_site(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        project: EntityRef,
    ) -> EntityRef {
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let placement = self.add_entity(format!(
            "IFCLOCALPLACEMENT($,IFCAXIS2PLACEMENT3D({},{},{}))",
            origin, "$", "$"
        ));

        let site = self.add_entity(format!(
            "IFCSITE('{}',{},'{}','',$,{},$,$,.ELEMENT.,$,$,$,$,$)",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            placement
        ));

        // Aggregate site to project
        self.add_entity(format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            Self::generate_guid(),
            owner_history,
            project,
            site
        ));

        site
    }

    /// Add IfcBuilding
    pub fn add_building(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        site: EntityRef,
    ) -> EntityRef {
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let placement = self.add_entity(format!(
            "IFCLOCALPLACEMENT($,IFCAXIS2PLACEMENT3D({},{},{}))",
            origin, "$", "$"
        ));

        let building = self.add_entity(format!(
            "IFCBUILDING('{}',{},'{}','',$,{},$,$,.ELEMENT.,$,$,$)",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            placement
        ));

        // Aggregate building to site
        self.add_entity(format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            Self::generate_guid(),
            owner_history,
            site,
            building
        ));

        building
    }

    /// Add IfcBuildingStorey
    pub fn add_storey(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        building: EntityRef,
    ) -> EntityRef {
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let placement = self.add_entity(format!(
            "IFCLOCALPLACEMENT($,IFCAXIS2PLACEMENT3D({},{},{}))",
            origin, "$", "$"
        ));

        let storey = self.add_entity(format!(
            "IFCBUILDINGSTOREY('{}',{},'{}','',$,{},$,$,.ELEMENT.,0.)",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            placement
        ));

        // Aggregate storey to building
        self.add_entity(format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            Self::generate_guid(),
            owner_history,
            building,
            storey
        ));

        storey
    }

    // =========================================================================
    // Light fixture entities
    // =========================================================================

    /// Add IfcLightFixtureType
    pub fn add_light_fixture_type(
        &mut self,
        name: &str,
        manufacturer: &str,
        predefined_type: LightFixtureTypeEnum,
        owner_history: EntityRef,
    ) -> EntityRef {
        let fixture_type = self.add_entity(format!(
            "IFCLIGHTFIXTURETYPE('{}',{},'{}','Luminaire from GLDF',$,$,$,$,$,{})",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            predefined_type.to_step()
        ));

        // Add manufacturer property set
        self.add_manufacturer_pset(fixture_type, owner_history, manufacturer, name);

        fixture_type
    }

    /// Add IfcLightFixture occurrence
    pub fn add_light_fixture(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        storey: EntityRef,
        fixture_type: Option<EntityRef>,
    ) -> EntityRef {
        // Placement at origin
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let placement = self.add_entity(format!(
            "IFCLOCALPLACEMENT($,IFCAXIS2PLACEMENT3D({},{},{}))",
            origin, "$", "$"
        ));

        let fixture = self.add_entity(format!(
            "IFCLIGHTFIXTURE('{}',{},'{}','',$,{},{},{},.NOTDEFINED.)",
            Self::generate_guid(),
            owner_history,
            Self::escape_string(name),
            placement,
            "$", // Representation (TODO)
            "$"  // Tag
        ));

        // Assign to spatial container (storey)
        self.add_entity(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE('{}',{},$,$,({}),{})",
            Self::generate_guid(),
            owner_history,
            fixture,
            storey
        ));

        // Assign type if provided
        if let Some(ft) = fixture_type {
            self.add_entity(format!(
                "IFCRELDEFINESBYTYPE('{}',{},$,$,({}),{})",
                Self::generate_guid(),
                owner_history,
                fixture,
                ft
            ));
        }

        fixture
    }

    /// Add manufacturer property set
    fn add_manufacturer_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        manufacturer: &str,
        model_reference: &str,
    ) {
        let mfr_value = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('{}'),$)",
            Self::escape_string(manufacturer)
        ));

        let model_value = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('{}'),$)",
            Self::escape_string(model_reference)
        ));

        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_ManufacturerTypeInformation',$,({},{}))",
            Self::generate_guid(),
            owner_history,
            mfr_value,
            model_value
        ));

        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            Self::generate_guid(),
            owner_history,
            element,
            pset
        ));
    }

    // =========================================================================
    // Photometric data entities
    // =========================================================================

    /// Add IfcExternalReference for LDT/IES photometric file
    ///
    /// This creates a reference to an external photometric data file.
    /// The file should be distributed alongside the IFC file.
    pub fn add_external_reference(
        &mut self,
        location: &str,
        identification: Option<&str>,
        name: &str,
    ) -> EntityRef {
        let ident = identification
            .map(|s| format!("'{}'", Self::escape_string(s)))
            .unwrap_or_else(|| "$".to_string());

        self.add_entity(format!(
            "IFCEXTERNALREFERENCE('{}',{},'{}'))",
            Self::escape_string(location),
            ident,
            Self::escape_string(name)
        ))
    }

    /// Add IfcLightSourceGoniometric
    ///
    /// Represents a light source with goniometric (angular) distribution data.
    /// The distribution data is referenced via IfcExternalReference to an LDT/IES file.
    ///
    /// # Parameters
    /// - `name`: Light source name
    /// - `colour_appearance`: RGB color (0.0-1.0) or None for default white
    /// - `colour_temperature`: Color temperature in Kelvin (e.g., 3000.0, 4000.0)
    /// - `luminous_flux`: Total luminous flux in lumens
    /// - `emission_source`: Type of light emission (LED, etc.)
    /// - `photometry_file`: Path/URI to external LDT/IES file, or None
    pub fn add_light_source_goniometric(
        &mut self,
        name: &str,
        colour_appearance: Option<(f64, f64, f64)>,
        colour_temperature: f64,
        luminous_flux: f64,
        emission_source: LightEmissionSourceEnum,
        photometry_file: Option<&str>,
    ) -> EntityRef {
        // Position at origin
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,-1.))".to_string()); // Light points down
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let position = self.add_entity(format!(
            "IFCAXIS2PLACEMENT3D({},{},{})",
            origin, axis, ref_dir
        ));

        // Colour appearance (IfcColourRgb)
        let colour_ref = if let Some((r, g, b)) = colour_appearance {
            let colour = self.add_entity(format!("IFCCOLOURRGB($,{:.4},{:.4},{:.4})", r, g, b));
            OptionalRef::Some(colour)
        } else {
            OptionalRef::None
        };

        // External photometric reference
        let distribution_ref = if let Some(file_path) = photometry_file {
            let ext_ref = self.add_entity(format!(
                "IFCEXTERNALREFERENCE('{}',$,'{}')",
                Self::escape_string(file_path),
                Self::escape_string(name)
            ));
            OptionalRef::Some(ext_ref)
        } else {
            OptionalRef::None
        };

        // IfcLightSourceGoniometric
        // Attributes: Name, LightColour, AmbientIntensity, Intensity, Position,
        //             ColourAppearance, ColourTemperature, LuminousFlux, LightEmissionSource,
        //             LightDistributionDataSource
        self.add_entity(format!(
            "IFCLIGHTSOURCEGONIOMETRIC('{}',{},0.,1.,{},{},{:.1},{:.1},{},{})",
            Self::escape_string(name),
            colour_ref, // LightColour (optional)
            position,
            colour_ref, // ColourAppearance
            colour_temperature,
            luminous_flux,
            emission_source.to_step(),
            distribution_ref
        ))
    }

    /// Add IfcLightIntensityDistribution for embedded photometric data
    ///
    /// This is an alternative to external file reference - embeds the distribution
    /// directly in the IFC file. Useful for simple distributions.
    pub fn add_light_intensity_distribution(
        &mut self,
        distribution_type: &str, // "TYPE_A", "TYPE_B", "TYPE_C", "NOTDEFINED"
        distribution_data: &[(f64, f64, f64)], // (main_angle, secondary_angle, intensity)
    ) -> EntityRef {
        // Create distribution data entries
        let mut data_entries = Vec::new();
        for (main, secondary, intensity) in distribution_data {
            let entry = self.add_entity(format!(
                "IFCLIGHTDISTRIBUTIONDATA({:.2},{:.2},({:.4}))",
                main, secondary, intensity
            ));
            data_entries.push(entry.to_string());
        }

        let data_list = if data_entries.is_empty() {
            "$".to_string()
        } else {
            format!("({})", data_entries.join(","))
        };

        self.add_entity(format!(
            "IFCLIGHTINTENSITYDISTRIBUTION(.{}.,{})",
            distribution_type, data_list
        ))
    }

    // =========================================================================
    // Property sets for light fixtures
    // =========================================================================

    /// Add Pset_LightFixtureTypeCommon property set
    ///
    /// Standard IFC property set for light fixture types with common properties.
    pub fn add_light_fixture_common_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        number_of_sources: Option<i32>,
        total_wattage: Option<f64>,
        light_fixture_mounting_type: Option<&str>,
        light_fixture_placement_type: Option<&str>,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(n) = number_of_sources {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfSources',$,IFCINTEGER({}),$)",
                n
            ));
            properties.push(prop);
        }

        if let Some(w) = total_wattage {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('TotalWattage',$,IFCPOWERMEASURE({:.2}),$)",
                w
            ));
            properties.push(prop);
        }

        if let Some(mt) = light_fixture_mounting_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('LightFixtureMountingType',$,IFCLABEL('{}'),$)",
                Self::escape_string(mt)
            ));
            properties.push(prop);
        }

        if let Some(pt) = light_fixture_placement_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('LightFixturePlacementType',$,IFCLABEL('{}'),$)",
                Self::escape_string(pt)
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            // Return a dummy ref if no properties
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_LightFixtureTypeCommon',$,({}))",
            Self::generate_guid(),
            owner_history,
            prop_list.join(",")
        ));

        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            Self::generate_guid(),
            owner_history,
            element,
            pset
        ));

        pset
    }

    /// Add electrical properties property set
    pub fn add_electrical_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        rated_voltage: Option<f64>,
        rated_current: Option<f64>,
        power_factor: Option<f64>,
        ip_code: Option<&str>,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(v) = rated_voltage {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('RatedVoltage',$,IFCELECTRICVOLTAGEMEASURE({:.1}),$)",
                v
            ));
            properties.push(prop);
        }

        if let Some(i) = rated_current {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('RatedCurrent',$,IFCELECTRICCURRENTMEASURE({:.3}),$)",
                i
            ));
            properties.push(prop);
        }

        if let Some(pf) = power_factor {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('PowerFactor',$,IFCNORMALISEDRATIOMEASURE({:.3}),$)",
                pf
            ));
            properties.push(prop);
        }

        if let Some(ip) = ip_code {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('IPCode',$,IFCLABEL('{}'),$)",
                Self::escape_string(ip)
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_ElectricalDeviceCommon',$,({}))",
            Self::generate_guid(),
            owner_history,
            prop_list.join(",")
        ));

        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            Self::generate_guid(),
            owner_history,
            element,
            pset
        ));

        pset
    }

    // =========================================================================
    // Output
    // =========================================================================

    /// Get current Unix timestamp
    fn current_timestamp() -> i64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, return a placeholder timestamp (2024-01-01 00:00:00 UTC)
            // In a real app, you'd get this from JavaScript via js_sys::Date
            1704067200_i64
        }
    }

    /// Generate the complete IFC STEP file content
    pub fn to_step_string(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        let timestamp = {
            let now: DateTime<Utc> = Utc::now();
            now.format("%Y-%m-%dT%H:%M:%S").to_string()
        };

        #[cfg(target_arch = "wasm32")]
        let timestamp = "2024-01-01T00:00:00".to_string();

        let mut output = String::new();

        // HEADER section
        output.push_str("ISO-10303-21;\n");
        output.push_str("HEADER;\n");
        output.push_str("FILE_DESCRIPTION(('GLDF to IFC Export'),'2;1');\n");
        output.push_str(&format!(
            "FILE_NAME('export.ifc','{}',(''),(''),'gldf-rs','gldf-rs','');\n",
            timestamp
        ));
        output.push_str(&format!("FILE_SCHEMA(('{}'));\n", self.schema));
        output.push_str("ENDSEC;\n");
        output.push('\n');

        // DATA section
        output.push_str("DATA;\n");
        for entity in &self.entities {
            output.push_str(entity);
            output.push_str(";\n");
        }
        output.push_str("ENDSEC;\n");
        output.push('\n');

        // End
        output.push_str("END-ISO-10303-21;\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_ifc() {
        let mut writer = StepWriter::new("IFC4");
        let oh = writer.add_owner_history("Test Corp");
        let project = writer.add_project("Test Project", oh);
        let site = writer.add_site("Test Site", oh, project);
        let building = writer.add_building("Test Building", oh, site);
        let storey = writer.add_storey("Ground Floor", oh, building);

        let fixture_type = writer.add_light_fixture_type(
            "LED Downlight",
            "Test Corp",
            LightFixtureTypeEnum::PointSource,
            oh,
        );

        let _fixture =
            writer.add_light_fixture("LED Downlight 001", oh, storey, Some(fixture_type));

        let output = writer.to_step_string();

        // Verify structure
        assert!(output.contains("ISO-10303-21"));
        assert!(output.contains("IFC4"));
        assert!(output.contains("IFCPROJECT"));
        assert!(output.contains("IFCSITE"));
        assert!(output.contains("IFCBUILDING"));
        assert!(output.contains("IFCBUILDINGSTOREY"));
        assert!(output.contains("IFCLIGHTFIXTURETYPE"));
        assert!(output.contains("IFCLIGHTFIXTURE"));
        assert!(output.contains("Pset_ManufacturerTypeInformation"));

        println!("{}", output);
    }

    #[test]
    fn test_light_source_goniometric() {
        let mut writer = StepWriter::new("IFC4");
        let oh = writer.add_owner_history("Test Corp");
        let project = writer.add_project("Test Project", oh);
        let site = writer.add_site("Test Site", oh, project);
        let building = writer.add_building("Test Building", oh, site);
        let storey = writer.add_storey("Ground Floor", oh, building);

        let fixture_type = writer.add_light_fixture_type(
            "LED Panel",
            "Test Corp",
            LightFixtureTypeEnum::DirectionSource,
            oh,
        );

        // Add goniometric light source with external LDT reference
        let _light_source = writer.add_light_source_goniometric(
            "LED Panel Light Source",
            Some((1.0, 0.95, 0.9)), // Warm white
            3000.0,                 // 3000K color temperature
            4500.0,                 // 4500 lumens
            LightEmissionSourceEnum::Led,
            Some("photometry/led_panel.ldt"),
        );

        // Add common property set
        writer.add_light_fixture_common_pset(
            fixture_type,
            oh,
            Some(1),    // 1 light source
            Some(36.0), // 36W
            Some("SURFACE"),
            Some("CEILING"),
        );

        // Add electrical property set
        writer.add_electrical_pset(
            fixture_type,
            oh,
            Some(230.0), // 230V
            Some(0.16),  // 0.16A
            Some(0.95),  // 0.95 power factor
            Some("IP20"),
        );

        let _fixture = writer.add_light_fixture("LED Panel 001", oh, storey, Some(fixture_type));

        let output = writer.to_step_string();

        // Verify goniometric light source
        assert!(output.contains("IFCLIGHTSOURCEGONIOMETRIC"));
        assert!(output.contains("led_panel.ldt"));
        assert!(output.contains("3000.0")); // Color temp
        assert!(output.contains("4500.0")); // Lumens
        assert!(output.contains(".LED."));

        // Verify property sets
        assert!(output.contains("Pset_LightFixtureTypeCommon"));
        assert!(output.contains("NumberOfSources"));
        assert!(output.contains("TotalWattage"));
        assert!(output.contains("IFCPOWERMEASURE(36.00)"));

        assert!(output.contains("Pset_ElectricalDeviceCommon"));
        assert!(output.contains("RatedVoltage"));
        assert!(output.contains("PowerFactor"));
        assert!(output.contains("IPCode"));

        println!("{}", output);
    }

    #[test]
    fn test_embedded_light_distribution() {
        let mut writer = StepWriter::new("IFC4");

        // Simple lambertian distribution (cosine)
        let distribution_data: Vec<(f64, f64, f64)> = vec![
            (0.0, 0.0, 1.0),
            (0.0, 30.0, 0.866),
            (0.0, 60.0, 0.5),
            (0.0, 90.0, 0.0),
        ];

        let _dist = writer.add_light_intensity_distribution("TYPE_C", &distribution_data);

        let output = writer.to_step_string();
        assert!(output.contains("IFCLIGHTINTENSITYDISTRIBUTION"));
        assert!(output.contains("IFCLIGHTDISTRIBUTIONDATA"));
        assert!(output.contains(".TYPE_C."));
    }
}
