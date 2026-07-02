"""
Rename Deko-Light ULD files to: family_article.uld

Extracts family name and article number from ULD metadata.
Handles two metadata layouts found in Deko-Light files.

Usage:
    python3 scripts/dekolight_dedup.py
"""

import hashlib
import re
import struct
from pathlib import Path
from collections import defaultdict

ULD_DIR = Path("scripts/dekolight_uld")


def extract_strings(data: bytes, max_strings: int = 10) -> list:
    """Extract length-prefixed UTF-16LE strings from binary data."""
    strings = []
    off = 16  # skip CLSID
    while off < len(data) - 4 and len(strings) < max_strings:
        slen = int.from_bytes(data[off:off + 4], "little")
        if 2 <= slen <= 100 and off + 4 + slen * 2 <= len(data):
            try:
                s = data[off + 4:off + 4 + slen * 2].decode("utf-16-le")
                if s.isprintable() and len(s) >= 2:
                    strings.append(s.strip())
                    off += 4 + slen * 2
                    continue
            except (UnicodeDecodeError, ValueError):
                pass
        off += 1
    return strings


def is_garbled(s: str) -> bool:
    """Check if a string is garbled (non-Latin characters that aren't meaningful)."""
    non_ascii = sum(1 for c in s if ord(c) > 127)
    return non_ascii > len(s) * 0.3


def extract_cct(filepath: Path) -> str:
    """Extract CCT from LampData0."""
    try:
        import olefile
        ole = olefile.OleFileIO(str(filepath))
        entries = ["/".join(e) for e in ole.listdir()]
        lamp_path = "DataItem0/LightEmittingObj0/LampData0/CONTENTS"
        if lamp_path in entries:
            data = ole.openstream(lamp_path).read()
            lamp_strings = extract_strings(data, max_strings=5)
            for s in lamp_strings:
                m = re.match(r"^(\d{4})K$", s)
                if m:
                    ole.close()
                    return m.group(1)
        ole.close()
    except Exception:
        pass
    return ""


CATEGORIES = {
    "Pendant lamp", "Pendelleuchte", "Ceiling light", "LED panel",
    "Surface-mounted wall light", "Recessed wall light", "Standing lamp",
    "Bollard light", "Display light", "Display case light", "Floodlights",
    "Surface-mounted furniture light", "Surface-mounted solar lights",
    "Wall washers", "High bay lights", "Power distributor",
    "Modular system", "Recessed light", "Surface-mounted ceiling light",
    "Recessed ceiling light", "Three-phase spotlight", "Three-phase pendant lamp",
    "Pendant luminaire", "Downlight", "Track light",
    "LED Strip", "LED Stripe", "Strahler",
}


def extract_family_article(filepath: Path) -> tuple:
    """Extract (family, article) from ULD CONTENTS.

    Returns (family_name, article_number).

    Layout A (clean [0]):
        [0]=Manufacturer  [1]="Coming soon" or product_name
        If [1]=="Coming soon": [2]=Family, [3]=Category, [4]=Article
        If [1]!=Coming soon:   [1]=Family, [2]=Category, [3]=Article

    Layout B (garbled [0]):
        [0]=garbled  [1]=Manufacturer  [2]=garbled
        [3]=Family or "Coming soon"  [4]=Category or Family  [5]=Article
    """
    try:
        import olefile
        ole = olefile.OleFileIO(str(filepath))
        data = ole.openstream("CONTENTS").read()
        ole.close()
    except Exception:
        return ("", "")

    strings = extract_strings(data, max_strings=10)
    if len(strings) < 4:
        return ("", "")

    family = ""
    article = ""

    # Both layouts start with Manufacturer at [0] or [1]
    # Find manufacturer index
    mfr_idx = 0
    if is_garbled(strings[0]):
        mfr_idx = 1  # [0] is garbled, [1] is manufacturer

    # After manufacturer, find family and article
    # Skip garbled strings, "Coming soon", categories, and long descriptions
    remaining = strings[mfr_idx + 1:]

    # Find article number first (first all-digit >= 5 chars)
    for s in remaining:
        if s.isdigit() and len(s) >= 5:
            article = s
            break

    # Find family: first short, meaningful string that's not a category
    def is_description(s):
        """Long variant description like 'Round white cover, boss for Alwaid'"""
        return "," in s and len(s) > 25

    for s in remaining:
        if s == article:
            break
        if s == "Coming soon" or is_garbled(s) or s in CATEGORIES:
            continue
        if is_description(s):
            continue
        family = s
        break

    return (family, article)


def sanitize(name: str) -> str:
    name = re.sub(r'[<>:"/\\|?*,]', '_', name)
    name = re.sub(r'\s+', '_', name)
    name = re.sub(r'_+', '_', name)
    return name.strip('_. ')[:80]


def main():
    # Remove artifacts
    for ext in ("*.l3d", "*.gldf"):
        for f in ULD_DIR.glob(ext):
            f.unlink()

    uld_files = sorted(ULD_DIR.glob("*.uld"))
    print(f"Total ULD files: {len(uld_files)}")

    # Step 1: Remove identical duplicates
    md5_map = defaultdict(list)
    for f in uld_files:
        md5 = hashlib.md5(f.read_bytes()).hexdigest()
        md5_map[md5].append(f)

    removed = 0
    for md5, files in md5_map.items():
        if len(files) > 1:
            files.sort(key=lambda f: len(f.name))
            for dup in files[1:]:
                dup.unlink()
                removed += 1
    if removed:
        print(f"Removed {removed} identical duplicates")

    # Reload
    uld_files = sorted(ULD_DIR.glob("*.uld"))
    print(f"Unique files: {len(uld_files)}")

    # Step 2: Extract family + article from each file, build rename plan
    rename_plan = {}  # old_path -> new_name
    name_counts = defaultdict(int)

    for f in uld_files:
        family, article = extract_family_article(f)
        cct = extract_cct(f)

        if not article:
            # Keep current name
            continue

        parts = [sanitize(family)] if family else []
        parts.append(article)
        new_name = "_".join(parts) + ".uld"

        # Check for collision
        name_counts[new_name] += 1
        if name_counts[new_name] > 1:
            # Disambiguate with CCT
            if cct:
                parts_cct = ([sanitize(family)] if family else []) + [article, f"{cct}K"]
                new_name = "_".join(parts_cct) + ".uld"
                if new_name in name_counts:
                    name_counts[new_name] += 1
                    short = hashlib.md5(f.read_bytes()).hexdigest()[:6]
                    new_name = "_".join(parts_cct + [short]) + ".uld"
            else:
                short = hashlib.md5(f.read_bytes()).hexdigest()[:6]
                new_name = "_".join(parts + [short]) + ".uld"

        name_counts[new_name] = name_counts.get(new_name, 0)
        rename_plan[f] = new_name

    # Execute renames via temp files
    temp_map = {}
    for f, new_name in rename_plan.items():
        if f.name != new_name:
            temp = f.parent / (f.stem + ".tmp")
            f.rename(temp)
            temp_map[temp] = ULD_DIR / new_name

    renamed = 0
    for temp, final in temp_map.items():
        if not final.exists():
            temp.rename(final)
            renamed += 1
        else:
            # Collision — append hash
            short = hashlib.md5(temp.read_bytes()).hexdigest()[:6]
            alt = final.parent / (final.stem + f"_{short}.uld")
            temp.rename(alt)
            renamed += 1

    print(f"Renamed: {renamed}")

    # Final stats
    final_files = sorted(ULD_DIR.glob("*.uld"))
    print(f"\nFinal: {len(final_files)} files")

    # Count families
    families = defaultdict(int)
    for f in final_files:
        parts = f.stem.split("_")
        # Family is everything before the article number
        fam_parts = []
        for p in parts:
            if p.isdigit() and len(p) >= 5:
                break
            fam_parts.append(p)
        family = "_".join(fam_parts) if fam_parts else "unknown"
        families[family] += 1

    print(f"\nProduct families: {len(families)}")
    for fam, count in sorted(families.items(), key=lambda x: (-x[1], x[0]))[:20]:
        print(f"  {fam}: {count}")

    print(f"\nSample files:")
    for f in final_files[:15]:
        print(f"  {f.name}")


if __name__ == "__main__":
    main()
