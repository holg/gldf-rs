//! Translation table for the `Marketing/Applications/Application` XSD
//! enumeration (gldf-1.0.0-rc-3.xsd lines 4902–4970).
//!
//! Each canonical entry is a hierarchical string with `:` separators
//! (e.g. `"Interior: Office: Group Offices"`). Translations preserve
//! the `:` structure so the viewer's existing path/leaf split logic
//! continues to work unchanged — only the displayed text changes.
//!
//! The XSD declares 79 unique application areas; the file's emitter
//! must use one of these exact strings (the value is `xs:string` with
//! `xs:enumeration` restriction). When this viewer ships a localised
//! string for a value, [`display_label`] returns it; otherwise it
//! returns the canonical English form so the user still sees something
//! meaningful and the path structure stays aligned.

/// The 79 canonical XSD enum values, in spec order.
pub const CANONICAL_VALUES: &[&str] = &[
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

/// Borrow the canonical list (e.g. for an editor dropdown).
pub fn canonical_values() -> &'static [&'static str] {
    CANONICAL_VALUES
}

/// Resolve the human-readable label for a canonical XSD value in the
/// chosen UI language. Falls back to the canonical (English) form when
/// the language code isn't supported or has no entry for this value.
///
/// Translations preserve the `:` separator structure so the viewer's
/// hierarchical-path split logic (`rsplit(':')`) renders the same way
/// regardless of the active UI language.
pub fn display_label(canonical: &str, ui_language: &str) -> String {
    if ui_language == "en" {
        return canonical.to_string();
    }
    let table = match ui_language {
        "de" => DE,
        "es" => ES,
        "fr" => FR,
        "it" => IT,
        "ru" => RU,
        "zh" => ZH,
        _ => return canonical.to_string(),
    };
    for (k, v) in table {
        if *k == canonical {
            return v.to_string();
        }
    }
    canonical.to_string()
}

const DE: &[(&str, &str)] = &[
    ("Interior: Traffic Zones", "Innen: Verkehrszonen"),
    (
        "Interior: Traffic Zones: Corridors",
        "Innen: Verkehrszonen: Flure",
    ),
    (
        "Interior: Traffic Zones: Staircases",
        "Innen: Verkehrszonen: Treppenhäuser",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "Innen: Verkehrszonen: Ladezonen",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "Innen: Verkehrszonen: Voutenbeleuchtung / Sims (Innen)",
    ),
    (
        "Interior: General Areas (Interior)",
        "Innen: Allgemeine Bereiche (Innen)",
    ),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "Innen: Allgemeine Bereiche (Innen): Pausenräume",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "Innen: Allgemeine Bereiche (Innen): Empfangsbereiche",
    ),
    ("Interior: Office", "Innen: Büro"),
    (
        "Interior: Office: Office Desks",
        "Innen: Büro: Schreibtische",
    ),
    (
        "Interior: Office: Group Offices",
        "Innen: Büro: Großraumbüros",
    ),
    (
        "Interior: Office: Discussions",
        "Innen: Büro: Besprechungsräume",
    ),
    ("Interior: Office: Archives", "Innen: Büro: Archive"),
    ("Interior: Industry/Craft", "Innen: Industrie/Handwerk"),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "Innen: Industrie/Handwerk: Industriewerkstätten",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "Innen: Industrie/Handwerk: Lagerhallen",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "Innen: Industrie/Handwerk: Kühllager",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "Innen: Industrie/Handwerk: Großküchen",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "Innen: Industrie/Handwerk: Montagearbeitsplätze",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "Innen: Industrie/Handwerk: Maschinenbeleuchtung",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "Innen: Industrie/Handwerk: Kontrollarbeitsplätze",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "Innen: Industrie/Handwerk: Laboratorien",
    ),
    (
        "Interior: Industry/Craft: Hangars",
        "Innen: Industrie/Handwerk: Hangars",
    ),
    ("Interior: Shop Lighting", "Innen: Verkaufsbeleuchtung"),
    (
        "Interior: Shop Lighting: Retail",
        "Innen: Verkaufsbeleuchtung: Einzelhandel",
    ),
    (
        "Interior: Shop Lighting: Food",
        "Innen: Verkaufsbeleuchtung: Lebensmittel",
    ),
    (
        "Interior: Shop Lighting: Clothing",
        "Innen: Verkaufsbeleuchtung: Bekleidung",
    ),
    (
        "Interior: Shop Lighting: Display Windows",
        "Innen: Verkaufsbeleuchtung: Schaufenster",
    ),
    (
        "Interior: Shop Lighting: Halls",
        "Innen: Verkaufsbeleuchtung: Hallen",
    ),
    (
        "Interior: Shop Lighting: Great Halls",
        "Innen: Verkaufsbeleuchtung: Große Hallen",
    ),
    (
        "Interior: Shop Lighting: Mirrors",
        "Innen: Verkaufsbeleuchtung: Spiegel",
    ),
    ("Interior: Public Areas", "Innen: Öffentliche Bereiche"),
    (
        "Interior: Public Areas: Restaurants",
        "Innen: Öffentliche Bereiche: Restaurants",
    ),
    (
        "Interior: Public Areas: Theatres",
        "Innen: Öffentliche Bereiche: Theater",
    ),
    (
        "Interior: Public Areas: Railway Stations",
        "Innen: Öffentliche Bereiche: Bahnhöfe",
    ),
    (
        "Interior: Public Areas: Museums",
        "Innen: Öffentliche Bereiche: Museen",
    ),
    (
        "Interior: Public Areas: Fairs",
        "Innen: Öffentliche Bereiche: Messen",
    ),
    (
        "Interior: Public Areas: Prisons",
        "Innen: Öffentliche Bereiche: Gefängnisse",
    ),
    (
        "Interior: Public Areas: Canteens",
        "Innen: Öffentliche Bereiche: Kantinen",
    ),
    ("Interior: Emergency Lighting", "Innen: Notbeleuchtung"),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "Innen: Notbeleuchtung: Notbeleuchtung",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "Innen: Notbeleuchtung: Signalbeleuchtung",
    ),
    (
        "Interior: Educational Facilities",
        "Innen: Bildungseinrichtungen",
    ),
    (
        "Interior: Educational Facilities: Classrooms",
        "Innen: Bildungseinrichtungen: Klassenzimmer",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "Innen: Bildungseinrichtungen: Bibliotheken",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "Innen: Bildungseinrichtungen: Aufenthaltsräume",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "Innen: Bildungseinrichtungen: Sporthallen",
    ),
    ("Interior: Private Areas", "Innen: Private Bereiche"),
    (
        "Interior: Private Areas: Living Areas",
        "Innen: Private Bereiche: Wohnbereiche",
    ),
    (
        "Interior: Private Areas: Baths",
        "Innen: Private Bereiche: Bäder",
    ),
    (
        "Interior: Private Areas: Kitchens",
        "Innen: Private Bereiche: Wohnküchen",
    ),
    (
        "Interior: Hospitals and Care Places",
        "Innen: Krankenhäuser und Pflegeeinrichtungen",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "Innen: Krankenhäuser und Pflegeeinrichtungen: Krankenstationen",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "Innen: Krankenhäuser und Pflegeeinrichtungen: Patientenzimmer",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "Innen: Krankenhäuser und Pflegeeinrichtungen: Reinraumbereiche",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "Innen: Krankenhäuser und Pflegeeinrichtungen: Untersuchungsräume",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "Innen: Krankenhäuser und Pflegeeinrichtungen: Verkehrsflächen",
    ),
    (
        "Exterior: General Areas (Exterior)",
        "Außen: Allgemeine Bereiche (Außen)",
    ),
    (
        "Exterior: General Areas (Exterior): Places",
        "Außen: Allgemeine Bereiche (Außen): Plätze",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "Außen: Allgemeine Bereiche (Außen): Parks",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "Außen: Allgemeine Bereiche (Außen): Unterführungen",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "Außen: Allgemeine Bereiche (Außen): (Außen-)Treppen",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "Außen: Allgemeine Bereiche (Außen): Bahnsteigdächer",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "Außen: Allgemeine Bereiche (Außen): Parkplätze (innen)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "Außen: Allgemeine Bereiche (Außen): Außenparkplätze",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "Außen: Allgemeine Bereiche (Außen): Schwimmbecken",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "Außen: Allgemeine Bereiche (Außen): Brunnen",
    ),
    ("Exterior: Streets", "Außen: Straßen"),
    ("Exterior: Streets: Motorways", "Außen: Straßen: Autobahnen"),
    (
        "Exterior: Streets: Access Roads",
        "Außen: Straßen: Zufahrtsstraßen",
    ),
    (
        "Exterior: Streets: Residential Areas",
        "Außen: Straßen: Wohngebiete",
    ),
    (
        "Exterior: Streets: Bicycle Paths",
        "Außen: Straßen: Radwege",
    ),
    ("Exterior: Streets: Footpaths", "Außen: Straßen: Fußwege"),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "Außen: Straßen: Tankstellen",
    ),
    ("Exterior: Streets: Tunnels", "Außen: Straßen: Tunnel"),
    ("Exterior: Sports Fields", "Außen: Sportplätze"),
    (
        "Exterior: Sports Fields: Spotlightings",
        "Außen: Sportplätze: Spotbeleuchtung",
    ),
    ("Exterior: Other", "Außen: Sonstiges"),
    ("Exterior: Other: Facades", "Außen: Sonstiges: Fassaden"),
];

const ES: &[(&str, &str)] = &[
    ("Interior: Traffic Zones", "Interior: Zonas de tránsito"),
    (
        "Interior: Traffic Zones: Corridors",
        "Interior: Zonas de tránsito: Pasillos",
    ),
    (
        "Interior: Traffic Zones: Staircases",
        "Interior: Zonas de tránsito: Escaleras",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "Interior: Zonas de tránsito: Zonas de carga",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "Interior: Zonas de tránsito: Iluminación de cornisa (interior)",
    ),
    (
        "Interior: General Areas (Interior)",
        "Interior: Áreas generales (interior)",
    ),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "Interior: Áreas generales (interior): Salas de descanso",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "Interior: Áreas generales (interior): Áreas de recepción",
    ),
    ("Interior: Office", "Interior: Oficina"),
    (
        "Interior: Office: Office Desks",
        "Interior: Oficina: Escritorios",
    ),
    (
        "Interior: Office: Group Offices",
        "Interior: Oficina: Oficinas colectivas",
    ),
    (
        "Interior: Office: Discussions",
        "Interior: Oficina: Salas de reunión",
    ),
    ("Interior: Office: Archives", "Interior: Oficina: Archivos"),
    ("Interior: Industry/Craft", "Interior: Industria/Artesanía"),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "Interior: Industria/Artesanía: Talleres industriales",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "Interior: Industria/Artesanía: Almacenes",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "Interior: Industria/Artesanía: Cámaras frigoríficas",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "Interior: Industria/Artesanía: Cocinas industriales",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "Interior: Industria/Artesanía: Puestos de montaje",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "Interior: Industria/Artesanía: Iluminación de máquinas",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "Interior: Industria/Artesanía: Puestos de control",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "Interior: Industria/Artesanía: Laboratorios",
    ),
    (
        "Interior: Industry/Craft: Hangars",
        "Interior: Industria/Artesanía: Hangares",
    ),
    ("Interior: Shop Lighting", "Interior: Iluminación comercial"),
    (
        "Interior: Shop Lighting: Retail",
        "Interior: Iluminación comercial: Comercio minorista",
    ),
    (
        "Interior: Shop Lighting: Food",
        "Interior: Iluminación comercial: Alimentación",
    ),
    (
        "Interior: Shop Lighting: Clothing",
        "Interior: Iluminación comercial: Textil",
    ),
    (
        "Interior: Shop Lighting: Display Windows",
        "Interior: Iluminación comercial: Escaparates",
    ),
    (
        "Interior: Shop Lighting: Halls",
        "Interior: Iluminación comercial: Naves",
    ),
    (
        "Interior: Shop Lighting: Great Halls",
        "Interior: Iluminación comercial: Grandes naves",
    ),
    (
        "Interior: Shop Lighting: Mirrors",
        "Interior: Iluminación comercial: Espejos",
    ),
    ("Interior: Public Areas", "Interior: Áreas públicas"),
    (
        "Interior: Public Areas: Restaurants",
        "Interior: Áreas públicas: Restaurantes",
    ),
    (
        "Interior: Public Areas: Theatres",
        "Interior: Áreas públicas: Teatros",
    ),
    (
        "Interior: Public Areas: Railway Stations",
        "Interior: Áreas públicas: Estaciones ferroviarias",
    ),
    (
        "Interior: Public Areas: Museums",
        "Interior: Áreas públicas: Museos",
    ),
    (
        "Interior: Public Areas: Fairs",
        "Interior: Áreas públicas: Ferias",
    ),
    (
        "Interior: Public Areas: Prisons",
        "Interior: Áreas públicas: Prisiones",
    ),
    (
        "Interior: Public Areas: Canteens",
        "Interior: Áreas públicas: Cantinas",
    ),
    (
        "Interior: Emergency Lighting",
        "Interior: Iluminación de emergencia",
    ),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "Interior: Iluminación de emergencia: Iluminación de emergencia",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "Interior: Iluminación de emergencia: Iluminación de señalización",
    ),
    (
        "Interior: Educational Facilities",
        "Interior: Centros educativos",
    ),
    (
        "Interior: Educational Facilities: Classrooms",
        "Interior: Centros educativos: Aulas",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "Interior: Centros educativos: Bibliotecas",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "Interior: Centros educativos: Salas de descanso",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "Interior: Centros educativos: Pabellones deportivos",
    ),
    ("Interior: Private Areas", "Interior: Áreas privadas"),
    (
        "Interior: Private Areas: Living Areas",
        "Interior: Áreas privadas: Salones",
    ),
    (
        "Interior: Private Areas: Baths",
        "Interior: Áreas privadas: Baños",
    ),
    (
        "Interior: Private Areas: Kitchens",
        "Interior: Áreas privadas: Cocinas",
    ),
    (
        "Interior: Hospitals and Care Places",
        "Interior: Hospitales y centros asistenciales",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "Interior: Hospitales y centros asistenciales: Salas hospitalarias",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "Interior: Hospitales y centros asistenciales: Habitaciones de pacientes",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "Interior: Hospitales y centros asistenciales: Salas blancas",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "Interior: Hospitales y centros asistenciales: Salas de exploración",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "Interior: Hospitales y centros asistenciales: Áreas de circulación",
    ),
    (
        "Exterior: General Areas (Exterior)",
        "Exterior: Áreas generales (exterior)",
    ),
    (
        "Exterior: General Areas (Exterior): Places",
        "Exterior: Áreas generales (exterior): Plazas",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "Exterior: Áreas generales (exterior): Parques",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "Exterior: Áreas generales (exterior): Pasos subterráneos",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "Exterior: Áreas generales (exterior): Escaleras (exterior)",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "Exterior: Áreas generales (exterior): Marquesinas de andén",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "Exterior: Áreas generales (exterior): Plazas de aparcamiento (interior)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "Exterior: Áreas generales (exterior): Aparcamientos exteriores",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "Exterior: Áreas generales (exterior): Piscinas",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "Exterior: Áreas generales (exterior): Fuentes",
    ),
    ("Exterior: Streets", "Exterior: Carreteras"),
    (
        "Exterior: Streets: Motorways",
        "Exterior: Carreteras: Autopistas",
    ),
    (
        "Exterior: Streets: Access Roads",
        "Exterior: Carreteras: Vías de acceso",
    ),
    (
        "Exterior: Streets: Residential Areas",
        "Exterior: Carreteras: Zonas residenciales",
    ),
    (
        "Exterior: Streets: Bicycle Paths",
        "Exterior: Carreteras: Carriles bici",
    ),
    (
        "Exterior: Streets: Footpaths",
        "Exterior: Carreteras: Sendas peatonales",
    ),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "Exterior: Carreteras: Estaciones de servicio",
    ),
    (
        "Exterior: Streets: Tunnels",
        "Exterior: Carreteras: Túneles",
    ),
    (
        "Exterior: Sports Fields",
        "Exterior: Instalaciones deportivas",
    ),
    (
        "Exterior: Sports Fields: Spotlightings",
        "Exterior: Instalaciones deportivas: Iluminación focal",
    ),
    ("Exterior: Other", "Exterior: Otros"),
    ("Exterior: Other: Facades", "Exterior: Otros: Fachadas"),
];

const FR: &[(&str, &str)] = &[
    (
        "Interior: Traffic Zones",
        "Intérieur : Zones de circulation",
    ),
    (
        "Interior: Traffic Zones: Corridors",
        "Intérieur : Zones de circulation : Couloirs",
    ),
    (
        "Interior: Traffic Zones: Staircases",
        "Intérieur : Zones de circulation : Escaliers",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "Intérieur : Zones de circulation : Zones de chargement",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "Intérieur : Zones de circulation : Éclairage en corniche (intérieur)",
    ),
    (
        "Interior: General Areas (Interior)",
        "Intérieur : Zones générales (intérieur)",
    ),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "Intérieur : Zones générales (intérieur) : Salles de pause",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "Intérieur : Zones générales (intérieur) : Espaces d'accueil",
    ),
    ("Interior: Office", "Intérieur : Bureau"),
    (
        "Interior: Office: Office Desks",
        "Intérieur : Bureau : Postes de travail",
    ),
    (
        "Interior: Office: Group Offices",
        "Intérieur : Bureau : Bureaux collectifs",
    ),
    (
        "Interior: Office: Discussions",
        "Intérieur : Bureau : Salles de réunion",
    ),
    (
        "Interior: Office: Archives",
        "Intérieur : Bureau : Archives",
    ),
    (
        "Interior: Industry/Craft",
        "Intérieur : Industrie/Artisanat",
    ),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "Intérieur : Industrie/Artisanat : Ateliers industriels",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "Intérieur : Industrie/Artisanat : Entrepôts",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "Intérieur : Industrie/Artisanat : Entrepôts frigorifiques",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "Intérieur : Industrie/Artisanat : Cuisines professionnelles",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "Intérieur : Industrie/Artisanat : Postes d'assemblage",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "Intérieur : Industrie/Artisanat : Éclairage de machine",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "Intérieur : Industrie/Artisanat : Postes de contrôle",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "Intérieur : Industrie/Artisanat : Laboratoires",
    ),
    (
        "Interior: Industry/Craft: Hangars",
        "Intérieur : Industrie/Artisanat : Hangars",
    ),
    (
        "Interior: Shop Lighting",
        "Intérieur : Éclairage de magasin",
    ),
    (
        "Interior: Shop Lighting: Retail",
        "Intérieur : Éclairage de magasin : Commerce de détail",
    ),
    (
        "Interior: Shop Lighting: Food",
        "Intérieur : Éclairage de magasin : Alimentation",
    ),
    (
        "Interior: Shop Lighting: Clothing",
        "Intérieur : Éclairage de magasin : Habillement",
    ),
    (
        "Interior: Shop Lighting: Display Windows",
        "Intérieur : Éclairage de magasin : Vitrines",
    ),
    (
        "Interior: Shop Lighting: Halls",
        "Intérieur : Éclairage de magasin : Halls",
    ),
    (
        "Interior: Shop Lighting: Great Halls",
        "Intérieur : Éclairage de magasin : Grandes halls",
    ),
    (
        "Interior: Shop Lighting: Mirrors",
        "Intérieur : Éclairage de magasin : Miroirs",
    ),
    ("Interior: Public Areas", "Intérieur : Espaces publics"),
    (
        "Interior: Public Areas: Restaurants",
        "Intérieur : Espaces publics : Restaurants",
    ),
    (
        "Interior: Public Areas: Theatres",
        "Intérieur : Espaces publics : Théâtres",
    ),
    (
        "Interior: Public Areas: Railway Stations",
        "Intérieur : Espaces publics : Gares ferroviaires",
    ),
    (
        "Interior: Public Areas: Museums",
        "Intérieur : Espaces publics : Musées",
    ),
    (
        "Interior: Public Areas: Fairs",
        "Intérieur : Espaces publics : Foires",
    ),
    (
        "Interior: Public Areas: Prisons",
        "Intérieur : Espaces publics : Prisons",
    ),
    (
        "Interior: Public Areas: Canteens",
        "Intérieur : Espaces publics : Cantines",
    ),
    (
        "Interior: Emergency Lighting",
        "Intérieur : Éclairage de secours",
    ),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "Intérieur : Éclairage de secours : Éclairage de secours",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "Intérieur : Éclairage de secours : Éclairage de balisage",
    ),
    (
        "Interior: Educational Facilities",
        "Intérieur : Établissements scolaires",
    ),
    (
        "Interior: Educational Facilities: Classrooms",
        "Intérieur : Établissements scolaires : Salles de classe",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "Intérieur : Établissements scolaires : Bibliothèques",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "Intérieur : Établissements scolaires : Salons",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "Intérieur : Établissements scolaires : Salles de sport",
    ),
    ("Interior: Private Areas", "Intérieur : Espaces privés"),
    (
        "Interior: Private Areas: Living Areas",
        "Intérieur : Espaces privés : Pièces de vie",
    ),
    (
        "Interior: Private Areas: Baths",
        "Intérieur : Espaces privés : Salles de bain",
    ),
    (
        "Interior: Private Areas: Kitchens",
        "Intérieur : Espaces privés : Cuisines",
    ),
    (
        "Interior: Hospitals and Care Places",
        "Intérieur : Hôpitaux et centres de soins",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "Intérieur : Hôpitaux et centres de soins : Services hospitaliers",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "Intérieur : Hôpitaux et centres de soins : Chambres de patients",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "Intérieur : Hôpitaux et centres de soins : Salles blanches",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "Intérieur : Hôpitaux et centres de soins : Salles d'examen",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "Intérieur : Hôpitaux et centres de soins : Zones de circulation",
    ),
    (
        "Exterior: General Areas (Exterior)",
        "Extérieur : Zones générales (extérieur)",
    ),
    (
        "Exterior: General Areas (Exterior): Places",
        "Extérieur : Zones générales (extérieur) : Places",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "Extérieur : Zones générales (extérieur) : Parcs",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "Extérieur : Zones générales (extérieur) : Passages souterrains",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "Extérieur : Zones générales (extérieur) : Escaliers (extérieur)",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "Extérieur : Zones générales (extérieur) : Auvents de quai",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "Extérieur : Zones générales (extérieur) : Places de parking (intérieur)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "Extérieur : Zones générales (extérieur) : Parkings extérieurs",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "Extérieur : Zones générales (extérieur) : Piscines",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "Extérieur : Zones générales (extérieur) : Fontaines",
    ),
    ("Exterior: Streets", "Extérieur : Voirie"),
    (
        "Exterior: Streets: Motorways",
        "Extérieur : Voirie : Autoroutes",
    ),
    (
        "Exterior: Streets: Access Roads",
        "Extérieur : Voirie : Voies d'accès",
    ),
    (
        "Exterior: Streets: Residential Areas",
        "Extérieur : Voirie : Zones résidentielles",
    ),
    (
        "Exterior: Streets: Bicycle Paths",
        "Extérieur : Voirie : Pistes cyclables",
    ),
    (
        "Exterior: Streets: Footpaths",
        "Extérieur : Voirie : Cheminements piétons",
    ),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "Extérieur : Voirie : Stations-service",
    ),
    ("Exterior: Streets: Tunnels", "Extérieur : Voirie : Tunnels"),
    ("Exterior: Sports Fields", "Extérieur : Terrains de sport"),
    (
        "Exterior: Sports Fields: Spotlightings",
        "Extérieur : Terrains de sport : Projecteurs",
    ),
    ("Exterior: Other", "Extérieur : Autres"),
    ("Exterior: Other: Facades", "Extérieur : Autres : Façades"),
];

const IT: &[(&str, &str)] = &[
    ("Interior: Traffic Zones", "Interno: Zone di transito"),
    (
        "Interior: Traffic Zones: Corridors",
        "Interno: Zone di transito: Corridoi",
    ),
    (
        "Interior: Traffic Zones: Staircases",
        "Interno: Zone di transito: Scale",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "Interno: Zone di transito: Aree di carico",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "Interno: Zone di transito: Illuminazione a soffitto / cornici (interno)",
    ),
    (
        "Interior: General Areas (Interior)",
        "Interno: Aree generali (interno)",
    ),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "Interno: Aree generali (interno): Sale pausa",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "Interno: Aree generali (interno): Aree di ricevimento",
    ),
    ("Interior: Office", "Interno: Ufficio"),
    (
        "Interior: Office: Office Desks",
        "Interno: Ufficio: Scrivanie",
    ),
    (
        "Interior: Office: Group Offices",
        "Interno: Ufficio: Uffici open space",
    ),
    (
        "Interior: Office: Discussions",
        "Interno: Ufficio: Sale riunioni",
    ),
    ("Interior: Office: Archives", "Interno: Ufficio: Archivi"),
    ("Interior: Industry/Craft", "Interno: Industria/Artigianato"),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "Interno: Industria/Artigianato: Officine industriali",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "Interno: Industria/Artigianato: Magazzini",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "Interno: Industria/Artigianato: Celle frigorifere",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "Interno: Industria/Artigianato: Cucine industriali",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "Interno: Industria/Artigianato: Postazioni di montaggio",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "Interno: Industria/Artigianato: Illuminazione di macchine",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "Interno: Industria/Artigianato: Postazioni di controllo",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "Interno: Industria/Artigianato: Laboratori",
    ),
    (
        "Interior: Industry/Craft: Hangars",
        "Interno: Industria/Artigianato: Hangar",
    ),
    ("Interior: Shop Lighting", "Interno: Illuminazione retail"),
    (
        "Interior: Shop Lighting: Retail",
        "Interno: Illuminazione retail: Commercio al dettaglio",
    ),
    (
        "Interior: Shop Lighting: Food",
        "Interno: Illuminazione retail: Alimentari",
    ),
    (
        "Interior: Shop Lighting: Clothing",
        "Interno: Illuminazione retail: Abbigliamento",
    ),
    (
        "Interior: Shop Lighting: Display Windows",
        "Interno: Illuminazione retail: Vetrine",
    ),
    (
        "Interior: Shop Lighting: Halls",
        "Interno: Illuminazione retail: Sale",
    ),
    (
        "Interior: Shop Lighting: Great Halls",
        "Interno: Illuminazione retail: Grandi sale",
    ),
    (
        "Interior: Shop Lighting: Mirrors",
        "Interno: Illuminazione retail: Specchi",
    ),
    ("Interior: Public Areas", "Interno: Aree pubbliche"),
    (
        "Interior: Public Areas: Restaurants",
        "Interno: Aree pubbliche: Ristoranti",
    ),
    (
        "Interior: Public Areas: Theatres",
        "Interno: Aree pubbliche: Teatri",
    ),
    (
        "Interior: Public Areas: Railway Stations",
        "Interno: Aree pubbliche: Stazioni ferroviarie",
    ),
    (
        "Interior: Public Areas: Museums",
        "Interno: Aree pubbliche: Musei",
    ),
    (
        "Interior: Public Areas: Fairs",
        "Interno: Aree pubbliche: Fiere",
    ),
    (
        "Interior: Public Areas: Prisons",
        "Interno: Aree pubbliche: Carceri",
    ),
    (
        "Interior: Public Areas: Canteens",
        "Interno: Aree pubbliche: Mense",
    ),
    (
        "Interior: Emergency Lighting",
        "Interno: Illuminazione di emergenza",
    ),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "Interno: Illuminazione di emergenza: Illuminazione di emergenza",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "Interno: Illuminazione di emergenza: Illuminazione di segnalazione",
    ),
    (
        "Interior: Educational Facilities",
        "Interno: Strutture didattiche",
    ),
    (
        "Interior: Educational Facilities: Classrooms",
        "Interno: Strutture didattiche: Aule",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "Interno: Strutture didattiche: Biblioteche",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "Interno: Strutture didattiche: Sale comuni",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "Interno: Strutture didattiche: Palestre",
    ),
    ("Interior: Private Areas", "Interno: Aree private"),
    (
        "Interior: Private Areas: Living Areas",
        "Interno: Aree private: Zone giorno",
    ),
    (
        "Interior: Private Areas: Baths",
        "Interno: Aree private: Bagni",
    ),
    (
        "Interior: Private Areas: Kitchens",
        "Interno: Aree private: Cucine",
    ),
    (
        "Interior: Hospitals and Care Places",
        "Interno: Ospedali e strutture di cura",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "Interno: Ospedali e strutture di cura: Reparti ospedalieri",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "Interno: Ospedali e strutture di cura: Camere di degenza",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "Interno: Ospedali e strutture di cura: Camere bianche",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "Interno: Ospedali e strutture di cura: Sale visita",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "Interno: Ospedali e strutture di cura: Aree di circolazione",
    ),
    (
        "Exterior: General Areas (Exterior)",
        "Esterno: Aree generali (esterno)",
    ),
    (
        "Exterior: General Areas (Exterior): Places",
        "Esterno: Aree generali (esterno): Piazze",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "Esterno: Aree generali (esterno): Parchi",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "Esterno: Aree generali (esterno): Sottopassi",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "Esterno: Aree generali (esterno): Scale (esterne)",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "Esterno: Aree generali (esterno): Pensiline",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "Esterno: Aree generali (esterno): Parcheggi (interno)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "Esterno: Aree generali (esterno): Parcheggi esterni",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "Esterno: Aree generali (esterno): Piscine",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "Esterno: Aree generali (esterno): Fontane",
    ),
    ("Exterior: Streets", "Esterno: Strade"),
    (
        "Exterior: Streets: Motorways",
        "Esterno: Strade: Autostrade",
    ),
    (
        "Exterior: Streets: Access Roads",
        "Esterno: Strade: Strade di accesso",
    ),
    (
        "Exterior: Streets: Residential Areas",
        "Esterno: Strade: Aree residenziali",
    ),
    (
        "Exterior: Streets: Bicycle Paths",
        "Esterno: Strade: Piste ciclabili",
    ),
    (
        "Exterior: Streets: Footpaths",
        "Esterno: Strade: Percorsi pedonali",
    ),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "Esterno: Strade: Stazioni di servizio",
    ),
    ("Exterior: Streets: Tunnels", "Esterno: Strade: Gallerie"),
    ("Exterior: Sports Fields", "Esterno: Impianti sportivi"),
    (
        "Exterior: Sports Fields: Spotlightings",
        "Esterno: Impianti sportivi: Illuminazione a faro",
    ),
    ("Exterior: Other", "Esterno: Altro"),
    ("Exterior: Other: Facades", "Esterno: Altro: Facciate"),
];

const RU: &[(&str, &str)] = &[
    ("Interior: Traffic Zones", "Внутри: Транзитные зоны"),
    (
        "Interior: Traffic Zones: Corridors",
        "Внутри: Транзитные зоны: Коридоры",
    ),
    (
        "Interior: Traffic Zones: Staircases",
        "Внутри: Транзитные зоны: Лестницы",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "Внутри: Транзитные зоны: Зоны погрузки",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "Внутри: Транзитные зоны: Карнизная подсветка (внутри)",
    ),
    (
        "Interior: General Areas (Interior)",
        "Внутри: Общие зоны (внутри)",
    ),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "Внутри: Общие зоны (внутри): Комнаты отдыха",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "Внутри: Общие зоны (внутри): Зоны рецепции",
    ),
    ("Interior: Office", "Внутри: Офис"),
    (
        "Interior: Office: Office Desks",
        "Внутри: Офис: Рабочие столы",
    ),
    (
        "Interior: Office: Group Offices",
        "Внутри: Офис: Открытые офисы",
    ),
    (
        "Interior: Office: Discussions",
        "Внутри: Офис: Переговорные",
    ),
    ("Interior: Office: Archives", "Внутри: Офис: Архивы"),
    ("Interior: Industry/Craft", "Внутри: Промышленность/Ремёсла"),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "Внутри: Промышленность/Ремёсла: Промышленные мастерские",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "Внутри: Промышленность/Ремёсла: Склады",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "Внутри: Промышленность/Ремёсла: Холодильные склады",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "Внутри: Промышленность/Ремёсла: Производственные кухни",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "Внутри: Промышленность/Ремёсла: Сборочные посты",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "Внутри: Промышленность/Ремёсла: Подсветка станков",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "Внутри: Промышленность/Ремёсла: Контрольные посты",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "Внутри: Промышленность/Ремёсла: Лаборатории",
    ),
    (
        "Interior: Industry/Craft: Hangars",
        "Внутри: Промышленность/Ремёсла: Ангары",
    ),
    ("Interior: Shop Lighting", "Внутри: Освещение магазинов"),
    (
        "Interior: Shop Lighting: Retail",
        "Внутри: Освещение магазинов: Розничная торговля",
    ),
    (
        "Interior: Shop Lighting: Food",
        "Внутри: Освещение магазинов: Продукты",
    ),
    (
        "Interior: Shop Lighting: Clothing",
        "Внутри: Освещение магазинов: Одежда",
    ),
    (
        "Interior: Shop Lighting: Display Windows",
        "Внутри: Освещение магазинов: Витрины",
    ),
    (
        "Interior: Shop Lighting: Halls",
        "Внутри: Освещение магазинов: Залы",
    ),
    (
        "Interior: Shop Lighting: Great Halls",
        "Внутри: Освещение магазинов: Большие залы",
    ),
    (
        "Interior: Shop Lighting: Mirrors",
        "Внутри: Освещение магазинов: Зеркала",
    ),
    ("Interior: Public Areas", "Внутри: Общественные зоны"),
    (
        "Interior: Public Areas: Restaurants",
        "Внутри: Общественные зоны: Рестораны",
    ),
    (
        "Interior: Public Areas: Theatres",
        "Внутри: Общественные зоны: Театры",
    ),
    (
        "Interior: Public Areas: Railway Stations",
        "Внутри: Общественные зоны: Железнодорожные вокзалы",
    ),
    (
        "Interior: Public Areas: Museums",
        "Внутри: Общественные зоны: Музеи",
    ),
    (
        "Interior: Public Areas: Fairs",
        "Внутри: Общественные зоны: Ярмарки",
    ),
    (
        "Interior: Public Areas: Prisons",
        "Внутри: Общественные зоны: Тюрьмы",
    ),
    (
        "Interior: Public Areas: Canteens",
        "Внутри: Общественные зоны: Столовые",
    ),
    (
        "Interior: Emergency Lighting",
        "Внутри: Аварийное освещение",
    ),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "Внутри: Аварийное освещение: Аварийное освещение",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "Внутри: Аварийное освещение: Сигнальное освещение",
    ),
    (
        "Interior: Educational Facilities",
        "Внутри: Образовательные учреждения",
    ),
    (
        "Interior: Educational Facilities: Classrooms",
        "Внутри: Образовательные учреждения: Классы",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "Внутри: Образовательные учреждения: Библиотеки",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "Внутри: Образовательные учреждения: Зоны отдыха",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "Внутри: Образовательные учреждения: Спортзалы",
    ),
    ("Interior: Private Areas", "Внутри: Частные помещения"),
    (
        "Interior: Private Areas: Living Areas",
        "Внутри: Частные помещения: Жилые зоны",
    ),
    (
        "Interior: Private Areas: Baths",
        "Внутри: Частные помещения: Ванные комнаты",
    ),
    (
        "Interior: Private Areas: Kitchens",
        "Внутри: Частные помещения: Кухни",
    ),
    (
        "Interior: Hospitals and Care Places",
        "Внутри: Больницы и медицинские учреждения",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "Внутри: Больницы и медицинские учреждения: Больничные отделения",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "Внутри: Больницы и медицинские учреждения: Палаты",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "Внутри: Больницы и медицинские учреждения: Чистые помещения",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "Внутри: Больницы и медицинские учреждения: Смотровые",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "Внутри: Больницы и медицинские учреждения: Зоны передвижения",
    ),
    (
        "Exterior: General Areas (Exterior)",
        "Снаружи: Общие зоны (снаружи)",
    ),
    (
        "Exterior: General Areas (Exterior): Places",
        "Снаружи: Общие зоны (снаружи): Площади",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "Снаружи: Общие зоны (снаружи): Парки",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "Снаружи: Общие зоны (снаружи): Подземные переходы",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "Снаружи: Общие зоны (снаружи): Лестницы (наружные)",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "Снаружи: Общие зоны (снаружи): Платформенные навесы",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "Снаружи: Общие зоны (снаружи): Парковочные места (внутри)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "Снаружи: Общие зоны (снаружи): Открытые парковки",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "Снаружи: Общие зоны (снаружи): Бассейны",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "Снаружи: Общие зоны (снаружи): Фонтаны",
    ),
    ("Exterior: Streets", "Снаружи: Дороги"),
    (
        "Exterior: Streets: Motorways",
        "Снаружи: Дороги: Автомагистрали",
    ),
    (
        "Exterior: Streets: Access Roads",
        "Снаружи: Дороги: Подъездные пути",
    ),
    (
        "Exterior: Streets: Residential Areas",
        "Снаружи: Дороги: Жилые зоны",
    ),
    (
        "Exterior: Streets: Bicycle Paths",
        "Снаружи: Дороги: Велодорожки",
    ),
    (
        "Exterior: Streets: Footpaths",
        "Снаружи: Дороги: Пешеходные дорожки",
    ),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "Снаружи: Дороги: АЗС",
    ),
    ("Exterior: Streets: Tunnels", "Снаружи: Дороги: Тоннели"),
    ("Exterior: Sports Fields", "Снаружи: Спортивные площадки"),
    (
        "Exterior: Sports Fields: Spotlightings",
        "Снаружи: Спортивные площадки: Прожекторное освещение",
    ),
    ("Exterior: Other", "Снаружи: Прочее"),
    ("Exterior: Other: Facades", "Снаружи: Прочее: Фасады"),
];

const ZH: &[(&str, &str)] = &[
    ("Interior: Traffic Zones", "室内: 交通区"),
    ("Interior: Traffic Zones: Corridors", "室内: 交通区: 走廊"),
    (
        "Interior: Traffic Zones: Staircases",
        "室内: 交通区: 楼梯间",
    ),
    (
        "Interior: Traffic Zones: Loading Zones",
        "室内: 交通区: 装卸区",
    ),
    (
        "Interior: Traffic Zones: Cove Lighting / Cornices (Indoor)",
        "室内: 交通区: 灯槽/檐口照明(室内)",
    ),
    ("Interior: General Areas (Interior)", "室内: 通用区域(室内)"),
    (
        "Interior: General Areas (Interior): Break Rooms",
        "室内: 通用区域(室内): 休息室",
    ),
    (
        "Interior: General Areas (Interior): Reception Areas",
        "室内: 通用区域(室内): 接待区",
    ),
    ("Interior: Office", "室内: 办公"),
    ("Interior: Office: Office Desks", "室内: 办公: 办公桌"),
    ("Interior: Office: Group Offices", "室内: 办公: 集体办公室"),
    ("Interior: Office: Discussions", "室内: 办公: 会议室"),
    ("Interior: Office: Archives", "室内: 办公: 档案室"),
    ("Interior: Industry/Craft", "室内: 工业/手工"),
    (
        "Interior: Industry/Craft: Industrial Workshops",
        "室内: 工业/手工: 工业车间",
    ),
    (
        "Interior: Industry/Craft: Warehouses",
        "室内: 工业/手工: 仓库",
    ),
    (
        "Interior: Industry/Craft: Cold Storage Facilities",
        "室内: 工业/手工: 冷库",
    ),
    (
        "Interior: Industry/Craft: Kitchens",
        "室内: 工业/手工: 工业厨房",
    ),
    (
        "Interior: Industry/Craft: Assembly Work Stations",
        "室内: 工业/手工: 装配工位",
    ),
    (
        "Interior: Industry/Craft: Machine Illumination",
        "室内: 工业/手工: 机器照明",
    ),
    (
        "Interior: Industry/Craft: Control Work Stations",
        "室内: 工业/手工: 控制工位",
    ),
    (
        "Interior: Industry/Craft: Laboratories",
        "室内: 工业/手工: 实验室",
    ),
    ("Interior: Industry/Craft: Hangars", "室内: 工业/手工: 机库"),
    ("Interior: Shop Lighting", "室内: 商业照明"),
    ("Interior: Shop Lighting: Retail", "室内: 商业照明: 零售"),
    ("Interior: Shop Lighting: Food", "室内: 商业照明: 食品"),
    ("Interior: Shop Lighting: Clothing", "室内: 商业照明: 服装"),
    (
        "Interior: Shop Lighting: Display Windows",
        "室内: 商业照明: 橱窗",
    ),
    ("Interior: Shop Lighting: Halls", "室内: 商业照明: 大厅"),
    (
        "Interior: Shop Lighting: Great Halls",
        "室内: 商业照明: 大型大厅",
    ),
    ("Interior: Shop Lighting: Mirrors", "室内: 商业照明: 镜面"),
    ("Interior: Public Areas", "室内: 公共区域"),
    (
        "Interior: Public Areas: Restaurants",
        "室内: 公共区域: 餐厅",
    ),
    ("Interior: Public Areas: Theatres", "室内: 公共区域: 剧院"),
    (
        "Interior: Public Areas: Railway Stations",
        "室内: 公共区域: 火车站",
    ),
    ("Interior: Public Areas: Museums", "室内: 公共区域: 博物馆"),
    ("Interior: Public Areas: Fairs", "室内: 公共区域: 展会"),
    ("Interior: Public Areas: Prisons", "室内: 公共区域: 监狱"),
    ("Interior: Public Areas: Canteens", "室内: 公共区域: 食堂"),
    ("Interior: Emergency Lighting", "室内: 应急照明"),
    (
        "Interior: Emergency Lighting: Emergency Lighting",
        "室内: 应急照明: 应急照明",
    ),
    (
        "Interior: Emergency Lighting: Signal Lighting",
        "室内: 应急照明: 信号照明",
    ),
    ("Interior: Educational Facilities", "室内: 教育设施"),
    (
        "Interior: Educational Facilities: Classrooms",
        "室内: 教育设施: 教室",
    ),
    (
        "Interior: Educational Facilities: Libraries",
        "室内: 教育设施: 图书馆",
    ),
    (
        "Interior: Educational Facilities: Lounges",
        "室内: 教育设施: 休息区",
    ),
    (
        "Interior: Educational Facilities: Sports Halls",
        "室内: 教育设施: 体育馆",
    ),
    ("Interior: Private Areas", "室内: 私人区域"),
    (
        "Interior: Private Areas: Living Areas",
        "室内: 私人区域: 起居室",
    ),
    ("Interior: Private Areas: Baths", "室内: 私人区域: 浴室"),
    ("Interior: Private Areas: Kitchens", "室内: 私人区域: 厨房"),
    (
        "Interior: Hospitals and Care Places",
        "室内: 医院和护理场所",
    ),
    (
        "Interior: Hospitals and Care Places: Hospital Wards",
        "室内: 医院和护理场所: 病房",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Patient Rooms",
        "室内: 医院和护理场所: 病房单间",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Clean Room Areas",
        "室内: 医院和护理场所: 洁净室区域",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Examination Rooms",
        "室内: 医院和护理场所: 检查室",
    ),
    (
        "Interior: Hospitals and Care Places: Health Care Circulation Areas",
        "室内: 医院和护理场所: 通行区",
    ),
    ("Exterior: General Areas (Exterior)", "室外: 通用区域(室外)"),
    (
        "Exterior: General Areas (Exterior): Places",
        "室外: 通用区域(室外): 广场",
    ),
    (
        "Exterior: General Areas (Exterior): Parks",
        "室外: 通用区域(室外): 公园",
    ),
    (
        "Exterior: General Areas (Exterior): Underpasses",
        "室外: 通用区域(室外): 地下通道",
    ),
    (
        "Exterior: General Areas (Exterior): (Outdoor) Stairs",
        "室外: 通用区域(室外): 室外楼梯",
    ),
    (
        "Exterior: General Areas (Exterior): Platform-Roofs",
        "室外: 通用区域(室外): 月台雨棚",
    ),
    (
        "Exterior: General Areas (Exterior): Parking Spaces (Indoor)",
        "室外: 通用区域(室外): 停车位(室内)",
    ),
    (
        "Exterior: General Areas (Exterior): Outdoor Parkings",
        "室外: 通用区域(室外): 室外停车场",
    ),
    (
        "Exterior: General Areas (Exterior): Pools",
        "室外: 通用区域(室外): 泳池",
    ),
    (
        "Exterior: General Areas (Exterior): Fountains",
        "室外: 通用区域(室外): 喷泉",
    ),
    ("Exterior: Streets", "室外: 街道"),
    ("Exterior: Streets: Motorways", "室外: 街道: 高速公路"),
    ("Exterior: Streets: Access Roads", "室外: 街道: 进出道路"),
    ("Exterior: Streets: Residential Areas", "室外: 街道: 居民区"),
    ("Exterior: Streets: Bicycle Paths", "室外: 街道: 自行车道"),
    ("Exterior: Streets: Footpaths", "室外: 街道: 人行道"),
    (
        "Exterior: Streets: Petrol-Gas Stations",
        "室外: 街道: 加油站",
    ),
    ("Exterior: Streets: Tunnels", "室外: 街道: 隧道"),
    ("Exterior: Sports Fields", "室外: 运动场"),
    (
        "Exterior: Sports Fields: Spotlightings",
        "室外: 运动场: 投光照明",
    ),
    ("Exterior: Other", "室外: 其他"),
    ("Exterior: Other: Facades", "室外: 其他: 外立面"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_returns_canonical_unchanged() {
        assert_eq!(
            display_label("Interior: Office: Group Offices", "en"),
            "Interior: Office: Group Offices"
        );
    }

    #[test]
    fn unknown_language_falls_back_to_canonical() {
        assert_eq!(display_label("Interior: Office", "ja"), "Interior: Office");
    }

    #[test]
    fn unknown_value_returns_input_unchanged() {
        assert_eq!(display_label("not-a-valid-enum", "de"), "not-a-valid-enum");
    }

    #[test]
    fn known_translations_preserve_colon_structure() {
        let de = display_label("Interior: Office: Group Offices", "de");
        assert_eq!(
            de.matches(':').count(),
            2,
            "german translation must keep both `:` separators"
        );
        assert_eq!(de, "Innen: Büro: Großraumbüros");
    }

    #[test]
    fn every_language_covers_every_canonical_value() {
        for (lang, table) in &[
            ("de", DE),
            ("es", ES),
            ("fr", FR),
            ("it", IT),
            ("ru", RU),
            ("zh", ZH),
        ] {
            for canonical in CANONICAL_VALUES {
                let in_table = table.iter().any(|(k, _)| k == canonical);
                assert!(
                    in_table,
                    "language {} missing entry for {:?}",
                    lang, canonical
                );
            }
            // Every translated value preserves the `:` separator structure.
            for (k, v) in *table {
                assert_eq!(
                    v.matches(':').count(),
                    k.matches(':').count(),
                    "translation for {:?} in {} drops a `:` separator: {:?}",
                    k,
                    lang,
                    v
                );
            }
        }
    }
}
