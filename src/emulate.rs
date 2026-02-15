use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    ColorScheme, EmulateArgs, EmulateCommand, EmulateSetArgs, GlobalOpts, NetworkProfile,
};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
pub struct EmulateStatusOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<GeolocationOutput>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(rename = "colorScheme", skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<ViewportOutput>,
    #[serde(rename = "deviceScaleFactor", skip_serializing_if = "Option::is_none")]
    pub device_scale_factor: Option<f64>,
    pub mobile: bool,
}

#[derive(Serialize)]
pub struct GeolocationOutput {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize)]
pub struct ViewportOutput {
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
pub struct EmulateResetOutput {
    pub reset: bool,
}

#[derive(Serialize)]
pub struct ResizeOutput {
    pub width: u32,
    pub height: u32,
}

// =============================================================================
// Emulation state persistence
// =============================================================================

/// Persisted emulation state for CDP-only overrides that cannot be queried via
/// JavaScript. Written by `emulate set`, read by `emulate status`, deleted by
/// `emulate reset`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmulateState {
    pub mobile: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u32>,
}

/// Returns the path to the emulation state file: `~/.chrome-cli/emulate-state.json`.
fn emulate_state_path() -> Result<PathBuf, AppError> {
    #[cfg(unix)]
    let key = "HOME";
    #[cfg(windows)]
    let key = "USERPROFILE";

    let home = std::env::var(key)
        .map(PathBuf::from)
        .map_err(|_| AppError {
            message: "could not determine home directory".to_string(),
            code: ExitCode::GeneralError,
        })?;
    Ok(home.join(".chrome-cli").join("emulate-state.json"))
}

/// Write emulation state to the given path (atomic write + `0o600` permissions on Unix).
fn write_emulate_state_to(path: &Path, state: &EmulateState) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError {
            message: format!("emulate state dir error: {e}"),
            code: ExitCode::GeneralError,
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).map_err(
                |e| AppError {
                    message: format!("emulate state dir permissions error: {e}"),
                    code: ExitCode::GeneralError,
                },
            )?;
        }
    }

    let json = serde_json::to_string_pretty(state).map_err(|e| AppError {
        message: format!("emulate state serialization error: {e}"),
        code: ExitCode::GeneralError,
    })?;

    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| AppError {
        message: format!("emulate state write error: {e}"),
        code: ExitCode::GeneralError,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600)).map_err(
            |e| AppError {
                message: format!("emulate state file permissions error: {e}"),
                code: ExitCode::GeneralError,
            },
        )?;
    }

    std::fs::rename(&tmp_path, path).map_err(|e| AppError {
        message: format!("emulate state rename error: {e}"),
        code: ExitCode::GeneralError,
    })?;
    Ok(())
}

/// Write emulation state to the default path.
fn write_emulate_state(state: &EmulateState) -> Result<(), AppError> {
    let path = emulate_state_path()?;
    write_emulate_state_to(&path, state)
}

/// Read emulation state from the given path. Returns `Ok(None)` if the file does not exist.
fn read_emulate_state_from(path: &Path) -> Result<Option<EmulateState>, AppError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let state: EmulateState = serde_json::from_str(&contents).map_err(|e| AppError {
                message: format!("invalid emulate state file: {e}"),
                code: ExitCode::GeneralError,
            })?;
            Ok(Some(state))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError {
            message: format!("emulate state read error: {e}"),
            code: ExitCode::GeneralError,
        }),
    }
}

/// Read emulation state from the default path.
fn read_emulate_state() -> Result<Option<EmulateState>, AppError> {
    let path = emulate_state_path()?;
    read_emulate_state_from(&path)
}

/// Delete emulation state at the given path. Returns `Ok(())` if already absent.
fn delete_emulate_state_from(path: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError {
            message: format!("emulate state delete error: {e}"),
            code: ExitCode::GeneralError,
        }),
    }
}

/// Delete emulation state from the default path.
fn delete_emulate_state() -> Result<(), AppError> {
    let path = emulate_state_path()?;
    delete_emulate_state_from(&path)
}

// =============================================================================
// Plain text display
// =============================================================================

impl fmt::Display for EmulateStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref network) = self.network {
            writeln!(f, "Network: {network}")?;
        }
        if let Some(cpu) = self.cpu {
            writeln!(f, "CPU throttling: {cpu}x")?;
        }
        if let Some(ref geo) = self.geolocation {
            writeln!(f, "Geolocation: {},{}", geo.latitude, geo.longitude)?;
        }
        if let Some(ref ua) = self.user_agent {
            writeln!(f, "User-Agent: {ua}")?;
        }
        if let Some(ref cs) = self.color_scheme {
            writeln!(f, "Color scheme: {cs}")?;
        }
        if let Some(ref vp) = self.viewport {
            writeln!(f, "Viewport: {}x{}", vp.width, vp.height)?;
        }
        if let Some(dsf) = self.device_scale_factor {
            writeln!(f, "Device scale: {dsf}")?;
        }
        if self.mobile {
            writeln!(f, "Mobile: true")?;
        }
        Ok(())
    }
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_output(value: &impl Serialize, output: &crate::cli::OutputFormat) -> Result<(), AppError> {
    let json = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    let json = json.map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
    })?;
    println!("{json}");
    Ok(())
}

// =============================================================================
// Config helper
// =============================================================================

fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

    Ok((client, managed))
}

// =============================================================================
// Parsing helpers
// =============================================================================

/// Parse a viewport string like `"1280x720"` into `(width, height)`.
pub fn parse_viewport(input: &str) -> Result<(u32, u32), AppError> {
    let parts: Vec<&str> = input.split('x').collect();
    if parts.len() != 2 {
        return Err(AppError::invalid_viewport(input));
    }
    let width: u32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| AppError::invalid_viewport(input))?;
    let height: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|_| AppError::invalid_viewport(input))?;
    if width == 0 || height == 0 {
        return Err(AppError::invalid_viewport(input));
    }
    Ok((width, height))
}

/// Parse a geolocation string like `"37.7749,-122.4194"` into `(latitude, longitude)`.
pub fn parse_geolocation(input: &str) -> Result<(f64, f64), AppError> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 2 {
        return Err(AppError::invalid_geolocation(input));
    }
    let lat: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| AppError::invalid_geolocation(input))?;
    let lon: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| AppError::invalid_geolocation(input))?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(AppError::invalid_geolocation(input));
    }
    Ok((lat, lon))
}

/// Return CDP `Network.emulateNetworkConditions` parameters for a profile.
#[must_use]
pub fn network_profile_params(profile: NetworkProfile) -> serde_json::Value {
    match profile {
        NetworkProfile::Offline => serde_json::json!({
            "offline": true,
            "latency": 0,
            "downloadThroughput": 0,
            "uploadThroughput": 0,
        }),
        NetworkProfile::Slow4g => serde_json::json!({
            "offline": false,
            "latency": 150,
            "downloadThroughput": 1_600_000,
            "uploadThroughput": 750_000,
        }),
        NetworkProfile::FourG => serde_json::json!({
            "offline": false,
            "latency": 20,
            "downloadThroughput": 4_000_000,
            "uploadThroughput": 3_000_000,
        }),
        NetworkProfile::ThreeG => serde_json::json!({
            "offline": false,
            "latency": 100,
            "downloadThroughput": 750_000,
            "uploadThroughput": 250_000,
        }),
        NetworkProfile::None => serde_json::json!({
            "offline": false,
            "latency": 0,
            "downloadThroughput": -1,
            "uploadThroughput": -1,
        }),
    }
}

/// Map a `NetworkProfile` variant to its CLI string representation.
fn network_profile_name(profile: NetworkProfile) -> &'static str {
    match profile {
        NetworkProfile::Offline => "offline",
        NetworkProfile::Slow4g => "slow-4g",
        NetworkProfile::FourG => "4g",
        NetworkProfile::ThreeG => "3g",
        NetworkProfile::None => "none",
    }
}

/// Map a `ColorScheme` variant to its CLI string representation.
fn color_scheme_name(scheme: ColorScheme) -> &'static str {
    match scheme {
        ColorScheme::Dark => "dark",
        ColorScheme::Light => "light",
        ColorScheme::Auto => "auto",
    }
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `emulate` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_emulate(global: &GlobalOpts, args: &EmulateArgs) -> Result<(), AppError> {
    match &args.command {
        EmulateCommand::Set(set_args) => execute_set(global, set_args).await,
        EmulateCommand::Reset => execute_reset(global).await,
        EmulateCommand::Status => execute_status(global).await,
    }
}

// =============================================================================
// emulate set
// =============================================================================

#[allow(clippy::too_many_lines)]
async fn execute_set(global: &GlobalOpts, args: &EmulateSetArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let mut status = EmulateStatusOutput {
        network: None,
        cpu: None,
        geolocation: None,
        user_agent: None,
        color_scheme: None,
        viewport: None,
        device_scale_factor: None,
        mobile: false,
    };

    // --- Network throttling ---
    if let Some(ref profile) = args.network {
        managed
            .ensure_domain("Network")
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        let params = network_profile_params(*profile);
        managed
            .send_command("Network.emulateNetworkConditions", Some(params))
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        status.network = Some(network_profile_name(*profile).to_string());
    }

    // --- CPU throttling ---
    if let Some(rate) = args.cpu {
        managed
            .send_command(
                "Emulation.setCPUThrottlingRate",
                Some(serde_json::json!({ "rate": rate })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        status.cpu = Some(rate);
    }

    // --- Geolocation ---
    if let Some(ref geo_str) = args.geolocation {
        let (lat, lon) = parse_geolocation(geo_str)?;
        managed
            .send_command(
                "Emulation.setGeolocationOverride",
                Some(serde_json::json!({
                    "latitude": lat,
                    "longitude": lon,
                    "accuracy": 1,
                })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        status.geolocation = Some(GeolocationOutput {
            latitude: lat,
            longitude: lon,
        });
    } else if args.no_geolocation {
        managed
            .send_command("Emulation.clearGeolocationOverride", None)
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;
        // geolocation remains None in status
    }

    // --- User Agent ---
    if let Some(ref ua) = args.user_agent {
        managed
            .send_command(
                "Emulation.setUserAgentOverride",
                Some(serde_json::json!({ "userAgent": ua })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        status.user_agent = Some(ua.clone());
    } else if args.no_user_agent {
        managed
            .send_command(
                "Emulation.setUserAgentOverride",
                Some(serde_json::json!({ "userAgent": "" })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;
        // user_agent remains None in status
    }

    // --- Color Scheme ---
    if let Some(ref scheme) = args.color_scheme {
        let value = match scheme {
            ColorScheme::Auto => "",
            ColorScheme::Dark => "dark",
            ColorScheme::Light => "light",
        };

        managed
            .send_command(
                "Emulation.setEmulatedMedia",
                Some(serde_json::json!({
                    "features": [{ "name": "prefers-color-scheme", "value": value }]
                })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        status.color_scheme = Some(color_scheme_name(*scheme).to_string());
    }

    // --- Viewport / Device Metrics ---
    if let Some(scale) = args.device_scale {
        if scale <= 0.0 {
            return Err(AppError::emulation_failed(
                "device scale factor must be a positive number",
            ));
        }
    }

    let viewport_requested = args.viewport.is_some() || args.device_scale.is_some() || args.mobile;

    if viewport_requested {
        let (width, height) = if let Some(ref vp_str) = args.viewport {
            parse_viewport(vp_str)?
        } else {
            // Query current viewport dimensions as defaults
            let result = managed
                .send_command(
                    "Runtime.evaluate",
                    Some(serde_json::json!({
                        "expression": "JSON.stringify({w:window.innerWidth,h:window.innerHeight})",
                        "returnByValue": true,
                    })),
                )
                .await
                .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

            let val_str = result["result"]["value"]
                .as_str()
                .unwrap_or(r#"{"w":1280,"h":720}"#);
            let dims: serde_json::Value =
                serde_json::from_str(val_str).unwrap_or(serde_json::json!({"w":1280,"h":720}));

            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let w = dims["w"].as_u64().unwrap_or(1280) as u32;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let h = dims["h"].as_u64().unwrap_or(720) as u32;
            (w, h)
        };

        let device_scale = args.device_scale.unwrap_or(1.0);
        let mobile = args.mobile;

        managed
            .send_command(
                "Emulation.setDeviceMetricsOverride",
                Some(serde_json::json!({
                    "width": width,
                    "height": height,
                    "deviceScaleFactor": device_scale,
                    "mobile": mobile,
                })),
            )
            .await
            .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

        if mobile {
            managed
                .send_command(
                    "Emulation.setTouchEmulationEnabled",
                    Some(serde_json::json!({ "enabled": true })),
                )
                .await
                .map_err(|e| AppError::emulation_failed(&e.to_string()))?;
        }

        status.viewport = Some(ViewportOutput { width, height });
        if args.device_scale.is_some() {
            status.device_scale_factor = Some(device_scale);
        }
        status.mobile = mobile;
    }

    // --- Persist CDP-only state ---
    let mut persisted = read_emulate_state()?.unwrap_or_default();
    if args.mobile || args.viewport.is_some() {
        persisted.mobile = args.mobile;
    }
    if args.network.is_some() {
        persisted.network.clone_from(&status.network);
    }
    if args.cpu.is_some() {
        persisted.cpu = status.cpu;
    }
    write_emulate_state(&persisted)?;

    // Output
    if global.output.plain {
        print!("{status}");
        return Ok(());
    }
    print_output(&status, &global.output)
}

// =============================================================================
// emulate reset
// =============================================================================

async fn execute_reset(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Reset network throttling
    managed
        .ensure_domain("Network")
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    let no_throttle = network_profile_params(NetworkProfile::None);
    managed
        .send_command("Network.emulateNetworkConditions", Some(no_throttle))
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Reset CPU throttling
    managed
        .send_command(
            "Emulation.setCPUThrottlingRate",
            Some(serde_json::json!({ "rate": 1 })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Clear geolocation
    managed
        .send_command("Emulation.clearGeolocationOverride", None)
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Reset user agent
    managed
        .send_command(
            "Emulation.setUserAgentOverride",
            Some(serde_json::json!({ "userAgent": "" })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Clear color scheme
    managed
        .send_command(
            "Emulation.setEmulatedMedia",
            Some(serde_json::json!({
                "features": [{ "name": "prefers-color-scheme", "value": "" }]
            })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Clear device metrics
    managed
        .send_command("Emulation.clearDeviceMetricsOverride", None)
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Disable touch emulation
    managed
        .send_command(
            "Emulation.setTouchEmulationEnabled",
            Some(serde_json::json!({ "enabled": false })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Clear persisted emulation state
    delete_emulate_state()?;

    let output = EmulateResetOutput { reset: true };

    if global.output.plain {
        println!("All emulation overrides have been reset.");
        return Ok(());
    }
    print_output(&output, &global.output)
}

// =============================================================================
// emulate status
// =============================================================================

async fn execute_status(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed
        .ensure_domain("Runtime")
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    // Query detectable settings via JavaScript
    let js = r"JSON.stringify({
        viewport: { width: window.innerWidth, height: window.innerHeight },
        userAgent: navigator.userAgent,
        darkMode: window.matchMedia('(prefers-color-scheme: dark)').matches,
        lightMode: window.matchMedia('(prefers-color-scheme: light)').matches,
        devicePixelRatio: window.devicePixelRatio
    })";

    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": js,
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    let val_str = result["result"]["value"].as_str().unwrap_or("{}");
    let data: serde_json::Value = serde_json::from_str(val_str).unwrap_or(serde_json::json!({}));

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let viewport = data.get("viewport").map(|vp| ViewportOutput {
        width: vp["width"].as_u64().unwrap_or(0) as u32,
        height: vp["height"].as_u64().unwrap_or(0) as u32,
    });

    let user_agent = data["userAgent"].as_str().map(String::from);

    let color_scheme = if data["darkMode"].as_bool() == Some(true) {
        Some("dark".to_string())
    } else if data["lightMode"].as_bool() == Some(true) {
        Some("light".to_string())
    } else {
        None
    };

    let device_scale_factor = data["devicePixelRatio"].as_f64();

    // Read persisted CDP-only state (mobile, network, cpu)
    let persisted = read_emulate_state()?.unwrap_or_default();

    let status = EmulateStatusOutput {
        network: persisted.network,
        cpu: persisted.cpu,
        geolocation: None,
        user_agent,
        color_scheme,
        viewport,
        device_scale_factor,
        mobile: persisted.mobile,
    };

    if global.output.plain {
        print!("{status}");
        return Ok(());
    }
    print_output(&status, &global.output)
}

// =============================================================================
// page resize (shared helper)
// =============================================================================

/// Execute the `page resize` shorthand command.
///
/// # Errors
///
/// Returns `AppError` if parsing or CDP fails.
pub async fn execute_resize(global: &GlobalOpts, size: &str) -> Result<(), AppError> {
    let (width, height) = parse_viewport(size)?;

    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed
        .send_command(
            "Emulation.setDeviceMetricsOverride",
            Some(serde_json::json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": 1,
                "mobile": false,
            })),
        )
        .await
        .map_err(|e| AppError::emulation_failed(&e.to_string()))?;

    let output = ResizeOutput { width, height };

    if global.output.plain {
        println!("Viewport resized to {width}x{height}");
        return Ok(());
    }
    print_output(&output, &global.output)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // parse_viewport tests
    // =========================================================================

    #[test]
    fn parse_viewport_valid() {
        let (w, h) = parse_viewport("1280x720").unwrap();
        assert_eq!(w, 1280);
        assert_eq!(h, 720);
    }

    #[test]
    fn parse_viewport_mobile() {
        let (w, h) = parse_viewport("375x667").unwrap();
        assert_eq!(w, 375);
        assert_eq!(h, 667);
    }

    #[test]
    fn parse_viewport_invalid_no_x() {
        let err = parse_viewport("bad").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
        assert!(err.message.contains("bad"));
    }

    #[test]
    fn parse_viewport_invalid_letters() {
        let err = parse_viewport("abcxdef").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
    }

    #[test]
    fn parse_viewport_zero_width() {
        let err = parse_viewport("0x720").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
    }

    #[test]
    fn parse_viewport_zero_height() {
        let err = parse_viewport("1280x0").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
    }

    #[test]
    fn parse_viewport_negative() {
        let err = parse_viewport("-1x100").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
    }

    #[test]
    fn parse_viewport_too_many_parts() {
        let err = parse_viewport("100x200x300").unwrap_err();
        assert!(err.message.contains("WIDTHxHEIGHT"));
    }

    // =========================================================================
    // parse_geolocation tests
    // =========================================================================

    #[test]
    fn parse_geolocation_valid() {
        let (lat, lon) = parse_geolocation("37.7749,-122.4194").unwrap();
        assert!((lat - 37.7749).abs() < f64::EPSILON);
        assert!((lon - (-122.4194)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_geolocation_zero() {
        let (lat, lon) = parse_geolocation("0,0").unwrap();
        assert!((lat).abs() < f64::EPSILON);
        assert!((lon).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_geolocation_invalid_text() {
        let err = parse_geolocation("not-a-coord").unwrap_err();
        assert!(err.message.contains("LAT,LONG"));
    }

    #[test]
    fn parse_geolocation_missing_longitude() {
        let err = parse_geolocation("37.7749").unwrap_err();
        assert!(err.message.contains("LAT,LONG"));
    }

    #[test]
    fn parse_geolocation_latitude_out_of_range() {
        let err = parse_geolocation("91.0,0.0").unwrap_err();
        assert!(err.message.contains("LAT,LONG"));
    }

    #[test]
    fn parse_geolocation_longitude_out_of_range() {
        let err = parse_geolocation("0.0,181.0").unwrap_err();
        assert!(err.message.contains("LAT,LONG"));
    }

    #[test]
    fn parse_geolocation_too_many_parts() {
        let err = parse_geolocation("37.7749,-122.4194,0").unwrap_err();
        assert!(err.message.contains("LAT,LONG"));
    }

    // =========================================================================
    // network_profile_params tests
    // =========================================================================

    #[test]
    fn network_profile_offline() {
        let params = network_profile_params(NetworkProfile::Offline);
        assert_eq!(params["offline"], true);
        assert_eq!(params["downloadThroughput"], 0);
        assert_eq!(params["uploadThroughput"], 0);
    }

    #[test]
    fn network_profile_slow_4g() {
        let params = network_profile_params(NetworkProfile::Slow4g);
        assert_eq!(params["offline"], false);
        assert_eq!(params["latency"], 150);
        assert_eq!(params["downloadThroughput"], 1_600_000);
        assert_eq!(params["uploadThroughput"], 750_000);
    }

    #[test]
    fn network_profile_4g() {
        let params = network_profile_params(NetworkProfile::FourG);
        assert_eq!(params["offline"], false);
        assert_eq!(params["latency"], 20);
        assert_eq!(params["downloadThroughput"], 4_000_000);
        assert_eq!(params["uploadThroughput"], 3_000_000);
    }

    #[test]
    fn network_profile_3g() {
        let params = network_profile_params(NetworkProfile::ThreeG);
        assert_eq!(params["offline"], false);
        assert_eq!(params["latency"], 100);
        assert_eq!(params["downloadThroughput"], 750_000);
        assert_eq!(params["uploadThroughput"], 250_000);
    }

    #[test]
    fn network_profile_none() {
        let params = network_profile_params(NetworkProfile::None);
        assert_eq!(params["offline"], false);
        assert_eq!(params["latency"], 0);
        assert_eq!(params["downloadThroughput"], -1);
        assert_eq!(params["uploadThroughput"], -1);
    }

    // =========================================================================
    // Output serialization tests
    // =========================================================================

    #[test]
    fn emulate_status_output_full() {
        let output = EmulateStatusOutput {
            network: Some("slow-4g".to_string()),
            cpu: Some(4),
            geolocation: Some(GeolocationOutput {
                latitude: 37.7749,
                longitude: -122.4194,
            }),
            user_agent: Some("Mozilla/5.0 Custom".to_string()),
            color_scheme: Some("dark".to_string()),
            viewport: Some(ViewportOutput {
                width: 375,
                height: 667,
            }),
            device_scale_factor: Some(2.0),
            mobile: true,
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert_eq!(json["network"], "slow-4g");
        assert_eq!(json["cpu"], 4);
        assert_eq!(json["geolocation"]["latitude"], 37.7749);
        assert_eq!(json["geolocation"]["longitude"], -122.4194);
        assert_eq!(json["userAgent"], "Mozilla/5.0 Custom");
        assert_eq!(json["colorScheme"], "dark");
        assert_eq!(json["viewport"]["width"], 375);
        assert_eq!(json["viewport"]["height"], 667);
        assert_eq!(json["deviceScaleFactor"], 2.0);
        assert_eq!(json["mobile"], true);
    }

    #[test]
    fn emulate_status_output_minimal() {
        let output = EmulateStatusOutput {
            network: None,
            cpu: None,
            geolocation: None,
            user_agent: None,
            color_scheme: None,
            viewport: None,
            device_scale_factor: None,
            mobile: false,
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert!(json.get("network").is_none());
        assert!(json.get("cpu").is_none());
        assert!(json.get("geolocation").is_none());
        assert!(json.get("userAgent").is_none());
        assert!(json.get("colorScheme").is_none());
        assert!(json.get("viewport").is_none());
        assert!(json.get("deviceScaleFactor").is_none());
        assert_eq!(json["mobile"], false);
    }

    #[test]
    fn emulate_reset_output() {
        let output = EmulateResetOutput { reset: true };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert_eq!(json["reset"], true);
    }

    #[test]
    fn resize_output() {
        let output = ResizeOutput {
            width: 1280,
            height: 720,
        };
        let json: serde_json::Value = serde_json::to_value(&output).unwrap();
        assert_eq!(json["width"], 1280);
        assert_eq!(json["height"], 720);
    }

    #[test]
    fn emulate_status_display_full() {
        let output = EmulateStatusOutput {
            network: Some("slow-4g".to_string()),
            cpu: Some(4),
            geolocation: Some(GeolocationOutput {
                latitude: 37.7749,
                longitude: -122.4194,
            }),
            user_agent: Some("Custom UA".to_string()),
            color_scheme: Some("dark".to_string()),
            viewport: Some(ViewportOutput {
                width: 375,
                height: 667,
            }),
            device_scale_factor: Some(2.0),
            mobile: true,
        };
        let text = format!("{output}");
        assert!(text.contains("Network: slow-4g"));
        assert!(text.contains("CPU throttling: 4x"));
        assert!(text.contains("37.7749,-122.4194"));
        assert!(text.contains("User-Agent: Custom UA"));
        assert!(text.contains("Color scheme: dark"));
        assert!(text.contains("Viewport: 375x667"));
        assert!(text.contains("Device scale: 2"));
        assert!(text.contains("Mobile: true"));
    }

    #[test]
    fn emulate_status_display_empty() {
        let output = EmulateStatusOutput {
            network: None,
            cpu: None,
            geolocation: None,
            user_agent: None,
            color_scheme: None,
            viewport: None,
            device_scale_factor: None,
            mobile: false,
        };
        let text = format!("{output}");
        assert!(text.is_empty());
    }

    // =========================================================================
    // EmulateState persistence tests
    // =========================================================================

    #[test]
    fn emulate_state_round_trip() {
        let dir = std::env::temp_dir().join("chrome-cli-test-emstate-rt");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("emulate-state.json");

        let state = EmulateState {
            mobile: true,
            network: Some("slow-4g".to_string()),
            cpu: Some(4),
        };

        write_emulate_state_to(&path, &state).unwrap();
        let read = read_emulate_state_from(&path).unwrap().unwrap();

        assert!(read.mobile);
        assert_eq!(read.network.as_deref(), Some("slow-4g"));
        assert_eq!(read.cpu, Some(4));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn emulate_state_read_when_missing_returns_none() {
        let path = Path::new("/tmp/chrome-cli-test-emstate-missing/emulate-state.json");
        let result = read_emulate_state_from(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn emulate_state_delete_when_missing_returns_ok() {
        let path = Path::new("/tmp/chrome-cli-test-emstate-del-missing/emulate-state.json");
        assert!(delete_emulate_state_from(path).is_ok());
    }

    #[test]
    fn emulate_state_delete_existing_removes_file() {
        let dir = std::env::temp_dir().join("chrome-cli-test-emstate-del");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("emulate-state.json");
        std::fs::write(&path, "{}").unwrap();
        assert!(path.exists());

        delete_emulate_state_from(&path).unwrap();
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn emulate_state_default_values() {
        let state = EmulateState::default();
        assert!(!state.mobile);
        assert!(state.network.is_none());
        assert!(state.cpu.is_none());
    }

    #[test]
    fn emulate_state_skips_none_fields_in_json() {
        let state = EmulateState {
            mobile: false,
            network: None,
            cpu: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("network"));
        assert!(!json.contains("cpu"));
    }
}
