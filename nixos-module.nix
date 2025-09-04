{ lib ? (import <nixpkgs> {}).lib
, config
, pkgs
, package
, ... }:

with lib;

let
  cfg = config.services.geoclue-prometheus-exporter;
  
  # Target to use for scraping based on bind address
  isLocalhost = cfg.bind == "127.0.0.1" || cfg.bind == "localhost";
  target = if isLocalhost then "localhost:${toString cfg.port}" else "${cfg.bind}:${toString cfg.port}";
in {
  # Define the options for the service
  options.services.geoclue-prometheus-exporter = {
    enable = mkEnableOption "Enable the Geoclue to Prometheus exporter";
    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open firewall for Prometheus to scrape metrics on port 9090.";
    };
    bind = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address to bind the metrics server to.";
    };
    port = mkOption {
      type = types.int;
      default = 9090;
      description = "Port to bind the metrics server to.";
    };
    logLevel = mkOption {
      type = types.enum [ "debug" "info" "warn" "error" ];
      default = "info";
      description = "The log level for the exporter.";
    };
  };

  config = mkIf cfg.enable {
    security.polkit.extraConfig = ''
      polkit.addRule(function(action, subject) {
        if (action.id == "org.freedesktop.location.geoclue" &&
            subject.user == "geoclue-prometheus-exporter") {
          return polkit.Result.YES;
        }
      });
    '';

    users.users.geoclue-prometheus-exporter = {
      isSystemUser = true;
      group = "geoclue-prometheus-exporter";
    };

    users.groups.geoclue-prometheus-exporter = {};

    # Group all services-related config together to avoid multiple definitions
    services = {
      # Main geoclue service configuration
      geoclue2.appConfig."geoclue-prometheus-exporter" = {
        isAllowed = true;
        isSystem = true;
      };
    };

    systemd.services = {
      geoclue-prometheus-exporter = {
        description = "Geoclue to Prometheus Exporter";
        wantedBy = [ "multi-user.target" ];
        wants = [ "network-online.target" ];
        after = [ "network-online.target" ];
        serviceConfig = {
          Type = "exec";
          ExecStart = "${package}/bin/geoclue-prometheus-exporter --bind-address ${cfg.bind} --metrics-port ${toString cfg.port} --log-level ${cfg.logLevel}";
          Restart = "on-failure";
          RestartSec = "5s";
          User = "geoclue-prometheus-exporter";
          Group = "geoclue-prometheus-exporter";
        };
      };
    };

    # Conditionally open the firewall
    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = [ cfg.port ];
    };
  };
}
