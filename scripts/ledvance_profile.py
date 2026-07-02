"""
Phase 1: Create LEDVANCE browser profile

Run this first:
    python3 ledvance_profile.py

It opens a real browser with a persistent profile.
1. Navigate to ledvance.com
2. Let the WAF challenge complete
3. Click around a product page to confirm it works
4. Close the browser (Ctrl+C or close window)

The profile (cookies + session) is saved to ./ledvance_profile/
Then run ledvance_download.py to use it.
"""

from playwright.sync_api import sync_playwright
import sys

PROFILE_DIR = "./ledvance_profile"
START_URL = "https://www.ledvance.com/en-int/professional-lighting/products/luminaires/professional-luminaires/floodlights/ip-66-high-power-floodlight-for-large-area-and-sports-lighting/high-power-floodlight-for-large-area-and-sports-lighting--up-to-164-klm-c164205?productId=134000"

def main():
    print("=" * 60)
    print("LEDVANCE Profile Creator")
    print("=" * 60)
    print(f"Profile dir: {PROFILE_DIR}")
    print(f"Opening: {START_URL[:80]}...")
    print()
    print("Instructions:")
    print("  1. Wait for the WAF challenge to resolve")
    print("  2. Verify you can see the product page")
    print("  3. Click a few download links to confirm they work")
    print("  4. Close the browser window when done")
    print("=" * 60)

    with sync_playwright() as p:
        context = p.chromium.launch_persistent_context(
            PROFILE_DIR,
            headless=False,
            accept_downloads=True,
            args=["--no-sandbox"],
            user_agent="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
                       "AppleWebKit/537.36 (KHTML, like Gecko) "
                       "Chrome/131.0.0.0 Safari/537.36",
        )
        page = context.new_page()
        page.goto(START_URL)

        print("\nBrowser is open. Navigate freely, then close the window.")
        try:
            # Block until user closes the browser
            page.wait_for_event("close", timeout=0)
        except KeyboardInterrupt:
            pass
        except Exception:
            pass

        context.close()

    print("\n✅ Profile saved to", PROFILE_DIR)
    print("Now run: python3 ledvance_download.py")


if __name__ == "__main__":
    main()
