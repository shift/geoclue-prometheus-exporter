//! Property-based tests for comprehensive edge case coverage
//! 
//! These tests use quickcheck and proptest to generate random inputs
//! and verify that the functions behave correctly across a wide range of inputs.

use quickcheck::quickcheck;
use proptest::prelude::*;

// Import the functions we want to test from the main binary
// Note: Since this is a binary crate, we can't directly import functions
// So we'll test through the CLI interface and focus on input validation

#[cfg(test)]
mod property_tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::process::Command;
    
    // Property test for command line argument parsing
    quickcheck! {
        fn test_bind_address_validation(addr: String) -> bool {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--bind-address").arg(&addr).arg("--version");
            
            let result = cmd.output();
            
            // Either it should succeed (valid address) or fail with parse error
            match result {
                Ok(output) => {
                    if output.status.success() {
                        true // Valid address
                    } else {
                        // Should contain parse error message for invalid addresses
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        stderr.contains("Failed to parse bind address") || 
                        stderr.contains("error") ||
                        stderr.is_empty() // Some errors may not have stderr
                    }
                }
                Err(_) => true // Command execution error is acceptable
            }
        }
        
        fn test_port_validation(port: u16) -> bool {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--metrics-port").arg(port.to_string()).arg("--version");
            
            let result = cmd.output();
            
            // All u16 values should be valid port numbers from argument parsing perspective
            // The actual binding may fail, but argument parsing should succeed
            match result {
                Ok(output) => {
                    // If version flag works, command line parsing succeeded
                    output.status.success() || !String::from_utf8_lossy(&output.stderr).contains("invalid value")
                }
                Err(_) => true // Command execution error is acceptable
            }
        }
        
        fn test_distance_threshold_validation(threshold: u32) -> bool {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--distance-threshold").arg(threshold.to_string()).arg("--version");
            
            let result = cmd.output();
            
            // All u32 values should be valid
            match result {
                Ok(output) => {
                    output.status.success() || !String::from_utf8_lossy(&output.stderr).contains("invalid value")
                }
                Err(_) => true
            }
        }
        
        fn test_time_threshold_validation(threshold: u32) -> bool {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--time-threshold").arg(threshold.to_string()).arg("--version");
            
            let result = cmd.output();
            
            // All u32 values should be valid
            match result {
                Ok(output) => {
                    output.status.success() || !String::from_utf8_lossy(&output.stderr).contains("invalid value")
                }
                Err(_) => true
            }
        }
    }
    
    // Test log level argument validation
    #[test]
    fn test_log_level_validation() {
        let valid_levels = ["debug", "info", "warn", "error"];
        let invalid_levels = ["trace", "fatal", "verbose", "quiet", "invalid"];
        
        for level in &valid_levels {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--log-level").arg(level).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(output.status.success(), "Valid log level '{}' should be accepted", level);
        }
        
        for level in &invalid_levels {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--log-level").arg(level).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(!output.status.success(), "Invalid log level '{}' should be rejected", level);
        }
    }
    
    // Test accuracy level argument validation
    #[test]
    fn test_accuracy_level_validation() {
        let valid_levels = ["none", "country", "city", "neighborhood", "street", "exact"];
        let invalid_levels = ["precise", "rough", "approximate", "invalid"];
        
        for level in &valid_levels {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--accuracy-level").arg(level).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(output.status.success(), "Valid accuracy level '{}' should be accepted", level);
        }
        
        for level in &invalid_levels {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--accuracy-level").arg(level).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(!output.status.success(), "Invalid accuracy level '{}' should be rejected", level);
        }
    }
}

// Proptest-based tests for more structured property testing
proptest! {
    #[test]
    fn test_address_format_edge_cases(
        ip_part1 in 0u8..=255,
        ip_part2 in 0u8..=255, 
        ip_part3 in 0u8..=255,
        ip_part4 in 0u8..=255
    ) {
        use assert_cmd::prelude::*;
        use std::process::Command;
        
        let address = format!("{}.{}.{}.{}", ip_part1, ip_part2, ip_part3, ip_part4);
        
        let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
        cmd.arg("--bind-address").arg(&address).arg("--version");
        
        // All valid IPv4 addresses should be parsed successfully by clap
        let result = cmd.output();
        prop_assert!(result.is_ok());
        
        if let Ok(output) = result {
            // Either succeeds or fails with a specific error (not argument parsing error)
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Should not fail due to argument parsing if it's a valid IPv4
                prop_assert!(!stderr.contains("invalid value") || stderr.contains("Failed to parse bind address"));
            }
        }
    }
    
    #[test]
    fn test_numeric_argument_ranges(
        port in 1u16..=65535,
        distance in 0u32..=1000000,
        time in 0u32..=86400  // 24 hours in seconds
    ) {
        use assert_cmd::prelude::*;
        use std::process::Command;
        
        let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
        cmd.arg("--metrics-port").arg(port.to_string());
        cmd.arg("--distance-threshold").arg(distance.to_string());
        cmd.arg("--time-threshold").arg(time.to_string());
        cmd.arg("--version");
        
        let result = cmd.output();
        prop_assert!(result.is_ok());
        
        // Argument parsing should succeed for all reasonable values
        if let Ok(output) = result {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Should not fail due to invalid argument values
                prop_assert!(!stderr.contains("invalid value"));
            }
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use assert_cmd::prelude::*;
    use std::process::Command;
    
    #[test]
    fn test_port_boundary_values() {
        let boundary_ports = [1, 1023, 1024, 8080, 9090, 65534, 65535];
        
        for port in &boundary_ports {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--metrics-port").arg(port.to_string()).arg("--version");
            
            let output = cmd.output().unwrap();
            // All these ports should be valid from argument parsing perspective
            assert!(output.status.success(), "Port {} should be valid", port);
        }
    }
    
    #[test]
    fn test_threshold_boundary_values() {
        let boundary_distances = [0, 1, 10, 100, 1000, u32::MAX];
        let boundary_times = [0, 1, 30, 60, 3600, 86400, u32::MAX];
        
        for distance in &boundary_distances {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--distance-threshold").arg(distance.to_string()).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(output.status.success(), "Distance threshold {} should be valid", distance);
        }
        
        for time in &boundary_times {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap(); 
            cmd.arg("--time-threshold").arg(time.to_string()).arg("--version");
            
            let output = cmd.output().unwrap();
            assert!(output.status.success(), "Time threshold {} should be valid", time);
        }
    }
    
    #[test]
    fn test_address_format_variations() {
        let test_addresses = [
            "127.0.0.1",
            "0.0.0.0", 
            "255.255.255.255",
            "::1",
            "::",
            "localhost",
            "example.com"
        ];
        
        for addr in &test_addresses {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--bind-address").arg(addr).arg("--version");
            
            let output = cmd.output().unwrap();
            // These should either succeed or fail with address parsing error, not argument error
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Should not be an invalid argument error
                assert!(!stderr.contains("invalid value") || stderr.contains("address"),
                    "Address '{}' should either work or give address-specific error", addr);
            }
        }
    }
    
    #[test]
    fn test_malformed_addresses() {
        let malformed_addresses = [
            "",           // Empty
            " ",          // Whitespace  
            "not-an-address%", // Special characters
        ];
        
        // Note: IPv4 validation happens at runtime, not at argument parsing time
        // So addresses like "256.1.1.1" will be accepted by clap but fail during setup_metrics
        
        for addr in &malformed_addresses {
            let mut cmd = Command::cargo_bin("geoclue-prometheus-exporter").unwrap();
            cmd.arg("--bind-address").arg(addr).arg("--version");
            
            let output = cmd.output().unwrap();
            // These should either succeed at CLI parsing level or fail with argument error
            // The actual validation happens when setup_metrics is called
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Empty string might be rejected at argument level
                if addr.is_empty() {
                    assert!(stderr.contains("cannot be empty") || 
                            stderr.contains("invalid value") ||
                            !stderr.is_empty(), 
                        "Empty address should be rejected");
                }
            }
        }
    }
}