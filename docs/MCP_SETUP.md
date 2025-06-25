# MCP Server Setup for GitHub Copilot

This document describes the Multi-Cloud Platform (MCP) server setup to enable optimal GitHub Copilot functionality for the geoclue-prometheus-exporter repository.

## Overview

The MCP setup includes:
- VS Code workspace configuration optimized for Rust and Nix development
- GitHub Copilot configuration with project-specific context
- Development container for consistent environment
- Security configurations and best practices

## Files Overview

### VS Code Configuration (`.vscode/`)

- **`extensions.json`**: Recommended extensions including GitHub Copilot, Rust Analyzer, and Nix IDE
- **`launch.json`**: Debug configurations for the main binary and tests
- **`tasks.json`**: Common development tasks (build, test, format, lint)
- **`settings.example.json`**: Example workspace settings (copy to `settings.json` and customize)

### MCP and AI Configuration

- **`.mcp-config.json`**: Model Context Protocol server configuration
- **`.copilot-config.json`**: GitHub Copilot specific settings and project context
- **`.copilotignore`**: Files to exclude from Copilot suggestions for security

### Development Environment

- **`.devcontainer/devcontainer.json`**: Development container configuration for Nix-based environment
- **`geoclue-prometheus-exporter.code-workspace`**: VS Code workspace file

## Setup Instructions

### 1. VS Code Setup

1. Open the repository in VS Code
2. Install recommended extensions when prompted
3. Copy `.vscode/settings.example.json` to `.vscode/settings.json` and customize as needed
4. Open the workspace file for the best experience: `geoclue-prometheus-exporter.code-workspace`

### 2. GitHub Copilot Configuration

The repository is pre-configured with:
- Project context including description, dependencies, and coding patterns
- Language-specific stops and completion preferences
- Security exclusions via `.copilotignore`

### 3. Development Container (Optional)

For a consistent development environment:

```bash
# With VS Code Dev Containers extension
code --folder-uri vscode-remote://dev-container+$(pwd)
```

Or use GitHub Codespaces for cloud development.

### 4. MCP Server Setup

The `.mcp-config.json` configures three MCP servers:

1. **Filesystem Server**: Provides file system access to AI assistants
2. **GitHub Server**: Enables GitHub API integration
3. **Rust Analyzer Server**: Provides language server capabilities

## Security Considerations

### Environment Variables

Ensure these environment variables are set securely:
- `GITHUB_TOKEN`: GitHub Personal Access Token for GitHub MCP server
- `RUST_LOG`: Logging level for Rust applications

### File Exclusions

The following files are excluded from AI suggestions:
- Sensitive files (`.env`, secrets, credentials)
- Binary files and build artifacts
- Large files that could impact performance
- Temporary and cache files

### Resource Limits

MCP servers are configured with:
- Maximum memory: 512MB
- Maximum CPU: 50%
- Maximum file size: 100MB
- Sandboxed execution

## Usage Tips

### GitHub Copilot

1. **Inline Suggestions**: Type and wait for suggestions to appear
2. **Chat**: Use Ctrl+I (Cmd+I on Mac) for inline chat
3. **Explain Code**: Select code and ask Copilot to explain it
4. **Generate Tests**: Ask Copilot to generate tests for your functions

### Rust Development

The configuration is optimized for:
- Async/await patterns with tokio
- Error handling with anyhow
- D-Bus communication with zbus
- Prometheus metrics with the metrics crate

### Project Patterns

Copilot is configured to understand these project patterns:
- Use `anyhow::Result` for error handling
- Use structured logging with key-value pairs
- Keep functions focused and single-purpose
- Follow Rust naming conventions

## Troubleshooting

### Common Issues

1. **Rust Analyzer not working**: Check that `rust-analyzer` is installed and in PATH
2. **Copilot not suggesting**: Verify GitHub Copilot extension is installed and authenticated
3. **MCP servers not starting**: Check environment variables and permissions
4. **Nix commands failing in container**: Ensure container has necessary privileges

### Debug Information

Check the VS Code output panels:
- Rust Analyzer
- GitHub Copilot
- Extension Host

## Security Best Practices

1. **Never commit sensitive data**: Use `.gitignore` and `.copilotignore`
2. **Review AI suggestions**: Always review generated code before accepting
3. **Limit scope**: Use workspace-specific settings rather than global ones
4. **Regular updates**: Keep extensions and tools updated
5. **Environment isolation**: Use development containers when possible

## Contributing

When contributing to this repository:

1. Follow the existing patterns and conventions
2. Use the provided development tools and configurations
3. Test your changes with both local and container environments
4. Ensure security best practices are followed
5. Update documentation when adding new MCP configurations

## Further Reading

- [GitHub Copilot Documentation](https://docs.github.com/en/copilot)
- [Model Context Protocol Specification](https://spec.modelcontextprotocol.io/)
- [VS Code Remote Development](https://code.visualstudio.com/docs/remote/remote-overview)
- [Rust Analyzer User Manual](https://rust-analyzer.github.io/manual.html)