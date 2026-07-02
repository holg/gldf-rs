"""
Rename LEDVANCE downloaded assets based on file content.

Parses LDT (EULUMDAT) and IES headers to extract the real product name,
then renames files accordingly. Also renames STEP/ROLF/ULD by size fingerprint.

Usage:
    python3 ledvance_rename.py [path_to_ledvance_assets]

Default path: ./ledvance_assets
"""

import os
import re
import sys
from pathlib import Path


def parse_ldt_name(filepath: Path) -> str | None:
    """Extract luminaire name from EULUMDAT (.ldt) file.
    
    EULUMDAT format (line numbers 1-based):
      Line 1: Company identification/database/version
      Line 2: Type indicator (1=point source with symm, etc.)
      Line 3: Symmetry indicator
      Line 4: Number of C-planes (Mc)
      Line 5: Distance between C-planes (Dc)
      Line 6: Number of intensities per half plane (Ng)
      ...
      Line 10: Luminaire name
      Line 11: Luminaire number (catalog number)
    
    But many files use a simplified format where:
      Line 1: Company
      Line 2: Ityp
      Line 3: Isym
      ...
    We need line 10 (0-indexed: line 9) for the luminaire name.
    """
    try:
        with open(filepath, "r", encoding="latin-1") as f:
            lines = [l.strip() for l in f.readlines()]
        
        if len(lines) < 11:
            return None
        
        # Line 10 (index 9) = Luminaire name
        # Line 11 (index 10) = Luminaire number  
        luminaire_name = lines[9]
        luminaire_number = lines[10] if len(lines) > 10 else ""
        
        # Sometimes the name is more descriptive, sometimes the number is
        # Pick whichever has more info, prefer the one with wattage info
        candidates = [luminaire_name, luminaire_number]
        best = luminaire_name
        for c in candidates:
            if re.search(r'\d+W', c, re.IGNORECASE):
                best = c
                break
        
        if not best or best.isspace():
            # Fallback: scan first 20 lines for anything with W (wattage)
            for line in lines[:20]:
                if re.search(r'FL\s*MAX.*\d+W', line, re.IGNORECASE):
                    return line.strip()
            return None
        
        return best.strip()
    except Exception as e:
        print(f"  ⚠ Error parsing {filepath.name}: {e}")
        return None


def parse_ies_name(filepath: Path) -> str | None:
    """Extract luminaire name from IES (.ies) file.
    
    IESNA LM-63 format:
      [IESNA:LM-63-2002] or [IESNA91] header
      Keyword lines like:
        [TEST] ...
        [MANUFAC] ...
        [LUMCAT] catalog number  <-- this is what we want
        [LUMINAIRE] description
      Then TILT= line starts the photometric data.
    """
    try:
        with open(filepath, "r", encoding="latin-1") as f:
            content = f.read(4096)  # header is in first few KB
        
        # Look for LUMCAT (luminaire catalog number)
        m = re.search(r'\[LUMCAT\]\s*(.+)', content)
        if m:
            return m.group(1).strip()
        
        # Fallback: LUMINAIRE description
        m = re.search(r'\[LUMINAIRE\]\s*(.+)', content)
        if m:
            return m.group(1).strip()
        
        # Fallback: look for any line with FL MAX pattern
        for line in content.splitlines():
            if re.search(r'FL\s*MAX.*\d+W', line, re.IGNORECASE):
                # Clean up bracket prefixes
                cleaned = re.sub(r'^\[.*?\]\s*', '', line).strip()
                return cleaned if cleaned else line.strip()
        
        return None
    except Exception as e:
        print(f"  ⚠ Error parsing {filepath.name}: {e}")
        return None


def sanitize_filename(name: str) -> str:
    """Convert a product name to a safe, clean filename."""
    # Normalize whitespace
    name = re.sub(r'\s+', ' ', name).strip()
    # Replace problematic chars
    name = re.sub(r'[<>:"/\\|?*°×]', '', name)
    # Replace spaces and special chars with underscores
    name = re.sub(r'[\s,;]+', '_', name)
    # Collapse multiple underscores
    name = re.sub(r'_+', '_', name)
    # Remove trailing/leading underscores
    name = name.strip('_')
    return name


# Size-based fingerprints for STEP files (from LEDVANCE product pages)
STEP_SIZE_MAP = {
    # (min_bytes, max_bytes): name
    (4_900_000, 5_100_000): "FL_MAX_1200W",
    (4_350_000, 4_550_000): "FL_MAX_900W",
    (3_950_000, 4_150_000): "FL_MAX_600W",
    (9_000_000, 9_500_000): "FL_PFM_150W",
}

# ULD sizes from product pages
ULD_SIZE_MAP = {
    (310_000, 320_000): "FL_MAX_1200W_757_SYM_10",
    (304_000, 307_000): "FL_MAX_1200W_757_SYM_60_or_SYM_30",
    (301_000, 303_000): "FL_MAX_1200W_757_ASYM_50x110",
    (306_000, 310_000): "FL_MAX_900W_757_SYM_10",
    (296_000, 304_000): "FL_MAX_900W_or_600W",
    (120_000, 125_000): "FL_PFM_150W_SYM_60",
}


def identify_by_size(filepath: Path, size_map: dict) -> str | None:
    """Try to identify a file by its size using known fingerprints."""
    size = filepath.stat().st_size
    for (lo, hi), name in size_map.items():
        if lo <= size <= hi:
            return name
    return None


def rename_file(filepath: Path, new_name: str, ext: str) -> Path | None:
    """Rename a file, handling conflicts."""
    safe_name = sanitize_filename(new_name)
    # Avoid double extensions like .ldt.ldt
    if safe_name.lower().endswith(ext):
        safe_name = safe_name[:-len(ext)]
    safe_name += ext
    new_path = filepath.parent / safe_name
    
    if new_path == filepath:
        return filepath
    
    # Handle duplicates
    if new_path.exists():
        for i in range(2, 100):
            alt = filepath.parent / f"{sanitize_filename(new_name)}_{i}{ext}"
            if not alt.exists():
                new_path = alt
                break
        else:
            return None
    
    filepath.rename(new_path)
    return new_path


def process_directory(base_dir: Path):
    """Walk the asset directory and rename files based on content."""
    
    stats = {"renamed": 0, "skipped": 0, "failed": 0}
    
    print("=" * 60)
    print("LEDVANCE Asset Renamer")
    print(f"Directory: {base_dir}")
    print("=" * 60)
    
    # ── LDT files ────────────────────────────────────────────
    ldt_files = sorted(base_dir.rglob("*.ldt"))
    if ldt_files:
        print(f"\n📄 LDT files ({len(ldt_files)}):")
        for f in ldt_files:
            name = parse_ldt_name(f)
            if name:
                new_path = rename_file(f, name, ".ldt")
                if new_path:
                    print(f"  ✅ {f.name}  →  {new_path.name}")
                    stats["renamed"] += 1
                else:
                    print(f"  ⚠ {f.name}  →  conflict for '{name}'")
                    stats["failed"] += 1
            else:
                print(f"  ⏭  {f.name}  (could not parse name)")
                stats["skipped"] += 1
    
    # ── IES files ────────────────────────────────────────────
    ies_files = sorted(base_dir.rglob("*.ies"))
    if ies_files:
        print(f"\n📄 IES files ({len(ies_files)}):")
        for f in ies_files:
            name = parse_ies_name(f)
            if name:
                new_path = rename_file(f, name, ".ies")
                if new_path:
                    print(f"  ✅ {f.name}  →  {new_path.name}")
                    stats["renamed"] += 1
                else:
                    print(f"  ⚠ {f.name}  →  conflict for '{name}'")
                    stats["failed"] += 1
            else:
                print(f"  ⏭  {f.name}  (could not parse name)")
                stats["skipped"] += 1
    
    # ── STEP files ───────────────────────────────────────────
    step_files = sorted(base_dir.rglob("*.step")) + sorted(base_dir.rglob("*.stp"))
    if step_files:
        print(f"\n📐 STEP files ({len(step_files)}):")
        for f in step_files:
            name = identify_by_size(f, STEP_SIZE_MAP)
            if name:
                orig_name = f.name
                size = f.stat().st_size
                new_path = rename_file(f, name, ".step")
                if new_path:
                    print(f"  ✅ {orig_name} ({size:,} B)  →  {new_path.name}")
                    stats["renamed"] += 1
                else:
                    stats["failed"] += 1
            else:
                print(f"  ⏭  {f.name} ({f.stat().st_size:,} B)  (unknown size)")
                stats["skipped"] += 1
    
    # ── ROLF files ───────────────────────────────────────────
    # ROLF is binary (Relux format) — try to find ASCII product name in header
    rolf_files = sorted(base_dir.rglob("*.rolf"))
    if rolf_files:
        print(f"\n📦 ROLF files ({len(rolf_files)}):")
        for f in rolf_files:
            name = None
            try:
                with open(f, "rb") as fh:
                    header = fh.read(4096)
                # Look for FL MAX pattern in binary
                text = header.decode("latin-1", errors="replace")
                m = re.search(r'(FL\s*MAX[^\x00\n]{5,60})', text)
                if m:
                    name = m.group(1).strip()
                else:
                    # Try broader pattern
                    m = re.search(r'(FLOODLIGHT[^\x00\n]{5,60})', text, re.IGNORECASE)
                    if m:
                        name = m.group(1).strip()
            except Exception:
                pass
            
            if name:
                new_path = rename_file(f, name, ".rolf")
                if new_path:
                    print(f"  ✅ {f.name}  →  {new_path.name}")
                    stats["renamed"] += 1
                else:
                    stats["failed"] += 1
            else:
                print(f"  ⏭  {f.name} ({f.stat().st_size:,} B)  (no name found in header)")
                stats["skipped"] += 1
    
    # ── ULD files ────────────────────────────────────────────
    uld_files = sorted(base_dir.rglob("*.uld"))
    if uld_files:
        print(f"\n📦 ULD files ({len(uld_files)}):")
        for f in uld_files:
            name = None
            try:
                # ULD is a ZIP-based format (DIALux) — try reading as text first
                with open(f, "rb") as fh:
                    header = fh.read(8192)
                text = header.decode("latin-1", errors="replace")
                m = re.search(r'(FL\s*MAX[^\x00\n]{5,60})', text)
                if not m:
                    m = re.search(r'(FLOODLIGHT[^\x00\n]{5,60})', text, re.IGNORECASE)
                if m:
                    name = m.group(1).strip()
            except Exception:
                pass
            
            if name:
                new_path = rename_file(f, name, ".uld")
                if new_path:
                    print(f"  ✅ {f.name}  →  {new_path.name}")
                    stats["renamed"] += 1
                else:
                    stats["failed"] += 1
            else:
                # Fallback: size-based
                size_name = identify_by_size(f, ULD_SIZE_MAP)
                if size_name:
                    orig_name = f.name
                    size = f.stat().st_size
                    new_path = rename_file(f, size_name, ".uld")
                    if new_path:
                        print(f"  ✅ {orig_name} ({size:,} B)  →  {new_path.name}")
                        stats["renamed"] += 1
                    else:
                        stats["failed"] += 1
                else:
                    print(f"  ⏭  {f.name} ({f.stat().st_size:,} B)")
                    stats["skipped"] += 1
    
    # ── Summary ──────────────────────────────────────────────
    print(f"\n{'=' * 60}")
    print(f"Renamed: {stats['renamed']}  |  Skipped: {stats['skipped']}  |  Failed: {stats['failed']}")
    print(f"{'=' * 60}")


def main():
    if len(sys.argv) > 1:
        base_dir = Path(sys.argv[1])
    else:
        base_dir = Path("ledvance_assets")
    
    if not base_dir.exists():
        print(f"❌ Directory not found: {base_dir}")
        print(f"Usage: python3 {sys.argv[0]} [path_to_ledvance_assets]")
        sys.exit(1)
    
    process_directory(base_dir)


if __name__ == "__main__":
    main()