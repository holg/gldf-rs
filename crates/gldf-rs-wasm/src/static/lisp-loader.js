/**
 * LISP Loader for gldf-rs
 *
 * Loads the acadlisp WASM module from acadlisp.de
 * Provides AutoLISP interpretation with SVG/DXF output
 */

// Remote URL for acadlisp WASM
const ACADLISP_BASE_URL = 'https://acadlisp.de';

let xlispModule = null;
let xlispEngine = null;
let isLoading = false;

/**
 * Load the acadlisp WASM module
 * @returns {Promise<boolean>} - true if loaded successfully
 */
async function loadAcadLisp() {
    if (xlispEngine) {
        return true; // Already loaded
    }

    if (isLoading) {
        // Wait for existing load to complete
        while (isLoading) {
            await new Promise(r => setTimeout(r, 100));
        }
        return !!xlispEngine;
    }

    // Skip loading on localhost due to CORS restrictions
    const isLocalhost = window.location.hostname === 'localhost' ||
                        window.location.hostname === '127.0.0.1';
    if (isLocalhost) {
        console.log('[LISP] Skipping acadlisp load on localhost (CORS restriction)');
        console.log('[LISP] AutoLISP features require deployment to gldf.icu');
        return false;
    }

    isLoading = true;
    console.log('[LISP] Loading acadlisp from', ACADLISP_BASE_URL);

    try {
        // Dynamically load the xlisp.js script
        const script = document.createElement('script');
        script.src = `${ACADLISP_BASE_URL}/xlisp.js`;
        script.async = true;

        await new Promise((resolve, reject) => {
            script.onload = resolve;
            script.onerror = () => reject(new Error('Failed to load xlisp.js'));
            document.head.appendChild(script);
        });

        // Wait for createXLisp to be available
        let attempts = 0;
        while (!window.createXLisp && attempts < 50) {
            await new Promise(r => setTimeout(r, 100));
            attempts++;
        }

        if (!window.createXLisp) {
            throw new Error('createXLisp not found after loading script');
        }

        // Initialize the module
        xlispModule = await window.createXLisp({
            locateFile: (path) => {
                if (path.endsWith('.wasm')) {
                    return `${ACADLISP_BASE_URL}/xlisp.wasm`;
                }
                return path;
            }
        });

        // Create engine instance
        xlispEngine = new xlispModule.WasmEngine();

        console.log('[LISP] acadlisp loaded successfully');
        console.log('[LISP] Engine info:', xlispEngine.engine_info());

        isLoading = false;
        return true;

    } catch (error) {
        console.error('[LISP] Failed to load acadlisp:', error);
        isLoading = false;
        return false;
    }
}

/**
 * Check if acadlisp is loaded (either local or from embedded viewer)
 */
function isAcadLispLoaded() {
    return !!xlispEngine || !!window.xlispEngine;
}

/**
 * Get the active engine (local or embedded)
 */
function getEngine() {
    return xlispEngine || window.xlispEngine;
}

/**
 * Evaluate LISP code and return result
 * @param {string} code - LISP code to evaluate
 * @returns {string} - Result as JSON string
 */
function evalLisp(code) {
    const engine = getEngine();
    if (!engine) {
        return JSON.stringify({ error: 'LISP engine not loaded' });
    }

    try {
        const result = engine.eval(code);
        return result;
    } catch (error) {
        return JSON.stringify({ error: error.message });
    }
}

/**
 * Get drawing entities as SVG
 * @returns {string} - SVG string
 */
function getLispSvg() {
    const engine = getEngine();
    if (!engine) {
        return '<svg><text x="10" y="20">LISP engine not loaded</text></svg>';
    }
    return engine.get_entities_svg();
}

/**
 * Get drawing entities as DXF
 * @returns {string} - DXF string
 */
function getLispDxf() {
    const engine = getEngine();
    if (!engine) {
        return '; LISP engine not loaded';
    }
    return engine.get_entities_dxf();
}

/**
 * Get drawing entities as JSON
 * @returns {string} - JSON string
 */
function getLispEntitiesJson() {
    const engine = getEngine();
    if (!engine) {
        return '[]';
    }
    return engine.get_entities_json();
}

/**
 * Get LISP output buffer (PRINC/PRINT output)
 * @returns {string} - Output text
 */
function getLispOutput() {
    const engine = getEngine();
    if (!engine) {
        return '';
    }
    return engine.get_output();
}

/**
 * Get entity count
 * @returns {number} - Number of drawing entities
 */
function getLispEntityCount() {
    const engine = getEngine();
    if (!engine) {
        return 0;
    }
    return engine.entity_count();
}

/**
 * Clear the drawing
 */
function clearLispDrawing() {
    const engine = getEngine();
    if (engine) {
        engine.clear();
    }
}

/**
 * Get example LISP code by name
 * @param {string} name - Example name
 * @returns {string} - LISP code
 */
function getLispExample(name) {
    const engine = getEngine();
    if (!engine) {
        return '; LISP engine not loaded';
    }
    return engine.get_example(name);
}

/**
 * Get list of available examples
 * @returns {string} - JSON array of example names
 */
function getLispExampleNames() {
    const engine = getEngine();
    if (!engine) {
        return '[]';
    }
    return engine.get_example_names();
}

// Expose to window for WASM to call
window.loadAcadLisp = loadAcadLisp;
window.isAcadLispLoaded = isAcadLispLoaded;
window.evalLisp = evalLisp;
window.getLispSvg = getLispSvg;
window.getLispDxf = getLispDxf;
window.getLispEntitiesJson = getLispEntitiesJson;
window.getLispOutput = getLispOutput;
window.getLispEntityCount = getLispEntityCount;
window.clearLispDrawing = clearLispDrawing;
window.getLispExample = getLispExample;
window.getLispExampleNames = getLispExampleNames;

console.log('[LISP] Loader ready');
