/**
 * WASM Plugin Loader for GLDF
 *
 * A plugin architecture where embedded WASM modules self-describe their capabilities.
 * The host discovers and uses whatever the plugin provides - no hardcoded knowledge needed.
 *
 * Plugin Manifest Format (manifest.json):
 * {
 *   "id": "acadlisp",
 *   "name": "AcadLISP Engine",
 *   "version": "0.2.0",
 *   "description": "AutoLISP interpreter with CAD output",
 *   "js": "acadlisp-xxx.js",
 *   "wasm": "acadlisp-xxx_bg.wasm",
 *   "init": "default",  // or constructor name like "WasmEngine"
 *   "capabilities": {
 *     "functions": {
 *       "eval": { "args": ["code"], "returns": "string", "description": "Evaluate LISP code" },
 *       "get_entities_svg": { "args": [], "returns": "string", "description": "Get drawing as SVG" },
 *       "get_kicad_sym": { "args": ["lib", "name"], "returns": "string", "description": "Export KiCad symbol" },
 *       "clear": { "args": [], "returns": "void", "description": "Clear drawing" }
 *     },
 *     "examples": ["hello", "math", "box", "kicad_sym"],
 *     "outputFormats": ["svg", "dxf", "json", "kicad_sym", "kicad_mod"],
 *     "ui": {
 *       "icon": "λ",
 *       "primaryAction": "eval",
 *       "showExamples": true
 *     }
 *   },
 *   "files": ["acadlisp-xxx.js", "acadlisp-xxx_bg.wasm", "xlisp.js", "xlisp.wasm"]
 * }
 */

// Registry of loaded plugins
const wasmPlugins = new Map();

// Loading state
const loadingPlugins = new Set();

/**
 * Plugin instance wrapper - provides unified interface to any WASM module
 */
class WasmPlugin {
    constructor(id, manifest, module, instance) {
        this.id = id;
        this.manifest = manifest;
        this.module = module;
        this.instance = instance;
        this.loaded = true;
    }

    /** Get plugin name */
    get name() { return this.manifest.name || this.id; }

    /** Get plugin version */
    get version() { return this.manifest.version || '0.0.0'; }

    /** Get plugin description */
    get description() { return this.manifest.description || ''; }

    /** Get available functions */
    get functions() {
        return this.manifest.capabilities?.functions || {};
    }

    /** Get available examples */
    get examples() {
        return this.manifest.capabilities?.examples || [];
    }

    /** Get output formats */
    get outputFormats() {
        return this.manifest.capabilities?.outputFormats || [];
    }

    /** Get UI configuration */
    get ui() {
        return this.manifest.capabilities?.ui || {};
    }

    /** Check if plugin has a specific function */
    hasFunction(name) {
        // Check both manifest declaration and actual instance
        return typeof this.instance[name] === 'function';
    }

    /** Call a function on the plugin */
    call(functionName, ...args) {
        if (!this.hasFunction(functionName)) {
            throw new Error(`Plugin ${this.id} does not have function: ${functionName}`);
        }
        return this.instance[functionName](...args);
    }

    /** Get function metadata */
    getFunctionInfo(name) {
        return this.manifest.capabilities?.functions?.[name] || null;
    }

    /** List all available functions (discovered from instance) */
    listFunctions() {
        const functions = [];
        for (const key of Object.keys(this.instance)) {
            if (typeof this.instance[key] === 'function' && !key.startsWith('__')) {
                const info = this.getFunctionInfo(key);
                functions.push({
                    name: key,
                    args: info?.args || [],
                    returns: info?.returns || 'unknown',
                    description: info?.description || ''
                });
            }
        }
        return functions;
    }
}

/**
 * Register and load a WASM plugin from embedded files
 * @param {string} pluginId - Plugin identifier (folder name in other/viewer/)
 * @param {string} manifestJson - manifest.json content
 * @param {Object} files - Map of filename -> Uint8Array/string content
 */
async function registerWasmPlugin(pluginId, manifestJson, files) {
    console.log(`[WasmPlugin] Registering plugin: ${pluginId}`);

    if (wasmPlugins.has(pluginId)) {
        console.log(`[WasmPlugin] Plugin ${pluginId} already registered`);
        return wasmPlugins.get(pluginId);
    }

    if (loadingPlugins.has(pluginId)) {
        console.log(`[WasmPlugin] Plugin ${pluginId} is already loading, waiting...`);
        while (loadingPlugins.has(pluginId)) {
            await new Promise(r => setTimeout(r, 100));
        }
        return wasmPlugins.get(pluginId);
    }

    loadingPlugins.add(pluginId);

    try {
        const manifest = JSON.parse(manifestJson);
        console.log(`[WasmPlugin] Manifest:`, manifest);

        // Get JS and WASM files
        const jsFileName = manifest.js;
        const wasmFileName = manifest.wasm;

        if (!jsFileName || !wasmFileName) {
            throw new Error('Manifest must specify js and wasm files');
        }

        const jsContent = files[jsFileName];
        const wasmBytes = files[wasmFileName];

        if (!jsContent || !wasmBytes) {
            throw new Error(`Missing files: js=${!!jsContent}, wasm=${!!wasmBytes}`);
        }

        // Create blob URLs
        const wasmBlob = new Blob([wasmBytes], { type: 'application/wasm' });
        const wasmUrl = URL.createObjectURL(wasmBlob);

        // Modify JS to use blob URL for WASM
        let modifiedJs = typeof jsContent === 'string' ? jsContent : new TextDecoder().decode(jsContent);

        // Replace various WASM URL patterns
        modifiedJs = modifiedJs
            .replace(/new URL\(['"]([^'"]+\.wasm)['"],\s*import\.meta\.url\)/g, `"${wasmUrl}"`)
            .replace(/fetch\(\s*['"]([^'"]+\.wasm)['"]\s*\)/g, `fetch("${wasmUrl}")`)
            .replace(/['"]([^'"\/]+_bg\.wasm)['"]/g, `"${wasmUrl}"`);

        const jsBlob = new Blob([modifiedJs], { type: 'application/javascript' });
        const jsUrl = URL.createObjectURL(jsBlob);

        // Handle additional dependencies (like xlisp.js for acadlisp)
        const additionalDeps = (manifest.files || []).filter(f =>
            f !== jsFileName && f !== wasmFileName && f.endsWith('.js')
        );

        for (const depFile of additionalDeps) {
            const depContent = files[depFile];
            if (depContent) {
                console.log(`[WasmPlugin] Loading dependency: ${depFile}`);
                const depWasmFile = depFile.replace('.js', '.wasm');
                const depWasmBytes = files[depWasmFile];

                if (depWasmBytes) {
                    const depWasmBlob = new Blob([depWasmBytes], { type: 'application/wasm' });
                    const depWasmUrl = URL.createObjectURL(depWasmBlob);
                    window[`_${depFile.replace('.', '_')}_wasmUrl`] = depWasmUrl;
                }

                // Load dependency script
                const depJs = typeof depContent === 'string' ? depContent : new TextDecoder().decode(depContent);
                const depBlob = new Blob([depJs], { type: 'application/javascript' });
                const depUrl = URL.createObjectURL(depBlob);

                const script = document.createElement('script');
                script.src = depUrl;
                await new Promise((resolve, reject) => {
                    script.onload = resolve;
                    script.onerror = reject;
                    document.head.appendChild(script);
                });
            }
        }

        // Import and initialize the main module
        console.log(`[WasmPlugin] Importing module from blob URL`);
        const module = await import(jsUrl);

        // Initialize based on manifest.init
        let instance;
        const initMethod = manifest.init || 'default';

        if (initMethod === 'default') {
            // Call default export with WASM URL
            await module.default(wasmUrl);
            // Look for a constructor class
            const constructorName = Object.keys(module).find(k =>
                typeof module[k] === 'function' && /^[A-Z]/.test(k)
            );
            if (constructorName) {
                instance = new module[constructorName]();
            } else {
                instance = module;
            }
        } else if (module[initMethod]) {
            instance = new module[initMethod]();
        } else {
            throw new Error(`Init method not found: ${initMethod}`);
        }

        const plugin = new WasmPlugin(pluginId, manifest, module, instance);
        wasmPlugins.set(pluginId, plugin);

        console.log(`[WasmPlugin] Plugin ${pluginId} loaded successfully`);
        console.log(`[WasmPlugin] Available functions:`, plugin.listFunctions().map(f => f.name));

        loadingPlugins.delete(pluginId);
        return plugin;

    } catch (error) {
        console.error(`[WasmPlugin] Failed to register ${pluginId}:`, error);
        loadingPlugins.delete(pluginId);
        throw error;
    }
}

/**
 * Get a loaded plugin by ID
 */
function getPlugin(pluginId) {
    return wasmPlugins.get(pluginId) || null;
}

/**
 * Check if a plugin is loaded
 */
function hasPlugin(pluginId) {
    return wasmPlugins.has(pluginId);
}

/**
 * List all loaded plugins
 */
function listPlugins() {
    return Array.from(wasmPlugins.values()).map(p => ({
        id: p.id,
        name: p.name,
        version: p.version,
        description: p.description,
        functions: p.listFunctions().map(f => f.name)
    }));
}

/**
 * Call a function on a plugin
 */
function callPluginFunction(pluginId, functionName, ...args) {
    const plugin = getPlugin(pluginId);
    if (!plugin) {
        throw new Error(`Plugin not found: ${pluginId}`);
    }
    return plugin.call(functionName, ...args);
}

/**
 * Get plugin capabilities as JSON (for Rust/WASM to query)
 */
function getPluginCapabilities(pluginId) {
    const plugin = getPlugin(pluginId);
    if (!plugin) {
        return null;
    }
    return JSON.stringify({
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        description: plugin.description,
        functions: plugin.listFunctions(),
        examples: plugin.examples,
        outputFormats: plugin.outputFormats,
        ui: plugin.ui
    });
}

/**
 * Call a plugin function with 1 string argument (wrapper for Rust binding)
 */
function callPluginFunction1(pluginId, functionName, arg1) {
    return callPluginFunction(pluginId, functionName, arg1);
}

/**
 * Call a plugin function with 2 string arguments (wrapper for Rust binding)
 */
function callPluginFunction2(pluginId, functionName, arg1, arg2) {
    return callPluginFunction(pluginId, functionName, arg1, arg2);
}

/**
 * Call a plugin function with numeric arguments (for plot_function, benchmark)
 */
function callPluginFunctionNums(pluginId, functionName, arg1, arg2, arg3, arg4, arg5, arg6) {
    return callPluginFunction(pluginId, functionName, arg1, arg2, arg3, arg4, arg5, arg6);
}

/**
 * List all loaded plugins as JSON string (for Rust binding)
 */
function listPluginsJson() {
    return JSON.stringify(listPlugins());
}

// Expose to window - primary API
window.registerWasmPlugin = registerWasmPlugin;
window.getPlugin = getPlugin;
window.hasPlugin = hasPlugin;
window.listPlugins = listPluginsJson;  // Return JSON for Rust
window.callPluginFunction = callPluginFunction;
window.callPluginFunction1 = callPluginFunction1;
window.callPluginFunction2 = callPluginFunction2;
window.callPluginFunctionNums = callPluginFunctionNums;
window.getPluginCapabilities = getPluginCapabilities;

// Also expose the WasmPlugin class for advanced usage
window.WasmPlugin = WasmPlugin;
window.wasmPlugins = wasmPlugins;  // For debugging

console.log('[WasmPlugin] Plugin loader ready');
