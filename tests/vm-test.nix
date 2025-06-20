{ pkgs, nodes, testScript }:

pkgs.nixosTest {
  name = "geoclue-prometheus-exporter-test";
  
  inherit nodes;
  
  # Execute the test script in the VM
  inherit testScript;
}
