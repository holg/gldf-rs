"""Pytest configuration for GLDF WASM E2E tests."""
import subprocess
import time
import socket
import pytest
from pathlib import Path


# Test data paths
PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
TEST_DATA_DIR = PROJECT_ROOT / "tests" / "data"
GLDF_RS_WASM_DIR = PROJECT_ROOT / "gldf-rs-wasm"

# Test files
TEST_GLDF_WITH_L3D = TEST_DATA_DIR / "Freestand_Belviso-2l3d.gldf"
TEST_LDT_FILE = TEST_DATA_DIR / "test.ldt"
TEST_ROAD_LDT = TEST_DATA_DIR / "road.ldt"

BASE_URL = "http://127.0.0.1:8080"


def is_port_in_use(port):
    """Check if a port is in use."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        return s.connect_ex(('127.0.0.1', port)) == 0


@pytest.fixture(scope="session")
def browser_context_args(browser_context_args):
    """Configure browser context."""
    return {
        **browser_context_args,
        "viewport": {"width": 1280, "height": 720},
    }


@pytest.fixture(scope="session")
def trunk_server():
    """Start trunk serve before tests and stop after."""
    # Check if server is already running
    if is_port_in_use(8080):
        print("Trunk server already running on port 8080")
        yield BASE_URL
        return

    # Start trunk serve
    print("Starting trunk serve...")
    proc = subprocess.Popen(
        ["trunk", "serve", "--port", "8080"],
        cwd=GLDF_RS_WASM_DIR,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    # Wait for server to be ready
    max_wait = 120  # 2 minutes for WASM build
    for i in range(max_wait):
        if is_port_in_use(8080):
            print(f"Trunk server ready after {i} seconds")
            break
        time.sleep(1)
    else:
        proc.kill()
        raise RuntimeError("Trunk server failed to start within 2 minutes")

    yield BASE_URL

    # Cleanup
    print("Stopping trunk serve...")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


@pytest.fixture(scope="function")
def app_url(trunk_server):
    """Provide base URL for tests (function scoped)."""
    return trunk_server
