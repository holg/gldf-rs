//! Canonical XSD enumeration tables for the closed-list fields the editor
//! exposes. Source: `tests/data/gldf-1.0.0-rc-3.xsd`. These lists are the
//! exact strings the schema's `<xs:enumeration value="..."/>` declares —
//! editors save through these strings verbatim so the resulting GLDF
//! validates against the XSD regardless of the UI's display language.
//!
//! `LightDistribution` and `Application` already have their own modules
//! because they need full multi-language label tables; the values here
//! are typically code-like (`IK03`, `IP65`, `DALI Broadcast`, etc.) where
//! translation isn't useful, so we ship them without an i18n table.
//!
//! `is_canonical(value)` is the validation gate: viewers tag any
//! non-matching string with ⚠ so the user can spot legacy data that
//! would fail XSD validation.

/// `Mechanical/IKRating` — IEC 62262 impact protection grades. 12 values,
/// XSD lines ~3995-4007.
pub const IK_RATINGS: &[&str] = &[
    "IK00", "IK01", "IK02", "IK03", "IK04", "IK05", "IK06", "IK07", "IK08", "IK09", "IK10", "IK10+",
];

/// `Electrical/IngressProtectionIPCode` — IEC 60529. 47 distinct codes,
/// from `IP20` to `IP69K`, plus the legacy two-digit forms. XSD lines
/// ~4621-4669.
pub const IP_CODES: &[&str] = &[
    "IP20", "IP21", "IP22", "IP23", "IP24", "IP25", "IP26", "IP27", "IP28", //
    "IP30", "IP31", "IP32", "IP33", "IP34", "IP35", "IP36", "IP37", "IP38", //
    "IP40", "IP41", "IP42", "IP43", "IP44", "IP45", "IP46", "IP47", "IP48", //
    "IP50", "IP51", "IP52", "IP53", "IP54", "IP55", "IP56", "IP57", "IP58", //
    "IP60", "IP61", "IP62", "IP63", "IP64", "IP65", "IP66", "IP67", "IP68", //
    "IP69", "IP69K",
];

/// `Electrical/ElectricalSafetyClass` — IEC 61140. 5 values; we previously
/// shipped only `I/II/III` in the dropdown. The full set adds `0` (no
/// protective earth, no double insulation) and `0I` (basic insulation +
/// earth-only). XSD lines ~4480-4486.
pub const ELECTRICAL_SAFETY_CLASSES: &[&str] = &["0", "I", "0I", "II", "III"];

/// `ControlGear/Interfaces/Interface` — control-bus types. 12 values.
/// XSD lines ~2774-2785.
pub const INTERFACES: &[&str] = &[
    "DALI Broadcast",
    "DALI Addressable",
    "KNX",
    "0-10V",
    "1-10V",
    "230V",
    "RF",
    "WiFi",
    "Bluetooth",
    "Inter-Connection",
    "DMX",
    "DMX/RDM",
];

/// `LightEmitter@emergencyBehaviour` — already used by my photometry
/// resolver. 3 values declared 3× in the XSD (one per LightEmitter type)
/// but the canonical set is one. XSD lines ~3155, ~3203, ~3251.
pub const EMERGENCY_BEHAVIOURS: &[&str] = &["None", "Combined", "EmergencyOnly"];

/// `Marketing/Labels/Label` — quality / regulatory marks. 14 values.
/// XSD lines ~5085-5100.
pub const LABELS: &[&str] = &[
    "CE", "GS", "ENEC", "CCC", "VDE", "EAC", "D", "M", "MM", "F", "FF", "UL", "Handwarm",
    "IFS Food",
];

/// `Marketing/DedicatedEmergencyLightingType` — function of an emergency
/// luminaire. 6 values. XSD lines ~4731-4738.
pub const EMERGENCY_LIGHTING_TYPES: &[&str] = &[
    "Exit Light",
    "Guide Light",
    "Evacuation Light",
    "Reference Light",
    "For Signage",
    "For Lighting",
];

/// `Pictures/Image@imageType` — purpose of the embedded picture. 4
/// values. XSD lines ~5246-5251.
pub const IMAGE_TYPES: &[&str] = &[
    "Product Picture",
    "Application Picture",
    "Technical Sketch",
    "Other",
];

/// `Files/File@contentType` — declares what each embedded file is. 16
/// values. The `ldc/*` and `geo/*` prefixes are crucial — DIALux/Relux
/// importers refuse files where the contentType doesn't match the actual
/// payload. XSD lines ~547-562.
pub const CONTENT_TYPES: &[&str] = &[
    "ldc/eulumdat",
    "ldc/ies",
    "ldc/iesxml",
    "image/png",
    "image/svg",
    "image/jpg",
    "geo/l3d",
    "geo/r3d",
    "geo/m3d",
    "document/pdf",
    "symbol/dxf",
    "symbol/svg",
    "sensor/sensxml",
    "sensor/sensldt",
    "spectrum/text",
    "other",
];

// ──────────────────────────────────────────────────────────────────
// Layer B — completeness coverage. Canonical lists for every closed
// XSD enum we hadn't surfaced yet. These power both editor dropdowns
// (when the corresponding editor exists) and the post-load
// `validate_against_xsd_enums()` walker that surfaces ⚠ on legacy
// values across the whole document.
// ──────────────────────────────────────────────────────────────────

/// `Mechanical/Adjustabilities/Adjustability` — how the luminaire moves.
/// 7 values, XSD lines 4491-4497.
pub const ADJUSTABILITIES: &[&str] = &[
    "Fixed",
    "Orientation",
    "Turn",
    "Tilt",
    "Cardanic",
    "Height adjustable",
    "User defined",
];

/// `Mechanical/ProtectiveAreas/Area` — special-environment certifications.
/// 3 values, XSD lines 4537-4539.
pub const PROTECTIVE_AREAS: &[&str] = &[
    "Cleanroom suitable",
    "Ball-impact proof",
    "Drive/Roll-over proof",
];

/// `Mechanical/ProductForm` — closed shape enum. 10 values, XSD lines
/// 4461-4470. (Already wired in MechanicalEditor; surfacing the canonical
/// list here for the validator.)
pub const PRODUCT_FORMS: &[&str] = &[
    "Round", "Rounded", "Square", "Linear", "Areal", "Sphere", "Cuboid", "Cylinder", "Cone",
    "Special",
];

/// `Sensors/Sensor/DetectorType`. 4 values, XSD lines 668-671.
pub const DETECTOR_TYPES: &[&str] = &[
    "Motion Detector",
    "Presence Detector",
    "Daylight Detector",
    "Other",
];

/// `Sensors/Sensor/DetectionMethod`. 6 values, XSD lines 644-649.
pub const DETECTION_METHODS: &[&str] = &[
    "Passive Infrared",
    "High Frequency",
    "Microwave",
    "Ultrasonic",
    "Camera",
    "Other",
];

/// `Sensors/Sensor/DetectorCharacteristic`. 3 values, XSD lines 623-625.
pub const DETECTOR_CHARACTERISTICS: &[&str] = &["Round", "Square", "Other"];

/// `LightSource/Cie97LampType` — CIE 97 maintenance category for the
/// lamp.
///
/// Two of the historical XSD values contained the misspelling
/// `Flourescent` (missing the second `u`). rc.4 introduces the corrected
/// `Fluorescent` forms while keeping the typo'd ones as deprecated
/// aliases (removal target: 2.0.0). gldf-rs-lib's parser upconverts
/// inbound files so the in-memory model always carries the corrected
/// spelling; the rc.3 export path downconverts on the way out. So
/// **only the corrected forms appear in this list** — anything else
/// reaching this layer is genuinely off-spec.
pub const CIE97_LAMP_TYPES: &[&str] = &[
    "Incandescent",
    "Halogen",
    "Fluorescent Triphosphor",
    "Fluorescent Halophosphate",
    "Compact Fluorescent",
    "Mercury",
    "Metal Halide (250/400W)",
    "Ceramic Metal Halide (50/150W)",
    "High Pressure Sodium (250/400W)",
];

/// `Cie97LuminaireType` — CIE 97 maintenance category for the luminaire
/// housing. 7 values, XSD lines 3652-3658.
pub const CIE97_LUMINAIRE_TYPES: &[&str] = &[
    "Bare Batten",
    "Open Top Housing (Natural Ventilated)",
    "Closed Top Housing (Unventilated)",
    "Enclosed IP2X",
    "Dust Proof IP5X",
    "Enclosed Indirect (Uplight)",
    "Airhandling, Forced Ventilated",
];

/// `roomCondition` attribute on the CIE 97 maintenance block. The XSD
/// declares this 3× with overlapping but not identical sets (4 / 5 / 4
/// values per site); the union below covers all sites. 6 distinct
/// values total, XSD lines 3691-3733.
pub const ROOM_CONDITIONS: &[&str] = &[
    "Very Clean",
    "Clean",
    "Normal",
    "Moderate",
    "Dirty",
    "Very Dirty",
];

/// `InitialColorTolerance` / `MaintainedColorTolerance` — MacAdam ellipse
/// step ratings. Same 7 values declared 8× across the XSD (different
/// LightSource subtypes); one canonical set covers all sites.
pub const COLOR_TOLERANCES: &[&str] = &[
    "1 SDCM", "2 SDCM", "3 SDCM", "4 SDCM", "5 SDCM", "6 SDCM", "7 SDCM",
];

/// `EmitterReference@targetModelType` — which 3D model format the
/// EmitterObjectExternalName resolves against. 3 values, XSD lines
/// 4288-4290.
pub const TARGET_MODEL_TYPES: &[&str] = &["l3d", "m3d", "r3d"];

/// `Geometry@levelOfDetail` — model-fidelity hint. 3 values, XSD lines
/// 3582-3584.
pub const LEVELS_OF_DETAIL: &[&str] = &["Low", "Medium", "High"];

/// `MelanopicFactor@type` — color-channel label. 10 values, XSD line
/// 2592 area.
pub const MELANOPIC_CHANNEL_TYPES: &[&str] = &[
    "Red",
    "Green",
    "Blue",
    "Amber",
    "White",
    "CoolWhite",
    "WarmWhite",
    "DaylightWhite",
    "NaturalWhite",
    "NeutralWhite",
];

/// `ActivePowerTable/FluxFactor@type`. 2 values.
pub const FLUX_FACTOR_TYPES: &[&str] = &["Steps", "Continuously"];

/// `File@type` — local file vs URL reference. 2 values, XSD lines 572-573.
pub const FILE_TYPES: &[&str] = &["localFileName", "url"];

/// `Voltage@Type` — current type. 3 values; also bound in Rust as the
/// `CurrentType` enum on the lib side (compile-checked).
pub const CURRENT_TYPES: &[&str] = &["AC", "DC", "UC"];

/// `Voltage@Frequency` — mains frequency. 4 values; also bound as the
/// `Frequency` Rust enum on the lib side.
pub const FREQUENCIES: &[&str] = &["50", "60", "50/60", "400"];

// ── Hazardous-area (ATEX/IECEx/CEC/NEC) family ───────────────────────
//
// These appear under `RegulatoryEquipment/HazardousArea/...`. Several
// share an element name across contexts (e.g. `Group` differs between
// Gas and Dust subtrees). The validator uses the *union* of all valid
// values per element name (lenient — won't flag a context-correct value
// just because we don't know which subtree the field came from). When
// editors are added, they should use the per-context subset directly.

/// `RegulatoryEquipment/Directive` — which approval directive applies.
/// 4 values, XSD lines 5075-5078.
pub const DIRECTIVES: &[&str] = &["ATEX", "IECEx", "CEC", "NEC"];

/// `Class` — IEC 61140 class within hazardous-area context (CEC/NEC
/// classify gas/dust/fiber by Class I/II/III). 3 values.
pub const HAZARDOUS_CLASSES: &[&str] = &["I", "II", "III"];

/// `Division` — CEC/NEC explosion-likelihood division. 2 values.
pub const DIVISIONS: &[&str] = &["1", "2"];

/// `Group` — union across all four contexts (Gas A-D, Dust E-G,
/// IECEx I/II/IIA/IIB/IIC/IIIA/IIIB/IIIC, plus IIB+H2). 17 unique
/// values across the XSD.
///
/// Editors should use the **per-context subset** instead. The lenient
/// union here is for the validator only.
pub const HAZARDOUS_GROUPS_UNION: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "I", "II", "IIA", "IIB", "IIB + H2", "IIC", "III", "IIIA",
    "IIIB", "IIIC",
];

/// `Zone` — union of Gas (0/1/2) and Dust (20/21/22) zones. 6 values.
pub const HAZARDOUS_ZONES: &[&str] = &["0", "1", "2", "20", "21", "22"];

/// `TemperatureClass` — auto-ignition temperature rating. 14 values.
pub const TEMPERATURE_CLASSES: &[&str] = &[
    "T1", "T2", "T2A", "T2B", "T2C", "T2D", "T3", "T3A", "T3B", "T3C", "T4", "T4A", "T5", "T6",
];

/// `ExCode` — IEC 60079 protection-method code. 26 values.
pub const EX_CODES: &[&str] = &[
    "da", "db", "dc", "eb", "ec", "ia", "ib", "ic", "ma", "mb", "mc", "nC", "nR", "ob", "oc",
    "op is", "op pr", "op sh", "pxb", "pyb", "pyc", "pzc", "q", "ta", "tb", "tc",
];

/// `EquipmentProtectionLevel` — IEC 60079 EPL. 8 values.
pub const EQUIPMENT_PROTECTION_LEVELS: &[&str] = &["Ga", "Gb", "Gc", "Da", "Db", "Dc", "Ma", "Mb"];

/// `EquipmentGroup`. 2 values: I (mining), II (other).
pub const EQUIPMENT_GROUPS: &[&str] = &["I", "II"];

/// `EquipmentCategory` — ATEX category. 8 values (M1, M2 for mining;
/// 1G/2G/3G for gas; 1D/2D/3D for dust).
pub const EQUIPMENT_CATEGORIES: &[&str] = &["M1", "M2", "1G", "2G", "3G", "1D", "2D", "3D"];

/// `Atmosphere`. 2 values: G (gas), D (dust).
pub const ATMOSPHERES: &[&str] = &["G", "D"];

/// `Electrical/LightDistribution` — closed photometric-distribution
/// taxonomy. 15 values, XSD lines 4670-4692.
///
/// The viewer's `i18n::light_distribution` module pairs these strings
/// with translated labels for display; this lib-side list is the bare
/// canonical set for validation and any non-UI consumer.
pub const LIGHT_DISTRIBUTIONS: &[&str] = &[
    "Laterally symmetrical narrow",
    "Laterally symmetrical medium",
    "Laterally symmetrical wide",
    "Symmetrical in each quadrant",
    "Symmetric about 0-180 plane",
    "Symmetric about 90-270 plane",
    "Asymmetrical",
    "Asymmetrical flood",
    "Asymmetrical wide flood",
    "Diffuse half spherical",
    "Diffuse full spherical",
    "Direct",
    "Indirect",
    "Direct indirect",
    "Other",
];

/// `Marketing/Applications/Application` — closed application-area
/// taxonomy. 79 hierarchical strings (`A: B: C` style), XSD lines
/// 4908-4986. The viewer's `i18n::applications` module pairs these
/// with translated labels; this list is the bare canonical set.
pub const APPLICATIONS: &[&str] = &[
    "Interior: Traffic Zones",
    "Interior: Traffic Zones: Corridors",
    "Interior: Traffic Zones: Staircases",
    "Interior: Traffic Zones: Loading Zones",
    "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
    "Interior: General Areas (Interior)",
    "Interior: General Areas (Interior): Break Rooms",
    "Interior: General Areas (Interior): Reception Areas",
    "Interior: Office",
    "Interior: Office: Office Desks",
    "Interior: Office: Group Offices",
    "Interior: Office: Discussions",
    "Interior: Office: Archives",
    "Interior: Industry/Craft",
    "Interior: Industry/Craft: Industrial Workshops",
    "Interior: Industry/Craft: Warehouses",
    "Interior: Industry/Craft: Cold Storage Facilities",
    "Interior: Industry/Craft: Kitchens",
    "Interior: Industry/Craft: Assembly Work Stations",
    "Interior: Industry/Craft: Machine Illumination",
    "Interior: Industry/Craft: Control Work Stations",
    "Interior: Industry/Craft: Laboratories",
    "Interior: Industry/Craft: Hangars",
    "Interior: Shop Lighting",
    "Interior: Shop Lighting: Retail",
    "Interior: Shop Lighting: Food",
    "Interior: Shop Lighting: Clothing",
    "Interior: Shop Lighting: Display Windows",
    "Interior: Shop Lighting: Halls",
    "Interior: Shop Lighting: Great Halls",
    "Interior: Shop Lighting: Mirrors",
    "Interior: Public Areas",
    "Interior: Public Areas: Restaurants",
    "Interior: Public Areas: Theatres",
    "Interior: Public Areas: Railway Stations",
    "Interior: Public Areas: Museums",
    "Interior: Public Areas: Fairs",
    "Interior: Public Areas: Prisons",
    "Interior: Public Areas: Canteens",
    "Interior: Emergency Lighting",
    "Interior: Emergency Lighting: Emergency Lighting",
    "Interior: Emergency Lighting: Signal Lighting",
    "Interior: Educational Facilities",
    "Interior: Educational Facilities: Classrooms",
    "Interior: Educational Facilities: Libraries",
    "Interior: Educational Facilities: Lounges",
    "Interior: Educational Facilities: Sports Halls",
    "Interior: Private Areas",
    "Interior: Private Areas: Living Areas",
    "Interior: Private Areas: Baths",
    "Interior: Private Areas: Kitchens",
    "Interior: Hospitals and Care Places",
    "Interior: Hospitals and Care Places: Hospital Wards",
    "Interior: Hospitals and Care Places: Health Care Patient Rooms",
    "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
    "Interior: Hospitals and Care Places: Health Care Examination Rooms",
    "Interior: Hospitals and Care Places: Health Care Circulation Areas",
    "Exterior: General Areas (Exterior)",
    "Exterior: General Areas (Exterior): Places",
    "Exterior: General Areas (Exterior): Parks",
    "Exterior: General Areas (Exterior): Underpasses",
    "Exterior: General Areas (Exterior): (Outdoor) Stairs",
    "Exterior: General Areas (Exterior): Platform-Roofs",
    "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
    "Exterior: General Areas (Exterior): Outdoor Parkings",
    "Exterior: General Areas (Exterior): Pools",
    "Exterior: General Areas (Exterior): Fountains",
    "Exterior: Streets",
    "Exterior: Streets: Motorways",
    "Exterior: Streets: Access Roads",
    "Exterior: Streets: Residential Areas",
    "Exterior: Streets: Bicycle Paths",
    "Exterior: Streets: Footpaths",
    "Exterior: Streets: Petrol-Gas Stations",
    "Exterior: Streets: Tunnels",
    "Exterior: Sports Fields",
    "Exterior: Sports Fields: Spotlightings",
    "Exterior: Other",
    "Exterior: Other: Facades",
];

// ─── Validator surface ────────────────────────────────────────────────

/// Quick membership check, used by viewers to tag legacy/non-canonical
/// values with ⚠.
pub fn is_canonical(canonical_set: &'static [&'static str], value: &str) -> bool {
    canonical_set.iter().any(|v| *v == value)
}

/// Map an XSD element/attribute name to its canonical-values set. Used
/// by the post-load validator to walk the document and flag legacy
/// values that would fail XSD validation. Returns `None` for names that
/// aren't closed enums (free strings like `IKRating` are intentionally
/// not validated against a list — they're already a `<select>` in the
/// editor).
///
/// For context-dependent enums (`Group`, `Class`, `Zone`) the lookup
/// returns the lenient *union* across all contexts — see the per-set
/// docs above.
pub fn canonical_set_for(xsd_name: &str) -> Option<&'static [&'static str]> {
    match xsd_name {
        // Layer A
        "IKRating" => Some(IK_RATINGS),
        "IngressProtectionIPCode" => Some(IP_CODES),
        "ElectricalSafetyClass" => Some(ELECTRICAL_SAFETY_CLASSES),
        "Interface" => Some(INTERFACES),
        "@emergencyBehaviour" => Some(EMERGENCY_BEHAVIOURS),
        "Label" => Some(LABELS),
        "DedicatedEmergencyLightingType" => Some(EMERGENCY_LIGHTING_TYPES),
        "@imageType" => Some(IMAGE_TYPES),
        "@contentType" => Some(CONTENT_TYPES),
        // Layer B
        "Adjustability" => Some(ADJUSTABILITIES),
        "Area" => Some(PROTECTIVE_AREAS),
        "ProductForm" => Some(PRODUCT_FORMS),
        "DetectorType" => Some(DETECTOR_TYPES),
        "DetectionMethod" => Some(DETECTION_METHODS),
        "DetectorCharacteristic" => Some(DETECTOR_CHARACTERISTICS),
        "Cie97LampType" => Some(CIE97_LAMP_TYPES),
        "Cie97LuminaireType" => Some(CIE97_LUMINAIRE_TYPES),
        "@roomCondition" => Some(ROOM_CONDITIONS),
        "InitialColorTolerance" | "MaintainedColorTolerance" => Some(COLOR_TOLERANCES),
        "@targetModelType" => Some(TARGET_MODEL_TYPES),
        "@levelOfDetail" => Some(LEVELS_OF_DETAIL),
        // Hazardous-area family
        "Directive" => Some(DIRECTIVES),
        "Class" => Some(HAZARDOUS_CLASSES),
        "Division" => Some(DIVISIONS),
        "Group" => Some(HAZARDOUS_GROUPS_UNION),
        "Zone" => Some(HAZARDOUS_ZONES),
        "TemperatureClass" => Some(TEMPERATURE_CLASSES),
        "ExCode" => Some(EX_CODES),
        "EquipmentProtectionLevel" => Some(EQUIPMENT_PROTECTION_LEVELS),
        "EquipmentGroup" => Some(EQUIPMENT_GROUPS),
        "EquipmentCategory" => Some(EQUIPMENT_CATEGORIES),
        "Atmosphere" => Some(ATMOSPHERES),
        // Compile-checked Rust-enum types — validated at parse time.
        "@Frequency" => Some(FREQUENCIES),
        "@Type" => Some(CURRENT_TYPES),
        // Polymorphic `type` attribute — caller must qualify by context.
        // Use FILE_TYPES, FLUX_FACTOR_TYPES, or MELANOPIC_CHANNEL_TYPES
        // directly when you know which one applies.
        // The viewer's i18n modules carry translated labels for these,
        // but the bare canonical strings live here so the validator
        // works without any UI dependency.
        "LightDistribution" => Some(LIGHT_DISTRIBUTIONS),
        "Application" => Some(APPLICATIONS),
        _ => None,
    }
}

/// Result of validating a single field against its canonical set.
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalCheck {
    /// Value matches one of the canonical XSD enum strings.
    Canonical,
    /// Value is non-empty and not in the canonical set — schema-invalid.
    NonCanonical,
    /// Field is absent or empty; nothing to validate.
    Empty,
}

/// Validate a single value against the XSD enum identified by name.
pub fn check(xsd_name: &str, value: Option<&str>) -> CanonicalCheck {
    let v = match value {
        Some(s) if !s.is_empty() => s,
        _ => return CanonicalCheck::Empty,
    };
    match canonical_set_for(xsd_name) {
        Some(set) if is_canonical(set, v) => CanonicalCheck::Canonical,
        Some(_) => CanonicalCheck::NonCanonical,
        None => CanonicalCheck::Empty, // No canonical set known — pass.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every canonical list must round-trip a representative value through
    /// `is_canonical` and reject a clearly-bogus one.
    #[test]
    fn membership_checks() {
        assert!(is_canonical(IK_RATINGS, "IK03"));
        assert!(!is_canonical(IK_RATINGS, "IK11"));
        assert!(is_canonical(IP_CODES, "IP65"));
        assert!(!is_canonical(IP_CODES, "IP99"));
        assert!(is_canonical(ELECTRICAL_SAFETY_CLASSES, "0"));
        assert!(is_canonical(ELECTRICAL_SAFETY_CLASSES, "0I"));
        assert!(!is_canonical(ELECTRICAL_SAFETY_CLASSES, "IV"));
        assert!(is_canonical(INTERFACES, "DALI Broadcast"));
        assert!(!is_canonical(INTERFACES, "DALI Mesh"));
        assert!(is_canonical(LABELS, "CE"));
        assert!(is_canonical(EMERGENCY_BEHAVIOURS, "EmergencyOnly"));
        assert!(is_canonical(EMERGENCY_LIGHTING_TYPES, "Exit Light"));
        assert!(is_canonical(IMAGE_TYPES, "Product Picture"));
        assert!(is_canonical(CONTENT_TYPES, "ldc/eulumdat"));
        assert!(!is_canonical(CONTENT_TYPES, "ldc/dialux"));
    }
}
