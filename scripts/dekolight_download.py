"""
Download all Deko-Light ULD files from DIALux Silver catalog.

Usage:
    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python3 scripts/dekolight_download.py

Tracks downloaded UUIDs in a manifest file to avoid re-downloading
even after files are renamed by dekolight_dedup.py.
"""

import json
import re
import time
import urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright

OUTPUT_DIR = Path("scripts/dekolight_uld")
MANIFEST = OUTPUT_DIR / "_downloaded_uuids.json"
CATALOG_URL = "https://silver.dialux.com/2057/Deko-Light"


def sanitize_filename(name: str) -> str:
    name = re.sub(r'[<>:"/\\|?*]', '_', name)
    name = re.sub(r'\s+', '_', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_. ')
    return name[:120]


def dial_to_https(dial_url: str) -> str:
    return dial_url.replace("dial://", "https://")


def extract_uuid(url: str) -> str:
    m = re.search(r'([a-f0-9-]{36})\.uld', url)
    return m.group(1) if m else ""


CONTENT_HASHES = OUTPUT_DIR / "_content_hashes.json"


def load_manifest() -> set:
    """Load set of already-downloaded UUIDs."""
    if MANIFEST.exists():
        return set(json.loads(MANIFEST.read_text()))
    return set()


def save_manifest(uuids: set):
    MANIFEST.write_text(json.dumps(sorted(uuids), indent=2))


def load_content_hashes() -> set:
    """Load content hashes of already-downloaded files."""
    if CONTENT_HASHES.exists():
        data = json.loads(CONTENT_HASHES.read_text())
        return set(data.get("content_hashes", []))
    return set()


def save_content_hashes(hashes: set):
    CONTENT_HASHES.write_text(json.dumps({"content_hashes": sorted(hashes)}))


def collect_visible_uld_links(page):
    """Collect all dial:// ULD links currently rendered on the page."""
    return page.evaluate(r"""() => {
        const results = [];
        document.querySelectorAll('a[href^="dial://"][href*=".uld"]').forEach(a => {
            const url = a.href;
            let name = '';
            let el = a;
            for (let i = 0; i < 15 && el; i++) {
                el = el.parentElement;
                if (!el) break;
                const nameEl = el.querySelector(
                    '[class*="name"], [class*="Name"], [class*="title"], [class*="Title"], h3, h4, h5'
                );
                if (nameEl) { name = nameEl.textContent.trim(); break; }
            }
            if (!name) {
                const row = a.closest(
                    'tr, [class*="product"], [class*="Product"], [class*="row"], [class*="card"]'
                );
                if (row) {
                    const cells = row.querySelectorAll('td, span, div');
                    for (const c of cells) {
                        const t = c.textContent.trim();
                        if (t.length > 3 && t.length < 100 && !t.includes('dial://')) {
                            name = t; break;
                        }
                    }
                }
            }
            results.push({url, name});
        });
        return results;
    }""")


def scrape_all_products(page):
    """Scroll through the entire Blazor virtual list to collect all ULD links."""
    all_uld_urls = set()
    all_products = []

    scroll_pos = 0
    step = 800
    prev_count = 0
    stable = 0

    body_height = page.evaluate('document.body.scrollHeight')
    print(f"Page height: {body_height}px, scrolling in {step}px steps...")

    while scroll_pos < body_height + 2000:
        page.evaluate(f'window.scrollTo(0, {scroll_pos})')
        time.sleep(0.3)

        links = collect_visible_uld_links(page)
        for link in links:
            if link['url'] not in all_uld_urls:
                all_uld_urls.add(link['url'])
                all_products.append(link)

        current = len(all_uld_urls)
        if current != prev_count:
            if current % 100 < 20:
                print(f"  scroll={scroll_pos}: {current} unique ULDs")
            stable = 0
            prev_count = current
        else:
            stable += 1

        scroll_pos += step

        new_height = page.evaluate('document.body.scrollHeight')
        if new_height > body_height:
            body_height = new_height

        if scroll_pos > body_height and stable > 10:
            break

    return all_products


def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    import hashlib

    # Load manifest of already-downloaded UUIDs + content hashes
    downloaded_uuids = load_manifest()
    content_hashes = load_content_hashes()
    print(f"Already downloaded: {len(downloaded_uuids)} UUIDs, {len(content_hashes)} content hashes")

    # Phase 1: Scrape all product links
    print(f"\n=== Phase 1: Scraping catalog ===")
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                       "AppleWebKit/537.36 (KHTML, like Gecko) "
                       "Chrome/120.0.0.0 Safari/537.36"
        )
        page = context.new_page()

        print(f"Loading: {CATALOG_URL}")
        page.goto(CATALOG_URL, wait_until="networkidle", timeout=60000)
        time.sleep(3)
        print(f"Page: {page.title()}")

        products = scrape_all_products(page)
        browser.close()

    # Deduplicate by URL
    seen_urls = set()
    unique = []
    for prod in products:
        if prod['url'] not in seen_urls:
            seen_urls.add(prod['url'])
            unique.append(prod)
    products = unique

    print(f"\nTotal unique ULD products in catalog: {len(products)}")

    # Filter out already-downloaded
    to_download = []
    for prod in products:
        uuid = extract_uuid(prod['url'])
        if uuid and uuid not in downloaded_uuids:
            to_download.append(prod)

    print(f"Need to download: {len(to_download)} (skipping {len(products) - len(to_download)} already downloaded)")

    if not to_download:
        print("Nothing to download!")
        return

    # Phase 2: Download
    print(f"\n=== Phase 2: Downloading ===")
    downloaded = 0
    skipped = 0
    failed = 0

    for i, prod in enumerate(to_download):
        dial_url = prod['url']
        name = prod.get('name', '')
        uuid = extract_uuid(dial_url)
        https_url = dial_to_https(dial_url)

        if name:
            safe_name = sanitize_filename(name)
            filename = f"{safe_name}__{uuid[:8]}.uld"
        else:
            filename = f"dekolight_{uuid[:8]}.uld"

        filepath = OUTPUT_DIR / filename

        if (i + 1) % 50 == 0:
            print(f"  Progress: {downloaded} new, {skipped} dupe-content, {failed} failed ({i+1}/{len(to_download)})")

        try:
            req = urllib.request.Request(https_url, headers={
                "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                              "AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
                "Referer": CATALOG_URL,
            })
            with urllib.request.urlopen(req, timeout=30) as resp:
                data = resp.read()
                if len(data) > 100:
                    # Check if we already have this content
                    content_hash = hashlib.md5(data).hexdigest()
                    if content_hash in content_hashes:
                        downloaded_uuids.add(uuid)
                        skipped += 1
                        continue
                    filepath.write_bytes(data)
                    downloaded_uuids.add(uuid)
                    content_hashes.add(content_hash)
                    downloaded += 1
                else:
                    print(f"  WARN: {filename} too small ({len(data)} bytes)")
                    failed += 1
        except Exception as e:
            print(f"  FAIL: {filename}: {e}")
            failed += 1

        time.sleep(0.2)

        # Save manifests periodically
        if (downloaded + skipped) % 50 == 0 and downloaded > 0:
            save_manifest(downloaded_uuids)
            save_content_hashes(content_hashes)

    # Final manifest save
    save_manifest(downloaded_uuids)
    save_content_hashes(content_hashes)

    total_files = len(list(OUTPUT_DIR.glob('*.uld')))
    print(f"\nDone: {downloaded} new, {skipped} dupe-content skipped, {failed} failed")
    print(f"Total files on disk: {total_files}")
    print(f"Total UUIDs in manifest: {len(downloaded_uuids)}")


if __name__ == "__main__":
    main()
