{ nixpkgs, system ? "x86_64-linux" }:

rec {
  pkgs = import nixpkgs { inherit system; };
  
  # Helper to create simple VM tests
  makeTest = { name, machine, testScript }: 
    pkgs.nixosTest {
      inherit name;
      nodes = {
        inherit machine;
      };
      testScript = ''
        start_all()
        ${testScript}
      '';
    };
    
  # Helper to run self-tests of a package
  runSelfTests = pkg: pkg.overrideAttrs (old: {
    doCheck = true;
    checkPhase = old.checkPhase or "cargo test";
  });
}
