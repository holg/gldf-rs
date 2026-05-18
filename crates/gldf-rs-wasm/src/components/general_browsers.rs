//! Read-only browsers for the GeneralDefinitions sub-trees that don't have
//! dedicated editors yet: Photometries, Equipments, Emitters, Geometries,
//! Sensors.
//!
//! Each browser surfaces the entries in card form (id, key fields,
//! cross-references) so the user can confirm what the file actually carries
//! before drilling into a more specific editor. Editing these structures
//! through forms is its own design problem — keeping the browsers read-only
//! keeps the scope honest.

use crate::state::use_gldf;
use yew::prelude::*;

#[function_component(PhotometriesBrowser)]
pub fn photometries_browser() -> Html {
    let gldf = use_gldf();
    let photometries = gldf
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.as_slice())
        .unwrap_or(&[]);

    html! {
        <div class="editor-section browser photometries-browser">
            <h2>{ format!("Photometries ({})", photometries.len()) }</h2>
            <p class="section-description">
                { "Photometric data references in GeneralDefinitions. Each Photometry either points at a file (LDT/IES) or carries DescriptivePhotometry inline." }
            </p>
            if photometries.is_empty() {
                <p class="empty-message">{ "No photometries." }</p>
            } else {
                <div class="entry-cards">
                    { for photometries.iter().map(|p| html! {
                        <div class="entry-card" key={p.id.clone()}>
                            <div class="card-header">
                                <span class="card-id code">{ &p.id }</span>
                            </div>
                            <div class="card-body">
                                if let Some(ref fr) = p.photometry_file_reference {
                                    <div class="detail">
                                        <span class="label">{ "File:" }</span>
                                        <span class="value code">{ &fr.file_id }</span>
                                    </div>
                                }
                                if p.descriptive_photometry.is_some() {
                                    <div class="detail">
                                        <span class="label">{ "Descriptive:" }</span>
                                        <span class="value">{ "present (see Photometry editor)" }</span>
                                    </div>
                                }
                            </div>
                        </div>
                    }) }
                </div>
            }
        </div>
    }
}

#[function_component(EquipmentsBrowser)]
pub fn equipments_browser() -> Html {
    let gldf = use_gldf();
    let equipments = gldf
        .product
        .general_definitions
        .equipments
        .as_ref()
        .map(|e| e.equipment.as_slice())
        .unwrap_or(&[]);

    html! {
        <div class="editor-section browser equipments-browser">
            <h2>{ format!("Equipments ({})", equipments.len()) }</h2>
            <p class="section-description">
                { "Equipment groups one or more light sources with optional control gear and a rated input power. References from Variant.Geometry select an equipment to use." }
            </p>
            if equipments.is_empty() {
                <p class="empty-message">{ "No equipments." }</p>
            } else {
                <div class="entry-cards">
                    { for equipments.iter().map(|eq| html! {
                        <div class="entry-card" key={eq.id.clone()}>
                            <div class="card-header">
                                <span class="card-id code">{ &eq.id }</span>
                            </div>
                            <div class="card-body">
                                if let Some(ref ls) = eq.light_source_reference {
                                    <div class="detail">
                                        <span class="label">{ "Light source:" }</span>
                                        <span class="value code">
                                            { ls.fixed_light_source_id.clone()
                                                .or(ls.changeable_light_source_id.clone())
                                                .unwrap_or_else(|| "(missing)".to_string()) }
                                        </span>
                                        if let Some(c) = ls.light_source_count {
                                            <span class="value">{ format!(" ×{}", c) }</span>
                                        }
                                    </div>
                                }
                                if let Some(ref cg) = eq.control_gear_reference {
                                    <div class="detail">
                                        <span class="label">{ "Control gear:" }</span>
                                        <span class="value code">{ &cg.control_gear_id }</span>
                                    </div>
                                }
                                <div class="detail">
                                    <span class="label">{ "Rated input power:" }</span>
                                    <span class="value">{ format!("{} W", eq.rated_input_power) }</span>
                                </div>
                                if let Some(emerg) = eq.emergency_rated_luminous_flux {
                                    <div class="detail">
                                        <span class="label">{ "Emergency flux:" }</span>
                                        <span class="value">{ format!("{} lm", emerg) }</span>
                                    </div>
                                }
                            </div>
                        </div>
                    }) }
                </div>
            }
        </div>
    }
}

#[function_component(EmittersBrowser)]
pub fn emitters_browser() -> Html {
    let gldf = use_gldf();
    let emitters = gldf
        .product
        .general_definitions
        .emitters
        .as_ref()
        .map(|e| e.emitter.as_slice())
        .unwrap_or(&[]);

    html! {
        <div class="editor-section browser emitters-browser">
            <h2>{ format!("Emitters ({})", emitters.len()) }</h2>
            <p class="section-description">
                { "Emitters bind a photometry to a light source (and optional control gear). Variants reference these by id; an emitter can be a FixedLightEmitter, ChangeableLightEmitter, or SensorEmitter." }
            </p>
            if emitters.is_empty() {
                <p class="empty-message">{ "No emitters." }</p>
            } else {
                <div class="entry-cards">
                    { for emitters.iter().map(|em| html! {
                        <div class="entry-card" key={em.id.clone()}>
                            <div class="card-header">
                                <span class="card-id code">{ &em.id }</span>
                            </div>
                            <div class="card-body">
                                { for em.fixed_light_emitter.iter().map(|fle| html! {
                                    <div class="emitter-block">
                                        <span class="emitter-kind">{ "Fixed Light Emitter" }</span>
                                        <div class="detail">
                                            <span class="label">{ "Photometry:" }</span>
                                            <span class="value code">{ &fle.photometry_reference.photometry_id }</span>
                                        </div>
                                        <div class="detail">
                                            <span class="label">{ "Light source:" }</span>
                                            <span class="value code">
                                                { fle.light_source_reference.fixed_light_source_id.clone()
                                                    .or(fle.light_source_reference.changeable_light_source_id.clone())
                                                    .unwrap_or_else(|| "(missing)".to_string()) }
                                            </span>
                                        </div>
                                        if let Some(flux) = fle.rated_luminous_flux {
                                            <div class="detail">
                                                <span class="label">{ "Rated flux:" }</span>
                                                <span class="value">{ format!("{} lm", flux) }</span>
                                            </div>
                                        }
                                    </div>
                                }) }
                                { for em.changeable_light_emitter.iter().map(|cle| html! {
                                    <div class="emitter-block">
                                        <span class="emitter-kind changeable">{ "Changeable Light Emitter" }</span>
                                        <div class="detail">
                                            <span class="label">{ "Photometry:" }</span>
                                            <span class="value code">{ &cle.photometry_reference.photometry_id }</span>
                                        </div>
                                    </div>
                                }) }
                                { for em.sensor_emitter.iter().map(|se| html! {
                                    <div class="emitter-block">
                                        <span class="emitter-kind sensor">{ "Sensor Emitter" }</span>
                                        <div class="detail">
                                            <span class="label">{ "Sensor:" }</span>
                                            <span class="value code">{ &se.sensor_reference.sensor_id }</span>
                                        </div>
                                    </div>
                                }) }
                            </div>
                        </div>
                    }) }
                </div>
            }
        </div>
    }
}

#[function_component(GeometriesBrowser)]
pub fn geometries_browser() -> Html {
    let gldf = use_gldf();
    let geometries = gldf.product.general_definitions.geometries.as_ref();
    let model_count = geometries.map(|g| g.model_geometry.len()).unwrap_or(0);
    let simple_count = geometries.map(|g| g.simple_geometry.len()).unwrap_or(0);

    html! {
        <div class="editor-section browser geometries-browser">
            <h2>{ format!("Geometries ({} model, {} simple)", model_count, simple_count) }</h2>
            <p class="section-description">
                { "ModelGeometry references external L3D / 3D files; SimpleGeometry describes shape inline (cuboid, cylinder, etc.). Variants pick one by id." }
            </p>

            if let Some(g) = geometries {
                if g.model_geometry.is_empty() && g.simple_geometry.is_empty() {
                    <p class="empty-message">{ "No geometries." }</p>
                }
                if !g.model_geometry.is_empty() {
                    <h3>{ "Model Geometries" }</h3>
                    <div class="entry-cards">
                        { for g.model_geometry.iter().map(|m| html! {
                            <div class="entry-card" key={m.id.clone()}>
                                <div class="card-header">
                                    <span class="card-id code">{ &m.id }</span>
                                </div>
                                <div class="card-body">
                                    { for m.geometry_file_reference.iter().map(|fr| html! {
                                        <div class="detail">
                                            <span class="label">{ "File:" }</span>
                                            <span class="value code">{ &fr.file_id }</span>
                                            if let Some(ref lod) = fr.level_of_detail {
                                                <span class="value">{ format!(" (LOD: {})", lod) }</span>
                                            }
                                        </div>
                                    }) }
                                </div>
                            </div>
                        }) }
                    </div>
                }
                if !g.simple_geometry.is_empty() {
                    <h3>{ "Simple Geometries" }</h3>
                    <div class="entry-cards">
                        { for g.simple_geometry.iter().map(|sg| html! {
                            <div class="entry-card" key={sg.id.clone()}>
                                <div class="card-header">
                                    <span class="card-id code">{ &sg.id }</span>
                                </div>
                                <div class="card-body">
                                    <span class="value">{ "(see SimpleGeometry definition)" }</span>
                                </div>
                            </div>
                        }) }
                    </div>
                }
            } else {
                <p class="empty-message">{ "No geometries." }</p>
            }
        </div>
    }
}

#[function_component(SensorsBrowser)]
pub fn sensors_browser() -> Html {
    let gldf = use_gldf();
    let sensors = gldf
        .product
        .general_definitions
        .sensors
        .as_ref()
        .map(|s| s.sensor.as_slice())
        .unwrap_or(&[]);

    html! {
        <div class="editor-section browser sensors-browser">
            <h2>{ format!("Sensors ({})", sensors.len()) }</h2>
            <p class="section-description">
                { "Sensor definitions referenced by Emitter/SensorEmitter. Each carries an optional file reference plus detector characteristics, methods, and types." }
            </p>
            if sensors.is_empty() {
                <p class="empty-message">{ "No sensors." }</p>
            } else {
                <div class="entry-cards">
                    { for sensors.iter().map(|s| html! {
                        <div class="entry-card" key={s.id.clone()}>
                            <div class="card-header">
                                <span class="card-id code">{ &s.id }</span>
                            </div>
                            <div class="card-body">
                                if let Some(ref fr) = s.sensor_file_reference {
                                    <div class="detail">
                                        <span class="label">{ "File:" }</span>
                                        <span class="value code">{ &fr.file_id }</span>
                                    </div>
                                }
                                if let Some(ref dc) = s.detector_characteristics {
                                    if !dc.detector_characteristic.is_empty() {
                                        <div class="detail">
                                            <span class="label">{ "Characteristics:" }</span>
                                            <span class="value">{ dc.detector_characteristic.join(", ") }</span>
                                        </div>
                                    }
                                }
                                if let Some(ref dm) = s.detection_methods {
                                    if !dm.detection_method.is_empty() {
                                        <div class="detail">
                                            <span class="label">{ "Methods:" }</span>
                                            <span class="value">{ dm.detection_method.join(", ") }</span>
                                        </div>
                                    }
                                }
                                if let Some(ref dt) = s.detector_types {
                                    if !dt.detector_type.is_empty() {
                                        <div class="detail">
                                            <span class="label">{ "Types:" }</span>
                                            <span class="value">{ dt.detector_type.join(", ") }</span>
                                        </div>
                                    }
                                }
                            </div>
                        </div>
                    }) }
                </div>
            }
        </div>
    }
}
