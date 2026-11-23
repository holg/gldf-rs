//! Leptos UI components. Reborn one slice at a time from the Yew side;
//! see `docs/leptos_migration.md` for the porting checklist.

pub mod aggregate;
mod builder_widget;
mod edit_modal;
pub mod edit_util;
mod emitter_type_form;
mod id_rename_chip;
#[cfg(target_arch = "wasm32")]
mod l3d_panel;
mod ldt_panel;
mod light_source_export;
mod lisp_panel;
mod locale_edit;
mod locale_view;
pub mod pages;
pub mod photometry_decode;
mod scope_picker;
mod spd_viewer;
mod tm33_panel;
mod validation_banner;
mod wizard;

pub use builder_widget::BuilderWidget;
pub use edit_modal::EditModal;
pub use emitter_type_form::{EmitterDraft, EmitterTypeForm};
pub use id_rename_chip::IdRenameChip;
#[cfg(target_arch = "wasm32")]
pub use l3d_panel::L3dPanel;
pub use ldt_panel::LdtPanel;
pub use light_source_export::LightSourceExportMenu;
pub use lisp_panel::LispPanel;
pub use locale_edit::LocaleFooEditor;
pub use locale_view::LocaleFooView;
pub use pages::{
    ApplicationsEditorPage, ApplicationsReadPage, BevyViewerPage, ControlGearPage,
    CustomPropertiesPage, ElectricalEditorPage, ElectricalReadPage, EmergencyEditorPage,
    EmergencyReadPage, EmittersPage, EquipmentPage, FileViewerPage, FilesListPage, GeometriesPage,
    HeaderDashboardPage, HeaderEditorPage, HyperlinksEditorPage, HyperlinksReadPage,
    IdentityEditorPage, IfcViewerPage, LabelsEditorPage, LabelsReadPage, LightSourcesPage,
    LinkagePage, LinkageTarget, MarketingEditorPage, MechanicalEditorPage, MechanicalReadPage,
    NavJump, OverviewPage, PhotometryEditorPage, PhotometryReadPage, PicturesEditorPage,
    PicturesReadPage, PluginsPage, RawDataPage, SensorsEditorPage, SensorsReadPage, StatisticsPage,
    VariantEditorPage, VariantsPage,
};
pub use scope_picker::ScopePicker;
pub use spd_viewer::{SpdPanel, SpdViewer, SvgEmbed};
pub use tm33_panel::{is_tm33, Tm33Panel};
pub use validation_banner::ValidationBanner;
pub use wizard::NewGldfWizard;
