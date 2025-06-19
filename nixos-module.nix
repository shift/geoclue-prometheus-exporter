{ config, lib, pkgs, ... }:

let
  cfg = config.services.geoclue-prometheus-exporter;
in
{
  # Define the options for the service
  options.services.geoclue-prometheus-exporter = {
    enable = lib.mkEnableOption "Enable the Geoclue to Prometheus exporter";
    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall for Prometheus to scrape metrics on port 9090.";
    };
    bind = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Address to bind the metrics server to.";
    };
    port = lib.mkOption {
      type = lib.types.int;
      default = 9090;
      description = "Port to bind the metrics server to.";
    };
  };

  # A single config block that is applied if the service is enabled
  config = lib.mkIf cfg.enable {
    # Allow the exporter to access Geoclue2
    services.geoclue2.appConfig."geoclue-prometheus-exporter" = {
      isAllowed = true;
      isSystem = true;
    };

    # Define the systemd service
    systemd.services.geoclue-prometheus-exporter = {
      description = "Geoclue to Prometheus Exporter";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.geoclue-prometheus-exporter}/bin/geoclue-prometheus-exporter --bind-address ${cfg.bind} --metrics-port ${toString cfg.port}";
        Restart = "on-failure";
        RestartSec = "5s";
        User = "nobody";
        Group = "nogroup";
      };
    };

    # Conditionally open the firewall
    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts = [ cfg.port ];
    };
  };
}
