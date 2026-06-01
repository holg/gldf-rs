//! Leptos UI components. Reborn one slice at a time from the Yew side;
//! see `docs/leptos_migration.md` for the porting checklist.

pub mod aggregate;
mod builder_widget;
mod edit_modal;
pub mod edit_util;
mod emitter_type_form;
mod id_rename_chip;
mod l3d_panel;
pub mod photometry_decode;
mod wizard;
mod ldt_panel;
mod locale_edit;
mod locale_view;
pub mod pages;
mod scope_picker;
mod spd_viewer;

pub use builder_widget::BuilderWidget;
pub use edit_modal::EditModal;
pub use emitter_type_form::{EmitterDraft, EmitterTypeForm};
pub use id_rename_chip::IdRenameChip;
pub use wizard::NewGldfWizard;
pub use l3d_panel::L3dPanel;
pub use ldt_panel::LdtPanel;
pub use locale_edit::LocaleFooEditor;
pub use locale_view::LocaleFooView;
pub use pages::{
    ApplicationsEditorPage, ApplicationsReadPage, CustomPropertiesPage, ElectricalEditorPage,
    ElectricalReadPage, EmergencyEditorPage, EmergencyReadPage, EmittersPage, FileViewerPage,
    FilesListPage, HeaderDashboardPage, HeaderEditorPage, HyperlinksEditorPage,
    HyperlinksReadPage, IdentityEditorPage, LabelsEditorPage, LabelsReadPage, LightSourcesPage,
    LinkagePage, LinkageTarget, MechanicalEditorPage, MechanicalReadPage, NavJump, OverviewPage,
    PhotometryEditorPage, PhotometryReadPage, PicturesEditorPage, PicturesReadPage, PluginsPage,
    RawDataPage, StatisticsPage, VariantEditorPage, VariantsPage,
};
pub use scope_picker::ScopePicker;
pub use spd_viewer::{SpdPanel, SpdViewer, SvgEmbed};
