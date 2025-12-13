// Lazy loader for GLDF Bevy 3D Scene Viewer
// Loads Bevy WASM on demand when viewing L3D files

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

// Storage keys for L3D/LDT data
const L3D_STORAGE_KEY = 'gldf_current_l3d';
const LDT_STORAGE_KEY = 'gldf_current_ldt';
const EMITTER_CONFIG_KEY = 'gldf_emitter_config';
const GLDF_TIMESTAMP_KEY = 'gldf_timestamp';

/**
 * Save L3D data to localStorage for Bevy viewer
 * @param {Uint8Array} l3dData - L3D file bytes
 * @param {string|null} ldtData - LDT file content (optional)
 * @param {string|null} emitterConfig - JSON string of emitter configurations (optional)
 */
function saveL3dForBevy(l3dData, ldtData, emitterConfig) {
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

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(GLDF_TIMESTAMP_KEY, ts);
        console.log('[Bevy] ✅ All data saved to localStorage, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] ❌ Failed to save L3D data:', e);
    }
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

// Expose to window for WASM to call
window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;
window.saveL3dForBevy = saveL3dForBevy;

console.log('[Bevy] Loader ready');
