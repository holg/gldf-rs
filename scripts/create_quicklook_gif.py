#!/usr/bin/env python3
"""
Create a GIF demonstrating the Eulumdat Quick Look feature in Finder.

Usage:
1. Start screen recording: Cmd+Shift+5 → Record Selected Portion
2. Run this script
3. Stop recording
4. Convert with ffmpeg
"""

import subprocess
import time
import os

# Configuration
FOLDER_PATH = os.path.expanduser("~/Documents/develeop/Lichtdaten")
FILE_NAME = "road_luminaire.ldt"

def run_applescript(script: str):
    """Run an AppleScript."""
    subprocess.run(["osascript", "-e", script], check=True)

def main():
    print("🎬 Quick Look Demo Script")
    print("=" * 40)
    print(f"📁 Folder: {FOLDER_PATH}")
    print(f"📄 File: {FILE_NAME}")
    print()
    print("📹 START RECORDING FIRST:")
    print("   Cmd+Shift+5 → 'Record Selected Portion'")
    print("   Select the Finder window area")
    print()
    input("Press Enter when recording is running...")
    print()

    time.sleep(0.5)

    # Step 1: Open Finder and navigate to folder
    print("1. Opening Finder...")
    run_applescript(f'''
        tell application "Finder"
            activate
            open folder POSIX file "{FOLDER_PATH}"
        end tell
    ''')
    time.sleep(1.5)

    # Step 2: Select the file
    print(f"2. Selecting {FILE_NAME}...")
    run_applescript(f'''
        tell application "Finder"
            activate
            select file "{FILE_NAME}" of front window
        end tell
    ''')
    time.sleep(1.2)

    # Step 3: Trigger Quick Look with spacebar
    print("3. Opening Quick Look...")
    run_applescript('''
        tell application "System Events"
            keystroke space
        end tell
    ''')
    time.sleep(2.5)

    # Step 4: Close Quick Look
    print("4. Closing Quick Look...")
    run_applescript('''
        tell application "System Events"
            keystroke space
        end tell
    ''')
    time.sleep(0.8)

    print()
    print("✅ Done! Stop recording: Click Stop in menu bar")
    print()
    print("Convert to GIF:")
    print('  ffmpeg -i ~/Desktop/Screen\\ Recording*.mov -vf "fps=12,scale=640:-1:flags=lanczos" quicklook_demo.gif')

if __name__ == "__main__":
    main()