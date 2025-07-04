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
            # Wait for the metrics endpoint to be available
            machine.wait_until_succeeds("curl -s http://127.0.0.1:9090/metrics | grep -q 'geoclue_'")
            # Check that some metrics are present
            machine.succeed("curl -s http://127.0.0.1:9090/metrics | grep -q 'up 1'")
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
            machine.wait_until_succeeds("curl -s http://127.0.0.1:9090/metrics | grep -q 'geoclue_'")
          '';
        };
      };
    };
}
