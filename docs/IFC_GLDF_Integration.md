# IFC-GLDF Integration Exploration

## Overview

[IFC (Industry Foundation Classes)](https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/) is the open standard for BIM (Building Information Modeling), published as [ISO 16739-1:2024](https://www.iso.org/standard/84123.html). GLDF (Global Lighting Data Format) is the open standard for lighting product data.

**Goal**: Enable bidirectional conversion between IFC and GLDF to allow:
1. GLDF luminaires to be placed in IFC building models
2. IFC light fixtures to be exported with full GLDF photometric data
3. Lighting analysis tools to consume either format

## IFC Lighting Entities

### [IfcLightFixture](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/IfcLightFixture.htm)

The container entity for luminaires in IFC:

```
IfcLightFixture : IfcFlowTerminal
├── PredefinedType: IfcLightFixtureTypeEnum
│   ├── POINTSOURCE
│   ├── DIRECTIONSOURCE
│   ├── SECURITYLIGHTING
│   └── USERDEFINED/NOTDEFINED
├── Property Sets:
│   ├── Pset_LightFixtureTypeCommon
│   │   ├── NumberOfSources
│   │   ├── TotalWattage
│   │   ├── LightFixtureMountingType
│   │   └── MaintenanceFactor
│   └── Pset_ManufacturerTypeInformation
│       ├── ArticleNumber
│       ├── ModelReference
│       └── ModelLabel
└── Representations:
    └── 'LightSource' → IfcLightSource subtypes
```

### [IfcLightSourceGoniometric](https://standards.buildingsmart.org/IFC/DEV/IFC4_2/FINAL/HTML/schema/ifcpresentationorganizationresource/lexical/ifclightsourcegoniometric.htm)

The photometric data carrier:

```
IfcLightSourceGoniometric : IfcLightSource
├── Position: IfcAxis2Placement3D
├── ColourAppearance: IfcColourRgb (optional)
├── ColourTemperature: IfcThermodynamicTemperatureMeasure (Kelvin)
├── LuminousFlux: IfcLuminousFluxMeasure (lumens)
├── LightEmissionSource: IfcLightEmissionSourceEnum
│   ├── COMPACTFLUORESCENT
│   ├── FLUORESCENT
│   ├── HIGHPRESSUREMERCURY
│   ├── HIGHPRESSURESODIUM
│   ├── LED
│   ├── LIGHTEMITTINGDIODE
│   ├── LOWPRESSURESODIUM
│   ├── LOWVOLTAGEHALOGEN
│   ├── MAINVOLTAGEHALOGEN
│   ├── METALHALIDE
│   └── TUNGSTENFILAMENT
└── LightDistributionDataSource: IfcLightDistributionDataSourceSelect
    ├── IfcExternalReference (→ IES/LDT file)
    └── IfcLightIntensityDistribution (inline data)
```

## Mapping GLDF → IFC

| GLDF Element | IFC Entity |
|-------------|-----------|
| `Header/Manufacturer` | `Pset_ManufacturerTypeInformation.Manufacturer` |
| `GeneralDefinitions/Files/File[@type='ldt']` | `IfcLightDistributionDataSource` → external reference |
| `ProductDefinitions/ProductMetaData/Name` | `IfcLightFixture.Name` |
| `LightSource/LuminousFlux` | `IfcLightSourceGoniometric.LuminousFlux` |
| `LightSource/ColorTemperature` | `IfcLightSourceGoniometric.ColourTemperature` |
| `LightSource/RatedInputPower` | `Pset_LightFixtureTypeCommon.TotalWattage` |
| `Geometry/Model3D` (L3D) | `IfcShapeRepresentation` (geometry) |

## Mapping IFC → GLDF

| IFC Entity | GLDF Element |
|-----------|-------------|
| `IfcLightFixture.Name` | `ProductDefinitions/ProductMetaData/Name` |
| `Pset_ManufacturerTypeInformation` | `Header` attributes |
| `IfcLightSourceGoniometric.LuminousFlux` | `LightSource/RatedLuminousFlux` |
| `IfcLightSourceGoniometric.ColourTemperature` | `LightSource/ColorTemperature` |
| `IfcExternalReference` (IES/LDT) | `GeneralDefinitions/Files/File` |
| `IfcShapeRepresentation` | Convert to L3D geometry |

## Rust Implementation Strategy

### Using ifc_rs Crate

The [ifc_rs](https://github.com/MetabuildDev/ifc_rs) crate provides IFC4 parsing:

```rust
use ifc_rs::{IFC, parser};

// Parse IFC file
let ifc = IFC::from_file("building.ifc")?;

// Query light fixtures
for entity in ifc.entities() {
    if let Some(light_fixture) = entity.as_light_fixture() {
        // Extract properties
        let name = light_fixture.name();
        let psets = light_fixture.property_sets();
        // ...
    }
}
```

### Proposed gldf-rs Extensions

```rust
// crates/gldf-rs-lib/src/ifc.rs

/// Convert GLDF to IFC light fixture
pub fn gldf_to_ifc(gldf: &GldfProduct) -> Result<IfcLightFixture, Error> {
    let mut fixture = IfcLightFixture::new();

    // Map header
    fixture.set_manufacturer(&gldf.header.manufacturer);

    // Map photometry
    if let Some(ldt) = gldf.get_photometry() {
        let gonio = IfcLightSourceGoniometric::new();
        gonio.set_luminous_flux(ldt.luminous_flux);
        gonio.set_colour_temperature(ldt.color_temperature);
        fixture.add_light_source(gonio);
    }

    Ok(fixture)
}

/// Convert IFC light fixture to GLDF
pub fn ifc_to_gldf(fixture: &IfcLightFixture) -> Result<GldfProduct, Error> {
    let mut gldf = GldfProduct::new();

    // Extract manufacturer info
    if let Some(pset) = fixture.get_property_set("Pset_ManufacturerTypeInformation") {
        gldf.header.manufacturer = pset.get("Manufacturer")?;
    }

    // Extract photometry
    if let Some(gonio) = fixture.get_goniometric_source() {
        // Create minimal LDT from goniometric data
        // or reference external IES/LDT file
    }

    Ok(gldf)
}
```

## Use Cases

### 1. BIM Lighting Design Workflow

```
Revit/ArchiCAD → Export IFC → gldf-rs → GLDF with full photometry
                                      ↓
                              DIALux/Relux analysis
```

### 2. Manufacturer Data Delivery

```
GLDF luminaire catalog → gldf-rs → IFC fixtures for BIM models
```

### 3. Lighting Analysis Integration

```
IFC building model + GLDF luminaires → Combined model for simulation
```

## Fundamental Differences

### 1. Purpose & Scope

| Aspect | GLDF | IFC |
|--------|------|-----|
| **Focus** | Complete luminaire product data | Building element in context |
| **Scope** | Single product, all variants | Many products, positioned in space |
| **Primary Use** | Manufacturer → Lighting designer | Architect → All trades |
| **Detail Level** | Deep photometric/electrical data | Minimal lighting properties |

### 2. Data Model Philosophy

**GLDF**: Product-centric
```
GldfProduct
├── Header (manufacturer, version)
├── GeneralDefinitions
│   ├── Files (LDT, IES, L3D, images)
│   ├── LightSources (detailed specs)
│   ├── Photometries (full distribution)
│   └── Geometries (3D models)
├── ProductDefinitions
│   ├── ProductMetaData
│   └── Variants (configurations)
└── Embedded files in ZIP container
```

**IFC**: Building-centric
```
IfcProject
├── IfcSite
│   └── IfcBuilding
│       └── IfcBuildingStorey
│           └── IfcSpace
│               └── IfcLightFixture (placed here)
│                   ├── ObjectPlacement (position in building)
│                   ├── Representation (simplified geometry)
│                   └── PropertySets (flat key-value)
```

### 3. Photometric Data

| GLDF (via LDT/IES) | IFC (IfcLightSourceGoniometric) |
|--------------------|--------------------------------|
| Full C-gamma table (thousands of values) | Optional external reference OR simplified inline |
| Multiple C-planes | Single distribution curve set |
| Spectral data possible | RGB color only |
| Detailed lamp data | Basic flux + color temp |
| Multiple photometries per product | One per light source |

### 4. Geometry Representation

| GLDF (L3D) | IFC |
|------------|-----|
| OBJ-based mesh in ZIP | BREP, tessellated, or CSG |
| Level of Detail (LOD) variants | Single representation |
| Joints for articulation | Static geometry |
| Separate electrical/light output parts | Combined or separate |

### 5. Key Conversion Challenges

```
GLDF → IFC Export:
┌─────────────────────────────────────────────────────────────┐
│ Challenge                    │ Solution                     │
├─────────────────────────────────────────────────────────────┤
│ No building context          │ Create standalone fixture    │
│                              │ (user places in BIM)         │
├─────────────────────────────────────────────────────────────┤
│ Multiple variants            │ Export each as IfcTypeObject │
│                              │ OR let user choose one       │
├─────────────────────────────────────────────────────────────┤
│ Rich photometry              │ Reference external LDT/IES   │
│                              │ (IfcExternalReference)       │
├─────────────────────────────────────────────────────────────┤
│ L3D geometry                 │ Convert OBJ → tessellated    │
│                              │ IfcTriangulatedFaceSet       │
├─────────────────────────────────────────────────────────────┤
│ Embedded files               │ Extract to filesystem        │
│                              │ (IFC references external)    │
└─────────────────────────────────────────────────────────────┘
```

## Challenges

1. **Geometry Conversion**: L3D (GLDF) ↔ BREP/tessellated (IFC) requires complex mesh operations
2. **Photometric Data**: IFC `IfcLightIntensityDistribution` is simplified compared to full LDT/IES
3. **Property Set Mapping**: Custom properties need standardized mapping rules
4. **Version Compatibility**: IFC 2x3 vs IFC4 vs IFC4.3 have different capabilities
5. **Context Loss**: GLDF is self-contained; IFC expects external file references
6. **Variant Explosion**: One GLDF with 50 variants → 50 IFC type objects?

## Implementation Strategy

### Option A: Direct STEP Generation (Recommended for MVP)

Write IFC STEP format directly without depending on ifc_rs (which lacks lighting entities):

```rust
// Generate minimal valid IFC STEP file
fn gldf_to_ifc_step(gldf: &GldfProduct) -> String {
    let mut step = StepWriter::new("IFC4");

    // Required entities
    let project = step.add_project("GLDF Export");
    let site = step.add_site(project);
    let building = step.add_building(site);
    let storey = step.add_storey(building);

    // Light fixture type (from GLDF product)
    let fixture_type = step.add_light_fixture_type(
        &gldf.name(),
        &gldf.manufacturer(),
        LightFixtureTypeEnum::PointSource,
    );

    // Light fixture occurrence
    let fixture = step.add_light_fixture(
        &format!("{}_001", gldf.name()),
        fixture_type,
        storey,
    );

    // Photometric data reference
    if let Some(ldt_path) = gldf.get_ldt_path() {
        step.add_goniometric_source(fixture, ldt_path);
    }

    step.to_string()
}
```

### Option B: IfcOpenShell Python Bridge

Call IfcOpenShell from Rust via PyO3 (gldf-rs-python already exists):

```python
# Python script called from Rust
import ifcopenshell
import ifcopenshell.api

def gldf_to_ifc(gldf_json: str, output_path: str):
    gldf = json.loads(gldf_json)
    model = ifcopenshell.file(schema="IFC4")

    # Create hierarchy...
    light_fixture = ifcopenshell.api.run(
        "root.create_entity", model,
        ifc_class="IfcLightFixture",
        name=gldf["name"]
    )
    # Add properties from GLDF...

    model.write(output_path)
```

### Option C: Contribute to ifc_rs

Add lighting entities to the ifc_rs crate:

```rust
// Would need to add to ifc_rs:
// - IfcLightFixture
// - IfcLightFixtureType
// - IfcLightSource
// - IfcLightSourceGoniometric
// - IfcExternalReference (for LDT/IES)
// - Pset_LightFixtureTypeCommon
```

### Recommended Approach

**Phase 1: Direct STEP generation** (minimal dependencies, full control)
- Write IFC STEP text format directly
- Include only entities needed for lighting
- Reference external LDT/IES files

**Phase 2: Validate with IfcOpenShell**
- Use Python tests to validate generated IFC files
- Ensure compatibility with BIM tools

**Phase 3: Consider ifc_rs contribution**
- If community interest, upstream the lighting entities

## Next Steps

1. Implement STEP text writer for minimal IFC structure
2. Add IfcLightFixture and IfcLightFixtureType generation
3. Add IfcLightSourceGoniometric with external LDT reference
4. Add property set generation (Pset_LightFixtureTypeCommon)
5. Create CLI tool: `gldf ifc-export input.gldf output.ifc`
6. Validate with IfcOpenShell and real BIM tools

## References

- [buildingSMART IFC Documentation](https://technical.buildingsmart.org/standards/ifc/)
- [IFC 4.3 IfcLightFixture](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/IfcLightFixture.htm)
- [ifc_rs Rust crate](https://docs.rs/ifc_rs/latest/ifc_rs/)
- [LIGMAN IFC BIM Files](https://www.ligman.com/ifc-bim-files/)
