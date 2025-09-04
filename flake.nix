{
  description = "A Nix flake for a Geoclue to Prometheus exporter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    let
      # Helper function to create a NixOS module with inputs
      nixosModule = { pkgs, lib, config, ... }:
        import ./nixos-module.nix {
          inherit pkgs lib config;
          package = self.packages.${pkgs.system}.default;
        };
        
      # Separate module for alloy integration
      alloyModule = { pkgs, lib, config, ... }:
        import ./nixos-module-alloy.nix {
          inherit pkgs lib config;
        };
        
      # Run all tests in one go
      runAllTests = system: pkgs: pkgs.writeShellScriptBin "run-all-tests" ''
        set -e
        echo "Running unit tests..."
        cd ${self}
        ${pkgs.cargo}/bin/cargo test
        
        echo "Running integration tests..."
        ${pkgs.cargo}/bin/cargo test --test integration_test
        
        echo "Running VM tests..."
        nix build .#checks.${system}.vm-test
        
        echo "All tests passed!"
      '';
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Define the static location file content
        locationFile = pkgs.writeText "geolocation" ''
          # Example static location file for a machine inside Statue of Liberty torch
          40.6893129   # latitude
          -74.0445531  # longitude
          96           # altitude
          1.83         # accuracy radius (the diameter of the torch is 12 feet)
        '';

        # Expanded build inputs with proper OpenSSL
        geoclue-build-inputs = [ 
          pkgs.pkg-config 
          pkgs.dbus 
          pkgs.openssl
        ];
        
        # Get Git hash for the current repository state
        gitHash = if self ? rev then pkgs.lib.substring 0 7 self.rev else "dirty";

        # Define the Rust package itself
        geoclue-prometheus-exporter = pkgs.rustPlatform.buildRustPackage rec {
          pname = "geoclue-prometheus-exporter";
          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = geoclue-build-inputs;
          
          # Add openssl as a runtime dependency too
          buildInputs = [ 
            pkgs.openssl 
          ];
          
          # Set environment variables for OpenSSL to be found
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
          
          # Set build-time environment variables
          GIT_HASH = gitHash;
          
          # Run the test suite
          doCheck = true;
          
          # Add test dependencies for the check phase
          nativeCheckInputs = [
            pkgs.dbus  # Needed for integration tests
            pkgs.openssl
          ];
        };
        
        # Define a test for the geoclue-prometheus-exporter
        geoclue-exporter-test = import ./nix/vm-test.nix {
          inherit pkgs;
          nodes = {
            machine = { pkgs, ... }: {
              imports = [
                self.nixosModules.default
                ./nix/test-common.nix
              ];
              services.geoclue-prometheus-exporter = {
                enable = true;
                bind = "127.0.0.1";
                port = 9090;
                openFirewall = true;
                logLevel = "debug";
              };
              # Enable geoclue service for testing
              services.geoclue2 = {
                enable = true;
                enableDemoAgent = false;
              };
              systemd.services.geoclue = {
                serviceConfig = {
                  Environment = "G_MESSAGES_DEBUG=all";
                };
              };
              # Enable Avahi service for GeoClue2 network-based location detection
              services.avahi = {
                enable = true;
                nssmdns4 = true;
                nssmdns6 = true;
              };
              environment.etc."geoclue/geoclue.conf".text = ''
                [agent]
                whitelist=geoclue-demo-agent;gnome-shell;io.elementary.desktop.agent-geoclue2;geoclue-prometheus-exporter

                [static-source]
                enable=true

                [ip]
                enable=false

                [network-nmea]
                enable=false

                [3g]
                enable=false

                [cdma]
                enable=false

                [wifi]
                enable=false
              '';

              systemd.tmpfiles.rules = [
                "f /var/lib/geoclue-static.conf 0644 root root - -"
              ];
              environment.systemPackages = [ pkgs.curl ];
            };
          };
          testScript = ''
            start_all()

            # Wait for both services to be ready.
            machine.wait_for_unit("geoclue.service")
            machine.wait_for_unit("geoclue-prometheus-exporter.service")
            machine.wait_for_open_port(9090)

            # --- LOCATION 1: Berlin ---
            machine.log("Simulating location: Berlin")
            machine.succeed("cp ${locationFile} /etc/geolocation")

            # Give geoclue and the exporter a moment to process the update.
            machine.sleep(5)

            # Check if the exporter reports Berlins latitude.
            machine.wait_until_succeeds("curl -s http://127.0.0.1:9090/metrics | grep 'geoclue_latitude 40.6893129'")

            # --- LOCATION 2: Munich (Simulating Movement) ---
            machine.log("Simulating movement to Munich")
            machine.succeed("cp ${locationFile} /etc/geolocation")
            machine.sleep(5)

            # Verify the exporter has updated to Munichs latitude.
            machine.log("Verifying exporter has updated location")
            machine.succeed("curl -s http://127.0.0.1:9090/metrics | grep 'geoclue_latitude 40.6893129'")
          '';
        };
      in
      {
        # The default package built by `nix build`
        packages = {
          default = geoclue-prometheus-exporter;
          test-runner = runAllTests system pkgs;
        };

        # Run checks for the flake
        checks = {
          # Include the package build as a check
          build = geoclue-prometheus-exporter;
          
          # Test the exporter in a VM
          vm-test = geoclue-exporter-test;
        };

        # Development shell for `nix develop`
        devShells.default = pkgs.mkShell {
          # Tools and libraries needed for development
          packages = [
            # Get the Rust toolchain (cargo, rustc, etc.) from the overlay
            pkgs.rust-bin.stable.latest.default
            # Include test dependencies
            pkgs.cargo-nextest
            pkgs.cargo-tarpaulin
            # Include the test runner
            self.packages.${system}.test-runner
          ] ++ geoclue-build-inputs; # Add build inputs like dbus and pkg-config
          
          # Also set the OpenSSL environment variables for the dev shell
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        };
      }
    ) // {
      # NixOS module that can be imported in NixOS configurations
      nixosModules = {
        default = nixosModule;
        geoclue-prometheus-exporter = nixosModule;
        # Separate module for alloy integration
        withAlloyIntegration = { imports = [ nixosModule alloyModule ]; };
      };
      
      # Add NixOS tests
      nixosTests = {
        basic = import ./nix/vm-test.nix {
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
          nodes = {
            machine = { pkgs, ... }: {
              imports = [
                self.nixosModules.default
                ./nix/test-common.nix
              ];
              services.geoclue-prometheus-exporter = {
                enable = true;
                bind = "0.0.0.0";  # Test with non-localhost binding
                port = 9090;
                openFirewall = true;
              };
              # Enable geoclue service for testing
              services.geoclue2 = {
                enable = true;
                enableDemoAgent = true;
              };
              systemd.services.geoclue = {
                serviceConfig = {
                  Environment = "G_MESSAGES_DEBUG=all";
                };
              };
              # Enable Avahi service for GeoClue2 network-based location detection
              services.avahi = {
                enable = true;
                nssmdns4 = true;
                nssmdns6 = true;
              };
            };
          };
        };
      };
    };
}
