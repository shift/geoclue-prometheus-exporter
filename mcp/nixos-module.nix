# MCP servers integration for Nix
{ lib, pkgs, config, ... }:

with lib;

let
  cfg = config.services.geoclue-prometheus-exporter-mcp;
  
  mcpServersPackage = pkgs.stdenv.mkDerivation {
    pname = "geoclue-prometheus-exporter-mcp";
    version = "0.5.0";
    
    src = ./.;
    
    buildInputs = [ pkgs.nodejs ];
    nativeBuildInputs = [ pkgs.nodejs ];
    
    buildPhase = ''
      npm install
    '';
    
    installPhase = ''
      mkdir -p $out/lib/mcp
      cp -r * $out/lib/mcp/
      
      # Create wrapper scripts
      mkdir -p $out/bin
      cat > $out/bin/geoclue-mcp-metrics <<EOF
#!/bin/sh
exec ${pkgs.nodejs}/bin/node $out/lib/mcp/servers/metrics-server.js "\$@"
EOF
      
      cat > $out/bin/geoclue-mcp-config <<EOF
#!/bin/sh
exec ${pkgs.nodejs}/bin/node $out/lib/mcp/servers/config-server.js "\$@"
EOF
      
      cat > $out/bin/geoclue-mcp-monitoring <<EOF
#!/bin/sh
exec ${pkgs.nodejs}/bin/node $out/lib/mcp/servers/monitoring-server.js "\$@"
EOF
      
      chmod +x $out/bin/*
    '';
  };
in {
  options.services.geoclue-prometheus-exporter-mcp = {
    enable = mkEnableOption "Enable GeoClue Prometheus Exporter MCP servers";
    
    package = mkOption {
      type = types.package;
      default = mcpServersPackage;
      description = "Package containing the MCP servers";
    };
    
    servers = mkOption {
      type = types.submodule {
        options = {
          metrics = mkEnableOption "Enable metrics MCP server";
          config = mkEnableOption "Enable configuration MCP server";
          monitoring = mkEnableOption "Enable monitoring MCP server";
        };
      };
      default = {
        metrics = true;
        config = true;
        monitoring = true;
      };
      description = "Which MCP servers to enable";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    
    # Optionally, you could run the MCP servers as systemd services
    # though typically they are started by MCP clients on demand
    
    # systemd.services = mkMerge [
    #   (mkIf cfg.servers.metrics {
    #     geoclue-mcp-metrics = {
    #       description = "GeoClue MCP Metrics Server";
    #       serviceConfig = {
    #         ExecStart = "${cfg.package}/bin/geoclue-mcp-metrics";
    #         Restart = "on-failure";
    #       };
    #     };
    #   })
    # ];
  };
}