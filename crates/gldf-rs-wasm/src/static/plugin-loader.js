// GLDF Plugin Loader
// Dynamically loads and initializes WASM plugins embedded in GLDF files

/**
 * Plugin registry - tracks loaded plugins
 */
const loadedPlugins = new Map();

/**
 * Discover plugins in a GLDF file's embedded files
 * @param {Map<string, Uint8Array>} files - Map of file paths to contents from GLDF
 * @returns {Array} Array of plugin manifests with their file data
 */
function discoverPlugins(files) {
    const plugins = [];

    for (const [path, content] of files) {
        if (path.match(/other\/viewer\/[^/]+\/manifest\.json$/)) {
            try {
                const manifestJson = new TextDecoder().decode(content);
                const manifest = JSON.parse(manifestJson);
                const pluginDir = path.replace('/manifest.json', '');

                // Get the JS and WASM files for this plugin
                const jsPath = `${pluginDir}/${manifest.js}`;
                const wasmPath = `${pluginDir}/${manifest.wasm}`;

                const jsContent = files.get(jsPath);
                const wasmContent = files.get(wasmPath);

                if (jsContent && wasmContent) {
                    plugins.push({
                        manifest,
                        pluginDir,
                        jsContent,
                        wasmContent
                    });
                    console.log(`[Plugin] Discovered: ${manifest.name} (${manifest.type})`);
                } else {
                    console.warn(`[Plugin] Missing files for ${manifest.name}`);
                }
            } catch (e) {
                console.error(`[Plugin] Failed to parse manifest at ${path}:`, e);
            }
        }
    }

    return plugins;
}

/**
 * Load a plugin from its embedded files
 * @param {Object} plugin - Plugin object from discoverPlugins
 * @param {Map<string, Uint8Array>} allFiles - All files from GLDF for data access
 * @returns {Promise<Object>} Loaded plugin module
 */
async function loadPlugin(plugin, allFiles) {
    const { manifest, jsContent, wasmContent } = plugin;

    if (loadedPlugins.has(manifest.type)) {
        console.log(`[Plugin] ${manifest.type} already loaded`);
        return loadedPlugins.get(manifest.type);
    }

    console.log(`[Plugin] Loading ${manifest.name}...`);

    try {
        // IMPORTANT: Store data in localStorage BEFORE loading WASM
        // Bevy plugins read from localStorage during initialization

        // Store data if plugin specifies a data file (single key format)
        if (manifest.data_file && manifest.storage_key) {
            const dataContent = allFiles.get(manifest.data_file);
            if (dataContent) {
                const dataJson = new TextDecoder().decode(dataContent);
                localStorage.setItem(manifest.storage_key, dataJson);
                console.log(`[Plugin] Pre-stored ${manifest.data_file} as ${manifest.storage_key}`);
            }
        }

        // Handle plugins with multiple storage keys (e.g., stadium plugin)
        if (manifest.storage_keys) {
            // Store LDT file if specified
            if (manifest.storage_keys.ldt && manifest.ldt_file) {
                const ldtContent = allFiles.get(manifest.ldt_file);
                if (ldtContent) {
                    const ldtText = new TextDecoder().decode(ldtContent);
                    localStorage.setItem(manifest.storage_keys.ldt, ldtText);
                    console.log(`[Plugin] Pre-stored LDT (${ldtText.length} bytes) as ${manifest.storage_keys.ldt}`);
                }
            }
            // Store config file if specified
            if (manifest.storage_keys.config && manifest.data_file) {
                const configContent = allFiles.get(manifest.data_file);
                if (configContent) {
                    const configJson = new TextDecoder().decode(configContent);
                    localStorage.setItem(manifest.storage_keys.config, configJson);
                    console.log(`[Plugin] Pre-stored config as ${manifest.storage_keys.config}`);
                }
            }
        }

        // Create blob URLs for the JS and WASM files
        const wasmBlob = new Blob([wasmContent], { type: 'application/wasm' });
        const wasmUrl = URL.createObjectURL(wasmBlob);

        // Modify JS to use our blob URL for the WASM
        let jsText = new TextDecoder().decode(jsContent);

        // Replace the WASM import path with our blob URL
        // The wasm-bindgen generated JS typically has: new URL('xxx_bg.wasm', import.meta.url)
        // We need to replace this with our blob URL
        jsText = jsText.replace(
            /new URL\(['"]([^'"]+_bg\.wasm)['"]\s*,\s*import\.meta\.url\)/g,
            `"${wasmUrl}"`
        );

        // Also handle direct fetch patterns
        jsText = jsText.replace(
            /fetch\(\s*new URL\(['"]([^'"]+_bg\.wasm)['"]/g,
            `fetch("${wasmUrl}"`
        );

        // Create blob URL for modified JS
        const jsBlob = new Blob([jsText], { type: 'application/javascript' });
        const jsUrl = URL.createObjectURL(jsBlob);

        // Dynamically import the module
        const module = await import(jsUrl);

        // Initialize WASM (call default export)
        if (typeof module.default === 'function') {
            await module.default();
        }

        // Cache the loaded module
        const loadedPlugin = {
            manifest,
            module,
            wasmUrl,
            jsUrl
        };
        loadedPlugins.set(manifest.type, loadedPlugin);

        console.log(`[Plugin] ${manifest.name} loaded successfully`);
        return loadedPlugin;

    } catch (e) {
        const errorStr = e.toString();
        // Bevy uses exceptions for control flow - ignore these
        if (errorStr.includes("Using exceptions for control flow") ||
            errorStr.includes("don't mind me")) {
            console.log(`[Plugin] Ignoring control flow exception (Bevy is running)`);
            // Still cache the module
            const loadedPlugin = {
                manifest,
                module: null, // Module is running
                wasmUrl: null,
                jsUrl: null
            };
            loadedPlugins.set(manifest.type, loadedPlugin);
            return loadedPlugin;
        }
        console.error(`[Plugin] Failed to load ${manifest.name}:`, e);
        throw e;
    }
}

/**
 * Render a plugin into a container
 * @param {string} pluginType - Plugin type (e.g., 'starsky')
 * @param {HTMLElement} container - Container element for the plugin
 * @param {Object} options - Additional options
 */
async function renderPlugin(pluginType, container, options = {}) {
    const plugin = loadedPlugins.get(pluginType);
    if (!plugin) {
        console.error(`[Plugin] ${pluginType} not loaded`);
        return null;
    }

    const { manifest, module } = plugin;
    const canvasId = manifest.canvas_id || `${pluginType}-canvas`;

    // Create canvas if needed
    let canvas = container.querySelector('canvas');
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.id = canvasId;
        canvas.style.width = '100%';
        canvas.style.height = '100%';
        // Set actual pixel dimensions for WebGL
        canvas.width = container.clientWidth || 800;
        canvas.height = container.clientHeight || 600;
        container.appendChild(canvas);
    }

    // For Bevy/WebGL plugins, use CSS selector format
    const canvasSelector = `#${canvasId}`;

    // Call the entry point
    const entryPoint = manifest.entry_point || 'render_from_storage';
    const storageKey = manifest.storage_key || `gldf_${pluginType}_json`;

    if (typeof module[entryPoint] === 'function') {
        try {
            // Bevy plugins expect a CSS selector like "#canvas-id"
            // Canvas 2D plugins expect just the ID
            const canvasArg = (pluginType === 'stadium' || manifest.type === 'bevy')
                ? canvasSelector
                : canvasId;
            const renderer = module[entryPoint](canvasArg, storageKey);
            console.log(`[Plugin] ${manifest.name} rendered`);
            return renderer;
        } catch (e) {
            console.error(`[Plugin] ${manifest.name} render failed:`, e);
            throw e;
        }
    } else if (typeof module.StarSkyRenderer !== 'undefined') {
        // Fallback for StarSky plugin
        try {
            const renderer = new module.StarSkyRenderer(canvasId);
            renderer.load_from_storage(storageKey);
            renderer.render();
            console.log(`[Plugin] ${manifest.name} rendered via StarSkyRenderer`);
            return renderer;
        } catch (e) {
            console.error(`[Plugin] ${manifest.name} render failed:`, e);
        }
    }

    return null;
}

/**
 * Get list of available plugin types
 * @returns {Array<string>} Array of plugin type names
 */
function getAvailablePlugins() {
    return Array.from(loadedPlugins.keys());
}

/**
 * Get plugin info by type
 * @param {string} pluginType - Plugin type
 * @returns {Object|null} Plugin manifest or null
 */
function getPluginInfo(pluginType) {
    const plugin = loadedPlugins.get(pluginType);
    return plugin ? plugin.manifest : null;
}

/**
 * Cleanup a plugin (revoke blob URLs)
 * @param {string} pluginType - Plugin type to cleanup
 */
function unloadPlugin(pluginType) {
    const plugin = loadedPlugins.get(pluginType);
    if (plugin) {
        if (plugin.wasmUrl) URL.revokeObjectURL(plugin.wasmUrl);
        if (plugin.jsUrl) URL.revokeObjectURL(plugin.jsUrl);
        loadedPlugins.delete(pluginType);
        console.log(`[Plugin] ${pluginType} unloaded`);
    }
}

// Export to window for use from WASM
window.GldfPlugins = {
    discoverPlugins,
    loadPlugin,
    renderPlugin,
    getAvailablePlugins,
    getPluginInfo,
    unloadPlugin
};

console.log('[Plugin] GLDF Plugin Loader ready');
