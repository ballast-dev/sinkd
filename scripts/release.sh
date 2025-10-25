#!/usr/bin/env bash
# Release build script for sinkd
# Builds for all platforms and creates release packages

set -euo pipefail

VERSION=${1:-$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}
RELEASE_DIR="releases/v${VERSION}"

echo "🚀 Building sinkd release v${VERSION}"
echo "=================================="

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean
rm -rf dist/ releases/

# Create release directory
mkdir -p "${RELEASE_DIR}"

# Build for all platforms
echo "🔨 Building for all platforms..."
just build-all

# Create packages
echo "📦 Creating release packages..."
just package-all

# Move packages to release directory
mv dist/* "${RELEASE_DIR}/"

# Generate checksums
echo "🔐 Generating checksums..."
cd "${RELEASE_DIR}"
sha256sum * > checksums.sha256
cd - > /dev/null

# Generate release notes template
cat > "${RELEASE_DIR}/RELEASE_NOTES.md" << EOF
# sinkd v${VERSION}

## What's New

- [Add new features here]
- [Add bug fixes here]
- [Add improvements here]

## Downloads

### Windows
- **x64**: \`sinkd-windows-x64.zip\`
- **x86**: \`sinkd-windows-x86.zip\`

### Linux (Static Binaries)
- **x64**: \`sinkd-linux-x64.tar.gz\`
- **ARM64**: \`sinkd-linux-arm64.tar.gz\`

### macOS
- **Intel**: \`sinkd-macos-intel.tar.gz\`
- **Apple Silicon**: \`sinkd-macos-arm64.tar.gz\`

## Installation

### Windows
1. Download the appropriate zip file
2. Extract \`sinkd.exe\`
3. Run from command line or place in PATH

### Linux
1. Download the appropriate tar.gz file
2. Extract: \`tar -xzf sinkd-linux-*.tar.gz\`
3. Make executable: \`chmod +x sinkd\`
4. Move to PATH: \`sudo mv sinkd /usr/local/bin/\`

### macOS
1. Download the appropriate tar.gz file
2. Extract: \`tar -xzf sinkd-macos-*.tar.gz\`
3. Make executable: \`chmod +x sinkd\`
4. Move to PATH: \`sudo mv sinkd /usr/local/bin/\`
5. Allow execution: \`sudo xattr -d com.apple.quarantine /usr/local/bin/sinkd\`

## Quick Start

### Server Setup
\`\`\`bash
# Start server
sinkd server start

# Check status
sinkd server status
\`\`\`

### Client Setup
\`\`\`bash
# Configure paths in ~/.config/sinkd.conf
# Start client
sinkd client start

# Check status  
sinkd client status
\`\`\`

## Checksums
\`\`\`
$(cat checksums.sha256)
\`\`\`

---
Built with ❤️ using Rust and cross-compilation
EOF

# Summary
echo ""
echo "✅ Release v${VERSION} built successfully!"
echo "📁 Files created in: ${RELEASE_DIR}/"
echo ""
echo "📋 Release contents:"
ls -la "${RELEASE_DIR}/"
echo ""
echo "🔗 Next steps:"
echo "  1. Review release notes: ${RELEASE_DIR}/RELEASE_NOTES.md"
echo "  2. Test binaries on target platforms"
echo "  3. Create git tag: git tag v${VERSION}"
echo "  4. Push release: git push origin v${VERSION}"
echo "  5. Upload to GitHub/releases"
echo ""
