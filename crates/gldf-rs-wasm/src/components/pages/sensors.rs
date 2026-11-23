//! Sensors page — read + edit views of `<GeneralDefinitions/Sensors/Sensor>`.
//!
//! GLDF separates sensors from photometries: a `<Sensor>` references a
//! `sensor/sensxml` or `sensor/sensldt` file (the actual detector data),
//! plus three closed XSD enums describing the device — DetectorCharacteristic
//! (Round/Square/Other), DetectionMethod (Passive Infrared/High Frequency/
//! Microwave/Ultrasonic/Camera/Other), DetectorType (Motion/Presence/Daylight/
//! Other). All three are multi-select checkbox grids with capped cardinality
//! per the XSD (max 3, 6, 4 respectively).

use crate::components::IdRenameChip;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use eulumdat::diagram::SvgTheme;
use eulumdat::sensor::{Motion, Quality, SensorFile, SensorWarning};
use gldf_rs::gldf::general_definitions::sensors::Sensor;
use gldf_rs::validation::xsd_enums::{DETECTION_METHODS, DETECTOR_CHARACTERISTICS, DETECTOR_TYPES};
use leptos::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

/// Read view — no add/edit/remove controls.
#[component]
pub fn SensorsReadPage() -> impl IntoView {
    sensors_page(false)
}

/// Edit view — Add Sensor button + per-row rename + file picker + enum grids + Remove.
#[component]
pub fn SensorsEditorPage() -> impl IntoView {
    sensors_page(true)
}

fn sensors_page(editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));

    let on_add = move |_| {
        state.update(|s| s.apply(GldfAction::AddSensor));
    };

    view! {
        <div class="editor-section sensors-editor">
            <h2>{ move || tr("gldf.sensors.title") }</h2>
            <p class="section-description">
                { move || tr("gldf.sensors.description") }
            </p>

            {move || state.with(|s| {
                let sensors: Vec<Sensor> = s
                    .product
                    .general_definitions
                    .sensors
                    .as_ref()
                    .map(|sx| sx.sensor.clone())
                    .unwrap_or_default();

                // Sensor-typed files available for the file picker.
                let sensor_files: Vec<(String, String)> = s
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .filter(|f| f.content_type.starts_with("sensor/"))
                    .map(|f| (f.id.clone(), f.file_name.clone()))
                    .collect();

                if sensors.is_empty() && !editable {
                    return view! {
                        <p class="empty-message">{ tr("gldf.sensors.empty") }</p>
                    }.into_any();
                }

                view! {
                    <div class="sensors-list" style="display: flex; flex-direction: column; gap: 12px;">
                        {sensors.into_iter().enumerate().map(|(idx, sensor)| view! {
                            <SensorRow
                                idx=idx
                                sensor=sensor
                                sensor_files=sensor_files.clone()
                                editable=editable
                            />
                        }).collect::<Vec<_>>()}
                    </div>
                    {if editable {
                        view! {
                            <button
                                type="button"
                                on:click=on_add
                                data-testid="sensors-add"
                                style="margin-top: 12px; padding: 6px 12px;"
                            >{ tr("gldf.sensors.add") }</button>
                        }.into_any()
                    } else { ().into_any() }}
                }.into_any()
            })}
        </div>
    }
}

#[component]
fn SensorRow(
    idx: usize,
    sensor: Sensor,
    sensor_files: Vec<(String, String)>,
    editable: bool,
) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let id = sensor.id.clone();
    let id_for_rename = id.clone();
    let on_rename_id: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = id_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| {
                s.apply(GldfAction::RenameSensor {
                    old_id: old_id.clone(),
                    new_id,
                })
            });
        })
    };
    let current_file_id = sensor
        .sensor_file_reference
        .as_ref()
        .map(|r| r.file_id.clone())
        .unwrap_or_default();
    let detection_file_id = current_file_id.clone();
    let chars: Vec<String> = sensor
        .detector_characteristics
        .as_ref()
        .map(|d| d.detector_characteristic.clone())
        .unwrap_or_default();
    let methods: Vec<String> = sensor
        .detection_methods
        .as_ref()
        .map(|d| d.detection_method.clone())
        .unwrap_or_default();
    let types: Vec<String> = sensor
        .detector_types
        .as_ref()
        .map(|d| d.detector_type.clone())
        .unwrap_or_default();

    let on_remove = move |_| {
        let msg = ui_t(
            "gldf.sensors.remove_confirm",
            &state.with(|s| s.ui_language.clone()),
        );
        if crate::components::edit_util::confirm(&msg) {
            state.update(|s| s.apply(GldfAction::RemoveSensor { index: idx }));
        }
    };

    let on_file_change = move |e: crate::components::edit_util::EditEvent| {
        let v = crate::components::edit_util::event_value(&e).unwrap_or_default();
        let file_id = if v.is_empty() { None } else { Some(v) };
        state.update(|s| {
            s.apply(GldfAction::SetSensorFileId {
                index: idx,
                file_id,
            })
        });
    };

    view! {
        <div class="sensor-entry" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 12px;">
            <div class="entry-header" style="display: flex; gap: 8px; align-items: center; margin-bottom: 8px; flex-wrap: wrap;">
                <IdRenameChip current_id=id editable=editable on_rename=on_rename_id />
                {if editable {
                    view! {
                        <button
                            type="button"
                            on:click=on_remove
                            style="margin-left: auto; padding: 2px 10px; background: none; border: 1px solid var(--accent-orange); color: var(--accent-orange); border-radius: 4px; cursor: pointer; font-size: 12px;"
                        >{ tr("gldf.common.remove") }</button>
                    }.into_any()
                } else { ().into_any() }}
            </div>

            <div class="form-group">
                <label>{ tr("gldf.sensors.sensor_file_label") }</label>
                {if editable {
                    let current = current_file_id.clone();
                    view! {
                        <select on:change=on_file_change>
                            <option value="" selected=current.is_empty()>{ tr("gldf.sensors.sensor_file_none") }</option>
                            {sensor_files.into_iter().map(|(fid, fname)| {
                                let selected = fid == current;
                                let label = if fname.is_empty() { fid.clone() } else { format!("{} ({})", fid, fname) };
                                view! { <option value=fid selected=selected>{ label }</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    }.into_any()
                } else if current_file_id.is_empty() {
                    view! { <span style="color: var(--text-tertiary); font-style: italic;">{ tr("gldf.sensors.no_file") }</span> }.into_any()
                } else {
                    view! { <code>{ current_file_id }</code> }.into_any()
                }}
                <span class="form-help">{ tr("gldf.sensors.sensor_file_hint") }</span>
            </div>

            {(!detection_file_id.is_empty()).then(|| view! {
                <SensorDetection file_id=detection_file_id.clone() />
            })}

            <EnumGrid
                title=tr("gldf.sensors.detector_characteristic")
                hint=tr("gldf.sensors.detector_characteristic_hint")
                values=DETECTOR_CHARACTERISTICS
                max=3
                current=chars
                editable=editable
                on_change=Callback::new(move |v: Vec<String>| {
                    state.update(|s| s.apply(GldfAction::SetSensorDetectorCharacteristics {
                        index: idx,
                        values: v,
                    }));
                })
            />

            <EnumGrid
                title=tr("gldf.sensors.detection_method")
                hint=tr("gldf.sensors.detection_method_hint")
                values=DETECTION_METHODS
                max=6
                current=methods
                editable=editable
                on_change=Callback::new(move |v: Vec<String>| {
                    state.update(|s| s.apply(GldfAction::SetSensorDetectionMethods {
                        index: idx,
                        values: v,
                    }));
                })
            />

            <EnumGrid
                title=tr("gldf.sensors.detector_type")
                hint=tr("gldf.sensors.detector_type_hint")
                values=DETECTOR_TYPES
                max=4
                current=types
                editable=editable
                on_change=Callback::new(move |v: Vec<String>| {
                    state.update(|s| s.apply(GldfAction::SetSensorDetectorTypes {
                        index: idx,
                        values: v,
                    }));
                })
            />
        </div>
    }
}

/// Multi-select checkbox grid bound to a closed XSD enum. `max` is the
/// XSD's `maxOccurs` for the enclosing element; we let users tick past
/// it and surface a warning so the user can act, instead of silently
/// dropping their last click.
#[component]
fn EnumGrid(
    title: String,
    hint: String,
    values: &'static [&'static str],
    max: usize,
    current: Vec<String>,
    editable: bool,
    on_change: Callback<Vec<String>>,
) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let selected: HashSet<String> = current.iter().cloned().collect();
    let over_cap = current.len() > max;

    if !editable {
        return view! {
            <div class="form-group">
                <label>{ title }</label>
                {if current.is_empty() {
                    view! { <span style="color: var(--text-tertiary); font-style: italic;">{ tr("gldf.sensors.unset") }</span> }.into_any()
                } else {
                    view! {
                        <div class="tags-container" style="display: flex; gap: 6px; flex-wrap: wrap;">
                            {current.iter().cloned().map(|v| view! {
                                <span class="tag" style="display: inline-block; padding: 4px 8px; background: var(--bg-secondary); border-radius: 4px; font-size: 12px;">{ v }</span>
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>
        }.into_any();
    }

    let current_for_change = current.clone();
    let make_handler = move |value: &'static str| {
        let current_now = current_for_change.clone();
        move |_e: crate::components::edit_util::EditEvent| {
            let target = value.to_string();
            let mut next = current_now.clone();
            if let Some(pos) = next.iter().position(|v| v == &target) {
                next.remove(pos);
            } else {
                next.push(target);
            }
            on_change.run(next);
        }
    };

    view! {
        <div class="form-group">
            <label>{ title }</label>
            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 4px;">
                {values.iter().map(|v| {
                    let is_selected = selected.contains(*v);
                    let handler = make_handler(v);
                    view! {
                        <label class="application-checkbox-row">
                            <input type="checkbox" prop:checked=is_selected on:change=handler />
                            <span>{ *v }</span>
                        </label>
                    }
                }).collect::<Vec<_>>()}
            </div>
            <span class="form-help">{ hint }</span>
            {if over_cap {
                let warn = tr("gldf.sensors.over_cap_warning")
                    .replace("{count}", &current.len().to_string())
                    .replace("{max}", &max.to_string());
                view! {
                    <p style="color: var(--accent-orange); font-size: 11px; margin: 4px 0 0;">
                        { warn }
                    </p>
                }.into_any()
            } else { ().into_any() }}
        </div>
    }.into_any()
}

/// Parsed sensor file; equal only to itself (re-parsed when the bytes change).
#[derive(Clone)]
struct ParsedSensor(Option<Result<Arc<SensorFile>, String>>);

impl PartialEq for ParsedSensor {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(Ok(a)), Some(Ok(b))) => Arc::ptr_eq(a, b),
            (Some(Err(a)), Some(Err(b))) => a == b,
            _ => false,
        }
    }
}

/// Cheap fingerprint of the referenced file (length, FNV-1a hash, content
/// type), so editing other parts of the GLDF does not re-parse it.
fn file_fingerprint(s: &crate::state::GldfState, file_id: &str) -> Option<(usize, u64, String)> {
    let bytes = s.embedded_files.get(file_id)?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    let ct = s
        .product
        .general_definitions
        .files
        .file
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| f.content_type.clone())
        .unwrap_or_default();
    Some((bytes.len(), h, ct))
}

fn motion_key(m: Motion) -> &'static str {
    match m {
        Motion::Tangential => "gldf.sensors.motion_tangential",
        Motion::Radial => "gldf.sensors.motion_radial",
        Motion::Presence => "gldf.sensors.motion_presence",
    }
}

fn quality_key(q: Quality) -> &'static str {
    match q {
        Quality::Measured => "gldf.sensors.detection_q_measured",
        Quality::Interpolated => "gldf.sensors.detection_q_interpolated",
        Quality::Extrapolated => "gldf.sensors.detection_q_extrapolated",
    }
}

/// Detection areas of the referenced sensor file (sensor LDT or SensXML),
/// computed with `eulumdat::sensor`: plan view and one row per motion type
/// at an adjustable height.
#[component]
fn SensorDetection(file_id: String) -> impl IntoView {
    let state = use_gldf();
    let lang = move || state.with(|s| s.ui_language.clone());
    let tr = move |key: &str| ui_t(key, &lang());

    let fid = file_id.clone();
    let fingerprint = Memo::new(move |_| state.with(|s| file_fingerprint(s, &fid)));
    let fid = file_id.clone();
    let parsed = Memo::new(move |_| {
        let Some((_, _, ct)) = fingerprint.get() else {
            return ParsedSensor(None);
        };
        let res = state.with_untracked(|s| {
            s.embedded_files
                .get(&fid)
                .map(|bytes| SensorFile::from_bytes(bytes, &ct).map(Arc::new))
        });
        ParsedSensor(res)
    });
    let height: RwSignal<Option<f64>> = RwSignal::new(None);
    let planning = RwSignal::new(false);

    view! {
        <div class="sensor-detection" style="margin-top: 12px; border-top: 1px solid var(--border-color); padding-top: 10px;">
            <h4 style="margin: 0 0 6px;">{ move || tr("gldf.sensors.detection_title") }</h4>
            {move || match parsed.get().0 {
                None => view! {
                    <p class="form-help">{ tr("gldf.sensors.detection_no_bytes") }</p>
                }.into_any(),
                Some(Err(e)) => view! {
                    <p style="color: var(--accent-orange);">{ tr("gldf.sensors.detection_error").replace("{err}", &e) }</p>
                }.into_any(),
                Some(Ok(file)) => {
                    let is_xml = file.is_sensxml();
                    let default_h = file.default_height();
                    let measured = file.measured_heights();
                    let mut head = vec![file.kind_label().to_string()];
                    let name = file.name();
                    if !name.is_empty() {
                        head.push(name);
                    }
                    if let Some(m) = file.measurement_type() {
                        head.push(tr("gldf.sensors.detection_measurement").replace("{m}", &m));
                    }
                    let warnings: Vec<String> = file
                        .warnings()
                        .into_iter()
                        .map(|w| match w {
                            SensorWarning::ImplausibleRange { max } => tr("gldf.sensors.detection_implausible")
                                .replace("{max}", &format!("{max:.0}")),
                            SensorWarning::NoSensorParameters => tr("gldf.sensors.detection_no_params"),
                            SensorWarning::Reader(s) => s,
                        })
                        .collect();
                    let h_now = move || height.get().unwrap_or(default_h);
                    let f_svg = file.clone();
                    let svg = move || {
                        let l = lang();
                        f_svg.plan_svg_with_labels(h_now(), planning.get(), 420.0, &SvgTheme::light(), &|m| {
                            ui_t(motion_key(m), &l)
                        })
                    };
                    let f_rows = file.clone();
                    view! {
                        <p class="form-help" style="margin: 0 0 6px;">{ head.join(" · ") }</p>
                        {warnings.into_iter().map(|w| view! {
                            <p style="color: var(--accent-orange); margin: 0 0 6px;">{ format!("⚠ {w}") }</p>
                        }).collect::<Vec<_>>()}
                        <div style="display: flex; gap: 14px; align-items: center; flex-wrap: wrap;">
                            <label>
                                { if is_xml { tr("gldf.sensors.detection_height_xml") } else { tr("gldf.sensors.detection_height_ldt") } }
                                " "
                                <input
                                    type="number" step="0.1" min="0.3" max="40"
                                    style="width: 80px;"
                                    data-testid="sensor-height"
                                    prop:value=move || format!("{:.2}", h_now())
                                    on:change=move |e| {
                                        if let Ok(v) = event_target_value(&e).trim().parse::<f64>() {
                                            if v > 0.0 { height.set(Some(v)); }
                                        }
                                    }
                                />
                                " m"
                            </label>
                            {(!measured.is_empty()).then(|| {
                                let list = measured.iter().map(|h| format!("{h:.1}")).collect::<Vec<_>>().join(" / ");
                                view! { <span class="form-help">{ format!("{} {list} m", tr("gldf.sensors.detection_measured_heights")) }</span> }
                            })}
                            {is_xml.then(|| view! {
                                <label>
                                    <input type="checkbox"
                                        prop:checked=move || planning.get()
                                        on:change=move |e| planning.set(event_target_checked(&e)) />
                                    " " { tr("gldf.sensors.detection_planning") }
                                </label>
                            })}
                        </div>
                        <div style="display: flex; gap: 16px; flex-wrap: wrap; align-items: flex-start; margin-top: 8px;">
                            <div style="background: #fff; border-radius: 6px; width: 420px; max-width: 100%;" inner_html=svg />
                            <table class="data-table" style="border-collapse: collapse; font-size: 13px;">
                                <thead><tr>
                                    <th style="text-align: left; padding: 3px 8px;">{ tr("gldf.sensors.detection_motion") }</th>
                                    <th style="text-align: right; padding: 3px 8px;">{ tr("gldf.sensors.detection_max_range") }</th>
                                    <th style="text-align: right; padding: 3px 8px;">{ tr("gldf.sensors.detection_area") }</th>
                                    <th style="text-align: right; padding: 3px 8px;">{ tr("gldf.sensors.detection_diameter") }</th>
                                    {is_xml.then(|| view! { <th style="text-align: left; padding: 3px 8px;">{ tr("gldf.sensors.detection_quality") }</th> })}
                                </tr></thead>
                                <tbody>
                                    {move || {
                                        let l = lang();
                                        f_rows.areas(h_now(), planning.get()).into_iter().map(|a| {
                                            let label = ui_t(motion_key(a.motion), &l);
                                            let quality = a.quality.map(|q| ui_t(quality_key(q), &l)).unwrap_or_default();
                                            if a.detected() {
                                                view! {
                                                    <tr>
                                                        <td style="padding: 3px 8px;">{ label }</td>
                                                        <td style="text-align: right; padding: 3px 8px;">{ format!("{:.2} m", a.max_range()) }</td>
                                                        <td style="text-align: right; padding: 3px 8px;">{ format!("{:.1} m²", a.area_m2()) }</td>
                                                        <td style="text-align: right; padding: 3px 8px;">{ format!("{:.1} m", a.diameter()) }</td>
                                                        {is_xml.then(|| view! { <td style="padding: 3px 8px;">{ quality.clone() }</td> })}
                                                    </tr>
                                                }.into_any()
                                            } else {
                                                let mut why = ui_t("gldf.sensors.detection_not_detected", &l);
                                                if let Some(d) = a.depth_limit {
                                                    why = format!("{why} · {}", ui_t("gldf.sensors.detection_volume_ends", &l).replace("{d}", &format!("{d:.2}")));
                                                }
                                                view! {
                                                    <tr>
                                                        <td style="padding: 3px 8px;">{ label }</td>
                                                        <td colspan="4" style="padding: 3px 8px; color: var(--text-tertiary);">{ why }</td>
                                                    </tr>
                                                }.into_any()
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </tbody>
                            </table>
                        </div>
                        <p class="form-help" style="margin-top: 8px;">{ tr("gldf.sensors.detection_note") }</p>
                    }.into_any()
                }
            }}
        </div>
    }
}
