# TODOLIST: Star Catalogue Integration for Bevy 3D Sky (gldf-rs)

## Prerequisites - Language Server Setup

> **Important**: rust-analyzer is needed for code intelligence, NOT for building WASM.
> It provides autocomplete, go-to-definition, type hints, and helps Claude Code understand the codebase.

### rust-analyzer Setup
- [ ] Ensure `rust-analyzer` is installed: `rustup component add rust-analyzer`
- [ ] Alternative: `brew install rust-analyzer` (macOS) or download from releases
- [ ] Verify LSP works: `rust-analyzer --version`
- [ ] Editor integration configured (VS Code / Zed / Helix / etc.)

### Toolchain (separate from LSP)
- [ ] `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [ ] `wasm-bindgen-cli`: `cargo install wasm-bindgen-cli`
- [ ] `wasm-opt` (optional, for size): `brew install binaryen` or from releases

---

## Phase 1: HYG Star Catalogue Parser ✅ DONE

### 1.1 Create `hyg-rs` crate (or module in gldf-rs workspace)
- [x] Define `Star` struct with essential fields:
  ```rust
  pub struct Star {
      pub id: u32,
      pub hip: Option<u32>,      // Hipparcos ID
      pub proper: Option<String>, // Proper name (Sirius, Vega, etc.)
      pub ra: f64,               // Right Ascension (degrees)
      pub dec: f64,              // Declination (degrees)
      pub ra_rad: f64,           // RA in radians
      pub dec_rad: f64,          // Dec in radians
      pub mag: f32,              // Apparent magnitude
      pub absmag: f32,           // Absolute magnitude
      pub ci: Option<f32>,       // Color index (B-V)
      pub spect: Option<String>, // Spectral type
      pub lum: Option<f32>,      // Luminosity (solar units)
      pub x: f64, pub y: f64, pub z: f64, // Cartesian coords
  }
  ```
- [x] CSV parser using `csv` + `serde` crates
- [x] Handle missing/malformed data gracefully (distance=100000 = bad parallax)
- [x] Filter by magnitude threshold (e.g., mag < 6.5 for naked-eye stars)
- [x] Unit tests with sample HYG data

### 1.2 Download & Embed Data
- [x] Download HYG v4.2: https://www.astronexus.com/downloads/catalogs/hygdata_v42.csv.gz
- [x] Decision: runtime load (data/hygdata_v42.csv.gz - 13MB gzipped, 34MB uncompressed)
- [ ] For WASM: compressed binary format preferred (smaller download)
- [ ] Create build script to convert CSV → binary format

---

## Phase 2: Photometric Conversions ✅ DONE

### 2.1 Magnitude to Intensity
- [x] Implement `mag_to_relative_brightness(mag: f32) -> f32`
  - Formula: `10.0_f32.powf(-0.4 * (mag - reference_mag))`
- [x] Normalize to useful range for rendering (e.g., Vega = 0.0 mag as reference)

### 2.2 Color Index to RGB
- [x] Implement `bv_to_temperature(bv: f32) -> f32` (Kelvin)
  - Ballesteros formula: `T = 4600 * (1/(0.92*BV + 1.7) + 1/(0.92*BV + 0.62))`
- [x] Implement `temperature_to_rgb(kelvin: f32) -> [f32; 3]`
  - Tanner Helland algorithm for accurate color rendering
- [x] Spectral type fallback when B-V missing

### 2.3 Optional: Interface with eulumdat-rs types
- [ ] Consider trait abstraction for "photometric source"
- [ ] Could allow stars and luminaires to share rendering code

---

## Phase 3: Bevy Integration

### 3.1 Star Rendering Component
- [ ] `StarCatalogue` resource holding parsed star data
- [ ] `StarRenderer` component/system
- [ ] Point sprite or instanced mesh rendering
- [ ] LOD: bright stars as sprites, dim stars in skybox texture

### 3.2 Coordinate Systems
- [ ] RA/Dec → Bevy Vec3 conversion (handle coordinate system differences)
  ```rust
  fn radec_to_bevy_vec3(ra_rad: f64, dec_rad: f64, radius: f32) -> Vec3 {
      Vec3::new(
          (dec_rad.cos() * ra_rad.cos()) as f32 * radius,
          dec_rad.sin() as f32 * radius,
          -(dec_rad.cos() * ra_rad.sin()) as f32 * radius,
      )
  }
  ```
- [ ] Time-of-day rotation (sidereal time)
- [ ] Observer latitude/longitude support

### 3.3 Visual Quality
- [ ] Star twinkling shader (atmospheric scintillation)
- [ ] Bloom/glow for bright stars
- [ ] Magnitude-based size scaling
- [ ] Color temperature tinting

### 3.4 Performance (WASM considerations)
- [ ] Spatial culling (only render visible hemisphere)
- [ ] GPU instancing for thousands of stars
- [ ] Benchmark: target 10k+ stars at 60fps in WASM

---

## Phase 4: Integration with gldf.icu

### 4.1 Scene Integration
- [ ] Add star sky as optional background in 3D viewer
- [ ] Toggle between procedural atmosphere and star field
- [ ] Time-of-day slider affecting star visibility

### 4.2 Demo Scene
- [ ] Night scene with luminaire + realistic star background
- [ ] Show interplay between artificial lighting and natural sky

---

## Data Sources Reference

| Catalogue | Stars | Magnitude | Format | License |
|-----------|-------|-----------|--------|---------|
| HYG v4.2 | ~120k | all | CSV | CC BY-SA 4.0 |
| Yale BSC5 | 9,110 | < 6.5 | binary | public domain |
| Tycho-2 | 2.5M | < 12 | binary | ESA |
| Gaia DR3 | 1.8B | < 21 | various | ESA |

**Recommendation**: Start with HYG v4.2 (good balance of size/completeness)

---

## Notes

- LDT/EULUMDAT files describe angular intensity distributions for luminaires
- Stars are point sources at infinity → no angular distribution needed
- But: color temperature and intensity concepts transfer well
- gldf-rs L3D geometry could render "star marker" objects if needed

---

## Commands Cheatsheet

```bash
# Verify rust-analyzer
rust-analyzer --version

# Build for WASM
cargo build --target wasm32-unknown-unknown --release

# Run wasm-bindgen
wasm-bindgen target/wasm32-unknown-unknown/release/app.wasm --out-dir pkg

# Optimize WASM size
wasm-opt -Oz pkg/app_bg.wasm -o pkg/app_bg.wasm

# Check binary size
ls -lh pkg/*.wasm
```

---

*Last updated: 2025-12-29*
