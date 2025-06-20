{ lib ? (import <nixpkgs> {}).lib
, config
, pkgs
, ... }:

with lib;

let
  # Only proceed if both services are enabled
  gcCfg = config.services.geoclue-prometheus-exporter;
  hasAlloy = hasAttrByPath ["services" "grafana-alloy-laptop"] config;
  
  # Target to use for scraping based on bind address
  isLocalhost = gcCfg.bind == "127.0.0.1" || gcCfg.bind == "localhost";
  target = if isLocalhost then "localhost:${toString gcCfg.port}" else "${gcCfg.bind}:${toString gcCfg.port}";
in {
  # Only add the integration if all conditions are met
  config = mkIf (gcCfg.enable && hasAlloy && config.services.grafana-alloy-laptop.enable) {
    # Check if scrapeConfigs exist and add the Geoclue target
    services.grafana-alloy-laptop = mkIf (hasAttrByPath ["services" "grafana-alloy-laptop" "scrapeConfigs"] config) {
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
    
    # Add service ordering
    systemd.services.grafana-alloy = {
      after = [ "geoclue-prometheus-exporter.service" ];
      requires = [ "geoclue-prometheus-exporter.service" ];
    };
  };
}
