use anyhow::Result;
use futures_util::StreamExt;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::collector::collect;  // Import the collect function correctly
use zbus::{Connection, zvariant};
use chrono::Utc;
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tokio::signal::ctrl_c;
use clap::{Parser, ValueEnum};

// Get the package name from Cargo.toml at compile time
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
// Get the version from Cargo.toml at compile time
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

// Get build info from environment variables if available
// Using const-compatible unwrap_or_else for option_env
const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(hash) => hash,
    None => "unknown"
};

// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about = "GeoClue2 Prometheus Exporter")]
struct Args {
    /// Display version information
    #[arg(short, long)]
    version_info: bool,

    /// Log level filter
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,

    /// Distance threshold in meters
    #[arg(short = 'd', long, default_value_t = 10)]
    distance_threshold: u32,
    
    /// Time threshold in seconds
    #[arg(short = 't', long, default_value_t = 30)]
    time_threshold: u32,
    
    /// Accuracy level 
    #[arg(short = 'a', long, default_value = "street")]
    accuracy_level: AccuracyLevelArg,
    
    /// Prometheus metrics endpoint port
    #[arg(short = 'p', long, default_value_t = 9090)]
    metrics_port: u16,
    
    /// Bind address for the metrics server (IPv4 or IPv6)
    #[arg(short = 'b', long, default_value = "127.0.0.1")]
    bind_address: String,
}

// Generate a detailed version string including build information
fn get_version_string() -> String {
    format!("{} v{}\nBuild: {}",
            PKG_NAME, PKG_VERSION, GIT_HASH)
}

// Log level enum for command line arguments
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "lowercase")]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

// Accuracy level enum for command line arguments
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
#[clap(rename_all = "lowercase")]
enum AccuracyLevelArg {
    None,
    Country,
    City,
    Neighborhood,
    Street,
    Exact,
}

// Enum for GeoClue2 accuracy levels (internal representation)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum AccuracyLevel {
    None = 0,
    Country = 1,
    City = 4,
    Neighborhood = 5,
    Street = 6,
    Exact = 8,
}

impl From<AccuracyLevelArg> for AccuracyLevel {
    fn from(arg: AccuracyLevelArg) -> Self {
        match arg {
            AccuracyLevelArg::None => AccuracyLevel::None,
            AccuracyLevelArg::Country => AccuracyLevel::Country,
            AccuracyLevelArg::City => AccuracyLevel::City,
            AccuracyLevelArg::Neighborhood => AccuracyLevel::Neighborhood,
            AccuracyLevelArg::Street => AccuracyLevel::Street,
            AccuracyLevelArg::Exact => AccuracyLevel::Exact,
        }
    }
}

// Structure to track location update status
struct UpdateTracker {
    received_updates: u64,
}

// Structure to hold GeoClue2 connection components
struct GeoClueConnection {
    connection: Arc<Connection>,
    client_path: zvariant::OwnedObjectPath,
}

// Global log level
static mut LOG_LEVEL: LogLevel = LogLevel::Info;

fn setup_metrics(bind_address: &str, port: u16) -> Result<()> {
    // Parse the bind address - try both IPv4 and IPv6
    let socket_addr: SocketAddr = format!("{}:{}", bind_address, port).parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse bind address: {}", e))?;

    // Build and install the Prometheus recorder
    PrometheusBuilder::new()
        .with_http_listener(socket_addr)
        .install()
        .map_err(|e| anyhow::anyhow!("Failed to start Prometheus metrics server: {}", e))?;

    // Define metrics
    metrics::describe_gauge!("up", "Indicates if the exporter is operational (1 = up)");
    metrics::describe_gauge!("geoclue_latitude", "Latitude in degrees");
    metrics::describe_gauge!("geoclue_longitude", "Longitude in degrees");
    metrics::describe_gauge!("geoclue_accuracy", "Location accuracy in meters");
    metrics::describe_gauge!("geoclue_altitude", "Altitude in meters above sea level (not available = -1)");
    metrics::describe_gauge!("geoclue_speed", "Speed in meters per second");
    metrics::describe_gauge!("geoclue_heading", "Heading in degrees from North");
    metrics::describe_gauge!("geoclue_location_updates_received", "Number of location updates received");
    
    // Set the "up" metric to indicate the exporter is running
    metrics::gauge!("up").set(1.0);
    
    // Initialize process metrics collection
    // For metrics-process v2.4.0 we need to collect metrics manually
    collect();
    
    Ok(())
}

// Helper function to check if a message should be logged based on log level
fn should_log(message_level: LogLevel) -> bool {
    // Safety: This is safe because we set LOG_LEVEL once at startup and never modify it again
    unsafe {
        match LOG_LEVEL {
            LogLevel::Debug => true, // Debug logs everything
            LogLevel::Info => message_level != LogLevel::Debug, // Info logs Info, Warn, Error
            LogLevel::Warn => message_level == LogLevel::Warn || message_level == LogLevel::Error, // Warn logs Warn, Error
            LogLevel::Error => message_level == LogLevel::Error, // Error logs only Error
        }
    }
}

// Helper function to log in structured format
fn log(level: &str, message: &str, fields: &[(&str, String)]) {
    let message_level = match level {
        "DEBUG" => LogLevel::Debug,
        "INFO" => LogLevel::Info,
        "WARN" => LogLevel::Warn,
        "ERROR" => LogLevel::Error,
        _ => LogLevel::Info, // Default to Info for unknown levels
    };
    
    if !should_log(message_level) {
        return;
    }
    
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    
    let mut log_str = String::new();
    write!(&mut log_str, "timestamp=\"{}\" level={} message=\"{}\"", timestamp, level, message).unwrap();
    
    for (key, value) in fields {
        write!(&mut log_str, " {}={}", key, value).unwrap();
    }
    
    println!("{}", log_str);
}

// Helper function to set gauge only if the value is valid
fn set_gauge_if_valid(metric_name: &str, value: f64) -> bool {
    // Skip setting the metric if it's a sentinel value (-1 or extreme negative value)
    if value == -1.0 || value <= -1.7e308 {
        log("DEBUG", &format!("Skipping invalid metric {}", metric_name), &[
            ("metric", metric_name.to_string()), 
            ("value", value.to_string())
        ]);
        return false;
    }
    
    // Set the gauge with the appropriate name - use static string literals for metrics
    match metric_name {
        "latitude" => metrics::gauge!("geoclue_latitude").set(value),
        "longitude" => metrics::gauge!("geoclue_longitude").set(value),
        "accuracy" => metrics::gauge!("geoclue_accuracy").set(value),
        "altitude" => metrics::gauge!("geoclue_altitude").set(value),
        "speed" => metrics::gauge!("geoclue_speed").set(value),
        "heading" => metrics::gauge!("geoclue_heading").set(value),
        _ => {
            log("WARN", &format!("Unknown metric name: {}", metric_name), &[]);
            // Don't try to use a dynamic name with the gauge macro - it needs static strings
            return false;
        }
    }
    
    // Fixed clippy::needless_return warning
    true
}

// Function to establish GeoClue2 connection and setup client
async fn setup_geoclue_connection(args: &Args) -> Result<GeoClueConnection> {
    // Create a shared connection
    let connection = Arc::new(Connection::system().await?);
    log("INFO", "Connected to DBus system bus", &[]);

    // Get the manager proxy
    let manager = zbus::Proxy::new(
        &connection, 
        "org.freedesktop.GeoClue2", 
        "/org/freedesktop/GeoClue2/Manager", 
        "org.freedesktop.GeoClue2.Manager"
    ).await?;
    log("INFO", "Created GeoClue2 Manager proxy", &[]);
    
    // Call GetClient to get a client object path
    let client_path: zvariant::OwnedObjectPath = manager.call::<_, _, zvariant::OwnedObjectPath>(
        "GetClient", 
        &()
    ).await?;
    
    log("INFO", "Got client path", &[("path", format!("{}", client_path))]);

    // Create client proxy
    let client = zbus::Proxy::new(
        &connection, 
        "org.freedesktop.GeoClue2", 
        &client_path, 
        "org.freedesktop.GeoClue2.Client"
    ).await?;
    
    // Set client properties
    client.set_property("DesktopId", &PKG_NAME.to_string()).await?;
    log("INFO", "Set client desktop ID", &[("desktop_id", PKG_NAME.to_string())]);
    
    // Get accuracy level from command-line args
    let accuracy_level: AccuracyLevel = args.accuracy_level.into();
    
    // Set distance threshold (in meters)
    client.set_property("DistanceThreshold", &args.distance_threshold).await?;
    log("INFO", "Set distance threshold", &[("threshold_meters", args.distance_threshold.to_string())]);
    
    // Set time threshold (in seconds)
    client.set_property("TimeThreshold", &args.time_threshold).await?;
    log("INFO", "Set time threshold", &[("threshold_seconds", args.time_threshold.to_string())]);
    
    // Set requested accuracy level
    client.set_property("RequestedAccuracyLevel", &(accuracy_level as u32)).await?;
    log("INFO", "Set accuracy level", &[
        ("accuracy_level", format!("{:?}", accuracy_level)),
        ("level_value", (accuracy_level as u32).to_string()),
    ]);
    
    // Start the client
    client.call::<_, _, ()>("Start", &()).await?;
    log("INFO", "Started GeoClue2 client", &[]);

    Ok(GeoClueConnection {
        connection,
        client_path,
    })
}

// Check if an error indicates a DBus disconnection that warrants reconnection
fn is_disconnection_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("remote peer disconnected") ||
    error_str.contains("connection closed") ||
    error_str.contains("no reply") ||
    error_str.contains("noreply") ||
    error_str.contains("disconnected") ||
    error_str.contains("broken pipe") ||
    error_str.contains("transport endpoint is not connected")
}

// Function to monitor location updates with proper error handling
async fn monitor_location_updates(
    geoclue_conn: &GeoClueConnection,
    tracker: Arc<Mutex<UpdateTracker>>
) -> Result<()> {
    log("INFO", "Waiting for location updates", &[]);

    // Create client proxy from the connection
    let client = zbus::Proxy::new(
        &geoclue_conn.connection, 
        "org.freedesktop.GeoClue2", 
        &geoclue_conn.client_path, 
        "org.freedesktop.GeoClue2.Client"
    ).await?;

    // Monitor for location updates
    let mut location_updated_stream = client.receive_signal("LocationUpdated").await?;
    
    while let Some(signal) = location_updated_stream.next().await {
        // Update counter whenever we get a new location
        {
            let mut tracker = tracker.lock().unwrap();
            tracker.received_updates += 1;
            
            // Update the received updates counter
            metrics::gauge!("geoclue_location_updates_received").set(tracker.received_updates as f64);
            
            // Log the current update count
            log("DEBUG", "Location update received", &[
                ("received_updates", tracker.received_updates.to_string()),
            ]);
        }
        
        // Deserialize the entire body as a tuple
        let body_owned = signal.body().clone();
        let (old_path, new_path): (zvariant::ObjectPath, zvariant::ObjectPath) = 
            body_owned.deserialize()?;
        
        log("INFO", "Received location update", &[
            ("old_path", format!("{}", old_path)),
            ("new_path", format!("{}", new_path)),
        ]);

        // Create a location proxy for this location
        let location = zbus::Proxy::new(
            &geoclue_conn.connection, 
            "org.freedesktop.GeoClue2", 
            &new_path, 
            "org.freedesktop.GeoClue2.Location"
        ).await?;

        // Get location properties
        let lat: f64 = location.get_property("Latitude").await?;
        let lon: f64 = location.get_property("Longitude").await?;
        let acc: f64 = location.get_property("Accuracy").await?;
        let alt: f64 = location.get_property("Altitude").await?;
        let spd: f64 = location.get_property("Speed").await?;
        let head: f64 = location.get_property("Heading").await?;
        
        // Prepare field arrays for logging
        let mut update_fields = vec![
            ("latitude", format!("{}", lat)),
            ("longitude", format!("{}", lon)),
            ("accuracy", format!("{}", acc))
        ];
        
        // Add optional fields only if they're valid
        // Fixed the redundant comparison - if alt > -1.0 then alt > -1.7e308 is always true
        if alt > -1.0 {
            update_fields.push(("altitude", format!("{}", alt)));
        } else {
            update_fields.push(("altitude", "not_available".to_string()));
        }
        
        if spd > -1.0 {
            update_fields.push(("speed", format!("{}", spd)));
        } else {
            update_fields.push(("speed", "not_available".to_string()));
        }
        
        if head > -1.0 {
            update_fields.push(("heading", format!("{}", head)));
        } else {
            update_fields.push(("heading", "not_available".to_string()));
        }
        
        log("INFO", "Updated location metrics", &update_fields);

        // Log the complete raw data at debug level
        log("DEBUG", "Raw location data", &[
            ("latitude", format!("{}", lat)),
            ("longitude", format!("{}", lon)),
            ("accuracy", format!("{}", acc)),
            ("altitude", format!("{}", alt)),
            ("speed", format!("{}", spd)),
            ("heading", format!("{}", head)),
        ]);

        // Update metrics, but only if they are valid values
        set_gauge_if_valid("latitude", lat);
        set_gauge_if_valid("longitude", lon);
        set_gauge_if_valid("accuracy", acc);
        set_gauge_if_valid("altitude", alt);
        set_gauge_if_valid("speed", spd);
        set_gauge_if_valid("heading", head);
    }

    // This indicates the stream has ended (likely due to disconnection)
    Err(anyhow::anyhow!("Location update stream ended"))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // If --version-info flag is provided, display detailed version info and exit
    if args.version_info {
        println!("{}", get_version_string());
        std::process::exit(0);
    }
    
    // Set global log level
    // Safety: This is safe because we only set it once at startup
    unsafe {
        LOG_LEVEL = args.log_level;
    }
    
    // Set up metrics with the provided bind address and port
    match setup_metrics(&args.bind_address, args.metrics_port) {
        Ok(_) => {
            log("INFO", &format!("{} metrics endpoint started", PKG_NAME), &[
                ("endpoint", format!("http://{}:{}/metrics", args.bind_address, args.metrics_port)),
                ("version", PKG_VERSION.to_string()),
                ("build_hash", GIT_HASH.to_string()),
                ("log_level", format!("{:?}", args.log_level)),
            ]);
        },
        Err(e) => {
            log("ERROR", &format!("Failed to start {} metrics endpoint", PKG_NAME), &[
                ("error", format!("{}", e)),
                ("bind_address", args.bind_address.clone()),
                ("port", args.metrics_port.to_string()),
            ]);
            return Err(e);
        }
    }

    log("DEBUG", "Command line arguments", &[
        ("bind_address", args.bind_address.to_string()),
        ("distance_threshold", args.distance_threshold.to_string()),
        ("time_threshold", args.time_threshold.to_string()),
        ("accuracy_level", format!("{:?}", args.accuracy_level)),
        ("metrics_port", args.metrics_port.to_string()),
    ]);

    // Initialize update tracker
    let tracker = Arc::new(Mutex::new(UpdateTracker {
        received_updates: 0,
    }));

    // Periodically collect process metrics
    let _metrics_handle = tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            collect();
        }
    });

    // Shared variables for shutdown handling
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    // Handle graceful shutdown
    tokio::spawn(async move {
        if let Err(e) = ctrl_c().await {
            log("ERROR", "Failed to listen for ctrl+c signal", &[("error", format!("{}", e))]);
            return;
        }
        
        log("INFO", "Shutdown signal received", &[]);
        shutdown_flag_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    // Main reconnection loop
    let mut retry_count = 0;
    let max_retry_delay = 60; // Maximum delay between retries in seconds
    
    loop {
        // Check if shutdown was requested
        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            log("INFO", "Shutdown requested, exiting", &[]);
            break;
        }

        // Attempt to connect to GeoClue2
        match setup_geoclue_connection(&args).await {
            Ok(geoclue_conn) => {
                log("INFO", "Successfully connected to GeoClue2", &[]);
                retry_count = 0; // Reset retry count on successful connection
                
                // Set up shutdown handler for this connection
                let shutdown_connection = Arc::new(Connection::system().await?);
                let shutdown_client_path = geoclue_conn.client_path.clone();
                let shutdown_flag_monitor = shutdown_flag.clone();
                
                let shutdown_handle = tokio::spawn(async move {
                    // Wait for shutdown signal
                    while !shutdown_flag_monitor.load(std::sync::atomic::Ordering::Relaxed) {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    
                    log("INFO", "Stopping GeoClue2 client for shutdown", &[]);
                    
                    // Create a new client proxy specifically for shutdown
                    match zbus::Proxy::new(
                        &shutdown_connection,
                        "org.freedesktop.GeoClue2",
                        &shutdown_client_path,
                        "org.freedesktop.GeoClue2.Client"
                    ).await {
                        Ok(shutdown_client) => {
                            // Call Stop on the client for clean shutdown
                            if let Err(e) = shutdown_client.call::<_, _, ()>("Stop", &()).await {
                                log("ERROR", "Failed to stop GeoClue2 client", &[("error", format!("{}", e))]);
                            } else {
                                log("INFO", "GeoClue2 client stopped successfully", &[]);
                            }
                        },
                        Err(e) => {
                            log("ERROR", "Failed to create shutdown client proxy", &[("error", format!("{}", e))]);
                        }
                    }
                    
                    // Set the "up" metric to 0 to indicate the exporter is shutting down
                    metrics::gauge!("up").set(0.0);
                });

                // Monitor location updates
                let monitoring_result = monitor_location_updates(&geoclue_conn, tracker.clone()).await;
                
                // Cancel shutdown handler if we're not shutting down
                if !shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    shutdown_handle.abort();
                }
                
                // Handle monitoring result
                match monitoring_result {
                    Ok(_) => {
                        // This shouldn't happen normally
                        log("INFO", "Location monitoring completed normally", &[]);
                        break;
                    },
                    Err(e) => {
                        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            log("INFO", "Location monitoring stopped due to shutdown", &[]);
                            // Wait for shutdown handler to complete
                            let _ = shutdown_handle.await;
                            break;
                        } else if is_disconnection_error(&e) {
                            log("WARN", "GeoClue2 connection lost, will attempt to reconnect", &[
                                ("error", format!("{}", e)),
                                ("retry_count", retry_count.to_string()),
                            ]);
                            // Continue to retry logic
                        } else {
                            log("ERROR", "Non-recoverable error in location monitoring", &[
                                ("error", format!("{}", e)),
                            ]);
                            return Err(e);
                        }
                    }
                }
            },
            Err(e) => {
                if is_disconnection_error(&e) {
                    log("WARN", "Failed to connect to GeoClue2, will retry", &[
                        ("error", format!("{}", e)),
                        ("retry_count", retry_count.to_string()),
                    ]);
                } else {
                    log("ERROR", "Non-recoverable error connecting to GeoClue2", &[
                        ("error", format!("{}", e)),
                    ]);
                    return Err(e);
                }
            }
        }

        // Check if shutdown was requested before sleeping
        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        // Calculate exponential backoff delay
        retry_count += 1;
        let delay = std::cmp::min(2_u64.pow(std::cmp::min(retry_count, 6)), max_retry_delay);
        
        log("INFO", "Waiting before reconnection attempt", &[
            ("delay_seconds", delay.to_string()),
            ("retry_count", retry_count.to_string()),
        ]);
        
        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
    }

    log("INFO", "Exporter shutting down", &[]);
    Ok(())
}

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
            "org.freedesktop.DBus.Error.NoReply: Message recipient disconnected from message bus without replying",
            "Connection closed by peer",
            "Transport endpoint is not connected",
            "Broken pipe (os error 32)",
            "No reply from remote service",
            "DBus.Error.NoReply: Connection lost",
            "Service disconnected unexpectedly",
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
            "I/O error: No such file or directory",
        ];
        
        for error_msg in non_disconnection_errors {
            let error = anyhow::anyhow!("{}", error_msg);
            assert!(!is_disconnection_error(&error), "False positive for: {}", error_msg);
        }
    }
}
