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
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

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
              imports = [ self.nixosModules.default ];
              services.geoclue-prometheus-exporter = {
                enable = true;
                bind = "127.0.0.1";
                port = 9090;
                openFirewall = true;
              };
              # Enable geoclue service for testing
              services.geoclue2 = {
                enable = true;
                enableDemoAgent = true;
              };
              # Enable Avahi service for GeoClue2 network-based location detection
              services.avahi = {
                enable = true;
                nssmdns4 = true;
                nssmdns6 = true;
              };
            };
          };
          testScript = ''
            start_all()
            machine.wait_for_unit("geoclue-prometheus-exporter.service")
            # Check that the service is running
            machine.succeed("systemctl is-active geoclue-prometheus-exporter.service")
            # Wait for the metrics endpoint to be available and check for the 'up' metric
            machine.wait_until_succeeds("curl -s http://127.0.0.1:9090/metrics | grep -q 'up 1'")
            # Check that the metrics endpoint is serving Prometheus format data
            machine.succeed("curl -s http://127.0.0.1:9090/metrics | grep -q '# HELP'")
            # Debug: Show what metrics are actually available
            print(machine.succeed("curl -s http://127.0.0.1:9090/metrics | grep geoclue || echo 'No geoclue metrics found'"))
            # Verify that geoclue metrics are present - check for any geoclue metric
            machine.succeed("curl -s http://127.0.0.1:9090/metrics | grep -q 'geoclue_'")
          '';
        };
      in
      {
        # The default package built by `nix build`
        packages = {
          default = geoclue-prometheus-exporter;
        };

        # Run checks for the flake
        checks = {
          # Include the package build as a check (this includes unit tests)
          build = geoclue-prometheus-exporter;
          
          # Test the exporter in a VM
          vm-test = geoclue-exporter-test;
          
          # All tests including integration tests that require binary compilation
          all-tests = pkgs.runCommand "all-tests" {
            nativeBuildInputs = [ pkgs.cargo pkgs.rustc ] ++ geoclue-build-inputs;
            buildInputs = [ pkgs.openssl ];
            src = ./.;
            CARGO_TARGET_DIR = "/tmp/cargo-target";
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
            GIT_HASH = gitHash;
          } ''
            cp -r $src/* .
            cp -r $src/.git . 2>/dev/null || true
            # Build the binary first so integration tests can find it
            cargo build
            # Run all tests including integration tests
            cargo test --all
            touch $out
          '';
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
              imports = [ self.nixosModules.default ];
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
              # Enable Avahi service for GeoClue2 network-based location detection
              services.avahi = {
                enable = true;
                nssmdns4 = true;
                nssmdns6 = true;
              };
            };
          };
          testScript = ''
            start_all()
            machine.wait_for_unit("geoclue-prometheus-exporter.service")
            machine.succeed("systemctl is-active geoclue-prometheus-exporter.service")
            machine.wait_until_succeeds("curl -s http://127.0.0.1:9090/metrics | grep -q 'up 1'")
          '';
        };
      };
    };
}
