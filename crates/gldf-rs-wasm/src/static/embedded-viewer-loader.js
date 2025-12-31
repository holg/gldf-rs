/**
 * Embedded Viewer Loader for GLDF files
 *
 * Handles WASM viewers embedded inside GLDF files (in other/viewer/).
 * Creates blob URLs for the embedded content so they can be loaded
 * without external network requests.
 *
 * Tribute to Astrophysics!
 */

// Storage for embedded viewers
const embeddedViewers = {
    bevy: null,
    acadlisp: null,
    xlisp: null,
    starsky: null  // Lightweight 2D star sky viewer
};

/**
 * Register an embedded WASM viewer from GLDF
 * @param {string} viewerType - 'bevy', 'acadlisp', or 'xlisp'
 * @param {string} manifestJson - JSON string with viewer metadata
 * @param {string} jsContent - JavaScript module content
 * @param {Uint8Array} wasmBytes - WASM binary content
 */
function registerEmbeddedViewer(viewerType, manifestJson, jsContent, wasmBytes) {
    console.log(`[EmbeddedViewer] Registering ${viewerType} viewer`);

    try {
        // Create blob URL for WASM
        const wasmBlob = new Blob([wasmBytes], { type: 'application/wasm' });
        const wasmUrl = URL.createObjectURL(wasmBlob);

        // Modify JS to use the blob URL for WASM
        // Replace the relative WASM path with our blob URL
        let modifiedJs = jsContent;

        // Parse manifest to get original filenames
        const manifest = JSON.parse(manifestJson);

        if (viewerType === 'bevy') {
            // Replace WASM URL in Bevy loader
            // The JS looks for: new URL('gldf-bevy-viewer-*_bg.wasm', import.meta.url)
            const wasmPattern = /new URL\(['"]([^'"]+_bg\.wasm)['"],\s*import\.meta\.url\)/g;
            modifiedJs = modifiedJs.replace(wasmPattern, `"${wasmUrl}"`);

            // Also handle fetch patterns
            modifiedJs = modifiedJs.replace(
                /fetch\(\s*['"]([^'"]+_bg\.wasm)['"]\s*\)/g,
                `fetch("${wasmUrl}")`
            );
        } else if (viewerType === 'acadlisp' || viewerType === 'xlisp') {
            // Replace WASM URL patterns for acadlisp/xlisp
            const wasmPattern = /['"]([^'"]+\.wasm)['"]/g;
            modifiedJs = modifiedJs.replace(wasmPattern, (match, filename) => {
                if (filename.endsWith('_bg.wasm') || filename === 'xlisp.wasm') {
                    return `"${wasmUrl}"`;
                }
                return match;
            });
        }

        // Create blob URL for modified JS
        const jsBlob = new Blob([modifiedJs], { type: 'application/javascript' });
        const jsUrl = URL.createObjectURL(jsBlob);

        embeddedViewers[viewerType] = {
            manifest,
            jsUrl,
            wasmUrl,
            jsContent: modifiedJs,
            wasmBytes,
            loaded: false
        };

        console.log(`[EmbeddedViewer] ${viewerType} registered:`, {
            jsUrl: jsUrl.substring(0, 50) + '...',
            wasmUrl: wasmUrl.substring(0, 50) + '...',
            wasmSize: wasmBytes.length
        });

        // Also register with plugin system if available
        if (window.registerWasmPlugin && manifest.capabilities) {
            console.log(`[EmbeddedViewer] Also registering ${viewerType} with plugin system...`);
            // Create files map for plugin system
            const files = {};
            if (manifest.js) files[manifest.js] = modifiedJs;
            if (manifest.wasm) files[manifest.wasm] = wasmBytes;
            // Register async but don't wait
            window.registerWasmPlugin(viewerType, manifestJson, files).catch(e => {
                console.log(`[EmbeddedViewer] Plugin registration note:`, e.message || e);
            });
        }

    } catch (error) {
        console.error(`[EmbeddedViewer] Failed to register ${viewerType}:`, error);
    }
}

/**
 * Check if an embedded viewer is available
 * @param {string} viewerType - 'bevy', 'acadlisp', or 'xlisp'
 * @returns {boolean}
 */
function hasEmbeddedViewer(viewerType) {
    return embeddedViewers[viewerType] !== null;
}

/**
 * Get the JS module URL for an embedded viewer
 * @param {string} viewerType
 * @returns {string|null}
 */
function getEmbeddedViewerJsUrl(viewerType) {
    return embeddedViewers[viewerType]?.jsUrl || null;
}

/**
 * Get the WASM URL for an embedded viewer
 * @param {string} viewerType
 * @returns {string|null}
 */
function getEmbeddedViewerWasmUrl(viewerType) {
    return embeddedViewers[viewerType]?.wasmUrl || null;
}

/**
 * Load the embedded Bevy viewer (if available)
 * Falls back to external loading if not embedded
 */
async function loadEmbeddedBevyViewer() {
    if (!hasEmbeddedViewer('bevy')) {
        console.log('[EmbeddedViewer] No embedded Bevy viewer, using external');
        return window.loadBevyViewer ? window.loadBevyViewer() : Promise.reject('No Bevy loader');
    }

    const viewer = embeddedViewers.bevy;
    if (viewer.loaded) {
        console.log('[EmbeddedViewer] Bevy already loaded');
        return;
    }

    console.log('[EmbeddedViewer] Loading embedded Bevy viewer...');

    try {
        const module = await import(viewer.jsUrl);
        await module.default();
        module.run_on_canvas("#bevy-canvas");
        viewer.loaded = true;
        console.log('[EmbeddedViewer] Embedded Bevy viewer loaded!');
    } catch (error) {
        // Bevy uses exceptions for control flow
        if (error.toString().includes("Using exceptions for control flow") ||
            error.toString().includes("don't mind me")) {
            viewer.loaded = true;
            console.log('[EmbeddedViewer] Bevy loaded (control flow exception)');
            return;
        }
        console.error('[EmbeddedViewer] Failed to load Bevy:', error);
        throw error;
    }
}

/**
 * Load the embedded AcadLISP viewer (if available)
 * Falls back to external loading if not embedded
 *
 * The architecture is:
 * - xlisp.wasm: Emscripten-compiled LISP interpreter core
 * - acadlisp WASM: wasm-bindgen wrapper with WasmEngine class
 *
 * We need to load xlisp first (as a script), then acadlisp (as ES module)
 */
async function loadEmbeddedAcadLisp() {
    // First check for embedded xlisp core (Emscripten module)
    if (hasEmbeddedViewer('xlisp')) {
        const xlisp = embeddedViewers.xlisp;
        if (!xlisp.loaded) {
            console.log('[EmbeddedViewer] Loading embedded XLisp core...');
            try {
                // Load the xlisp.js script first (defines createXLisp global)
                const script = document.createElement('script');
                script.src = xlisp.jsUrl;
                await new Promise((resolve, reject) => {
                    script.onload = resolve;
                    script.onerror = reject;
                    document.head.appendChild(script);
                });
                console.log('[EmbeddedViewer] XLisp script loaded, waiting for createXLisp...');

                // Wait for createXLisp to be available
                let attempts = 0;
                while (!window.createXLisp && attempts < 50) {
                    await new Promise(r => setTimeout(r, 100));
                    attempts++;
                }

                if (!window.createXLisp) {
                    throw new Error('createXLisp not found after loading script');
                }

                // Store the blob WASM URL for acadlisp to use
                window._xlispWasmUrl = xlisp.wasmUrl;
                window._xlispJsUrl = xlisp.jsUrl;
                xlisp.loaded = true;
                console.log('[EmbeddedViewer] XLisp core script ready');
            } catch (error) {
                console.error('[EmbeddedViewer] Failed to load XLisp script:', error);
                throw error;
            }
        }
    }

    // Now load acadlisp which wraps xlisp with WasmEngine
    if (hasEmbeddedViewer('acadlisp')) {
        const acadlisp = embeddedViewers.acadlisp;
        if (!acadlisp.loaded) {
            console.log('[EmbeddedViewer] Loading embedded AcadLISP...');
            console.log('[EmbeddedViewer] AcadLISP jsUrl:', acadlisp.jsUrl?.substring(0, 60));
            console.log('[EmbeddedViewer] AcadLISP wasmUrl:', acadlisp.wasmUrl?.substring(0, 60));
            try {
                // Import the acadlisp ES module
                console.log('[EmbeddedViewer] Importing acadlisp module...');
                const module = await import(acadlisp.jsUrl);
                console.log('[EmbeddedViewer] Module imported, keys:', Object.keys(module));

                // Initialize with blob WASM URL
                console.log('[EmbeddedViewer] Calling module.default() with WASM URL...');
                await module.default(acadlisp.wasmUrl);
                console.log('[EmbeddedViewer] WASM initialized');

                // Create engine instance
                if (module.WasmEngine) {
                    window.xlispEngine = new module.WasmEngine();
                    console.log('[EmbeddedViewer] AcadLISP loaded! Engine info:', window.xlispEngine.engine_info());
                } else {
                    console.log('[EmbeddedViewer] No WasmEngine in module, keys:', Object.keys(module));
                }

                acadlisp.loaded = true;
                return true;
            } catch (error) {
                console.error('[EmbeddedViewer] Failed to load AcadLISP:', error);
                console.error('[EmbeddedViewer] Error stack:', error.stack);
                throw error;
            }
        }
    }

    if (window.xlispEngine) {
        console.log('[EmbeddedViewer] AcadLISP/XLisp ready via embedded engine');
        return true;
    }

    if (!hasEmbeddedViewer('acadlisp') && !hasEmbeddedViewer('xlisp')) {
        console.log('[EmbeddedViewer] No embedded AcadLISP/XLisp, using external');
        return window.loadAcadLisp ? window.loadAcadLisp() : Promise.reject('No AcadLISP loader');
    }

    return true;
}

/**
 * Load the embedded Star Sky viewer (if available)
 * This is a lightweight 2D canvas renderer for star visualization
 * @param {string} canvasId - Canvas element ID
 * @param {string} skyDataJson - Star data JSON
 * @param {string|null} highlightStar - Optional star name to highlight
 */
async function loadEmbeddedStarSky(canvasId, skyDataJson, highlightStar = null) {
    if (!hasEmbeddedViewer('starsky')) {
        console.log('[EmbeddedViewer] No embedded Star Sky viewer');
        return null;
    }

    const viewer = embeddedViewers.starsky;
    if (viewer.loaded && viewer.renderer) {
        console.log('[EmbeddedViewer] Star Sky already loaded, updating data');
        viewer.renderer.load_json(skyDataJson);
        viewer.renderer.render();
        if (highlightStar) {
            viewer.renderer.highlight_star(highlightStar);
        }
        return viewer.renderer;
    }

    console.log('[EmbeddedViewer] Loading embedded Star Sky viewer...');

    try {
        // Import the JS module
        const module = await import(viewer.jsUrl);

        // Initialize WASM by passing the blob URL directly
        // This avoids the "new URL(relative, import.meta.url)" issue with blob URLs
        await module.default(viewer.wasmUrl);

        // Create renderer and load data
        const renderer = new module.StarSkyRenderer(canvasId);
        renderer.load_json(skyDataJson);
        renderer.resize();
        renderer.render();

        // Highlight star if specified
        if (highlightStar) {
            console.log(`[EmbeddedViewer] Highlighting star: ${highlightStar}`);
            renderer.highlight_star(highlightStar);
        }

        viewer.loaded = true;
        viewer.renderer = renderer;
        console.log(`[EmbeddedViewer] Star Sky loaded! ${renderer.star_count()} stars`);
        return renderer;
    } catch (error) {
        console.error('[EmbeddedViewer] Failed to load Star Sky:', error);
        throw error;
    }
}

/**
 * Highlight a star in the already-loaded Star Sky viewer
 * @param {string} starName - Name of star to highlight
 */
function highlightStarInViewer(starName) {
    const viewer = embeddedViewers.starsky;
    if (viewer && viewer.loaded && viewer.renderer) {
        viewer.renderer.render(); // Re-render to clear previous highlights
        viewer.renderer.highlight_star(starName);
        return true;
    }
    return false;
}

// Also expose highlightStarInViewer
window.highlightStarInViewer = highlightStarInViewer;

// Expose to window for WASM to call
window.registerEmbeddedViewer = registerEmbeddedViewer;
window.hasEmbeddedViewer = hasEmbeddedViewer;
window.getEmbeddedViewerJsUrl = getEmbeddedViewerJsUrl;
window.getEmbeddedViewerWasmUrl = getEmbeddedViewerWasmUrl;
window.loadEmbeddedStarSky = loadEmbeddedStarSky;
window.loadEmbeddedAcadLisp = loadEmbeddedAcadLisp;
window.loadEmbeddedBevyViewer = loadEmbeddedBevyViewer;

// Override loaders after other scripts have loaded
// This ensures we capture the original loaders from lisp-loader.js, etc.
window.addEventListener('DOMContentLoaded', () => {
    // Override Bevy loader
    const originalLoadBevyViewer = window.loadBevyViewer;
    window.loadBevyViewer = async function() {
        if (hasEmbeddedViewer('bevy')) {
            console.log('[EmbeddedViewer] Using embedded Bevy viewer');
            return loadEmbeddedBevyViewer();
        }
        return originalLoadBevyViewer ? originalLoadBevyViewer() : Promise.reject('No loader');
    };

    // Override AcadLISP loader
    const originalLoadAcadLisp = window.loadAcadLisp;
    window.loadAcadLisp = async function() {
        if (hasEmbeddedViewer('acadlisp') || hasEmbeddedViewer('xlisp')) {
            console.log('[EmbeddedViewer] Using embedded AcadLISP/XLisp');
            return loadEmbeddedAcadLisp();
        }
        console.log('[EmbeddedViewer] No embedded AcadLISP, falling back to external');
        return originalLoadAcadLisp ? originalLoadAcadLisp() : Promise.reject('No loader');
    };

    console.log('[EmbeddedViewer] Loaders overridden - embedded versions will be used when available');
});

console.log('[EmbeddedViewer] Loader ready - Tribute to Astrophysics!');
