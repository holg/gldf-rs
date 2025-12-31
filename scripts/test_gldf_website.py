#!/usr/bin/env python3
"""
Playwright test for gldf.icu website - creates an animated GIF of the demo walkthrough
"""

import asyncio
import os
import glob
from pathlib import Path
from playwright.async_api import async_playwright
from PIL import Image
import io

OUTPUT_DIR = Path("/Users/htr/Documents/develeop/rust/gldf-rs/scripts/screenshots")
GIF_PATH = Path("/Users/htr/Documents/develeop/rust/gldf-rs/scripts/gldf_demo.gif")

async def capture_frame(page, frames: list, delay_ms: int = 100):
    """Capture a screenshot and add to frames list"""
    screenshot = await page.screenshot()
    img = Image.open(io.BytesIO(screenshot))
    frames.append(img.convert('RGB'))
    await asyncio.sleep(delay_ms / 1000)

async def capture_continuous(page, frames: list, duration_ms: int, interval_ms: int = 200):
    """Capture frames continuously for a duration"""
    iterations = duration_ms // interval_ms
    for _ in range(iterations):
        await capture_frame(page, frames, interval_ms)

async def main():
    OUTPUT_DIR.mkdir(exist_ok=True)
    frames = []

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=False)
        context = await browser.new_context(
            viewport={'width': 1400, 'height': 900}
        )
        page = await context.new_page()

        print("Opening gldf.icu...")
        await page.goto("https://gldf.icu", wait_until="networkidle")

        # Wait for WASM app to initialize
        print("Waiting for app to load...")
        await page.wait_for_timeout(3000)
        await capture_frame(page, frames)

        # Look for and click demo/load demo button
        print("Looking for demo button...")
        demo_button = page.locator("text=Demo").first
        if await demo_button.count() > 0:
            await demo_button.click()
            print("Clicked Demo button")
            await page.wait_for_timeout(2000)
            await capture_continuous(page, frames, 2000, 300)
        else:
            # Try alternative selectors
            demo_button = page.locator("button:has-text('demo'), a:has-text('demo'), [class*='demo']").first
            if await demo_button.count() > 0:
                await demo_button.click()
                print("Clicked demo element")
                await page.wait_for_timeout(2000)
                await capture_continuous(page, frames, 2000, 300)

        # Find sidebar/navigation items (excluding file viewer for now)
        print("Looking for sidebar navigation...")

        # Common patterns for sidebar items - try different selectors
        sidebar_selectors = [
            "nav a",
            ".sidebar a",
            ".sidebar button",
            "[class*='sidebar'] a",
            "[class*='sidebar'] button",
            "[class*='nav'] a",
            "[class*='menu'] a",
            "[class*='menu'] button",
            ".tree-item",
            "[role='treeitem']",
            "[role='tab']",
            "aside a",
            "aside button",
        ]

        sidebar_items = []
        for selector in sidebar_selectors:
            items = page.locator(selector)
            count = await items.count()
            if count > 0:
                print(f"Found {count} items with selector: {selector}")
                for i in range(min(count, 10)):  # Limit to first 10
                    item = items.nth(i)
                    text = await item.text_content()
                    if text and 'file' not in text.lower() and 'viewer' not in text.lower():
                        sidebar_items.append((item, text.strip()))
                break

        # If no sidebar found, look for any clickable elements in left portion
        if not sidebar_items:
            print("Trying to find any left-side clickable elements...")
            all_clickables = page.locator("button, a, [role='button'], [onclick]")
            count = await all_clickables.count()
            print(f"Found {count} total clickable elements")

            for i in range(count):
                item = all_clickables.nth(i)
                try:
                    box = await item.bounding_box()
                    if box and box['x'] < 400:  # Left side of screen
                        text = await item.text_content()
                        if text and len(text.strip()) > 0:
                            text = text.strip()[:50]
                            if 'file' not in text.lower() and 'viewer' not in text.lower():
                                sidebar_items.append((item, text))
                                if len(sidebar_items) >= 8:
                                    break
                except:
                    pass

        # Click through sidebar items briefly
        print(f"Clicking through {len(sidebar_items)} sidebar items...")
        for item, text in sidebar_items[:6]:  # Limit to 6 items
            try:
                print(f"  Clicking: {text[:30]}...")
                await item.click()
                await page.wait_for_timeout(800)
                await capture_continuous(page, frames, 800, 200)
            except Exception as e:
                print(f"  Failed to click {text}: {e}")

        # Now find and click File Viewer
        print("Looking for File Viewer...")
        file_viewer = None
        file_viewer_selectors = [
            "text=File Viewer",
            "text=FileViewer",
            "text=file viewer",
            "button:has-text('File')",
            "a:has-text('File')",
            "[class*='file']",
        ]

        for selector in file_viewer_selectors:
            fv = page.locator(selector).first
            if await fv.count() > 0:
                file_viewer = fv
                print(f"Found File Viewer with: {selector}")
                break

        if file_viewer:
            await file_viewer.click()
            print("Clicked File Viewer")
            await page.wait_for_timeout(1500)
            await capture_continuous(page, frames, 1500, 300)

        # Slowly scroll down in the main content area
        print("Scrolling down slowly...")

        # Try to find main scrollable area
        scroll_target = page.locator("main, .content, [class*='content'], [class*='viewer'], .scroll-container").first
        if await scroll_target.count() == 0:
            scroll_target = page  # Fallback to page

        # Scroll in increments
        for scroll_step in range(10):
            await page.evaluate("window.scrollBy(0, 150)")
            # Also try scrolling specific containers
            await page.evaluate("""
                document.querySelectorAll('[class*="scroll"], [class*="content"], main, [style*="overflow"]').forEach(el => {
                    el.scrollBy(0, 150);
                });
            """)
            await page.wait_for_timeout(400)
            await capture_frame(page, frames)

        # Final pause
        await capture_continuous(page, frames, 1000, 300)

        print(f"Captured {len(frames)} frames")

        await browser.close()

    # Create animated GIF
    if frames:
        print(f"Creating GIF with {len(frames)} frames...")
        frames[0].save(
            GIF_PATH,
            save_all=True,
            append_images=frames[1:],
            duration=300,  # 300ms per frame
            loop=0,
            optimize=True
        )
        print(f"GIF saved to: {GIF_PATH}")

        # Also save individual frames for debugging
        for i, frame in enumerate(frames[:5]):
            frame.save(OUTPUT_DIR / f"frame_{i:03d}.png")
        print(f"Sample frames saved to: {OUTPUT_DIR}")
    else:
        print("No frames captured!")

if __name__ == "__main__":
    asyncio.run(main())
