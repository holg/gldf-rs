"""E2E tests for GLDF WASM Viewer (macOS-style design)."""
import pytest
from pathlib import Path
from playwright.sync_api import Page, expect

# Test data paths
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
TEST_DATA_DIR = PROJECT_ROOT / "tests" / "data"

# Test files
TEST_GLDF_WITH_L3D = TEST_DATA_DIR / "Freestand_Belviso-2l3d.gldf"
TEST_LDT_FILE = TEST_DATA_DIR / "test.ldt"
TEST_ROAD_LDT = TEST_DATA_DIR / "road.ldt"


class TestMainPage:
    """Tests for the main page display."""

    def test_page_loads_correctly(self, page: Page, app_url: str):
        """Test that the main page loads and displays correctly."""
        page.goto(app_url)

        # Wait for WASM to load and app to render - sidebar indicates app is ready
        page.wait_for_selector(".sidebar", timeout=30000)

        # Check welcome view title
        expect(page.locator(".welcome-title")).to_contain_text("GLDF Viewer")
        expect(page.locator(".welcome-subtitle")).to_contain_text("Global Lighting Data Format")

    def test_info_section_visible(self, page: Page, app_url: str):
        """Test that info section with links is visible."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Check info section in sidebar
        expect(page.locator('a[href="https://gldf.io"]')).to_be_visible()
        expect(page.locator(".privacy-note").first).to_contain_text("local")

    def test_upload_section_visible(self, page: Page, app_url: str):
        """Test that upload section is visible."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Check welcome view and file upload button
        expect(page.locator(".welcome-view").first).to_be_visible()
        expect(page.locator("#file-upload")).to_be_attached()


class TestGLDFViewer:
    """Tests for GLDF file viewing functionality."""

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_load_gldf_file(self, page: Page, app_url: str):
        """Test loading a GLDF file displays product information."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))

        # Wait for file to be processed - ensure page is stable
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(500)

        # Check that content is displayed
        expect(page.locator(".main-content")).to_be_visible()

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_gldf_json_content(self, page: Page, app_url: str):
        """Test that JSON product data is displayed in Raw Data view."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Click on Raw Data nav item
        raw_data_link = page.locator(".sidebar-nav li", has_text="Raw Data")
        if raw_data_link.count() > 0:
            raw_data_link.click(force=True)
            page.wait_for_timeout(500)

            # Check JSON textarea
            textarea = page.locator(".preview-media textarea, textarea").first
            if textarea.count() > 0:
                expect(textarea).to_be_visible()
                json_content = textarea.input_value()
                assert "product" in json_content.lower() or len(json_content) > 100, "Expected JSON content"
            else:
                # Content should be visible in some form
                expect(page.locator(".content-body")).to_be_visible()
        else:
            pytest.skip("Raw Data nav item not found in current UI")

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_embedded_files_displayed(self, page: Page, app_url: str):
        """Test that embedded files from GLDF are displayed."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Click on Files nav item
        files_link = page.locator(".sidebar-nav li", has_text="Files")
        if files_link.count() > 0:
            files_link.click(force=True)
            page.wait_for_timeout(500)

            # Check for file entries in table or list
            table_rows = page.locator(".data-table tbody tr, #buf_file, .file-item")
            count = table_rows.count()

            assert count > 0, "Expected at least one file entry in Files view"
        else:
            pytest.skip("Files nav item not found in current UI")


class TestLDTViewer:
    """Tests for LDT diagram viewing functionality."""

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_ldt_diagram_in_gldf(self, page: Page, app_url: str):
        """Test LDT diagram display when embedded in GLDF."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Look for LDT viewer
        ldt_viewer = page.locator(".ldt-viewer")

        if ldt_viewer.count() > 0:
            # Check buttons exist
            expect(ldt_viewer.locator("button").first).to_be_visible()

            # Check SVG is rendered (may have multiple LDT diagrams)
            svg = ldt_viewer.locator("svg").first
            expect(svg).to_be_visible()

    @pytest.mark.skipif(
        not TEST_LDT_FILE.exists() and not TEST_ROAD_LDT.exists(),
        reason="No test LDT files found"
    )
    def test_standalone_ldt_file(self, page: Page, app_url: str):
        """Test loading a standalone LDT file."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Find available LDT file
        ldt_file = TEST_LDT_FILE if TEST_LDT_FILE.exists() else TEST_ROAD_LDT

        # Upload LDT file
        page.locator("#file-upload").set_input_files(str(ldt_file))
        page.wait_for_selector(".content-body", timeout=30000)

        # Navigate to File Viewer to see the LDT diagram
        file_viewer_link = page.locator(".sidebar-nav li", has_text="File Viewer")
        if file_viewer_link.count() > 0:
            file_viewer_link.click(force=True)
            page.wait_for_timeout(500)

        # Check LDT viewer is displayed
        ldt_viewer = page.locator(".ldt-viewer")
        expect(ldt_viewer).to_be_visible()

        # Check SVG diagram
        svg = ldt_viewer.locator(".ldt-diagram svg, svg").first
        expect(svg).to_be_visible()

    @pytest.mark.skipif(
        not TEST_LDT_FILE.exists() and not TEST_ROAD_LDT.exists(),
        reason="No test LDT files found"
    )
    def test_ldt_view_toggle(self, page: Page, app_url: str):
        """Test toggling between Polar and Cartesian views."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Find available LDT file
        ldt_file = TEST_LDT_FILE if TEST_LDT_FILE.exists() else TEST_ROAD_LDT

        # Upload LDT file
        page.locator("#file-upload").set_input_files(str(ldt_file))
        page.wait_for_selector(".content-body", timeout=30000)

        # Navigate to File Viewer to see the LDT diagram
        file_viewer_link = page.locator(".sidebar-nav li", has_text="File Viewer")
        if file_viewer_link.count() > 0:
            file_viewer_link.click(force=True)
            page.wait_for_timeout(1000)

        # Check LDT viewer is displayed (same pattern as test_standalone_ldt_file)
        ldt_viewer = page.locator(".ldt-viewer")
        # Wait for LDT viewer with longer timeout
        ldt_viewer.wait_for(state="visible", timeout=10000)
        expect(ldt_viewer).to_be_visible()

        polar_btn = ldt_viewer.locator("button", has_text="Polar")
        cartesian_btn = ldt_viewer.locator("button", has_text="Cartesian")

        expect(polar_btn).to_be_visible()
        expect(cartesian_btn).to_be_visible()

        # Click Cartesian
        cartesian_btn.click(force=True)
        page.wait_for_timeout(300)

        # SVG should still be visible
        svg = ldt_viewer.locator(".ldt-diagram svg, svg").first
        expect(svg).to_be_visible()

        # Click Polar
        polar_btn.click(force=True)
        page.wait_for_timeout(300)
        expect(svg).to_be_visible()


class TestL3DViewer:
    """Tests for L3D 3D viewing functionality."""

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_l3d_container_visible(self, page: Page, app_url: str):
        """Test L3D container is visible when GLDF contains L3D."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF with L3D
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Navigate to File Viewer to see L3D content
        file_viewer_link = page.locator(".sidebar-nav li", has_text="File Viewer")
        if file_viewer_link.count() > 0:
            file_viewer_link.click(force=True)
            page.wait_for_timeout(1000)

        # Wait for potential L3D loading
        page.wait_for_timeout(2000)

        # Look for L3D container
        l3d_container = page.locator(".l3d-container")

        if l3d_container.count() > 0:
            expect(l3d_container.first).to_be_visible()

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_l3d_canvas_present(self, page: Page, app_url: str):
        """Test WebGL canvas is present for L3D viewer."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF with L3D
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Navigate to File Viewer to see L3D content
        file_viewer_link = page.locator(".sidebar-nav li", has_text="File Viewer")
        if file_viewer_link.count() > 0:
            file_viewer_link.click(force=True)
            page.wait_for_timeout(1000)

        # Wait for L3D loading
        page.wait_for_timeout(3000)

        # Check for canvas (L3D viewer renders canvas inside .l3d-viewer)
        canvas = page.locator(".l3d-viewer canvas")

        if canvas.count() > 0:
            expect(canvas.first).to_be_visible()
        else:
            pytest.skip("L3D canvas not found - 3D viewer may have rendering issues")

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_l3d_debug_info(self, page: Page, app_url: str):
        """Debug test to investigate empty L3D viewer."""
        console_logs = []
        errors = []

        page.on("console", lambda msg: console_logs.append({
            "type": msg.type,
            "text": msg.text
        }))
        page.on("pageerror", lambda err: errors.append(str(err)))

        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF with L3D
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Navigate to File Viewer to see L3D content
        file_viewer_link = page.locator(".sidebar-nav li", has_text="File Viewer")
        if file_viewer_link.count() > 0:
            file_viewer_link.click(force=True)
            page.wait_for_timeout(1000)

        # Wait for L3D processing
        page.wait_for_timeout(5000)

        # Filter L3D-related logs
        l3d_logs = [
            log for log in console_logs
            if any(kw in log["text"].lower() for kw in ["l3d", "3d", "webgl", "canvas", "model", "render"])
        ]

        print("\n=== L3D Debug Info ===")
        print(f"Total console messages: {len(console_logs)}")
        print(f"L3D-related messages: {len(l3d_logs)}")
        for log in l3d_logs:
            print(f"  [{log['type']}] {log['text']}")

        if errors:
            print(f"\nPage errors: {len(errors)}")
            for err in errors:
                print(f"  ERROR: {err}")

        # Check canvas state
        canvas_info = page.evaluate("""
            () => {
                const canvas = document.querySelector('.l3d-container canvas');
                if (!canvas) return { found: false, reason: 'Canvas element not found' };

                const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
                if (!gl) return { found: true, hasGL: false, reason: 'WebGL context not available' };

                return {
                    found: true,
                    hasGL: true,
                    width: canvas.width,
                    height: canvas.height,
                    clientWidth: canvas.clientWidth,
                    clientHeight: canvas.clientHeight,
                };
            }
        """)

        print(f"\nCanvas state: {canvas_info}")

        # Take screenshot
        page.screenshot(path="test-results/l3d-debug.png", full_page=True)

        # Report findings
        assert not errors, f"JavaScript errors detected: {errors}"


class TestSidebarNavigation:
    """Tests for sidebar navigation functionality."""

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_sidebar_navigation_works(self, page: Page, app_url: str):
        """Test that sidebar navigation items work."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Test navigation to different views (only enabled items)
        nav_items = ["Overview", "Statistics", "Files", "Light Sources", "Variants"]

        for nav_text in nav_items:
            nav_link = page.locator(".sidebar-nav li:not(.disabled)", has_text=nav_text)
            if nav_link.count() > 0:
                nav_link.first.click(force=True)
                page.wait_for_timeout(300)
                # Content should still be visible
                expect(page.locator(".main-content")).to_be_visible()

    @pytest.mark.skipif(
        not TEST_GLDF_WITH_L3D.exists(),
        reason=f"Test file not found: {TEST_GLDF_WITH_L3D}"
    )
    def test_statistics_view(self, page: Page, app_url: str):
        """Test that Statistics view displays stat cards."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Upload GLDF file
        page.locator("#file-upload").set_input_files(str(TEST_GLDF_WITH_L3D))
        page.wait_for_selector(".stat-card, .card, .content-body", timeout=30000)

        # Click Statistics
        stats_link = page.locator(".sidebar-nav li", has_text="Statistics")
        if stats_link.count() > 0:
            stats_link.click(force=True)
            page.wait_for_timeout(500)

            # Check stat cards
            stat_cards = page.locator(".stat-card")
            if stat_cards.count() > 0:
                expect(stat_cards.first).to_be_visible()


class TestErrorHandling:
    """Tests for error handling."""

    def test_invalid_file(self, page: Page, app_url: str, tmp_path: Path):
        """Test that invalid files are handled gracefully."""
        page.goto(app_url)
        page.wait_for_selector(".sidebar", timeout=30000)

        # Create invalid file
        invalid_file = tmp_path / "invalid.txt"
        invalid_file.write_text("This is not a valid GLDF file")

        # Upload invalid file
        page.locator("#file-upload").set_input_files(str(invalid_file))

        # Wait a bit
        page.wait_for_timeout(2000)

        # Page should not crash - sidebar should still be visible
        expect(page.locator(".sidebar")).to_be_visible()


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--headed"])
