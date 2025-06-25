# MCP Servers for GeoClue Prometheus Exporter

This directory contains Model Context Protocol (MCP) servers that provide AI agents with access to the GeoClue Prometheus Exporter service functionality.

## Overview

The MCP servers enable AI agents to:
- Query Prometheus metrics from the geoclue exporter
- Monitor service health and performance
- Manage configuration settings
- Access geolocation data in structured formats

## Available Servers

### 1. Metrics Server (`metrics-server.js`)
Provides access to Prometheus metrics endpoint and geolocation data.

**Capabilities:**
- Fetch raw Prometheus metrics
- Extract and parse geolocation metrics (latitude, longitude, accuracy, altitude)
- Health checks for the metrics endpoint
- Structured access to location data

**Tools:**
- `get_metrics` - Fetch all Prometheus metrics
- `get_geolocation_metrics` - Get filtered geolocation metrics with parsed data
- `check_service_health` - Health check for the metrics endpoint

**Resources:**
- `geoclue://metrics/current` - Current Prometheus metrics
- `geoclue://location/current` - Parsed current location data

### 2. Configuration Server (`config-server.js`)
Manages service configuration and settings.

**Capabilities:**
- Read current service configurations (systemd, NixOS, MCP)
- Update MCP server configurations
- Validate configuration files
- Get service command-line arguments and status

**Tools:**
- `get_service_config` - Get current service configuration
- `update_mcp_config` - Update MCP server configuration
- `get_service_status` - Get systemd service status
- `get_service_args` - Get current service arguments
- `validate_config` - Validate configuration files

**Resources:**
- `geoclue://config/mcp` - MCP server configuration
- `geoclue://config/systemd` - Systemd service configuration
- `geoclue://config/nixos` - NixOS module configuration

### 3. Monitoring Server (`monitoring-server.js`)
Provides comprehensive health checks and service monitoring.

**Capabilities:**
- Comprehensive health checks
- System resource monitoring
- Network connectivity testing
- Log analysis
- Dependency checking (GeoClue service)

**Tools:**
- `health_check` - Comprehensive health check
- `service_status` - Detailed systemd service status
- `system_resources` - System resource usage monitoring
- `network_connectivity` - Network connectivity testing
- `log_analysis` - Service log analysis
- `geoclue_dependency_check` - Check GeoClue service dependencies

**Resources:**
- `geoclue://monitoring/dashboard` - Complete monitoring dashboard
- `geoclue://monitoring/alerts` - Current service alerts
- `geoclue://monitoring/performance` - Performance metrics

## Installation

1. **Install Node.js dependencies:**
   ```bash
   cd mcp
   npm install
   ```

2. **Make servers executable:**
   ```bash
   chmod +x servers/*.js
   ```

## Usage

### Direct Server Execution
You can run each server directly for testing:

```bash
# Start metrics server
node servers/metrics-server.js

# Start configuration server  
node servers/config-server.js

# Start monitoring server
node servers/monitoring-server.js
```

### MCP Client Configuration
Add to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "geoclue-metrics": {
      "command": "node",
      "args": ["/path/to/geoclue-prometheus-exporter/mcp/servers/metrics-server.js"],
      "description": "GeoClue metrics access"
    },
    "geoclue-config": {
      "command": "node", 
      "args": ["/path/to/geoclue-prometheus-exporter/mcp/servers/config-server.js"],
      "description": "GeoClue configuration management"
    },
    "geoclue-monitoring": {
      "command": "node",
      "args": ["/path/to/geoclue-prometheus-exporter/mcp/servers/monitoring-server.js"],
      "description": "GeoClue service monitoring"
    }
  }
}
```

## Example Usage with AI Agents

### Query Current Location
```
Agent: "What's the current location from the geoclue exporter?"
→ Uses metrics server to fetch and parse geolocation metrics
→ Returns structured location data with coordinates and accuracy
```

### Health Check
```
Agent: "Is the geoclue service healthy?"
→ Uses monitoring server to run comprehensive health checks
→ Returns status of service, metrics endpoint, dependencies, etc.
```

### Configuration Management
```
Agent: "Show me the current service configuration"
→ Uses config server to read systemd and NixOS configurations
→ Returns formatted configuration details
```

## Configuration

The MCP servers can be configured via the `config.json` file:

```json
{
  "defaults": {
    "host": "127.0.0.1",
    "port": 9090,
    "metricsPath": "/metrics",
    "serviceTimeout": 30000
  }
}
```

## Requirements

- Node.js 18.0.0 or higher
- Access to the GeoClue Prometheus Exporter service
- Appropriate system permissions for:
  - Reading systemd service status
  - Accessing process information
  - Reading configuration files
  - Network access to metrics endpoint

## Security Considerations

- The servers run with the permissions of the user executing them
- Configuration file modifications require write permissions
- System monitoring requires appropriate process and system access
- Network operations are limited to the configured endpoints

## Troubleshooting

### Common Issues

1. **"Connection refused" errors:**
   - Verify the GeoClue Prometheus Exporter service is running
   - Check that the service is listening on the expected host/port
   - Verify firewall settings if connecting remotely

2. **"Permission denied" errors:**
   - Ensure appropriate file system permissions
   - Check systemctl permissions for service status queries
   - Verify process monitoring permissions

3. **"Module not found" errors:**
   - Run `npm install` in the mcp directory
   - Verify Node.js version compatibility
   - Check that @modelcontextprotocol/sdk is installed

4. **Configuration validation failures:**
   - Verify JSON syntax in configuration files
   - Check file paths and permissions
   - Ensure required configuration fields are present

## Development

To extend or modify the MCP servers:

1. **Adding new tools:** Add tool definitions to the `ListToolsRequestSchema` handler
2. **Adding new resources:** Add resource definitions to the `ListResourcesRequestSchema` handler  
3. **Error handling:** All tools should catch errors and return appropriate error responses
4. **Testing:** Test with a MCP client or by running servers directly and sending JSON-RPC messages

## Integration with NixOS

For NixOS deployments, consider adding the MCP servers to your system configuration:

```nix
# In your NixOS configuration
systemd.services.geoclue-mcp-servers = {
  description = "GeoClue MCP Servers";
  wantedBy = [ "multi-user.target" ];
  after = [ "geoclue-prometheus-exporter.service" ];
  serviceConfig = {
    Type = "forking";
    ExecStart = "${pkgs.nodejs}/bin/node /path/to/mcp/servers/metrics-server.js";
    User = "nobody";
    Group = "nogroup";
  };
};
```