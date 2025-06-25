#!/usr/bin/env bash
set -euo pipefail

# MCP Setup Validation Script
# This script validates that all MCP server configuration files are properly set up

echo "🔍 Validating MCP Server Setup for GitHub Copilot..."

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Error: Not in the repository root directory"
    exit 1
fi

echo "✅ Repository root directory confirmed"

# Check VS Code configuration files
echo "🔍 Checking VS Code configuration..."

VSCODE_FILES=(
    ".vscode/extensions.json"
    ".vscode/launch.json"
    ".vscode/tasks.json"
    ".vscode/settings.example.json"
)

for file in "${VSCODE_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        echo "✅ $file exists"
        # Validate JSON syntax
        if jq empty "$file" 2>/dev/null; then
            echo "✅ $file has valid JSON syntax"
        else
            echo "❌ $file has invalid JSON syntax"
            exit 1
        fi
    else
        echo "❌ $file missing"
        exit 1
    fi
done

# Check MCP configuration files
echo "🔍 Checking MCP configuration files..."

MCP_FILES=(
    ".mcp-config.json"
    ".copilot-config.json"
    ".copilotignore"
)

for file in "${MCP_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        echo "✅ $file exists"
        if [[ "$file" == *.json ]]; then
            if jq empty "$file" 2>/dev/null; then
                echo "✅ $file has valid JSON syntax"
            else
                echo "❌ $file has invalid JSON syntax"
                exit 1
            fi
        fi
    else
        echo "❌ $file missing"
        exit 1
    fi
done

# Check development container configuration
echo "🔍 Checking development container configuration..."

if [[ -f ".devcontainer/devcontainer.json" ]]; then
    echo "✅ .devcontainer/devcontainer.json exists"
    if jq empty ".devcontainer/devcontainer.json" 2>/dev/null; then
        echo "✅ .devcontainer/devcontainer.json has valid JSON syntax"
    else
        echo "❌ .devcontainer/devcontainer.json has invalid JSON syntax"
        exit 1
    fi
else
    echo "❌ .devcontainer/devcontainer.json missing"
    exit 1
fi

# Check workspace file
echo "🔍 Checking VS Code workspace file..."

if [[ -f "geoclue-prometheus-exporter.code-workspace" ]]; then
    echo "✅ geoclue-prometheus-exporter.code-workspace exists"
    if jq empty "geoclue-prometheus-exporter.code-workspace" 2>/dev/null; then
        echo "✅ geoclue-prometheus-exporter.code-workspace has valid JSON syntax"
    else
        echo "❌ geoclue-prometheus-exporter.code-workspace has invalid JSON syntax"
        exit 1
    fi
else
    echo "❌ geoclue-prometheus-exporter.code-workspace missing"
    exit 1
fi

# Check documentation
echo "🔍 Checking documentation..."

if [[ -f "docs/MCP_SETUP.md" ]]; then
    echo "✅ docs/MCP_SETUP.md exists"
else
    echo "❌ docs/MCP_SETUP.md missing"
    exit 1
fi

# Check if README.md was updated
if grep -q "MCP Setup Guide" README.md; then
    echo "✅ README.md references MCP setup"
else
    echo "❌ README.md does not reference MCP setup"
    exit 1
fi

# Check gitignore
echo "🔍 Checking gitignore configuration..."

if grep -q ".vscode/settings.json" .gitignore; then
    echo "✅ .gitignore excludes user-specific VS Code settings"
else
    echo "❌ .gitignore does not exclude user-specific VS Code settings"
    exit 1
fi

# Test Rust build still works
echo "🔍 Testing Rust build..."

if cargo check --quiet; then
    echo "✅ Rust project builds successfully"
else
    echo "❌ Rust project build failed"
    exit 1
fi

# Test Rust tests still work
echo "🔍 Testing Rust tests..."

if cargo test --quiet; then
    echo "✅ Rust tests pass"
else
    echo "❌ Rust tests failed"
    exit 1
fi

# Check code formatting
echo "🔍 Checking code formatting..."

if cargo fmt --check; then
    echo "✅ Code is properly formatted"
else
    echo "❌ Code needs formatting (run 'cargo fmt')"
    exit 1
fi

echo ""
echo "🎉 All MCP setup validation checks passed!"
echo ""
echo "📚 Next steps:"
echo "1. Open the repository in VS Code"
echo "2. Install recommended extensions when prompted"
echo "3. Copy .vscode/settings.example.json to .vscode/settings.json and customize"
echo "4. Ensure GitHub Copilot extension is installed and authenticated"
echo "5. Check that environment variables are set (especially GITHUB_TOKEN for MCP GitHub server)"
echo ""
echo "📖 For detailed setup instructions, see: docs/MCP_SETUP.md"