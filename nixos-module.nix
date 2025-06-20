{ lib ? (import <nixpkgs> {}).lib
, config
, pkgs
, package
, ... }:

with lib;

let
  cfg = config.services.geoclue-prometheus-exporter;
  
  # Use simple condition checks to avoid circular references
  hasAlloyModule = hasAttrByPath ["services" "grafana-alloy-laptop"] config;
  alloyEnabled = hasAlloyModule && config.services.grafana-alloy-laptop.enable or false;
  hasScrapeConfigs = hasAlloyModule && hasAttrByPath ["services" "grafana-alloy-laptop" "scrapeConfigs"] config;
  
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
    registerWithAlloy = mkOption {
      type = types.bool;
      default = alloyEnabled;
      description = "Register this exporter with Grafana Alloy if it's enabled.";
    };
  };

  # A single config block that is applied if the service is enabled
  config = mkIf cfg.enable {
    # Group all services-related config together to avoid multiple definitions
    services = {
      # Main geoclue service configuration
      geoclue2.appConfig."geoclue-prometheus-exporter" = {
        isAllowed = true;
        isSystem = true;
      };

      # Conditionally configure Grafana Alloy integration
      grafana-alloy-laptop = mkIf (cfg.registerWithAlloy && alloyEnabled && hasScrapeConfigs) {
        scrapeConfigs = [
          {
            job_name = "geoclue";
            targets = [ target ];
            metrics_path = "/metrics";
            scrape_interval = "30s"; # Geolocation doesn't change frequently
            scrape_timeout = "10s";
            labels = {
              service = "geoclue";
              instance = config.networking.hostName;
            };
          }
        ];
      };
    };

    # Define the systemd service
    systemd.services = {
      # Our main service
      geoclue-prometheus-exporter = {
        description = "Geoclue to Prometheus Exporter";
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          ExecStart = "${package}/bin/geoclue-prometheus-exporter --bind-address ${cfg.bind} --metrics-port ${toString cfg.port}";
          Restart = "on-failure";
          RestartSec = "5s";
          User = "nobody";
          Group = "nogroup";
        };
      };
      
      # Alloy service dependencies (only if integration is enabled)
      grafana-alloy = mkIf (cfg.registerWithAlloy && alloyEnabled && hasScrapeConfigs) {
        after = [ "geoclue-prometheus-exporter.service" ];
        requires = [ "geoclue-prometheus-exporter.service" ];
      };
    };

    # Conditionally open the firewall
    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = [ cfg.port ];
    };
  };
}
