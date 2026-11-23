//! Drive the TUI wizard overlay via synthetic key events and verify the
//! produced `WizardDraft` plus the post-`apply_wizard` `GldfState`.
//! No terminal needed — this is the model-side smoke test for the same
//! reducer the WASM editor uses.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gldf_rs_tui::wizard::{self, WizardOutcome, WizardOverlay};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn type_str(overlay: &mut WizardOverlay, s: &str) {
    for c in s.chars() {
        overlay.on_key(key(KeyCode::Char(c)));
    }
}
fn next_field(overlay: &mut WizardOverlay) {
    overlay.on_key(key(KeyCode::Down));
}
fn tab(overlay: &mut WizardOverlay) {
    overlay.on_key(key(KeyCode::Tab));
}

#[test]
fn wizard_overlay_produces_state_matching_cli_run() {
    let mut overlay = WizardOverlay::new();

    // Step 1: Header — type into the 4 fields, advance with ↓.
    type_str(&mut overlay, "TUI Tester");
    next_field(&mut overlay);
    type_str(&mut overlay, "Acme TUI");
    next_field(&mut overlay);
    // header_created_with has a default; just leave it.
    next_field(&mut overlay);
    // header_default_language defaults to "en"; leave it.
    tab(&mut overlay); // → Photometry

    // Step 2: Photometry — leave blank, advance.
    tab(&mut overlay); // → LightSource

    // Step 3: LightSource — Fixed is the default, skip Kind; fill Name.
    next_field(&mut overlay); // focus -> Name
    type_str(&mut overlay, "Main LED");
    next_field(&mut overlay); // focus -> Manufacturer
    type_str(&mut overlay, "Acme Optics");
    next_field(&mut overlay); // focus -> Rated power
    type_str(&mut overlay, "30");
    tab(&mut overlay); // → Emitter

    // Step 4: Emitter — keep "include = yes", focus emitter name, type.
    next_field(&mut overlay); // focus -> name
                              // The default name is "Primary"; replace with something different.
    for _ in 0.."Primary".len() {
        overlay.on_key(key(KeyCode::Backspace));
    }
    type_str(&mut overlay, "First");
    tab(&mut overlay); // → Geometry
    tab(&mut overlay); // → Variant
    tab(&mut overlay); // → Review

    // Step 5: Review — Enter to finish.
    let outcome = match overlay.on_key(key(KeyCode::Enter)) {
        WizardOutcome::Finished(d) => d,
        WizardOutcome::Continue => panic!("Enter on Review should finish"),
        WizardOutcome::Cancelled => panic!("Enter on Review should not cancel"),
    };

    assert_eq!(outcome.header_author, "TUI Tester");
    assert_eq!(outcome.header_manufacturer, "Acme TUI");
    assert_eq!(outcome.header_default_language, "en");
    assert_eq!(outcome.ls_name, "Main LED");
    assert_eq!(outcome.ls_manufacturer, "Acme Optics");
    assert_eq!(outcome.ls_rated_power, "30");
    let emitter = outcome.emitter_draft.as_ref().expect("emitter included");
    assert_eq!(emitter.child_name, "First");

    let state = wizard::finish(outcome).expect("apply_wizard succeeds");
    assert_eq!(state.product.header.author, "TUI Tester");
    assert_eq!(state.product.header.manufacturer, "Acme TUI");
    let ls = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources present");
    assert_eq!(ls.fixed_light_source.len(), 1);
    assert_eq!(
        ls.fixed_light_source[0].manufacturer.as_deref(),
        Some("Acme Optics")
    );
    assert_eq!(ls.fixed_light_source[0].rated_input_power, Some(30.0));

    let emitters = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .expect("emitters present");
    assert_eq!(emitters.emitter.len(), 1);

    // Sanity: the GLDF saves to bytes.
    let bytes = state.save_to_buf().expect("save");
    assert!(bytes.len() > 100);
}

#[test]
fn wizard_overlay_cancels_on_esc() {
    let mut overlay = WizardOverlay::new();
    match overlay.on_key(key(KeyCode::Esc)) {
        WizardOutcome::Cancelled => {}
        _ => panic!("Esc should cancel"),
    }
}

#[test]
fn quitting_with_unsaved_wizard_output_opens_confirm_modal() {
    use gldf_rs_tui::app::{App, Modal};

    let mut app = App::new();
    // Pretend the wizard just finished — content is in state, modified flag set.
    app.has_content = true;
    app.state.is_modified = true;

    // The user presses `q`. With unsaved content this MUST NOT exit;
    // it must open the QuitConfirm modal so they can choose to save.
    app.request_quit();
    assert!(app.running, "still running while modal is up");
    match &app.modal {
        Some(Modal::QuitConfirm { selected }) => assert_eq!(*selected, 0),
        other => panic!("expected QuitConfirm modal, got {:?}", other.is_some()),
    }
}

#[test]
fn header_action_menu_dispatches_setauthor() {
    use gldf_rs_tui::app::Section;
    use gldf_rs_tui::edit_modal::{section_fields, EditOutcome};

    let mut state = gldf_rs_editor::GldfState::default();
    // 1. Build the section's editable fields and find the Author field.
    let fields = section_fields(Section::Header, 0, &state);
    let author = fields
        .iter()
        .find(|f| f.label.contains("Author"))
        .expect("Author field present on Header section");
    // 2. Build the FieldEdit, type a name, commit.
    let mut field = (author.make)(&state, 0);
    for c in "Acme TUI".chars() {
        let (next, _) = field.on_key(KeyCode::Char(c));
        field = next.expect("typing keeps the editor open");
    }
    let (_done, outcome) = field.on_key(KeyCode::Enter);
    let action = match outcome {
        EditOutcome::Committed(a) => a,
        EditOutcome::Continue => panic!("Enter should commit"),
        EditOutcome::Cancelled => panic!("Enter should not cancel"),
    };
    state.apply(action);
    assert_eq!(state.product.header.author, "Acme TUI");
    assert!(state.is_modified);
}

#[test]
fn lightsource_rename_dispatches_rename_action() {
    use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
    use gldf_rs_editor::GldfAction;
    use gldf_rs_tui::app::Section;
    use gldf_rs_tui::edit_modal::{section_fields, EditOutcome};

    let mut state = gldf_rs_editor::GldfState::default();
    state.apply(GldfAction::AddFixedLightSource {
        payload: Box::new(FixedLightSource::default()),
    });
    let old_id = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .unwrap()
        .fixed_light_source[0]
        .id
        .clone();

    let fields = section_fields(Section::LightSources, 0, &state);
    let rename = fields
        .iter()
        .find(|f| f.label.starts_with("Rename id"))
        .expect("Rename field present on LightSource row");
    let mut field = (rename.make)(&state, 0);
    // Wipe the seeded id and type a new one.
    while !field.buffer.is_empty() {
        let (next, _) = field.on_key(KeyCode::Backspace);
        field = next.expect("backspace keeps the editor open");
    }
    for c in "ls_main".chars() {
        let (next, _) = field.on_key(KeyCode::Char(c));
        field = next.expect("typing keeps the editor open");
    }
    let (_done, outcome) = field.on_key(KeyCode::Enter);
    let action = match outcome {
        EditOutcome::Committed(a) => a,
        _ => panic!("Enter should commit"),
    };
    state.apply(action);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);
    let new_id = &state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .unwrap()
        .fixed_light_source[0]
        .id;
    assert_eq!(new_id, "ls_main", "renamed from {} to ls_main", old_id);
}

#[test]
fn polar_view_parses_test_create_ldt() {
    use gldf_rs_tui::polar_view;

    let ldt = include_bytes!("../../../tests/test-create/road.ldt");
    let parsed = polar_view::parse_eulumdat(ldt);
    let (ldt, polar) = parsed.expect("road.ldt should parse as EULUMDAT");
    assert!(
        !ldt.luminaire_name.is_empty() || polar.scale.max_intensity > 0.0,
        "polar diagram has meaningful data"
    );
}

#[test]
fn quitting_without_unsaved_changes_exits_directly() {
    use gldf_rs_tui::app::App;

    let mut app = App::new();
    // Welcome screen: nothing loaded, nothing modified.
    app.request_quit();
    assert!(!app.running, "no content + no changes → exit directly");
    assert!(app.modal.is_none());
}
