//! Main application state and logic

use eframe::egui;
use gldf_rs::{gldf::GldfProduct, BufFile, FileBufGldf};
use std::path::PathBuf;

/// Navigation items for sidebar
#[derive(Clone, Copy, PartialEq, Default, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum NavItem {
    #[default]
    Overview,
    RawData,
    FileViewer,
    Header,
    Statistics,
    Files,
    LightSources,
    Variants,
}

/// File type categorization
#[derive(Clone, PartialEq, Debug)]
pub enum FileCategory {
    Photometry,
    Image,
    Geometry,
    Document,
    Other,
}

/// Information about embedded files
#[derive(Clone, Debug)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub category: FileCategory,
}

/// Statistics about the loaded GLDF
#[derive(Clone, Debug, Default)]
pub struct GldfStats {
    pub files_count: usize,
    pub fixed_light_sources: usize,
    pub changeable_light_sources: usize,
    pub variants_count: usize,
    pub photometries_count: usize,
    pub geometries_count: usize,
}

/// Main application state
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct GldfViewerApp {
    /// Current navigation item
    pub nav_item: NavItem,

    /// Whether sidebar is expanded
    pub sidebar_expanded: bool,

    /// Status message
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub status_message: String,

    /// Is file being dragged over the window
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub is_dragging: bool,

    /// Loaded GLDF data
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub loaded_gldf: Option<FileBufGldf>,

    /// Current file path (native only)
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub current_path: Option<PathBuf>,

    /// Current file name
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub current_file_name: Option<String>,

    /// Cached statistics
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub stats: GldfStats,

    /// Cached file list
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub files_list: Vec<FileInfo>,

    /// Selected file for viewing
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub selected_file_id: Option<String>,

    /// Raw JSON content (cached for display)
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub raw_json: Option<String>,

    /// Dark mode
    pub dark_mode: bool,
}

impl Default for GldfViewerApp {
    fn default() -> Self {
        Self {
            nav_item: NavItem::Overview,
            sidebar_expanded: true,
            status_message: String::new(),
            is_dragging: false,
            loaded_gldf: None,
            current_path: None,
            current_file_name: None,
            stats: GldfStats::default(),
            files_list: Vec::new(),
            selected_file_id: None,
            raw_json: None,
            dark_mode: true,
        }
    }
}

impl GldfViewerApp {
    /// Create a new instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders for egui_extras
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Configure fonts
        configure_fonts(&cc.egui_ctx);

        // Set dark mode by default
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        // Load previous state if persistence is enabled
        #[cfg(feature = "persistence")]
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    /// Load GLDF from byte buffer
    pub fn load_from_bytes(&mut self, data: Vec<u8>, file_name: Option<String>) {
        self.status_message = "Loading...".to_string();

        match GldfProduct::load_gldf_from_buf_all(data) {
            Ok(gldf) => {
                // Collect URL-type files before consuming gldf
                let url_files: Vec<String> = gldf
                    .gldf
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .filter(|f| f.type_attr == "url")
                    .map(|f| f.file_name.clone())
                    .collect();

                self.update_from_gldf(gldf);
                self.current_file_name = file_name;

                // Start fetching URL files in background
                if !url_files.is_empty() {
                    self.status_message = format!("Loading {} URL files...", url_files.len());
                    crate::ui::file_viewer::fetch_all_urls(url_files);
                } else {
                    self.status_message = "File loaded successfully".to_string();
                }

                self.nav_item = NavItem::Overview;
            }
            Err(e) => {
                self.status_message = format!("Error loading file: {}", e);
            }
        }
    }

    /// Load GLDF from file path (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_path(&mut self, path: PathBuf) {
        self.status_message = "Loading...".to_string();

        match std::fs::read(&path) {
            Ok(data) => {
                let file_name = path.file_name().map(|s| s.to_string_lossy().to_string());
                self.current_path = Some(path);
                self.load_from_bytes(data, file_name);
            }
            Err(e) => {
                self.status_message = format!("Error reading file: {}", e);
            }
        }
    }

    /// Update app state from loaded GLDF
    fn update_from_gldf(&mut self, mut gldf: FileBufGldf) {
        // Calculate statistics
        let light_sources = gldf.gldf.general_definitions.light_sources.as_ref();
        let fixed_ls = light_sources
            .map(|ls| ls.fixed_light_source.len())
            .unwrap_or(0);
        let changeable_ls = light_sources
            .map(|ls| ls.changeable_light_source.len())
            .unwrap_or(0);

        let variants_count = gldf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .map(|v| v.variant.len())
            .unwrap_or(0);

        let photometries_count = gldf
            .gldf
            .general_definitions
            .photometries
            .as_ref()
            .map(|p| p.photometry.len())
            .unwrap_or(0);

        let geometries = gldf.gldf.general_definitions.geometries.as_ref();
        let simple_geo = geometries.map(|g| g.simple_geometry.len()).unwrap_or(0);
        let model_geo = geometries.map(|g| g.model_geometry.len()).unwrap_or(0);

        // Add URL-type files from XML definitions to the files list
        // These are files referenced by URL, not embedded in the ZIP
        for file_def in &gldf.gldf.general_definitions.files.file {
            if file_def.type_attr == "url" {
                // Check if this URL file is already in the list
                let already_exists = gldf
                    .files
                    .iter()
                    .any(|f| f.name.as_deref() == Some(&file_def.file_name));
                if !already_exists {
                    // Add a BufFile entry for the URL reference (no content, will be fetched)
                    gldf.files.push(BufFile {
                        name: Some(file_def.file_name.clone()),
                        content: None, // No content - this is a URL to be fetched
                        file_id: Some(file_def.id.clone()),
                        path: Some(file_def.file_name.clone()),
                    });
                }
            }
        }

        self.stats = GldfStats {
            files_count: gldf.files.len(),
            fixed_light_sources: fixed_ls,
            changeable_light_sources: changeable_ls,
            variants_count,
            photometries_count,
            geometries_count: simple_geo + model_geo,
        };

        // Build file list
        self.files_list = gldf
            .gldf
            .general_definitions
            .files
            .file
            .iter()
            .map(|f| {
                let category = categorize_content_type(&f.content_type);
                FileInfo {
                    id: f.id.clone(),
                    name: f.file_name.clone(),
                    content_type: f.content_type.clone(),
                    category,
                }
            })
            .collect();

        // Cache raw JSON
        self.raw_json = gldf.gldf.to_pretty_json().ok();

        self.loaded_gldf = Some(gldf);
    }

    /// Check if a file is loaded
    pub fn has_file(&self) -> bool {
        self.loaded_gldf.is_some()
    }

    /// Get the header information
    pub fn header(&self) -> Option<&gldf_rs::gldf::header::Header> {
        self.loaded_gldf.as_ref().map(|g| &g.gldf.header)
    }

    /// Get embedded files
    pub fn embedded_files(&self) -> &[BufFile] {
        self.loaded_gldf
            .as_ref()
            .map(|g| g.files.as_slice())
            .unwrap_or(&[])
    }

    /// Export to JSON (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_json(&self) {
        if let Some(ref gldf) = self.loaded_gldf {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON Files", &["json"])
                .set_file_name("export.json")
                .save_file()
            {
                if let Ok(json) = gldf.gldf.to_pretty_json() {
                    if let Err(e) = std::fs::write(&path, json) {
                        eprintln!("Error saving JSON: {}", e);
                    }
                }
            }
        }
    }

    /// Export to XML (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_xml(&self) {
        if let Some(ref gldf) = self.loaded_gldf {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("XML Files", &["xml"])
                .set_file_name("product.xml")
                .save_file()
            {
                if let Ok(xml) = gldf.gldf.to_xml() {
                    if let Err(e) = std::fs::write(&path, xml) {
                        eprintln!("Error saving XML: {}", e);
                    }
                }
            }
        }
    }
}

impl eframe::App for GldfViewerApp {
    /// Called by eframe to save state before shutdown
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each frame to update and render
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Handle file drops
        crate::ui::handle_file_drops(ctx, self);

        // Render UI
        crate::ui::render_ui(ctx, self);
    }
}

/// Configure custom fonts
fn configure_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();

    // Configure font priorities if needed
    // fonts.font_data.insert(...);

    ctx.set_fonts(fonts);
}

/// Categorize file by content type
fn categorize_content_type(content_type: &str) -> FileCategory {
    if content_type.starts_with("ldc/") {
        FileCategory::Photometry
    } else if content_type.starts_with("image/") {
        FileCategory::Image
    } else if content_type.starts_with("geo/") {
        FileCategory::Geometry
    } else if content_type.starts_with("document/") {
        FileCategory::Document
    } else {
        FileCategory::Other
    }
}
