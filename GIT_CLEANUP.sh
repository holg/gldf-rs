#!/bin/bash
#
# Git Cleanup Script
# Removes files that should be gitignored from git history
#
# Usage: ./GIT_CLEANUP.sh

set -e  # Exit on error

echo "=== Git Cleanup Script ==="
echo "This will remove user-specific files from git tracking"
echo ""

# 1. Remove .DS_Store files (macOS metadata)
echo "Step 1: Removing .DS_Store files..."
find . -name .DS_Store -print0 2>/dev/null | xargs -0 git rm -f --ignore-unmatch 2>/dev/null || true

# 2. Remove xcuserstate files (Xcode user state)
echo "Step 2: Removing xcuserstate files..."
find . -name "*.xcuserstate" -print0 2>/dev/null | xargs -0 git rm -f --ignore-unmatch 2>/dev/null || true

# 3. Remove xcuserdata directories (Xcode user data)
echo "Step 3: Removing xcuserdata directories..."
git rm -rf --ignore-unmatch **/xcuserdata 2>/dev/null || true
git rm -rf --ignore-unmatch "**/*.xcuserdatad" 2>/dev/null || true

# 4. Remove any .env files if accidentally committed
echo "Step 4: Checking for .env files..."
git rm -f --ignore-unmatch .env 2>/dev/null || true
git rm -f --ignore-unmatch .env.local 2>/dev/null || true

# 5. Remove local xcconfig overrides
echo "Step 5: Removing local xcconfig overrides..."
git rm -f --ignore-unmatch Configuration/Development.xcconfig 2>/dev/null || true
git rm -f --ignore-unmatch GldfApp/Configuration/Development.xcconfig 2>/dev/null || true

# 6. Remove provisioning profiles and certificates
echo "Step 6: Removing provisioning profiles and certificates..."
find . -name "*.mobileprovision" -print0 2>/dev/null | xargs -0 git rm -f --ignore-unmatch 2>/dev/null || true
find . -name "*.provisionprofile" -print0 2>/dev/null | xargs -0 git rm -f --ignore-unmatch 2>/dev/null || true
find . -name "*.p12" -print0 2>/dev/null | xargs -0 git rm -f --ignore-unmatch 2>/dev/null || true

echo ""
echo "=== Cleanup Complete ==="
echo "Files removed from git tracking (but still on disk)"
echo ""
echo "Next steps:"
echo "1. Review changes: git status"
echo "2. Commit: git commit -m '[FIX] Remove user-specific files from git tracking'"
echo "3. Push: git push origin main"
echo ""
echo "Note: These files are now in .gitignore and won't be tracked in future commits"
