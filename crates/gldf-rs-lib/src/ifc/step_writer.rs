//! IFC STEP file writer
//!
//! Generates IFC files in STEP Physical File Format (ISO 10303-21)

use super::types::{
    DistributionSystemEnum, ElectricalDeviceProperties, EntityRef, FlowDirectionEnum,
    InsulationStandardClass, LightEmissionSourceEnum, LightFixtureCommonProperties,
    LightFixtureTypeEnum, ManufacturerTypeInfo, OptionalRef, ServiceLifeInfo, WarrantyInfo,
};
use std::collections::HashMap;

/// Triangulated mesh data for IFC export
///
/// Contains vertex positions and triangle indices in a format
/// suitable for IFC TRIANGULATEDFACESET representation.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    /// Vertex positions as (x, y, z) tuples in meters
    pub vertices: Vec<(f64, f64, f64)>,
    /// Triangle indices (1-based for IFC)
    /// Each tuple is (v1, v2, v3) referencing vertices
    pub triangles: Vec<(u32, u32, u32)>,
}

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
    // Track light sources for connecting to fixtures
    light_source_ids: Vec<EntityRef>,
    // GUID counter for uniqueness
    guid_counter: u64,
}

impl StepWriter {
    /// Create a new STEP writer for the given IFC schema
    pub fn new(schema: &str) -> Self {
        Self {
            schema: schema.to_string(),
            next_id: 1,
            entities: Vec::new(),
            entity_map: HashMap::new(),
            light_source_ids: Vec::new(),
            guid_counter: 1,
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

    /// Generate a valid IFC GlobalId (22 characters, base64-like encoding)
    ///
    /// IFC GlobalId is a compressed UUID representation using 64 characters:
    /// 0-9, A-Z, a-z, _ and $
    fn generate_guid(&mut self) -> String {
        // IFC base64 character set (NOT standard base64!)
        const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

        // Generate a pseudo-random 128-bit value
        #[cfg(not(target_arch = "wasm32"))]
        let seed = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let time_part = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            // Mix in counter for uniqueness within same timestamp
            time_part.wrapping_add(self.guid_counter as u128 * 0x517cc1b727220a95)
        };

        #[cfg(target_arch = "wasm32")]
        let seed = {
            // Use counter-based approach for WASM
            let base = 0x0123456789ABCDEFu128;
            base.wrapping_mul(self.guid_counter as u128)
                .wrapping_add(0xFEDCBA9876543210u128)
        };

        self.guid_counter += 1;

        // Convert 128-bit value to 22 base-64 characters
        // Each character encodes 6 bits, 22 * 6 = 132 bits (we use 128)
        let mut result = String::with_capacity(22);
        let mut n = seed;

        for _ in 0..22 {
            result.push(CHARS[(n % 64) as usize] as char);
            n /= 64;
        }

        // Reverse to get most-significant bits first
        result.chars().rev().collect()
    }

    /// Escape a string for STEP format
    fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "''")
    }

    /// Create an IFCAXIS2PLACEMENT3D entity (properly, not inline!)
    fn add_axis2_placement_3d(
        &mut self,
        origin: EntityRef,
        axis: Option<EntityRef>,
        ref_dir: Option<EntityRef>,
    ) -> EntityRef {
        let axis_str = axis
            .map(|a| a.to_string())
            .unwrap_or_else(|| "$".to_string());
        let ref_str = ref_dir
            .map(|r| r.to_string())
            .unwrap_or_else(|| "$".to_string());
        let entity_str = format!("IFCAXIS2PLACEMENT3D({},{},{})", origin, axis_str, ref_str);
        self.add_entity(entity_str)
    }

    /// Create an IFCLOCALPLACEMENT entity with proper references
    fn add_local_placement(
        &mut self,
        relative_to: Option<EntityRef>,
        placement: EntityRef,
    ) -> EntityRef {
        let relative_str = relative_to
            .map(|r| r.to_string())
            .unwrap_or_else(|| "$".to_string());
        let entity_str = format!("IFCLOCALPLACEMENT({},{})", relative_str, placement);
        self.add_entity(entity_str)
    }

    // =========================================================================
    // Core IFC entities
    // =========================================================================

    /// Add IfcOwnerHistory
    pub fn add_owner_history(&mut self, organization_name: &str) -> EntityRef {
        // Person
        let person = self.add_entity("IFCPERSON($,$,'',$,$,$,$,$)".to_string());

        // Organization
        let org_str = format!(
            "IFCORGANIZATION($,'{}','GLDF Export',$,$)",
            Self::escape_string(organization_name)
        );
        let org = self.add_entity(org_str);

        // PersonAndOrganization
        let po_str = format!("IFCPERSONANDORGANIZATION({},{},$)", person, org);
        let person_org = self.add_entity(po_str);

        // Application
        let app_str = format!("IFCAPPLICATION({},'0.3.3','gldf-rs','gldf-rs')", org);
        let app = self.add_entity(app_str);

        // OwnerHistory
        let oh_str = format!(
            "IFCOWNERHISTORY({},{},{},{},$,$,$,{})",
            person_org,
            app,
            ".READWRITE.",
            ".ADDED.",
            Self::current_timestamp()
        );
        self.add_entity(oh_str)
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

        let ua_str = format!(
            "IFCUNITASSIGNMENT(({},{},{},{},{},{}))",
            si_length, si_area, si_volume, si_angle, si_lumen, si_power
        );
        let unit_assignment = self.add_entity(ua_str);

        // Geometric context - create entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,1.))".to_string());
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let placement = self.add_axis2_placement_3d(origin, Some(axis), Some(ref_dir));

        let ctx_str = format!(
            "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.0E-5,{},{})",
            placement,
            OptionalRef::None
        );
        let context_3d = self.add_entity(ctx_str);

        // Project
        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let proj_str = format!(
            "IFCPROJECT('{}',{},'{}','GLDF to IFC Export',$,$,$,({}),{})",
            guid, owner_history, escaped_name, context_3d, unit_assignment
        );
        let project = self.add_entity(proj_str);

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
        // Create placement entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);
        let placement = self.add_local_placement(None, axis2_placement);

        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let site_str = format!(
            "IFCSITE('{}',{},'{}','',$,{},$,$,.ELEMENT.,$,$,$,$,$)",
            guid, owner_history, escaped_name, placement
        );
        let site = self.add_entity(site_str);

        // Aggregate site to project
        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            guid2, owner_history, project, site
        );
        self.add_entity(rel_str);

        site
    }

    /// Add IfcBuilding
    pub fn add_building(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        site: EntityRef,
    ) -> EntityRef {
        // Create placement entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);
        let placement = self.add_local_placement(None, axis2_placement);

        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let bldg_str = format!(
            "IFCBUILDING('{}',{},'{}','',$,{},$,$,.ELEMENT.,$,$,$)",
            guid, owner_history, escaped_name, placement
        );
        let building = self.add_entity(bldg_str);

        // Aggregate building to site
        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            guid2, owner_history, site, building
        );
        self.add_entity(rel_str);

        building
    }

    /// Add IfcBuildingStorey
    pub fn add_storey(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        building: EntityRef,
    ) -> EntityRef {
        // Create placement entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);
        let placement = self.add_local_placement(None, axis2_placement);

        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let storey_str = format!(
            "IFCBUILDINGSTOREY('{}',{},'{}','',$,{},$,$,.ELEMENT.,0.)",
            guid, owner_history, escaped_name, placement
        );
        let storey = self.add_entity(storey_str);

        // Aggregate storey to building
        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELAGGREGATES('{}',{},$,$,{},({}))",
            guid2, owner_history, building, storey
        );
        self.add_entity(rel_str);

        storey
    }

    // =========================================================================
    // Light fixture entities
    // =========================================================================

    /// Add IfcLightFixtureType with optional shared geometry
    ///
    /// # Arguments
    /// * `name` - Type name
    /// * `manufacturer` - Manufacturer name
    /// * `predefined_type` - Light fixture predefined type
    /// * `owner_history` - Owner history reference
    /// * `representation_map` - Optional shared geometry (IFCREPRESENTATIONMAP)
    ///
    /// When `representation_map` is provided, all fixture types share the same geometry,
    /// avoiding duplication. This is the correct IFC pattern for variants.
    pub fn add_light_fixture_type(
        &mut self,
        name: &str,
        manufacturer: &str,
        predefined_type: LightFixtureTypeEnum,
        owner_history: EntityRef,
    ) -> EntityRef {
        self.add_light_fixture_type_with_geometry(
            name,
            manufacturer,
            predefined_type,
            owner_history,
            None,
        )
    }

    /// Add IfcLightFixtureType with shared representation map
    pub fn add_light_fixture_type_with_geometry(
        &mut self,
        name: &str,
        manufacturer: &str,
        predefined_type: LightFixtureTypeEnum,
        owner_history: EntityRef,
        representation_map: Option<EntityRef>,
    ) -> EntityRef {
        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);

        // RepresentationMaps is parameter 8 (index 7) in IFCLIGHTFIXTURETYPE
        // IFCLIGHTFIXTURETYPE(GlobalId, OwnerHistory, Name, Description, ApplicableOccurrence,
        //                     HasPropertySets, RepresentationMaps, Tag, ElementType, PredefinedType)
        let rep_maps = representation_map
            .map(|r| format!("({})", r))
            .unwrap_or_else(|| "$".to_string());

        let type_str = format!(
            "IFCLIGHTFIXTURETYPE('{}',{},'{}','Luminaire from GLDF',$,$,{},$,$,{})",
            guid,
            owner_history,
            escaped_name,
            rep_maps,
            predefined_type.to_step()
        );
        let fixture_type = self.add_entity(type_str);

        // Add manufacturer property set
        self.add_manufacturer_pset(fixture_type, owner_history, manufacturer, name);

        fixture_type
    }

    /// Create simple box geometry for the light fixture (fallback when no L3D mesh)
    fn create_fixture_geometry(&mut self, width: f64, depth: f64, height: f64) -> EntityRef {
        let context_3d = self.entity_map.get("context_3d").copied();

        // Create a simple extruded box - Rectangle profile
        let profile_str = format!(
            "IFCRECTANGLEPROFILEDEF(.AREA.,$,$,{:.3},{:.3})",
            width, depth
        );
        let profile = self.add_entity(profile_str);

        // Placement for extrusion (at origin, extruding upward)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,1.))".to_string());
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let extrusion_placement = self.add_axis2_placement_3d(origin, Some(axis), Some(ref_dir));

        // Direction of extrusion
        let extrude_dir = self.add_entity("IFCDIRECTION((0.,0.,1.))".to_string());

        // Extruded solid
        let solid_str = format!(
            "IFCEXTRUDEDAREASOLID({},{},{},{:.3})",
            profile, extrusion_placement, extrude_dir, height
        );
        let solid = self.add_entity(solid_str);

        // Shape representation
        let context_str = context_3d
            .map(|c| c.to_string())
            .unwrap_or_else(|| "$".to_string());
        let shape_str = format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({}))",
            context_str, solid
        );
        let shape_rep = self.add_entity(shape_str);

        // Product definition shape
        let prod_str = format!("IFCPRODUCTDEFINITIONSHAPE($,$,({}))", shape_rep);
        self.add_entity(prod_str)
    }

    /// Create a representation map for shared geometry
    ///
    /// Creates geometry that can be referenced by multiple IFCLIGHTFIXTURETYPE entities.
    /// This avoids duplicating geometry for each variant.
    ///
    /// # Arguments
    /// * `mesh` - Optional triangulated mesh data. If None, creates default box.
    ///
    /// # Returns
    /// EntityRef to IFCREPRESENTATIONMAP
    pub fn create_representation_map(&mut self, mesh: Option<&MeshData>) -> EntityRef {
        // Create origin for the map
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);

        // Create the shape representation
        let shape_rep = if let Some(m) = mesh {
            if !m.vertices.is_empty() && !m.triangles.is_empty() {
                self.create_tessellated_shape_representation(m)
            } else {
                self.create_box_shape_representation(0.3, 0.3, 0.1)
            }
        } else {
            self.create_box_shape_representation(0.3, 0.3, 0.1)
        };

        // Create the representation map
        let map_str = format!("IFCREPRESENTATIONMAP({},{})", axis2_placement, shape_rep);
        self.add_entity(map_str)
    }

    /// Create tessellated shape representation (without ProductDefinitionShape wrapper)
    fn create_tessellated_shape_representation(&mut self, mesh: &MeshData) -> EntityRef {
        let context_3d = self.entity_map.get("context_3d").copied();

        // Build vertex list
        let coords_str: String = mesh
            .vertices
            .iter()
            .map(|(x, y, z)| format!("({:.6},{:.6},{:.6})", x, y, z))
            .collect::<Vec<_>>()
            .join(",");

        let point_list_str = format!("IFCCARTESIANPOINTLIST3D(({}))", coords_str);
        let point_list = self.add_entity(point_list_str);

        // Build triangle index list
        let triangles_str: String = mesh
            .triangles
            .iter()
            .map(|(a, b, c)| format!("({},{},{})", a, b, c))
            .collect::<Vec<_>>()
            .join(",");

        let faceset_str = format!(
            "IFCTRIANGULATEDFACESET({},$,.T.,({}),())",
            point_list, triangles_str
        );
        let faceset = self.add_entity(faceset_str);

        let context_str = context_3d
            .map(|c| c.to_string())
            .unwrap_or_else(|| "$".to_string());
        let shape_str = format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({}))",
            context_str, faceset
        );
        self.add_entity(shape_str)
    }

    /// Create box shape representation (without ProductDefinitionShape wrapper)
    fn create_box_shape_representation(
        &mut self,
        width: f64,
        height: f64,
        depth: f64,
    ) -> EntityRef {
        let context_3d = self.entity_map.get("context_3d").copied();

        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2 = self.add_axis2_placement_3d(origin, None, None);

        let profile_str = format!(
            "IFCRECTANGLEPROFILEDEF(.AREA.,'Luminaire Profile',{},{},{})",
            axis2, width, height
        );
        let profile = self.add_entity(profile_str);

        let dir_str = "IFCDIRECTION((0.,0.,1.))";
        let dir = self.add_entity(dir_str.to_string());

        let solid_str = format!(
            "IFCEXTRUDEDAREASOLID({},{},{},{})",
            profile, axis2, dir, depth
        );
        let solid = self.add_entity(solid_str);

        let context_str = context_3d
            .map(|c| c.to_string())
            .unwrap_or_else(|| "$".to_string());
        let shape_str = format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({}))",
            context_str, solid
        );
        self.add_entity(shape_str)
    }

    /// Create tessellated geometry from L3D mesh data
    ///
    /// Converts triangulated mesh to IFC format:
    /// - IFCCARTESIANPOINTLIST3D for vertex positions
    /// - IFCTRIANGULATEDFACESET for triangle indices
    /// - IFCSHAPEREPRESENTATION with 'Tessellation' type
    ///
    /// # Arguments
    /// * `mesh` - Triangulated mesh data with vertices and triangle indices
    ///
    /// # Returns
    /// EntityRef to IFCPRODUCTDEFINITIONSHAPE
    pub fn create_tessellated_geometry(&mut self, mesh: &MeshData) -> EntityRef {
        let context_3d = self.entity_map.get("context_3d").copied();

        // Build vertex list: ((x1,y1,z1),(x2,y2,z2),...)
        let coords_str: String = mesh
            .vertices
            .iter()
            .map(|(x, y, z)| format!("({:.6},{:.6},{:.6})", x, y, z))
            .collect::<Vec<_>>()
            .join(",");

        let point_list_str = format!("IFCCARTESIANPOINTLIST3D(({}))", coords_str);
        let point_list = self.add_entity(point_list_str);

        // Build triangle index list: ((1,2,3),(4,5,6),...)
        // IFC uses 1-based indices
        let triangles_str: String = mesh
            .triangles
            .iter()
            .map(|(a, b, c)| format!("({},{},{})", a, b, c))
            .collect::<Vec<_>>()
            .join(",");

        // IFCTRIANGULATEDFACESET(Coordinates, Normals, Closed, CoordIndex, PnIndex)
        // We use:
        // - Coordinates: the point list
        // - Normals: $ (auto-calculate)
        // - Closed: .T. (closed solid) or .F. (open surface)
        // - CoordIndex: triangle indices
        // - PnIndex: $ (not used when Normals is $)
        let faceset_str = format!(
            "IFCTRIANGULATEDFACESET({},$,.T.,({}),())",
            point_list, triangles_str
        );
        let faceset = self.add_entity(faceset_str);

        // Shape representation with 'Tessellation' type
        let context_str = context_3d
            .map(|c| c.to_string())
            .unwrap_or_else(|| "$".to_string());
        let shape_str = format!(
            "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({}))",
            context_str, faceset
        );
        let shape_rep = self.add_entity(shape_str);

        // Product definition shape
        let prod_str = format!("IFCPRODUCTDEFINITIONSHAPE($,$,({}))", shape_rep);
        self.add_entity(prod_str)
    }

    /// Add IfcLightFixture occurrence with geometry
    ///
    /// # Arguments
    /// * `name` - Fixture name
    /// * `owner_history` - Owner history reference
    /// * `storey` - Building storey reference
    /// * `fixture_type` - Optional fixture type reference
    /// * `representation_map` - Optional shared representation map. If provided, uses IFCMAPPEDITEM.
    ///   If None, creates inline geometry from mesh_data or default box.
    /// * `mesh_data` - Optional L3D mesh data for inline tessellated geometry (used when no rep map).
    pub fn add_light_fixture_with_map(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        storey: EntityRef,
        fixture_type: Option<EntityRef>,
        representation_map: Option<EntityRef>,
        mesh_data: Option<&MeshData>,
    ) -> EntityRef {
        // Create placement entities
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);
        let placement = self.add_local_placement(None, axis2_placement);

        // Create geometry - use mapped item if rep map provided, otherwise inline geometry
        let geometry = if let Some(rep_map) = representation_map {
            self.create_mapped_item_geometry(rep_map)
        } else if let Some(mesh) = mesh_data {
            if !mesh.vertices.is_empty() && !mesh.triangles.is_empty() {
                self.create_tessellated_geometry(mesh)
            } else {
                self.create_fixture_geometry(0.3, 0.3, 0.1)
            }
        } else {
            self.create_fixture_geometry(0.3, 0.3, 0.1)
        };

        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let fixture_str = format!(
            "IFCLIGHTFIXTURE('{}',{},'{}','',$,{},{},$,.NOTDEFINED.)",
            guid, owner_history, escaped_name, placement, geometry
        );
        let fixture = self.add_entity(fixture_str);

        // Assign to spatial container (storey)
        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE('{}',{},$,$,({}),{})",
            guid2, owner_history, fixture, storey
        );
        self.add_entity(rel_str);

        // Assign type if provided
        if let Some(ft) = fixture_type {
            let guid3 = self.generate_guid();
            let rel_type_str = format!(
                "IFCRELDEFINESBYTYPE('{}',{},$,$,({}),{})",
                guid3, owner_history, fixture, ft
            );
            self.add_entity(rel_type_str);
        }

        // Associate any accumulated light sources with this fixture
        if !self.light_source_ids.is_empty() {
            let guid4 = self.generate_guid();
            let sources_list = self
                .light_source_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let rel_group_str = format!(
                "IFCRELASSIGNSTOGROUP('{}',{},'LightSources','Light sources for fixture',$,({}),{})",
                guid4, owner_history, sources_list, fixture
            );
            self.add_entity(rel_group_str);
        }

        fixture
    }

    /// Create geometry using IFCMAPPEDITEM referencing a representation map
    fn create_mapped_item_geometry(&mut self, rep_map: EntityRef) -> EntityRef {
        let context_3d = self.entity_map.get("context_3d").copied();

        // Create identity transform for the mapped item
        // IFCCARTESIANTRANSFORMATIONOPERATOR3D(Axis1, Axis2, LocalOrigin, Scale, Axis3)
        // LocalOrigin must be IFCCARTESIANPOINT, not IFCAXIS2PLACEMENT3D
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let transform = self.add_entity(format!(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,{},1.,$)",
            origin
        ));

        // Create the mapped item
        let mapped_item = self.add_entity(format!("IFCMAPPEDITEM({},{})", rep_map, transform));

        // Shape representation using the mapped item
        let context_str = context_3d
            .map(|c| c.to_string())
            .unwrap_or_else(|| "$".to_string());
        let shape_str = format!(
            "IFCSHAPEREPRESENTATION({},'Body','MappedRepresentation',({}))",
            context_str, mapped_item
        );
        let shape_rep = self.add_entity(shape_str);

        // Product definition shape
        let prod_str = format!("IFCPRODUCTDEFINITIONSHAPE($,$,({}))", shape_rep);
        self.add_entity(prod_str)
    }

    /// Add IfcLightFixture occurrence with geometry and connected light sources
    ///
    /// # Arguments
    /// * `name` - Fixture name
    /// * `owner_history` - Owner history reference
    /// * `storey` - Building storey reference
    /// * `fixture_type` - Optional fixture type reference
    /// * `mesh_data` - Optional L3D mesh data for tessellated geometry.
    ///   If None, creates a default 0.3m x 0.3m x 0.1m box.
    pub fn add_light_fixture(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        storey: EntityRef,
        fixture_type: Option<EntityRef>,
        mesh_data: Option<&MeshData>,
    ) -> EntityRef {
        // Create placement entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis2_placement = self.add_axis2_placement_3d(origin, None, None);
        let placement = self.add_local_placement(None, axis2_placement);

        // Create geometry - use L3D mesh if available, otherwise fallback to box
        let geometry = if let Some(mesh) = mesh_data {
            if !mesh.vertices.is_empty() && !mesh.triangles.is_empty() {
                self.create_tessellated_geometry(mesh)
            } else {
                // Empty mesh, use default box
                self.create_fixture_geometry(0.3, 0.3, 0.1)
            }
        } else {
            // No mesh data, use default 0.3m x 0.3m x 0.1m box
            self.create_fixture_geometry(0.3, 0.3, 0.1)
        };

        let guid = self.generate_guid();
        let escaped_name = Self::escape_string(name);
        let fixture_str = format!(
            "IFCLIGHTFIXTURE('{}',{},'{}','',$,{},{},$,.NOTDEFINED.)",
            guid, owner_history, escaped_name, placement, geometry
        );
        let fixture = self.add_entity(fixture_str);

        // Assign to spatial container (storey)
        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE('{}',{},$,$,({}),{})",
            guid2, owner_history, fixture, storey
        );
        self.add_entity(rel_str);

        // Assign type if provided
        if let Some(ft) = fixture_type {
            let guid3 = self.generate_guid();
            let type_str = format!(
                "IFCRELDEFINESBYTYPE('{}',{},$,$,({}),{})",
                guid3, owner_history, fixture, ft
            );
            self.add_entity(type_str);
        }

        // Connect light sources to fixture (Issue 2 fix)
        if !self.light_source_ids.is_empty() {
            let source_refs: Vec<String> = self
                .light_source_ids
                .iter()
                .map(|s| s.to_string())
                .collect();
            let guid4 = self.generate_guid();
            let group_str = format!(
                "IFCRELASSIGNSTOGROUP('{}',{},'LightSources','Light sources for fixture',$,({}),{})",
                guid4, owner_history, source_refs.join(","), fixture
            );
            self.add_entity(group_str);
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
        let mfr_str = format!(
            "IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('{}'),$)",
            Self::escape_string(manufacturer)
        );
        let mfr_value = self.add_entity(mfr_str);

        let model_str = format!(
            "IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('{}'),$)",
            Self::escape_string(model_reference)
        );
        let model_value = self.add_entity(model_str);

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_ManufacturerTypeInformation',$,({},{}))",
            guid, owner_history, mfr_value, model_value
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);
    }

    // =========================================================================
    // Photometric data entities
    // =========================================================================

    /// Add IfcExternalReference for LDT/IES photometric file
    pub fn add_external_reference(
        &mut self,
        location: &str,
        identification: Option<&str>,
        name: &str,
    ) -> EntityRef {
        let ident = identification
            .map(|s| format!("'{}'", Self::escape_string(s)))
            .unwrap_or_else(|| "$".to_string());

        let ref_str = format!(
            "IFCEXTERNALREFERENCE('{}',{},'{}')",
            Self::escape_string(location),
            ident,
            Self::escape_string(name)
        );
        self.add_entity(ref_str)
    }

    /// Add IfcLightSourceGoniometric
    pub fn add_light_source_goniometric(
        &mut self,
        name: &str,
        colour_appearance: Option<(f64, f64, f64)>,
        colour_temperature: f64,
        luminous_flux: f64,
        emission_source: LightEmissionSourceEnum,
        photometry_file: Option<&str>,
    ) -> EntityRef {
        // Position at origin - create entities separately (Issue 1 fix)
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,-1.))".to_string()); // Light points down
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let position = self.add_axis2_placement_3d(origin, Some(axis), Some(ref_dir));

        // Colour appearance (IfcColourRgb)
        let colour_ref = if let Some((r, g, b)) = colour_appearance {
            let colour_str = format!("IFCCOLOURRGB($,{:.4},{:.4},{:.4})", r, g, b);
            let colour = self.add_entity(colour_str);
            OptionalRef::Some(colour)
        } else {
            OptionalRef::None
        };

        // External photometric reference
        let distribution_ref = if let Some(file_path) = photometry_file {
            let escaped_name = Self::escape_string(name);
            let ext_str = format!(
                "IFCEXTERNALREFERENCE('{}',$,'{}')",
                Self::escape_string(file_path),
                escaped_name
            );
            let ext_ref = self.add_entity(ext_str);
            OptionalRef::Some(ext_ref)
        } else {
            OptionalRef::None
        };

        // IfcLightSourceGoniometric
        let escaped_name = Self::escape_string(name);
        let ls_str = format!(
            "IFCLIGHTSOURCEGONIOMETRIC('{}',{},0.,1.,{},{},{:.1},{:.1},{},{})",
            escaped_name,
            colour_ref,
            position,
            colour_ref,
            colour_temperature,
            luminous_flux,
            emission_source.to_step(),
            distribution_ref
        );
        let light_source = self.add_entity(ls_str);

        // Track for later connection to fixture (Issue 2 fix)
        self.light_source_ids.push(light_source);

        light_source
    }

    /// Add IfcLightSourceGoniometric with embedded distribution data
    ///
    /// Instead of using an external file reference, this embeds the photometric
    /// distribution directly in the IFC using IFCLIGHTINTENSITYDISTRIBUTION.
    pub fn add_light_source_goniometric_with_distribution(
        &mut self,
        name: &str,
        colour_appearance: Option<(f64, f64, f64)>,
        colour_temperature: f64,
        luminous_flux: f64,
        emission_source: LightEmissionSourceEnum,
        distribution_data: &[(f64, f64, f64)],
    ) -> EntityRef {
        // Position at origin
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,-1.))".to_string());
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let position = self.add_axis2_placement_3d(origin, Some(axis), Some(ref_dir));

        // Colour appearance
        let colour_ref = if let Some((r, g, b)) = colour_appearance {
            let colour_str = format!("IFCCOLOURRGB($,{:.4},{:.4},{:.4})", r, g, b);
            let colour = self.add_entity(colour_str);
            OptionalRef::Some(colour)
        } else {
            OptionalRef::None
        };

        // Create embedded light intensity distribution
        let distribution_ref = if !distribution_data.is_empty() {
            let dist = self.add_light_intensity_distribution("TYPE_C", distribution_data);
            OptionalRef::Some(dist)
        } else {
            OptionalRef::None
        };

        // IfcLightSourceGoniometric
        let escaped_name = Self::escape_string(name);
        let ls_str = format!(
            "IFCLIGHTSOURCEGONIOMETRIC('{}',{},0.,1.,{},{},{:.1},{:.1},{},{})",
            escaped_name,
            colour_ref,
            position,
            colour_ref,
            colour_temperature,
            luminous_flux,
            emission_source.to_step(),
            distribution_ref
        );
        let light_source = self.add_entity(ls_str);

        // Track for later connection to fixture
        self.light_source_ids.push(light_source);

        light_source
    }

    /// Add IfcLightSourceGoniometric with a pre-existing distribution reference
    ///
    /// This allows sharing the same IFCLIGHTINTENSITYDISTRIBUTION across multiple light sources.
    pub fn add_light_source_goniometric_with_distribution_ref(
        &mut self,
        name: &str,
        colour_appearance: Option<(f64, f64, f64)>,
        colour_temperature: f64,
        luminous_flux: f64,
        emission_source: LightEmissionSourceEnum,
        distribution_ref: EntityRef,
    ) -> EntityRef {
        // Position at origin
        let origin = self.add_entity("IFCCARTESIANPOINT((0.,0.,0.))".to_string());
        let axis = self.add_entity("IFCDIRECTION((0.,0.,-1.))".to_string());
        let ref_dir = self.add_entity("IFCDIRECTION((1.,0.,0.))".to_string());
        let position = self.add_axis2_placement_3d(origin, Some(axis), Some(ref_dir));

        // Colour appearance
        let colour_ref = if let Some((r, g, b)) = colour_appearance {
            let colour_str = format!("IFCCOLOURRGB($,{:.4},{:.4},{:.4})", r, g, b);
            let colour = self.add_entity(colour_str);
            OptionalRef::Some(colour)
        } else {
            OptionalRef::None
        };

        // IfcLightSourceGoniometric with shared distribution reference
        let escaped_name = Self::escape_string(name);
        let ls_str = format!(
            "IFCLIGHTSOURCEGONIOMETRIC('{}',{},0.,1.,{},{},{:.1},{:.1},{},{})",
            escaped_name,
            colour_ref,
            position,
            colour_ref,
            colour_temperature,
            luminous_flux,
            emission_source.to_step(),
            distribution_ref
        );
        let light_source = self.add_entity(ls_str);

        // Track for later connection to fixture
        self.light_source_ids.push(light_source);

        light_source
    }

    /// Add IfcLightIntensityDistribution for embedded photometric data
    ///
    /// Distribution data format: Vec<(main_angle, secondary_angle, intensity)>
    /// This is grouped by main angle to create IFCLIGHTDISTRIBUTIONDATA entries
    pub fn add_light_intensity_distribution(
        &mut self,
        distribution_type: &str,
        distribution_data: &[(f64, f64, f64)],
    ) -> EntityRef {
        // Group data by main angle (C-plane)
        // IFC format: IFCLIGHTDISTRIBUTIONDATA(MainPlaneAngle, (SecondaryAngles...), (Intensities...))
        let mut grouped: std::collections::BTreeMap<i64, Vec<(f64, f64)>> =
            std::collections::BTreeMap::new();
        for &(main, secondary, intensity) in distribution_data {
            // Use integer key to group by main angle (avoid float comparison issues)
            let key = (main * 100.0) as i64;
            grouped.entry(key).or_default().push((secondary, intensity));
        }

        // Create distribution data entries for each C-plane
        let mut data_entries = Vec::new();
        for (main_key, values) in grouped {
            let main_angle = main_key as f64 / 100.0;

            // Sort by secondary angle (gamma)
            let mut sorted_values = values;
            sorted_values
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            // Build lists for secondary angles and intensities
            let secondary_list: Vec<String> = sorted_values
                .iter()
                .map(|(s, _)| format!("{:.2}", s))
                .collect();
            let intensity_list: Vec<String> = sorted_values
                .iter()
                .map(|(_, i)| format!("{:.4}", i))
                .collect();

            let entry_str = format!(
                "IFCLIGHTDISTRIBUTIONDATA({:.2},({}),({}))",
                main_angle,
                secondary_list.join(","),
                intensity_list.join(",")
            );
            let entry = self.add_entity(entry_str);
            data_entries.push(entry.to_string());
        }

        let data_list = if data_entries.is_empty() {
            "$".to_string()
        } else {
            format!("({})", data_entries.join(","))
        };

        let dist_str = format!(
            "IFCLIGHTINTENSITYDISTRIBUTION(.{}.,{})",
            distribution_type, data_list
        );
        self.add_entity(dist_str)
    }

    // =========================================================================
    // Property sets for light fixtures
    // =========================================================================

    /// Add Pset_LightFixtureTypeCommon property set
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
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfSources',$,IFCINTEGER({}),$)",
                n
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(w) = total_wattage {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('TotalWattage',$,IFCPOWERMEASURE({:.2}),$)",
                w
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(mt) = light_fixture_mounting_type {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('LightFixtureMountingType',$,IFCLABEL('{}'),$)",
                Self::escape_string(mt)
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(pt) = light_fixture_placement_type {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('LightFixturePlacementType',$,IFCLABEL('{}'),$)",
                Self::escape_string(pt)
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_LightFixtureTypeCommon',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

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
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('RatedVoltage',$,IFCELECTRICVOLTAGEMEASURE({:.1}),$)",
                v
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(i) = rated_current {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('RatedCurrent',$,IFCELECTRICCURRENTMEASURE({:.3}),$)",
                i
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(pf) = power_factor {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('PowerFactor',$,IFCNORMALISEDRATIOMEASURE({:.3}),$)",
                pf
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if let Some(ip) = ip_code {
            // Issue 5 fix: Don't double-prefix with IP
            let normalized_ip = ip.trim_start_matches("IP").trim_start_matches("ip");
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('IPCode',$,IFCLABEL('IP{}'),$)",
                Self::escape_string(normalized_ip)
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_ElectricalDeviceCommon',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Add comprehensive electrical device properties (Pset_ElectricalDeviceCommon)
    ///
    /// Full support for Electrical Information Exchange MVD (SPARKIE).
    /// See: https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/Pset_ElectricalDeviceCommon.htm
    pub fn add_electrical_device_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        props: &ElectricalDeviceProperties,
    ) -> EntityRef {
        let mut properties = Vec::new();

        // RatedCurrent (IfcElectricCurrentMeasure)
        if let Some(current) = props.rated_current {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('RatedCurrent',$,IFCELECTRICCURRENTMEASURE({:.3}),$)",
                current
            ));
            properties.push(prop);
        }

        // RatedVoltage (IfcElectricVoltageMeasure) - can be bounded
        if let Some(voltage) = props.rated_voltage {
            if let Some(voltage_max) = props.rated_voltage_max {
                // Bounded value with min/max
                let prop = self.add_entity(format!(
                    "IFCPROPERTYBOUNDEDVALUE('RatedVoltage',$,IFCELECTRICVOLTAGEMEASURE({:.1}),IFCELECTRICVOLTAGEMEASURE({:.1}),$,$)",
                    voltage, voltage_max
                ));
                properties.push(prop);
            } else {
                let prop = self.add_entity(format!(
                    "IFCPROPERTYSINGLEVALUE('RatedVoltage',$,IFCELECTRICVOLTAGEMEASURE({:.1}),$)",
                    voltage
                ));
                properties.push(prop);
            }
        }

        // NominalFrequencyRange (IfcFrequencyMeasure) - bounded
        if let (Some(freq_min), Some(freq_max)) =
            (props.nominal_frequency_min, props.nominal_frequency_max)
        {
            let prop = self.add_entity(format!(
                "IFCPROPERTYBOUNDEDVALUE('NominalFrequencyRange',$,IFCFREQUENCYMEASURE({:.1}),IFCFREQUENCYMEASURE({:.1}),$,$)",
                freq_min, freq_max
            ));
            properties.push(prop);
        }

        // PowerFactor (IfcNormalisedRatioMeasure) - 0.0 to 1.0
        if let Some(pf) = props.power_factor {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('PowerFactor',$,IFCNORMALISEDRATIOMEASURE({:.3}),$)",
                pf.clamp(0.0, 1.0)
            ));
            properties.push(prop);
        }

        // ConductorFunction (PEnum_ConductorFunctionEnum)
        if let Some(ref cf) = props.conductor_function {
            let prop = self.add_entity(format!(
                "IFCPROPERTYENUMERATEDVALUE('ConductorFunction',$,(IFCLABEL('{}')),{})",
                cf.to_step().trim_matches('.'),
                "$" // No enum reference needed for single value
            ));
            properties.push(prop);
        }

        // NumberOfPoles (IfcCountMeasure)
        if let Some(poles) = props.number_of_poles {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfPoles',$,IFCCOUNTMEASURE({}),$)",
                poles
            ));
            properties.push(prop);
        }

        // HasProtectiveEarth (IfcBoolean)
        if let Some(has_pe) = props.has_protective_earth {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('HasProtectiveEarth',$,IFCBOOLEAN({}),$)",
                if has_pe { ".T." } else { ".F." }
            ));
            properties.push(prop);
        }

        // InsulationStandardClass (PEnum_InsulationStandardClass)
        if let Some(ref isc) = props.insulation_standard_class {
            if *isc != InsulationStandardClass::NotDefined {
                let prop = self.add_entity(format!(
                    "IFCPROPERTYENUMERATEDVALUE('InsulationStandardClass',$,(IFCLABEL('{}')),{})",
                    isc.to_step().trim_matches('.'),
                    "$"
                ));
                properties.push(prop);
            }
        }

        // IP_Code (IfcLabel)
        if let Some(ref ip) = props.ip_code {
            // Normalize IP code - ensure it starts with "IP"
            let normalized_ip = if ip.to_uppercase().starts_with("IP") {
                ip.clone()
            } else {
                format!("IP{}", ip)
            };
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('IP_Code',$,IFCLABEL('{}'),$)",
                Self::escape_string(&normalized_ip)
            ));
            properties.push(prop);
        }

        // IK_Code (IfcLabel)
        if let Some(ref ik) = props.ik_code {
            // Normalize IK code - ensure it starts with "IK"
            let normalized_ik = if ik.to_uppercase().starts_with("IK") {
                ik.clone()
            } else {
                format!("IK{}", ik)
            };
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('IK_Code',$,IFCLABEL('{}'),$)",
                Self::escape_string(&normalized_ik)
            ));
            properties.push(prop);
        }

        // EarthingStyle (IfcLabel)
        if let Some(ref es) = props.earthing_style {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('EarthingStyle',$,IFCLABEL('{}'),$)",
                Self::escape_string(es)
            ));
            properties.push(prop);
        }

        // HeatDissipation (IfcPowerMeasure)
        if let Some(heat) = props.heat_dissipation {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('HeatDissipation',$,IFCPOWERMEASURE({:.2}),$)",
                heat
            ));
            properties.push(prop);
        }

        // Power (IfcPowerMeasure)
        if let Some(power) = props.power {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('Power',$,IFCPOWERMEASURE({:.2}),$)",
                power
            ));
            properties.push(prop);
        }

        // NominalPowerConsumption (IfcPowerMeasure)
        if let Some(npc) = props.nominal_power_consumption {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('NominalPowerConsumption',$,IFCPOWERMEASURE({:.2}),$)",
                npc
            ));
            properties.push(prop);
        }

        // NumberOfPowerSupplyPorts (IfcInteger)
        if let Some(ports) = props.number_of_power_supply_ports {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfPowerSupplyPorts',$,IFCINTEGER({}),$)",
                ports
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_ElectricalDeviceCommon',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        ));

        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        ));

        pset
    }

    /// Add light fixture common properties (Pset_LightFixtureTypeCommon)
    ///
    /// Full support for Electrical Information Exchange MVD.
    pub fn add_light_fixture_common_pset_full(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        props: &LightFixtureCommonProperties,
    ) -> EntityRef {
        let mut properties = Vec::new();

        // Reference (IfcIdentifier)
        if let Some(ref reference) = props.reference {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER('{}'),$)",
                Self::escape_string(reference)
            ));
            properties.push(prop);
        }

        // Status (PEnum_ElementStatus)
        if let Some(ref status) = props.status {
            let prop = self.add_entity(format!(
                "IFCPROPERTYENUMERATEDVALUE('Status',$,(IFCLABEL('{}')),$)",
                Self::escape_string(status)
            ));
            properties.push(prop);
        }

        // NumberOfSources (IfcInteger)
        if let Some(num) = props.number_of_sources {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('NumberOfSources',$,IFCINTEGER({}),$)",
                num
            ));
            properties.push(prop);
        }

        // TotalWattage (IfcPowerMeasure)
        if let Some(wattage) = props.total_wattage {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('TotalWattage',$,IFCPOWERMEASURE({:.2}),$)",
                wattage
            ));
            properties.push(prop);
        }

        // LightFixtureMountingType (PEnum_LightFixtureMountingType)
        if let Some(ref mounting) = props.mounting_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYENUMERATEDVALUE('LightFixtureMountingType',$,(IFCLABEL('{}')),$)",
                Self::escape_string(mounting)
            ));
            properties.push(prop);
        }

        // LightFixturePlacingType (PEnum_LightFixturePlacingType)
        if let Some(ref placing) = props.placing_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYENUMERATEDVALUE('LightFixturePlacingType',$,(IFCLABEL('{}')),$)",
                Self::escape_string(placing)
            ));
            properties.push(prop);
        }

        // MaintenanceFactor (IfcReal)
        if let Some(mf) = props.maintenance_factor {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('MaintenanceFactor',$,IFCREAL({:.3}),$)",
                mf
            ));
            properties.push(prop);
        }

        // MaximumPlenumSensibleLoad (IfcPowerMeasure)
        if let Some(load) = props.max_plenum_sensible_load {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('MaximumPlenumSensibleLoad',$,IFCPOWERMEASURE({:.2}),$)",
                load
            ));
            properties.push(prop);
        }

        // MaximumSpaceSensibleLoad (IfcPowerMeasure)
        if let Some(load) = props.max_space_sensible_load {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('MaximumSpaceSensibleLoad',$,IFCPOWERMEASURE({:.2}),$)",
                load
            ));
            properties.push(prop);
        }

        // SensibleLoadToRadiant (IfcPositiveRatioMeasure)
        if let Some(ratio) = props.sensible_load_to_radiant {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('SensibleLoadToRadiant',$,IFCPOSITIVERATIOMEASURE({:.3}),$)",
                ratio
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_LightFixtureTypeCommon',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        ));

        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        ));

        pset
    }

    /// Add manufacturer type information (Pset_ManufacturerTypeInformation) - COBie
    pub fn add_manufacturer_info_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        info: &ManufacturerTypeInfo,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(ref gtin) = info.global_trade_item_number {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('GlobalTradeItemNumber',$,IFCIDENTIFIER('{}'),$)",
                Self::escape_string(gtin)
            ));
            properties.push(prop);
        }

        if let Some(ref article) = info.article_number {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ArticleNumber',$,IFCIDENTIFIER('{}'),$)",
                Self::escape_string(article)
            ));
            properties.push(prop);
        }

        if let Some(ref model_ref) = info.model_reference {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('{}'),$)",
                Self::escape_string(model_ref)
            ));
            properties.push(prop);
        }

        if let Some(ref model_label) = info.model_label {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ModelLabel',$,IFCLABEL('{}'),$)",
                Self::escape_string(model_label)
            ));
            properties.push(prop);
        }

        if let Some(ref mfr) = info.manufacturer {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('{}'),$)",
                Self::escape_string(mfr)
            ));
            properties.push(prop);
        }

        if let Some(year) = info.production_year {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ProductionYear',$,IFCINTEGER({}),$)",
                year
            ));
            properties.push(prop);
        }

        if let Some(ref place) = info.assembly_place {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('AssemblyPlace',$,IFCLABEL('{}'),$)",
                Self::escape_string(place)
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_ManufacturerTypeInformation',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        ));

        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        ));

        pset
    }

    /// Add warranty information (Pset_Warranty) - COBie
    pub fn add_warranty_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        warranty: &WarrantyInfo,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(ref id) = warranty.warranty_identifier {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('WarrantyIdentifier',$,IFCIDENTIFIER('{}'),$)",
                Self::escape_string(id)
            ));
            properties.push(prop);
        }

        if let Some(ref start) = warranty.warranty_start_date {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('WarrantyStartDate',$,IFCDATE('{}'),$)",
                Self::escape_string(start)
            ));
            properties.push(prop);
        }

        if let Some(ref end) = warranty.warranty_end_date {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('WarrantyEndDate',$,IFCDATE('{}'),$)",
                Self::escape_string(end)
            ));
            properties.push(prop);
        }

        if let Some(period) = warranty.warranty_period {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('WarrantyPeriod',$,IFCREAL({:.1}),$)",
                period
            ));
            properties.push(prop);
        }

        if let Some(ref contact) = warranty.point_of_contact {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('PointOfContact',$,IFCLABEL('{}'),$)",
                Self::escape_string(contact)
            ));
            properties.push(prop);
        }

        if let Some(ref terms) = warranty.terms_and_conditions {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('TermsAndConditions',$,IFCTEXT('{}'),$)",
                Self::escape_string(terms)
            ));
            properties.push(prop);
        }

        if let Some(ref exclusions) = warranty.exclusions {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('Exclusions',$,IFCTEXT('{}'),$)",
                Self::escape_string(exclusions)
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_Warranty',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        ));

        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        ));

        pset
    }

    /// Add service life information (Pset_ServiceLife) - COBie
    pub fn add_service_life_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        service_life: &ServiceLifeInfo,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(duration) = service_life.service_life_duration {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ServiceLifeDuration',$,IFCREAL({:.1}),$)",
                duration
            ));
            properties.push(prop);
        }

        if let Some(ref sl_type) = service_life.service_life_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYENUMERATEDVALUE('ServiceLifeType',$,(IFCLABEL('{}')),$)",
                Self::escape_string(sl_type)
            ));
            properties.push(prop);
        }

        if let Some(mtbf) = service_life.mean_time_between_failure {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('MeanTimeBetweenFailure',$,IFCREAL({:.0}),$)",
                mtbf
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset = self.add_entity(format!(
            "IFCPROPERTYSET('{}',{},'Pset_ServiceLife',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        ));

        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        ));

        pset
    }

    /// Add a distribution port for electrical connection
    ///
    /// Creates an IfcDistributionPort to represent a power connection point on a light fixture.
    /// This enables circuit connectivity in Electrical MVD.
    pub fn add_distribution_port(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        parent_element: EntityRef,
        flow_direction: FlowDirectionEnum,
        system_type: DistributionSystemEnum,
    ) -> EntityRef {
        let guid = self.generate_guid();

        // Create the port
        let port = self.add_entity(format!(
            "IFCDISTRIBUTIONPORT('{}',{},'{}','Electrical connection port',$,$,{},{})",
            guid,
            owner_history,
            Self::escape_string(name),
            flow_direction.to_step(),
            system_type.to_step()
        ));

        // Nest the port within the parent element
        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELNESTS('{}',{},$,$,{},({}))",
            guid2, owner_history, parent_element, port
        ));

        port
    }

    /// Add a lighting distribution system
    ///
    /// Creates an IfcDistributionSystem to group light fixtures into a lighting circuit.
    pub fn add_lighting_system(
        &mut self,
        name: &str,
        owner_history: EntityRef,
        building: EntityRef,
    ) -> EntityRef {
        let guid = self.generate_guid();

        let system = self.add_entity(format!(
            "IFCDISTRIBUTIONSYSTEM('{}',{},'{}','Lighting distribution system',$,.LIGHTING.)",
            guid,
            owner_history,
            Self::escape_string(name)
        ));

        // Reference the system in the building
        let guid2 = self.generate_guid();
        self.add_entity(format!(
            "IFCRELSERVICESBUILDINGS('{}',{},$,$,{},({}))",
            guid2, owner_history, system, building
        ));

        system
    }

    /// Assign elements to a distribution system (e.g., light fixtures to lighting circuit)
    pub fn assign_to_system(
        &mut self,
        owner_history: EntityRef,
        system: EntityRef,
        elements: &[EntityRef],
    ) {
        if elements.is_empty() {
            return;
        }

        let guid = self.generate_guid();
        let element_list: Vec<String> = elements.iter().map(|e| e.to_string()).collect();

        self.add_entity(format!(
            "IFCRELASSIGNSTOGROUP('{}',{},$,$,({}),$.PRODUCT.,{})",
            guid,
            owner_history,
            element_list.join(","),
            system
        ));
    }

    /// Add variant-specific property set (Pset_LuminaireVariant)
    #[allow(clippy::too_many_arguments)]
    pub fn add_variant_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        variant_id: &str,
        cct: Option<i32>,
        luminous_flux: Option<f64>,
        power: Option<f64>,
        cri: Option<i32>,
        weight: Option<f64>,
    ) -> EntityRef {
        let mut properties = Vec::new();

        // GLDF Variant ID (for traceability)
        let variant_prop_str = format!(
            "IFCPROPERTYSINGLEVALUE('GLDF_VariantId',$,IFCLABEL('{}'),$)",
            Self::escape_string(variant_id)
        );
        let variant_prop = self.add_entity(variant_prop_str);
        properties.push(variant_prop);

        // Color Temperature
        if let Some(temp) = cct {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('ColorTemperature',$,IFCTHERMODYNAMICTEMPERATUREMEASURE({:.0}),$)",
                temp as f64
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        // Luminous Flux
        if let Some(flux) = luminous_flux {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('LuminousFlux',$,IFCLUMINOUSFLUXMEASURE({:.1}),$)",
                flux
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        // Power
        if let Some(p) = power {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('Power',$,IFCPOWERMEASURE({:.2}),$)",
                p
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        // Efficacy (calculated)
        if let (Some(flux), Some(p)) = (luminous_flux, power) {
            if p > 0.0 {
                let efficacy = flux / p;
                let prop_str = format!(
                    "IFCPROPERTYSINGLEVALUE('Efficacy',$,IFCREAL({:.1}),$)",
                    efficacy
                );
                let prop = self.add_entity(prop_str);
                properties.push(prop);
            }
        }

        // Color Rendering Index
        if let Some(index) = cri {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('ColorRenderingIndex',$,IFCINTEGER({}),$)",
                index
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        // Weight
        if let Some(w) = weight {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('Weight',$,IFCMASSMEASURE({:.3}),$)",
                w
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_LuminaireVariant',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Clear light source tracking for a new fixture type
    pub fn clear_light_sources(&mut self) {
        self.light_source_ids.clear();
    }

    /// Add photometry filenames property set (for roundtrip preservation)
    ///
    /// Stores original LDT filenames so they can be restored after IFC import.
    pub fn add_photometry_filenames_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        filenames: &[String],
    ) -> EntityRef {
        let mut properties = Vec::new();

        // Store each filename as a separate property
        for (i, filename) in filenames.iter().enumerate() {
            let prop_str = format!(
                "IFCPROPERTYSINGLEVALUE('PhotometryFile_{}',$,IFCLABEL('{}'),$)",
                i + 1,
                Self::escape_string(filename)
            );
            let prop = self.add_entity(prop_str);
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_GLDF_PhotometryFiles',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Add GLDF descriptive attributes property set
    ///
    /// Stores GLDF-specific descriptive attributes for roundtrip preservation:
    /// - ElectricalSafetyClass
    /// - MedianUsefulLife
    /// - MountingType (Ceiling/Wall/etc.)
    /// - RecessedDepth
    pub fn add_gldf_descriptive_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        safety_class: Option<&str>,
        median_useful_life: Option<&str>,
        mounting_type: Option<&str>,
        recessed_depth: Option<f64>,
    ) -> EntityRef {
        let mut properties = Vec::new();

        if let Some(sc) = safety_class {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('ElectricalSafetyClass',$,IFCLABEL('{}'),$)",
                Self::escape_string(sc)
            ));
            properties.push(prop);
        }

        if let Some(mul) = median_useful_life {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('MedianUsefulLife',$,IFCLABEL('{}'),$)",
                Self::escape_string(mul)
            ));
            properties.push(prop);
        }

        if let Some(mt) = mounting_type {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('GLDF_MountingType',$,IFCLABEL('{}'),$)",
                Self::escape_string(mt)
            ));
            properties.push(prop);
        }

        if let Some(rd) = recessed_depth {
            let prop = self.add_entity(format!(
                "IFCPROPERTYSINGLEVALUE('GLDF_RecessedDepth',$,IFCLENGTHMEASURE({:.1}),$)",
                rd
            ));
            properties.push(prop);
        }

        if properties.is_empty() {
            return EntityRef::new(0);
        }

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_GLDF_DescriptiveAttributes',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Add raw LDT file content as base64 for exact roundtrip preservation
    ///
    /// This stores the complete LDT file so it can be restored exactly.
    pub fn add_ldt_raw_content_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        index: usize,
        filename: &str,
        raw_content: &str,
    ) -> EntityRef {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let encoded = STANDARD.encode(raw_content.as_bytes());

        let filename_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Filename',$,IFCLABEL('{}'),$)",
            index,
            Self::escape_string(filename)
        ));

        let content_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Content',$,IFCLABEL('{}'),$)",
            index,
            Self::escape_string(&encoded)
        ));

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_GLDF_LDTRawContent',$,({},{}))",
            guid, owner_history, filename_prop, content_prop
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Add LDT metadata property set (for roundtrip preservation)
    ///
    /// Stores original LDT file metadata (symmetry, Mc, Ng, Dc, Dg, DR values, etc.)
    /// so they can be restored after IFC import.
    #[allow(clippy::too_many_arguments)]
    pub fn add_ldt_metadata_pset(
        &mut self,
        element: EntityRef,
        owner_history: EntityRef,
        index: usize,
        symmetry: i32,
        num_c_planes: i32,
        num_g_angles: i32,
        dc: f64,
        dg: f64,
        total_flux: f64,
        dr: &[f64; 10],
        luminaire_name: &str,
    ) -> EntityRef {
        let mut properties = Vec::new();

        // LDT index (for matching with photometry files)
        let idx_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Index',$,IFCINTEGER({}),$)",
            index, index
        ));
        properties.push(idx_prop);

        // Symmetry
        let sym_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Symmetry',$,IFCINTEGER({}),$)",
            index, symmetry
        ));
        properties.push(sym_prop);

        // Number of C-planes
        let mc_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_NumCPlanes',$,IFCINTEGER({}),$)",
            index, num_c_planes
        ));
        properties.push(mc_prop);

        // Number of gamma angles
        let ng_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_NumGAngles',$,IFCINTEGER({}),$)",
            index, num_g_angles
        ));
        properties.push(ng_prop);

        // C-plane distance (Dc)
        let dc_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Dc',$,IFCREAL({:.2}),$)",
            index, dc
        ));
        properties.push(dc_prop);

        // Gamma distance (Dg)
        let dg_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_Dg',$,IFCREAL({:.2}),$)",
            index, dg
        ));
        properties.push(dg_prop);

        // Total flux
        let flux_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_TotalFlux',$,IFCLUMINOUSFLUXMEASURE({:.1}),$)",
            index, total_flux
        ));
        properties.push(flux_prop);

        // DR values (as comma-separated string)
        let dr_str = dr
            .iter()
            .map(|v| format!("{:.5}", v))
            .collect::<Vec<_>>()
            .join(",");
        let dr_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_DR',$,IFCLABEL('{}'),$)",
            index,
            Self::escape_string(&dr_str)
        ));
        properties.push(dr_prop);

        // Luminaire name
        let name_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('LDT_{}_LuminaireName',$,IFCLABEL('{}'),$)",
            index,
            Self::escape_string(luminaire_name)
        ));
        properties.push(name_prop);

        let prop_list: Vec<String> = properties.iter().map(|p| p.to_string()).collect();

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_GLDF_LDTMetadata',$,({}))",
            guid,
            owner_history,
            prop_list.join(",")
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, element, pset
        );
        self.add_entity(rel_str);

        pset
    }

    /// Add a generic GLDF file as base64-encoded property set
    ///
    /// Used for preserving product images, sensor files, etc. through IFC roundtrip.
    pub fn add_gldf_file_pset(
        &mut self,
        project: EntityRef,
        owner_history: EntityRef,
        file_type: &str, // "image", "sensor", etc.
        filename: &str,
        content_type: &str, // "image/png", "sensor/sensldt", etc.
        content: &[u8],
    ) -> EntityRef {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let encoded = STANDARD.encode(content);

        let type_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('GLDF_FileType',$,IFCLABEL('{}'),$)",
            Self::escape_string(file_type)
        ));

        let filename_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('GLDF_Filename',$,IFCLABEL('{}'),$)",
            Self::escape_string(filename)
        ));

        let content_type_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('GLDF_ContentType',$,IFCLABEL('{}'),$)",
            Self::escape_string(content_type)
        ));

        let content_prop = self.add_entity(format!(
            "IFCPROPERTYSINGLEVALUE('GLDF_FileContent',$,IFCLABEL('{}'),$)",
            Self::escape_string(&encoded)
        ));

        let guid = self.generate_guid();
        let pset_str = format!(
            "IFCPROPERTYSET('{}',{},'Pset_GLDF_EmbeddedFile',$,({},{},{},{}))",
            guid, owner_history, type_prop, filename_prop, content_type_prop, content_prop
        );
        let pset = self.add_entity(pset_str);

        let guid2 = self.generate_guid();
        let rel_str = format!(
            "IFCRELDEFINESBYPROPERTIES('{}',{},$,$,({}),{})",
            guid2, owner_history, project, pset
        );
        self.add_entity(rel_str);

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
            writer.add_light_fixture("LED Downlight 001", oh, storey, Some(fixture_type), None);

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

        // Verify no inline entities (Issue 1)
        assert!(!output.contains("IFCLOCALPLACEMENT($,IFCAXIS2PLACEMENT3D"));

        // When fixture_type is provided without mesh_data, instance geometry should be $
        // (inheriting from type), not a separate geometry definition
        // Note: The type itself doesn't have geometry via add_light_fixture_type() without rep map,
        // but the instance correctly uses $ for representation when type is provided

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

        // Add goniometric light source
        let _light_source = writer.add_light_source_goniometric(
            "LED Panel Light Source",
            Some((1.0, 0.95, 0.9)),
            3000.0,
            4500.0,
            LightEmissionSourceEnum::Led,
            Some("photometry/led_panel.ldt"),
        );

        // Add common property set
        writer.add_light_fixture_common_pset(
            fixture_type,
            oh,
            Some(1),
            Some(36.0),
            Some("SURFACE"),
            Some("CEILING"),
        );

        // Add electrical property set
        writer.add_electrical_pset(
            fixture_type,
            oh,
            Some(230.0),
            Some(0.16),
            Some(0.95),
            Some("IP20"),
        );

        let _fixture =
            writer.add_light_fixture("LED Panel 001", oh, storey, Some(fixture_type), None);

        let output = writer.to_step_string();

        // Verify goniometric light source
        assert!(output.contains("IFCLIGHTSOURCEGONIOMETRIC"));
        assert!(output.contains("led_panel.ldt"));
        assert!(output.contains("3000.0"));
        assert!(output.contains("4500.0"));
        assert!(output.contains(".LED."));

        // Verify property sets
        assert!(output.contains("Pset_LightFixtureTypeCommon"));
        assert!(output.contains("NumberOfSources"));
        assert!(output.contains("TotalWattage"));

        assert!(output.contains("Pset_ElectricalDeviceCommon"));
        assert!(output.contains("RatedVoltage"));
        assert!(output.contains("PowerFactor"));
        assert!(output.contains("IPCode"));

        // Verify IP code is correct (Issue 5)
        assert!(output.contains("IFCLABEL('IP20')"));
        assert!(!output.contains("IPIP20"));

        // Verify light source is connected (Issue 2)
        assert!(output.contains("IFCRELASSIGNSTOGROUP"));

        println!("{}", output);
    }

    #[test]
    fn test_embedded_light_distribution() {
        let mut writer = StepWriter::new("IFC4");

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

    #[test]
    fn test_guid_format() {
        let mut writer = StepWriter::new("IFC4");

        for _ in 0..10 {
            let guid = writer.generate_guid();
            assert_eq!(
                guid.len(),
                22,
                "GUID should be exactly 22 characters: {}",
                guid
            );

            for c in guid.chars() {
                assert!(
                    c.is_ascii_alphanumeric() || c == '_' || c == '$',
                    "Invalid character in GUID: {} (full: {})",
                    c,
                    guid
                );
            }
        }
    }

    #[test]
    fn test_ip_code_normalization() {
        let mut writer = StepWriter::new("IFC4");
        let oh = writer.add_owner_history("Test");
        let project = writer.add_project("Test", oh);
        let site = writer.add_site("Site", oh, project);
        let building = writer.add_building("Building", oh, site);
        let _storey = writer.add_storey("Storey", oh, building);

        let fixture_type =
            writer.add_light_fixture_type("Test", "Test", LightFixtureTypeEnum::NotDefined, oh);

        // Test with "IP20" input
        writer.add_electrical_pset(fixture_type, oh, None, None, None, Some("IP20"));
        let output = writer.to_step_string();
        assert!(output.contains("IFCLABEL('IP20')"));
        assert!(!output.contains("IPIP"));

        // Test with "20" input (no prefix)
        let mut writer2 = StepWriter::new("IFC4");
        let oh2 = writer2.add_owner_history("Test");
        let project2 = writer2.add_project("Test", oh2);
        let site2 = writer2.add_site("Site", oh2, project2);
        let building2 = writer2.add_building("Building", oh2, site2);
        let _storey2 = writer2.add_storey("Storey", oh2, building2);
        let fixture_type2 =
            writer2.add_light_fixture_type("Test", "Test", LightFixtureTypeEnum::NotDefined, oh2);
        writer2.add_electrical_pset(fixture_type2, oh2, None, None, None, Some("20"));
        let output2 = writer2.to_step_string();
        assert!(output2.contains("IFCLABEL('IP20')"));
    }
}
