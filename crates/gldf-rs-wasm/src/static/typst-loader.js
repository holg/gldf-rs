// Typst WASM loader for PDF compilation
// Lazy-loads the 38MB typst WASM module on demand

let typstModule = null;
let typstInitPromise = null;

// Initialize typst WASM module
async function initTypst() {
    if (typstModule) return typstModule;
    if (typstInitPromise) return typstInitPromise;

    typstInitPromise = (async () => {
        try {
            console.log('Loading Typst WASM module...');
            const module = await import('./typst/typst_wasm.js');
            await module.default();
            typstModule = module;
            console.log('Typst WASM module loaded successfully');
            return module;
        } catch (e) {
            console.error('Failed to load Typst WASM:', e);
            typstInitPromise = null;
            throw e;
        }
    })();

    return typstInitPromise;
}

// Compile Typst source to PDF
// Returns Uint8Array of PDF bytes
window.compileTypstToPdf = async function(typstSource) {
    const module = await initTypst();
    try {
        const pdfBytes = module.compile_to_pdf(typstSource);
        return pdfBytes;
    } catch (e) {
        console.error('Typst compilation error:', e);
        throw new Error('Typst compilation failed: ' + e);
    }
};

// Check if Typst is loaded
window.isTypstLoaded = function() {
    return typstModule !== null;
};

// Preload Typst (optional, for faster first compile)
window.preloadTypst = async function() {
    await initTypst();
};
