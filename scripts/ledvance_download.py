"""
Phase 2: Download all LEDVANCE Floodlight MAX assets

Prerequisites:
    1. Run ledvance_profile.py first to create the browser profile
    2. Then: python3 ledvance_download.py

Uses the saved profile (cookies) to bypass Azure WAF.
Downloads LDT, IES, STEP, ULD, ROLF, images for all FL MAX variants.
"""

import os
import time
import re
import json
from pathlib import Path
from playwright.sync_api import sync_playwright

PROFILE_DIR = "./ledvance_profile"
OUTPUT_DIR = Path("ledvance_assets")

# ── All Floodlight MAX product variants ──────────────────────────────────────

PRODUCTS = [
    # 1200W variants
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-164-klm-c164205?productId=133998",
        "name": "FL_MAX_1200W_SYM10",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-164-klm-c164205?productId=133996",
        "name": "FL_MAX_1200W_SYM30",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-164-klm-c164205?productId=134000",
        "name": "FL_MAX_1200W_SYM60",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-164-klm-c164205?productId=134002",
        "name": "FL_MAX_1200W_ASYM",
        "dir": "floodlight_max",
    },
    # 900W variants
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-123-klm-c164204?productId=133992",
        "name": "FL_MAX_900W_SYM10",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-123-klm-c164204?productId=133990",
        "name": "FL_MAX_900W_SYM30",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-123-klm-c164204?productId=133994",
        "name": "FL_MAX_900W_SYM60",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-123-klm-c164204?productId=133988",
        "name": "FL_MAX_900W_ASYM",
        "dir": "floodlight_max",
    },
    # 600W variants
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-82-klm-c164203?productId=133984",
        "name": "FL_MAX_600W_SYM10",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-82-klm-c164203?productId=133982",
        "name": "FL_MAX_600W_SYM30",
        "dir": "floodlight_max",
    },
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-82-klm-c164203?productId=133986",
        "name": "FL_MAX_600W_ASYM",
        "dir": "floodlight_max",
    },
    # Floodlight FLEX 360W (one representative)
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/floodlight-with-lens-based-light-distribution--onoff-and-dali-d4i-version/floodlight-with-lens-based-light-distribution-up-to-166-lmw-c336781",
        "name": "FL_FLEX_360W",
        "dir": "floodlight_flex",
    },
    # Floodlight Performance SYM 60 150W
    {
        "url": "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/floodlight-performance/floodlight-performance-sym-60-c8905?productId=138552",
        "name": "FL_PFM_150W_SYM60",
        "dir": "floodlight_performance",
    },
]

# File extensions worth downloading, mapped to subdirectories
EXT_MAP = {
    ".ldt": "photometry",
    ".ies": "photometry",
    ".uld": "photometry",
    ".rolf": "photometry",
    ".step": "geometry",
    ".stp": "geometry",
    ".zip": "geometry",   # Revit BIM families come as ZIP
    ".jpg": "images",
    ".png": "images",
    ".svg": "images",
}

# Skip huge video files and duplicates
SKIP_PATTERNS = [".mp4", ".tif", "video", "case study"]


def wait_for_content(page, timeout_s=40):
    """Wait until the LEDVANCE SPA has loaded real content.
    
    Handles context destruction from WAF redirects gracefully.
    """
    for i in range(timeout_s):
        try:
            # wait_for_load_state is safer than polling title()
            page.wait_for_load_state("domcontentloaded", timeout=2000)
        except Exception:
            pass

        try:
            title = page.title()
        except Exception:
            # Context destroyed by navigation (WAF redirect) — just wait
            time.sleep(1)
            continue

        if not title or "Azure WAF" in title:
            time.sleep(1)
            continue

        # Title looks real — now check for actual product content
        try:
            content = page.content()
            if "LEDVANCE" in content or "Photometric" in content or "LDT" in content:
                return True
        except Exception:
            time.sleep(1)
            continue

        time.sleep(1)
    return False


def extract_download_links(page):
    """
    LEDVANCE renders download links dynamically in their SPA.
    We look for <a> tags whose href ends with a known extension
    OR whose visible text mentions a file type.
    """
    # Method 1: JS extraction from the rendered DOM
    links = page.evaluate("""
        () => {
            const results = [];
            // All anchor tags
            document.querySelectorAll('a[href]').forEach(a => {
                const href = a.href || '';
                const text = (a.textContent || '').trim();
                if (href && (href !== window.location.href)) {
                    results.push({ href, text });
                }
            });
            // Also check buttons with download behavior
            document.querySelectorAll('[data-href], [data-url], [data-download-url]').forEach(el => {
                const href = el.getAttribute('data-href') 
                          || el.getAttribute('data-url')
                          || el.getAttribute('data-download-url') || '';
                const text = (el.textContent || '').trim();
                if (href) results.push({ href, text });
            });
            return results;
        }
    """)

    download_links = {}
    for item in links:
        href = item["href"]
        text = item["text"]
        href_lower = href.lower()

        # Skip garbage
        if any(skip in href_lower for skip in SKIP_PATTERNS):
            continue
        if any(skip in text.lower() for skip in SKIP_PATTERNS):
            continue

        # Check if it looks like a downloadable file
        for ext in EXT_MAP:
            if href_lower.endswith(ext) or f"{ext}?" in href_lower:
                label = text if text else href.split("/")[-1].split("?")[0]
                download_links[label] = href
                break

    return download_links


def download_with_page(page, url, dest_path: Path):
    """Download a file using the browser context (preserves WAF cookies)."""
    if dest_path.exists() and dest_path.stat().st_size > 0:
        print(f"    ⏭  exists: {dest_path.name}")
        return True

    dest_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        # Trigger download via JS to stay in same context
        with page.expect_download(timeout=60_000) as dl_info:
            page.evaluate(f"""
                () => {{
                    const a = document.createElement('a');
                    a.href = '{url}';
                    a.download = '';
                    document.body.appendChild(a);
                    a.click();
                    a.remove();
                }}
            """)
        download = dl_info.value
        download.save_as(str(dest_path))
        size = dest_path.stat().st_size
        print(f"    ✅ {dest_path.name}  ({size:,} B)")
        return True
    except Exception:
        pass

    # Fallback: direct fetch via API request context
    try:
        resp = page.context.request.get(url)
        if resp.ok:
            dest_path.write_bytes(resp.body())
            size = dest_path.stat().st_size
            print(f"    ✅ {dest_path.name}  ({size:,} B) [direct]")
            return True
        else:
            print(f"    ❌ {dest_path.name}  HTTP {resp.status}")
    except Exception as e:
        print(f"    ❌ {dest_path.name}  {e}")

    return False


def sanitize_filename(name: str) -> str:
    """Make a safe filename from download link text."""
    # Remove common prefixes LEDVANCE uses
    for prefix in ["IES file (IES)", "LDT file (Eulumdat)", "ULD file (DIALux)",
                    "ROLF file (RELUX)", "BIM Revit 3D", "BIM_Revit_3D",
                    "CAD STEP 3D", "CAD_STEP_3D", "Light distribution curve type polar",
                    "LDC typ polar", "Combo Shot", "Packaging image",
                    "Product beauty shot", "Catalog info"]:
        if name.startswith(prefix):
            name = name[len(prefix):].strip()
            break

    # Clean up
    name = re.sub(r'[<>:"/\\|?*]', '_', name)
    name = re.sub(r'\s+', '_', name).strip('_')
    return name


def guess_extension(label: str, url: str) -> str:
    """Guess file extension from label or URL."""
    url_lower = url.lower()
    for ext in EXT_MAP:
        if url_lower.endswith(ext) or f"{ext}?" in url_lower:
            return ext
    label_lower = label.lower()
    if "ldt" in label_lower or "eulumdat" in label_lower:
        return ".ldt"
    if "ies" in label_lower:
        return ".ies"
    if "step" in label_lower:
        return ".step"
    if "uld" in label_lower or "dialux" in label_lower:
        return ".uld"
    if "rolf" in label_lower or "relux" in label_lower:
        return ".rolf"
    if "revit" in label_lower or "bim" in label_lower:
        return ".zip"
    return ""


def process_product(page, product):
    """Navigate to a product page and download all assets."""
    name = product["name"]
    url = product["url"]
    base_dir = OUTPUT_DIR / product["dir"]

    print(f"\n{'─' * 60}")
    print(f"  {name}")
    print(f"{'─' * 60}")

    try:
        page.goto(url, wait_until="commit", timeout=30_000)
    except Exception as e:
        # Navigation might timeout due to WAF redirect chain — that's OK
        print(f"  ⚠ Initial navigation: {e}")

    if not wait_for_content(page):
        print("  ⚠ Page didn't load properly. Skipping.")
        try:
            page.screenshot(path=str(base_dir / f"_debug_{name}.png"))
        except Exception:
            pass
        return []

    # Give SPA time to render download section
    try:
        page.wait_for_timeout(5000)
    except Exception:
        time.sleep(5)

    # Try expanding download sections
    for selector in [
        "text=Photometric and lighting design files",
        "text=CAD/BIM files",
        "text=Documents and certificates",
        "text=Product images",
    ]:
        try:
            el = page.locator(selector).first
            if el.is_visible(timeout=1000):
                el.click()
                page.wait_for_timeout(500)
        except Exception:
            pass

    try:
        links = extract_download_links(page)
    except Exception as e:
        print(f"  ⚠ Failed to extract links: {e}")
        links = {}
    if not links:
        print("  ⚠ No download links found")
        try:
            (base_dir).mkdir(parents=True, exist_ok=True)
            page.screenshot(path=str(base_dir / f"_debug_{name}.png"))
        except Exception:
            pass
        return []

    print(f"  Found {len(links)} downloadable files")

    downloaded = []
    seen_extensions = set()

    for label, href in links.items():
        ext = guess_extension(label, href)
        if not ext:
            continue

        subdir = EXT_MAP.get(ext, "other")

        # Build filename
        fname = sanitize_filename(label)
        if not fname.lower().endswith(ext):
            fname += ext

        dest = base_dir / subdir / fname

        # Skip duplicate STEP/ZIP (shared across variants)
        if ext in (".step", ".zip"):
            marker = f"{ext}:{fname.lower()}"
            if marker in seen_extensions:
                print(f"    ⏭  dup: {fname}")
                continue
            seen_extensions.add(marker)

        if download_with_page(page, href, dest):
            downloaded.append(dest)

    return downloaded


def main():
    if not Path(PROFILE_DIR).exists():
        print("❌ No profile found. Run ledvance_profile.py first!")
        return

    print("=" * 60)
    print("LEDVANCE Asset Downloader (using saved profile)")
    print("=" * 60)

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as p:
        context = p.chromium.launch_persistent_context(
            PROFILE_DIR,
            headless=True,          # headless works now — cookies solve WAF
            accept_downloads=True,
            args=["--no-sandbox"],
            user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                       "AppleWebKit/537.36 (KHTML, like Gecko) "
                       "Chrome/131.0.0.0 Safari/537.36",
        )
        page = context.new_page()

        # Quick check: can we load LEDVANCE with saved cookies?
        print("\nVerifying profile...")
        try:
            page.goto("https://www.ledvance.com/en-int/", wait_until="commit", timeout=15_000)
        except Exception:
            pass  # WAF redirect chain may timeout — OK
        if wait_for_content(page, timeout_s=20):
            print("✅ Profile works! WAF bypassed via saved cookies.\n")
        else:
            print("⚠ Profile might be stale. Run ledvance_profile.py again.")
            print("  Continuing anyway...\n")

        all_files = []
        for product in PRODUCTS:
            try:
                files = process_product(page, product)
                all_files.extend(files)
            except Exception as e:
                print(f"  ❌ Crashed on {product['name']}: {e}")
                # Try to recover by navigating away
                try:
                    page.goto("about:blank", timeout=5000)
                    time.sleep(1)
                except Exception:
                    pass
            time.sleep(2)  # polite delay

        context.close()

    # ── Summary ──────────────────────────────────────────────────────────────
    print(f"\n{'=' * 60}")
    print(f"Done! {len(all_files)} files downloaded to {OUTPUT_DIR.absolute()}")
    print(f"{'=' * 60}")

    # Group by type
    by_ext = {}
    for f in all_files:
        ext = f.suffix.lower()
        by_ext.setdefault(ext, []).append(f)

    for ext, files in sorted(by_ext.items()):
        total = sum(f.stat().st_size for f in files)
        print(f"  {ext:6s}  {len(files):3d} files  ({total:>12,} bytes)")

    print(f"\nTree:")
    for f in sorted(all_files):
        rel = f.relative_to(OUTPUT_DIR)
        print(f"  {rel}")


if __name__ == "__main__":
    main()
