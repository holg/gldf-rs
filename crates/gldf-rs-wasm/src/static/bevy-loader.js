// Lazy loader for GLDF Bevy 3D Scene Viewer
// Loads Bevy WASM on demand when viewing L3D files

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

// Storage keys for L3D/LDT data
const L3D_STORAGE_KEY = 'gldf_current_l3d';
const LDT_STORAGE_KEY = 'gldf_current_ldt';
const EMITTER_CONFIG_KEY = 'gldf_emitter_config';
const MOUNTING_CONFIG_KEY = 'gldf_mounting_config';
const GLDF_TIMESTAMP_KEY = 'gldf_timestamp';

// Storage keys for IFC geometry data
const IFC_GEOMETRY_KEY = 'ifc_current_geometry';
const IFC_VARIANT_KEY = 'ifc_current_variant';
const IFC_TIMESTAMP_KEY = 'ifc_timestamp';

/**
 * Save L3D data to localStorage for Bevy viewer
 * @param {Uint8Array} l3dData - L3D file bytes
 * @param {string|null} ldtData - LDT file content (optional)
 * @param {string|null} emitterConfig - JSON string of emitter configurations (optional)
 * @param {string|null} mountingConfig - JSON string of mounting configuration (optional)
 */
function saveL3dForBevy(l3dData, ldtData, emitterConfig, mountingConfig) {
    console.log('[Bevy] saveL3dForBevy called with:', l3dData?.length, 'bytes L3D');
    try {
        // Convert to base64 for storage (handle large arrays properly)
        let binary = '';
        const bytes = new Uint8Array(l3dData);
        const chunkSize = 0x8000; // Process in chunks to avoid stack overflow
        for (let i = 0; i < bytes.length; i += chunkSize) {
            const chunk = bytes.subarray(i, Math.min(i + chunkSize, bytes.length));
            binary += String.fromCharCode.apply(null, chunk);
        }
        const base64 = btoa(binary);
        console.log('[Bevy] Base64 length:', base64.length);
        localStorage.setItem(L3D_STORAGE_KEY, base64);

        if (ldtData) {
            localStorage.setItem(LDT_STORAGE_KEY, ldtData);
            console.log('[Bevy] LDT stored, length:', ldtData.length);
        } else {
            localStorage.removeItem(LDT_STORAGE_KEY);
        }

        // Store emitter config for per-emitter rendering
        if (emitterConfig) {
            localStorage.setItem(EMITTER_CONFIG_KEY, emitterConfig);
            console.log('[Bevy] Emitter config stored:', emitterConfig);
        } else {
            localStorage.removeItem(EMITTER_CONFIG_KEY);
        }

        // Store mounting config for luminaire positioning
        if (mountingConfig) {
            localStorage.setItem(MOUNTING_CONFIG_KEY, mountingConfig);
            console.log('[Bevy] Mounting config stored:', mountingConfig);
        } else {
            localStorage.removeItem(MOUNTING_CONFIG_KEY);
        }

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(GLDF_TIMESTAMP_KEY, ts);
        console.log('[Bevy] ✅ All data saved to localStorage, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] ❌ Failed to save L3D data:', e);
    }
}

/**
 * Clear all L3D/GLDF data from localStorage
 * Should be called before loading a new GLDF to prevent stale data
 */
function clearL3dData() {
    console.log('[Bevy] clearL3dData called - clearing all GLDF data');
    localStorage.removeItem(L3D_STORAGE_KEY);
    localStorage.removeItem(LDT_STORAGE_KEY);
    localStorage.removeItem(EMITTER_CONFIG_KEY);
    localStorage.removeItem(MOUNTING_CONFIG_KEY);
    localStorage.removeItem(GLDF_TIMESTAMP_KEY);
    // Also clear IFC data
    localStorage.removeItem(IFC_GEOMETRY_KEY);
    localStorage.removeItem(IFC_VARIANT_KEY);
    localStorage.removeItem(IFC_TIMESTAMP_KEY);
    console.log('[Bevy] ✅ All viewer data cleared');
}

/**
 * Load and initialize the Bevy 3D viewer
 * @returns {Promise<void>}
 */
async function loadBevyViewer() {
    console.log('[Bevy] 📥 loadBevyViewer called, bevyLoaded:', bevyLoaded, 'bevyLoading:', bevyLoading);

    if (bevyLoaded) {
        console.log('[Bevy] Already loaded, skipping');
        return;
    }
    if (bevyLoading && loadPromise) {
        console.log('[Bevy] Loading in progress, waiting...');
        return loadPromise;
    }

    bevyLoading = true;
    console.log('[Bevy] 🚀 Starting to load 3D viewer...');

    // Check localStorage before loading
    const l3dData = localStorage.getItem('gldf_current_l3d');
    const ldtData = localStorage.getItem('gldf_current_ldt');
    const timestamp = localStorage.getItem('gldf_timestamp');
    console.log('[Bevy] localStorage state: L3D:', l3dData?.length || 0, 'chars, LDT:', ldtData?.length || 0, 'chars, timestamp:', timestamp);

    loadPromise = (async () => {
        try {
            // Fetch manifest to get hashed filename
            console.log('[Bevy] 📦 Fetching manifest...');
            const manifestResp = await fetch('./bevy/manifest.json');
            const manifest = await manifestResp.json();
            console.log('[Bevy] 📦 Manifest:', manifest);

            // Import the Bevy module with hashed filename
            const modulePath = `./bevy/${manifest.js}`;
            console.log('[Bevy] 📦 Importing module:', modulePath);
            const bevy = await import(modulePath);
            console.log('[Bevy] 📦 Module imported, initializing WASM...');
            await bevy.default();
            console.log('[Bevy] 📦 WASM initialized, calling run_on_canvas("#bevy-canvas")...');
            bevy.run_on_canvas("#bevy-canvas");

            bevyLoaded = true;
            bevyLoading = false;
            console.log('[Bevy] ✅ 3D viewer loaded successfully');
        } catch (error) {
            const errorStr = error.toString();
            console.log('[Bevy] Caught error:', errorStr.substring(0, 200));
            // Bevy uses exceptions for control flow - ignore these
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log('[Bevy] ✅ Ignoring control flow exception (Bevy is running)');
                bevyLoaded = true;
                bevyLoading = false;
                return;
            }
            console.error('[Bevy] ❌ Failed to load 3D viewer:', error);
            bevyLoading = false;
            loadPromise = null;
            throw error;
        }
    })();

    return loadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

/**
 * Save IFC geometry data to localStorage for Bevy viewer
 * @param {string} geometryJson - JSON string containing geometry data (vertices, triangles)
 * @param {string|null} variantName - Optional variant name for display
 */
function saveIfcGeometryForBevy(geometryJson, variantName) {
    console.log('[Bevy] saveIfcGeometryForBevy called, JSON length:', geometryJson?.length);
    try {
        localStorage.setItem(IFC_GEOMETRY_KEY, geometryJson);

        if (variantName) {
            localStorage.setItem(IFC_VARIANT_KEY, variantName);
            console.log('[Bevy] IFC variant:', variantName);
        } else {
            localStorage.removeItem(IFC_VARIANT_KEY);
        }

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(IFC_TIMESTAMP_KEY, ts);
        console.log('[Bevy] ✅ IFC geometry saved to localStorage, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] ❌ Failed to save IFC geometry:', e);
    }
}

/**
 * Clear IFC geometry data from localStorage
 */
function clearIfcGeometry() {
    console.log('[Bevy] clearIfcGeometry called');
    localStorage.removeItem(IFC_GEOMETRY_KEY);
    localStorage.removeItem(IFC_VARIANT_KEY);
    localStorage.removeItem(IFC_TIMESTAMP_KEY);
    console.log('[Bevy] ✅ IFC geometry cleared');
}

// Expose to window for WASM to call
window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;
window.saveL3dForBevy = saveL3dForBevy;
window.saveIfcGeometryForBevy = saveIfcGeometryForBevy;
window.clearIfcGeometry = clearIfcGeometry;
window.clearL3dData = clearL3dData;

// Clear stale viewer data on page load to prevent showing old models
// This runs every time the page is refreshed
clearL3dData();
console.log('[Bevy] Loader ready - cleared any stale viewer data');
