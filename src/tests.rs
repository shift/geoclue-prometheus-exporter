#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    
    // Test the log level logic functions
    #[test]
    fn test_should_log() {
        unsafe {
            // Test Debug level
            LOG_LEVEL = LogLevel::Debug;
            assert!(should_log(LogLevel::Debug));
            assert!(should_log(LogLevel::Info));
            assert!(should_log(LogLevel::Warn));
            assert!(should_log(LogLevel::Error));
            
            // Test Info level
            LOG_LEVEL = LogLevel::Info;
            assert!(!should_log(LogLevel::Debug));
            assert!(should_log(LogLevel::Info));
            assert!(should_log(LogLevel::Warn));
            assert!(should_log(LogLevel::Error));
            
            // Test Warn level
            LOG_LEVEL = LogLevel::Warn;
            assert!(!should_log(LogLevel::Debug));
            assert!(!should_log(LogLevel::Info));
            assert!(should_log(LogLevel::Warn));
            assert!(should_log(LogLevel::Error));
            
            // Test Error level
            LOG_LEVEL = LogLevel::Error;
            assert!(!should_log(LogLevel::Debug));
            assert!(!should_log(LogLevel::Info));
            assert!(!should_log(LogLevel::Warn));
            assert!(should_log(LogLevel::Error));
        }
    }
    
    // Test the set_gauge_if_valid function
    #[test]
    fn test_set_gauge_if_valid() {
        // Test with valid values
        assert!(set_gauge_if_valid("latitude", 35.123));
        assert!(set_gauge_if_valid("longitude", 135.456));
        assert!(set_gauge_if_valid("accuracy", 10.5));
        assert!(set_gauge_if_valid("altitude", 123.4));
        assert!(set_gauge_if_valid("speed", 5.2));
        assert!(set_gauge_if_valid("heading", 270.0));
        
        // Test with invalid values (should return false)
        assert!(!set_gauge_if_valid("latitude", -1.0));
        assert!(!set_gauge_if_valid("longitude", -1.7e308));
        
        // Test with unknown metric name (should return false)
        assert!(!set_gauge_if_valid("unknown_metric", 123.0));
    }
    
    // Test the get_version_string function
    #[test]
    fn test_version_string_format() {
        let version_str = get_version_string();
        
        // Verify it contains the expected components
        assert!(version_str.contains(PKG_NAME));
        assert!(version_str.contains(PKG_VERSION));
        assert!(version_str.contains("Build:"));
    }
    
    // Test AccuracyLevel conversion
    #[test]
    fn test_accuracy_level_conversion() {
        // Test all conversions
        assert_eq!(AccuracyLevel::None as u32, 0);
        assert_eq!(AccuracyLevel::Country as u32, 1);
        assert_eq!(AccuracyLevel::City as u32, 4);
        assert_eq!(AccuracyLevel::Neighborhood as u32, 5);
        assert_eq!(AccuracyLevel::Street as u32, 6);
        assert_eq!(AccuracyLevel::Exact as u32, 8);
        
        // Test from conversions
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::None), AccuracyLevel::None));
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::Country), AccuracyLevel::Country));
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::City), AccuracyLevel::City));
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::Neighborhood), AccuracyLevel::Neighborhood));
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::Street), AccuracyLevel::Street));
        assert!(matches!(AccuracyLevel::from(AccuracyLevelArg::Exact), AccuracyLevel::Exact));
    }
    
    // Test UpdateTracker functionality
    #[test]
    fn test_update_tracker() {
        let tracker = Arc::new(Mutex::new(UpdateTracker {
            received_updates: 0,
        }));
        
        // Simulate receiving updates
        {
            let mut tracker_guard = tracker.lock().unwrap();
            tracker_guard.received_updates += 1;
            assert_eq!(tracker_guard.received_updates, 1);
        }
        
        // Simulate another update
        {
            let mut tracker_guard = tracker.lock().unwrap();
            tracker_guard.received_updates += 1;
            assert_eq!(tracker_guard.received_updates, 2);
        }
    }
    
    // Test disconnection error detection
    #[test]
    fn test_is_disconnection_error() {
        // Test errors that should be detected as disconnection errors
        let disconnection_errors = vec![
            "org.freedesktop.DBus.Error.NoReply: Remote peer disconnected",
            "Connection closed by peer",
            "Transport endpoint is not connected",
            "Broken pipe (os error 32)",
            "No reply from remote service",
        ];
        
        for error_msg in disconnection_errors {
            let error = anyhow::anyhow!("{}", error_msg);
            assert!(is_disconnection_error(&error), "Failed to detect: {}", error_msg);
        }
        
        // Test errors that should NOT be detected as disconnection errors
        let non_disconnection_errors = vec![
            "Permission denied",
            "Invalid argument",
            "File not found",
            "Service not found",
        ];
        
        for error_msg in non_disconnection_errors {
            let error = anyhow::anyhow!("{}", error_msg);
            assert!(!is_disconnection_error(&error), "False positive for: {}", error_msg);
        }
    }
}
