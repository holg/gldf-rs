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

## Challenges

1. **Geometry Conversion**: L3D (GLDF) ↔ BREP/tessellated (IFC) requires complex mesh operations
2. **Photometric Data**: IFC `IfcLightIntensityDistribution` is simplified compared to full LDT/IES
3. **Property Set Mapping**: Custom properties need standardized mapping rules
4. **Version Compatibility**: IFC 2x3 vs IFC4 vs IFC4.3 have different capabilities

## Next Steps

1. Add `ifc_rs` as optional dependency
2. Implement GLDF → IFC export for basic fixtures
3. Implement IFC → GLDF import with photometry extraction
4. Add geometry conversion utilities (L3D ↔ IFC geometry)
5. Create CLI tool: `gldf ifc-export input.gldf output.ifc`

## References

- [buildingSMART IFC Documentation](https://technical.buildingsmart.org/standards/ifc/)
- [IFC 4.3 IfcLightFixture](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/IfcLightFixture.htm)
- [ifc_rs Rust crate](https://docs.rs/ifc_rs/latest/ifc_rs/)
- [LIGMAN IFC BIM Files](https://www.ligman.com/ifc-bim-files/)
