# GeoClue2 Prometheus Exporter

A Prometheus metrics exporter for GeoClue2 geolocation data on Linux systems.

## Development Setup

For an optimal development experience with GitHub Copilot and AI assistants, see the [MCP Setup Guide](docs/MCP_SETUP.md).

## Features

- Exports geolocation metrics (latitude, longitude, accuracy, altitude, etc.) as Prometheus metrics
- Configurable minimum accuracy level
- Configurable metrics endpoint
- Easily integrates with Grafana Alloy for laptop metrics

## Dependencies

This project uses:

- **anyhow 1.0.75**: For flexible error handling
- **chrono 0.4.31**: For date and time functionality
- **clap 4.4.6**: For command line argument parsing
- **futures-util 0.3.28**: For async/await utilities
- **metrics 0.22.0**: For metrics collection and processing
- **metrics-exporter-prometheus 0.13.0**: For exposing metrics in Prometheus format
- **metrics-process 2.4.0**: For collecting process metrics
- **tokio 1.36.0**: For asynchronous runtime
- **zbus 4.1.2**: For D-Bus communication with GeoClue2

## Installation

### Using Nix/NixOS

Add this flake to your NixOS configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    geoclue-prometheus-exporter = {
      url = "github:shift/geoclue-prometheus-exporter";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  
  outputs = { self, nixpkgs, geoclue-prometheus-exporter, ... }: {
    nixosConfigurations.myHost = nixpkgs.lib.nixosSystem {
      # ...
      modules = [
        # ...
        geoclue-prometheus-exporter.nixosModules.default
        
        # Configure the service
        ({ ... }: {
          services.geoclue-prometheus-exporter = {
            enable = true;
            bind = "127.0.0.1";  # Default
            port = 9090;         # Default
            openFirewall = false; # Default
          };
        })
      ];
    };
  };
}
