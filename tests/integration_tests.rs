//! Integration tests for geoclue-prometheus-exporter
//! 
//! These tests verify that the exporter correctly initializes
//! and handles command line arguments.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_version_flag() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter")?;
    
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    
    Ok(())
}

#[test]
fn test_version_info_flag() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter")?;
    
    cmd.arg("--version-info");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Build:"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    
    Ok(())
}

#[test]
fn test_invalid_bind_address() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter")?;
    
    // Using an invalid bind address format should cause an error
    cmd.arg("--bind-address").arg("not-an-address%");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse bind address"));
    
    Ok(())
}

#[test]
fn test_help_flag() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter")?;
    
    cmd.arg("--help");
    cmd.assert()
        .success()
        // Update the expected help text to match actual output
        .stdout(predicate::str::contains("metrics-port"))
        .stdout(predicate::str::contains("bind-address"))
        .stdout(predicate::str::contains("Print help"));
    
    Ok(())
}

#[test]
fn test_disconnection_error_on_unavailable_service() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter")?;
    
    // Test that the application exits when GeoClue2 service is not available
    // This simulates the typical case where the service would need to handle reconnection
    cmd.arg("--log-level").arg("error");
    cmd.assert()
        .failure()
        .stderr(
            predicate::str::contains("ServiceUnknown")
                .or(predicate::str::contains("Service not found"))
                .or(predicate::str::contains("No such file or directory"))
        );
    
    Ok(())
}
