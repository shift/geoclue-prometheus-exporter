{
  description = "A Nix flake for a Geoclue to Prometheus exporter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Define the build inputs needed for the Rust package
        geoclue-build-inputs = [ pkgs.pkg-config pkgs.dbus pkgs.openssl ];
        
        # Get Git hash for the current repository state
        gitHash = if self ? rev then pkgs.lib.substring 0 7 self.rev else "dirty";

        # Define the Rust package itself
        geoclue-prometheus-exporter = pkgs.rustPlatform.buildRustPackage rec {
          pname = "geoclue-prometheus-exporter";
          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = geoclue-build-inputs;
          
          # Set build-time environment variables
          GIT_HASH = gitHash;
        };
      in
      {
        # The default package built by `nix build`
        packages.default = geoclue-prometheus-exporter;

        # The NixOS module to configure the service
        nixosModules.default = { config, lib, pkgs, ... }: {

          # Define the options for the service
          options.services.geoclue-prometheus-exporter = {
            enable = lib.mkEnableOption "Enable the Geoclue exporter";
            openFirewall = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Open firewall for Prometheus to scrape metrics on port configured port.";
            };
            bind = lib.mkOption {
              type = lib.types.str;
              default = "127.0.0.1";
              description = "Address to bind the metrics server to.";
            };
            port = lib.mkOption {
              type = lib.types.int;
              default = 9990;
              description = "Port to bind the metrics server to.";
            };
          };

          # A single config block that is applied if the service is enabled
          config = lib.mkIf config.services.geoclue-prometheus-exporter.enable {
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
                ExecStart = "${geoclue-prometheus-exporter}/bin/geoclue-prometheus-exporter --bind-address ${config.services.geoclue-prometheus-exporter.bind} --metrics-port ${toString config.services.geoclue-prometheus-exporter.port}";
                Restart = "on-failure";
                RestartSec = "5s";
                User = "nobody";
                Group = "nogroup";
              };
            };

            # Conditionally open the firewall
            networking.firewall = lib.mkIf config.services.geoclue-prometheus-exporter.openFirewall {
              allowedTCPPorts = [ config.services.geoclue-prometheus-exporter.port ];
            };
          };
        };

        # Development shell for `nix develop`
        devShells.default = pkgs.mkShell {
          # Tools and libraries needed for development
          packages = [
            # Get the Rust toolchain (cargo, rustc, etc.) from the overlay
            pkgs.rust-bin.stable.latest.default
          ] ++ geoclue-build-inputs; # Add build inputs like dbus and pkg-config
        };
      });
}
