//! L3D 3D rendering module
//!
//! Launches a separate viewer process for interactive 3D viewing

use std::io::Write;
use std::process::Command;

/// Open an interactive 3D viewer window for the L3D model
/// Spawns a separate process to avoid macOS event loop restrictions
pub fn open_l3d_viewer(content: Vec<u8>, title: &str) {
    let title = title.to_string();

    // Spawn in a separate thread to handle temp file creation
    std::thread::spawn(move || {
        if let Err(e) = launch_viewer_process(&content, &title) {
            log::error!("Failed to launch L3D viewer: {}", e);
        }
    });
}

fn launch_viewer_process(content: &[u8], title: &str) -> Result<(), String> {
    // Create a temp file for the L3D content
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("gldf_l3d_{}.l3d", std::process::id()));

    // Write content to temp file
    {
        let mut file = std::fs::File::create(&temp_file)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        file.write_all(content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
    }

    // Find the viewer executable
    let viewer_exe = find_viewer_executable()?;

    // Spawn the viewer process
    Command::new(&viewer_exe)
        .arg(temp_file.to_string_lossy().as_ref())
        .arg(title)
        .spawn()
        .map_err(|e| format!("Failed to spawn viewer process: {}", e))?;

    // Note: temp file will be cleaned up by OS or on next run
    // We can't delete it immediately as the subprocess needs it

    Ok(())
}

fn find_viewer_executable() -> Result<std::path::PathBuf, String> {
    // Try to find gldf-l3d-viewer in common locations
    let exe_name = if cfg!(windows) {
        "gldf-l3d-viewer.exe"
    } else {
        "gldf-l3d-viewer"
    };

    // 1. Same directory as current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let viewer_path = dir.join(exe_name);
            if viewer_path.exists() {
                return Ok(viewer_path);
            }
        }
    }

    // 2. Check PATH
    if let Ok(output) = Command::new("which").arg(exe_name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(std::path::PathBuf::from(path));
            }
        }
    }

    // 3. Check common development paths
    let dev_paths = [
        "./target/release/gldf-l3d-viewer",
        "./target/debug/gldf-l3d-viewer",
        "../target/release/gldf-l3d-viewer",
        "../target/debug/gldf-l3d-viewer",
    ];

    for path in dev_paths {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    Err(format!("Could not find '{}' executable. Make sure it's built and in the same directory as the main app.", exe_name))
}
