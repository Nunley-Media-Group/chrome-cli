use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default config file template with comments, used by `config init`.
const DEFAULT_CONFIG_TEMPLATE: &str = r#"# chrome-cli configuration file
# See: https://github.com/Nunley-Media-Group/chrome-cli

# Connection defaults
# [connection]
# host = "127.0.0.1"
# port = 9222
# timeout_ms = 30000

# Chrome launch defaults
# [launch]
# executable = "/path/to/chrome"
# channel = "stable"        # stable, beta, dev, canary
# headless = false
# extra_args = ["--disable-gpu"]

# Output defaults
# [output]
# format = "json"           # json, pretty, plain

# Default tab behavior
# [tabs]
# auto_activate = true
# filter_internal = true
"#;

// ---------------------------------------------------------------------------
// Config structs (parsed from TOML)
// ---------------------------------------------------------------------------

/// Represents the parsed TOML config file. All fields optional.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
    pub connection: ConnectionConfig,
    pub launch: LaunchConfig,
    pub output: OutputConfig,
    pub tabs: TabsConfig,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct LaunchConfig {
    pub executable: Option<String>,
    pub channel: Option<String>,
    pub headless: Option<bool>,
    pub extra_args: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct OutputConfig {
    pub format: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TabsConfig {
    pub auto_activate: Option<bool>,
    pub filter_internal: Option<bool>,
}

// ---------------------------------------------------------------------------
// Resolved config (all defaults filled in)
// ---------------------------------------------------------------------------

/// Fully resolved configuration with all defaults filled in.
#[derive(Debug, Serialize)]
pub struct ResolvedConfig {
    pub config_path: Option<PathBuf>,
    pub connection: ResolvedConnection,
    pub launch: ResolvedLaunch,
    pub output: ResolvedOutput,
    pub tabs: ResolvedTabs,
}

#[derive(Debug, Serialize)]
pub struct ResolvedConnection {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ResolvedLaunch {
    pub executable: Option<String>,
    pub channel: String,
    pub headless: bool,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ResolvedOutput {
    pub format: String,
}

#[derive(Debug, Serialize)]
pub struct ResolvedTabs {
    pub auto_activate: bool,
    pub filter_internal: bool,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ConfigError {
    /// I/O error reading/writing config file.
    Io(std::io::Error),
    /// Config file already exists (for `config init`).
    AlreadyExists(PathBuf),
    /// Could not determine config directory.
    NoConfigDir,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config file error: {e}"),
            Self::AlreadyExists(p) => {
                write!(f, "Config file already exists: {}", p.display())
            }
            Self::NoConfigDir => write!(f, "could not determine config directory"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ConfigError> for crate::error::AppError {
    fn from(e: ConfigError) -> Self {
        use crate::error::ExitCode;
        Self {
            message: e.to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Config file search
// ---------------------------------------------------------------------------

/// Find the first config file that exists, checking locations in priority order.
///
/// Search order:
/// 1. `explicit_path` (from `--config` flag)
/// 2. `$CHROME_CLI_CONFIG` environment variable
/// 3. `./.chrome-cli.toml` (project-local)
/// 4. `<config_dir>/chrome-cli/config.toml` (XDG / platform config dir)
/// 5. `~/.chrome-cli.toml` (home directory fallback)
#[must_use]
pub fn find_config_file(explicit_path: Option<&Path>) -> Option<PathBuf> {
    find_config_file_with(explicit_path, std::env::var("CHROME_CLI_CONFIG").ok())
}

/// Testable variant of [`find_config_file`] that accepts an explicit env value.
#[must_use]
pub fn find_config_file_with(
    explicit_path: Option<&Path>,
    env_config: Option<String>,
) -> Option<PathBuf> {
    // 1. Explicit --config path
    if let Some(p) = explicit_path {
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    // 2. $CHROME_CLI_CONFIG
    if let Some(env_path) = env_config {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 3. ./.chrome-cli.toml (project-local)
    let local = PathBuf::from(".chrome-cli.toml");
    if local.exists() {
        return Some(local);
    }

    // 4. XDG / platform config dir
    if let Some(config_dir) = dirs::config_dir() {
        let xdg = config_dir.join("chrome-cli").join("config.toml");
        if xdg.exists() {
            return Some(xdg);
        }
    }

    // 5. ~/.chrome-cli.toml
    if let Some(home) = dirs::home_dir() {
        let home_config = home.join(".chrome-cli.toml");
        if home_config.exists() {
            return Some(home_config);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Config loading
// ---------------------------------------------------------------------------

/// Load and parse a config file. Returns the file path (if found) and the parsed config.
///
/// On parse errors, prints a warning to stderr and returns `ConfigFile::default()`.
#[must_use]
pub fn load_config(explicit_path: Option<&Path>) -> (Option<PathBuf>, ConfigFile) {
    let path = find_config_file(explicit_path);
    match &path {
        Some(p) => {
            let config = load_config_from(p);
            (path, config)
        }
        None => (None, ConfigFile::default()),
    }
}

/// Load and parse a config file from a specific path.
///
/// On parse errors, prints a warning to stderr and returns `ConfigFile::default()`.
#[must_use]
pub fn load_config_from(path: &Path) -> ConfigFile {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "warning: could not read config file {}: {e}",
                path.display()
            );
            return ConfigFile::default();
        }
    };

    parse_config(&contents, path)
}

/// Parse TOML content into a `ConfigFile`.
///
/// Uses a two-pass strategy: first tries strict parsing (to detect unknown keys),
/// then falls back to lenient parsing if strict fails due to unknown fields.
#[must_use]
pub fn parse_config(contents: &str, path: &Path) -> ConfigFile {
    // First pass: strict (deny_unknown_fields via a wrapper)
    match toml::from_str::<StrictConfigFile>(contents) {
        Ok(strict) => strict.into(),
        Err(strict_err) => {
            // Second pass: lenient
            match toml::from_str::<ConfigFile>(contents) {
                Ok(config) => {
                    // Strict failed but lenient succeeded → unknown keys
                    eprintln!(
                        "warning: unknown keys in config file {}: {strict_err}",
                        path.display()
                    );
                    config
                }
                Err(parse_err) => {
                    // Both failed → invalid TOML
                    eprintln!(
                        "warning: could not parse config file {}: {parse_err}",
                        path.display()
                    );
                    ConfigFile::default()
                }
            }
        }
    }
}

/// Strict variant used for the first-pass parse to detect unknown keys.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictConfigFile {
    #[serde(default)]
    connection: StrictConnectionConfig,
    #[serde(default)]
    launch: StrictLaunchConfig,
    #[serde(default)]
    output: StrictOutputConfig,
    #[serde(default)]
    tabs: StrictTabsConfig,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictConnectionConfig {
    host: Option<String>,
    port: Option<u16>,
    timeout_ms: Option<u64>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictLaunchConfig {
    executable: Option<String>,
    channel: Option<String>,
    headless: Option<bool>,
    extra_args: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictOutputConfig {
    format: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictTabsConfig {
    auto_activate: Option<bool>,
    filter_internal: Option<bool>,
}

impl From<StrictConfigFile> for ConfigFile {
    fn from(s: StrictConfigFile) -> Self {
        Self {
            connection: ConnectionConfig {
                host: s.connection.host,
                port: s.connection.port,
                timeout_ms: s.connection.timeout_ms,
            },
            launch: LaunchConfig {
                executable: s.launch.executable,
                channel: s.launch.channel,
                headless: s.launch.headless,
                extra_args: s.launch.extra_args,
            },
            output: OutputConfig {
                format: s.output.format,
            },
            tabs: TabsConfig {
                auto_activate: s.tabs.auto_activate,
                filter_internal: s.tabs.filter_internal,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Config resolution
// ---------------------------------------------------------------------------

/// Default port for CDP connections.
const DEFAULT_PORT: u16 = 9222;
/// Default timeout for commands in milliseconds.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Resolve a config file into a fully-populated `ResolvedConfig` with all defaults.
#[must_use]
pub fn resolve_config(file: &ConfigFile, config_path: Option<PathBuf>) -> ResolvedConfig {
    let port = file.connection.port.unwrap_or(DEFAULT_PORT);
    let port = if port == 0 { DEFAULT_PORT } else { port };

    ResolvedConfig {
        config_path,
        connection: ResolvedConnection {
            host: file
                .connection
                .host
                .clone()
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            port,
            timeout_ms: file.connection.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS),
        },
        launch: ResolvedLaunch {
            executable: file.launch.executable.clone(),
            channel: file
                .launch
                .channel
                .clone()
                .unwrap_or_else(|| "stable".to_string()),
            headless: file.launch.headless.unwrap_or(false),
            extra_args: file.launch.extra_args.clone().unwrap_or_default(),
        },
        output: ResolvedOutput {
            format: file
                .output
                .format
                .clone()
                .unwrap_or_else(|| "json".to_string()),
        },
        tabs: ResolvedTabs {
            auto_activate: file.tabs.auto_activate.unwrap_or(true),
            filter_internal: file.tabs.filter_internal.unwrap_or(true),
        },
    }
}

// ---------------------------------------------------------------------------
// Config init
// ---------------------------------------------------------------------------

/// Default path for `config init`: `<config_dir>/chrome-cli/config.toml`.
///
/// # Errors
///
/// Returns `ConfigError::NoConfigDir` if the platform config directory cannot be determined.
pub fn default_init_path() -> Result<PathBuf, ConfigError> {
    dirs::config_dir()
        .map(|d| d.join("chrome-cli").join("config.toml"))
        .ok_or(ConfigError::NoConfigDir)
}

/// Create a default config file at the given path (or the default XDG path).
///
/// # Errors
///
/// - `ConfigError::AlreadyExists` if the file already exists
/// - `ConfigError::Io` on I/O failure
/// - `ConfigError::NoConfigDir` if no target path and platform config dir unknown
pub fn init_config(target_path: Option<&Path>) -> Result<PathBuf, ConfigError> {
    let path = match target_path {
        Some(p) => p.to_path_buf(),
        None => default_init_path()?,
    };

    init_config_to(&path)
}

/// Testable variant of [`init_config`] that writes to an explicit path.
///
/// # Errors
///
/// - `ConfigError::AlreadyExists` if the file already exists
/// - `ConfigError::Io` on I/O failure
pub fn init_config_to(path: &Path) -> Result<PathBuf, ConfigError> {
    if path.exists() {
        return Err(ConfigError::AlreadyExists(path.to_path_buf()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, DEFAULT_CONFIG_TEMPLATE)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path.to_path_buf())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_full_config() {
        let toml = r#"
[connection]
host = "10.0.0.1"
port = 9333
timeout_ms = 60000

[launch]
executable = "/usr/bin/chromium"
channel = "beta"
headless = true
extra_args = ["--disable-gpu", "--no-sandbox"]

[output]
format = "pretty"

[tabs]
auto_activate = false
filter_internal = false
"#;
        let config = parse_config(toml, Path::new("test.toml"));
        assert_eq!(config.connection.host.as_deref(), Some("10.0.0.1"));
        assert_eq!(config.connection.port, Some(9333));
        assert_eq!(config.connection.timeout_ms, Some(60000));
        assert_eq!(
            config.launch.executable.as_deref(),
            Some("/usr/bin/chromium")
        );
        assert_eq!(config.launch.channel.as_deref(), Some("beta"));
        assert_eq!(config.launch.headless, Some(true));
        assert_eq!(
            config.launch.extra_args.as_deref(),
            Some(&["--disable-gpu".to_string(), "--no-sandbox".to_string()][..])
        );
        assert_eq!(config.output.format.as_deref(), Some("pretty"));
        assert_eq!(config.tabs.auto_activate, Some(false));
        assert_eq!(config.tabs.filter_internal, Some(false));
    }

    #[test]
    fn parse_empty_config() {
        let config = parse_config("", Path::new("test.toml"));
        assert!(config.connection.host.is_none());
        assert!(config.connection.port.is_none());
        assert!(config.launch.executable.is_none());
        assert!(config.output.format.is_none());
        assert!(config.tabs.auto_activate.is_none());
    }

    #[test]
    fn parse_partial_config() {
        let toml = "[connection]\nport = 9333\n";
        let config = parse_config(toml, Path::new("test.toml"));
        assert_eq!(config.connection.port, Some(9333));
        assert!(config.connection.host.is_none());
        assert!(config.launch.executable.is_none());
    }

    #[test]
    fn parse_invalid_toml_returns_default() {
        let config = parse_config("this is not valid toml [[[", Path::new("test.toml"));
        assert!(config.connection.host.is_none());
        assert!(config.connection.port.is_none());
    }

    #[test]
    fn parse_unknown_keys_warns_but_keeps_known() {
        let toml = r#"
[connection]
port = 9333
unknown_key = "hello"
"#;
        let config = parse_config(toml, Path::new("test.toml"));
        assert_eq!(config.connection.port, Some(9333));
    }

    #[test]
    fn resolve_defaults() {
        let config = ConfigFile::default();
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.connection.host, "127.0.0.1");
        assert_eq!(resolved.connection.port, DEFAULT_PORT);
        assert_eq!(resolved.connection.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert_eq!(resolved.launch.channel, "stable");
        assert!(!resolved.launch.headless);
        assert!(resolved.launch.extra_args.is_empty());
        assert_eq!(resolved.output.format, "json");
        assert!(resolved.tabs.auto_activate);
        assert!(resolved.tabs.filter_internal);
        assert!(resolved.config_path.is_none());
    }

    #[test]
    fn resolve_overrides() {
        let config = ConfigFile {
            connection: ConnectionConfig {
                host: Some("10.0.0.1".into()),
                port: Some(9444),
                timeout_ms: Some(5000),
            },
            launch: LaunchConfig {
                executable: Some("/usr/bin/chromium".into()),
                channel: Some("canary".into()),
                headless: Some(true),
                extra_args: Some(vec!["--no-sandbox".into()]),
            },
            output: OutputConfig {
                format: Some("pretty".into()),
            },
            tabs: TabsConfig {
                auto_activate: Some(false),
                filter_internal: Some(false),
            },
        };
        let path = PathBuf::from("/tmp/test.toml");
        let resolved = resolve_config(&config, Some(path.clone()));
        assert_eq!(resolved.connection.host, "10.0.0.1");
        assert_eq!(resolved.connection.port, 9444);
        assert_eq!(resolved.connection.timeout_ms, 5000);
        assert_eq!(
            resolved.launch.executable.as_deref(),
            Some("/usr/bin/chromium")
        );
        assert_eq!(resolved.launch.channel, "canary");
        assert!(resolved.launch.headless);
        assert_eq!(resolved.launch.extra_args, vec!["--no-sandbox"]);
        assert_eq!(resolved.output.format, "pretty");
        assert!(!resolved.tabs.auto_activate);
        assert!(!resolved.tabs.filter_internal);
        assert_eq!(resolved.config_path, Some(path));
    }

    #[test]
    fn resolve_port_zero_uses_default() {
        let config = ConfigFile {
            connection: ConnectionConfig {
                port: Some(0),
                ..ConnectionConfig::default()
            },
            ..ConfigFile::default()
        };
        let resolved = resolve_config(&config, None);
        assert_eq!(resolved.connection.port, DEFAULT_PORT);
    }

    #[test]
    fn init_config_creates_file() {
        let dir = std::env::temp_dir().join("chrome-cli-test-config-init");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");

        let result = init_config_to(&path);
        assert!(result.is_ok());
        assert!(path.exists());

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("[connection]"));
        assert!(contents.contains("port = 9222"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn init_config_refuses_overwrite() {
        let dir = std::env::temp_dir().join("chrome-cli-test-config-overwrite");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "existing").unwrap();

        let result = init_config_to(&path);
        assert!(matches!(result, Err(ConfigError::AlreadyExists(_))));

        // Verify original content not overwritten
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "existing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_config_with_explicit_path() {
        let dir = std::env::temp_dir().join("chrome-cli-test-find-explicit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("my-config.toml");
        std::fs::write(&path, "").unwrap();

        let found = find_config_file_with(Some(&path), None);
        assert_eq!(found, Some(path.clone()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_config_with_env_var() {
        let dir = std::env::temp_dir().join("chrome-cli-test-find-env");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("env-config.toml");
        std::fs::write(&path, "").unwrap();

        let found = find_config_file_with(None, Some(path.to_string_lossy().into_owned()));
        assert_eq!(found, Some(path.clone()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_config_explicit_takes_priority_over_env() {
        let dir = std::env::temp_dir().join("chrome-cli-test-find-priority");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let explicit = dir.join("explicit.toml");
        let env = dir.join("env.toml");
        std::fs::write(&explicit, "").unwrap();
        std::fs::write(&env, "").unwrap();

        let found =
            find_config_file_with(Some(&explicit), Some(env.to_string_lossy().into_owned()));
        assert_eq!(found, Some(explicit.clone()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_config_nonexistent_returns_none() {
        let found = find_config_file_with(
            Some(Path::new("/nonexistent/path.toml")),
            Some("/also/nonexistent.toml".into()),
        );
        // May or may not find a config from project-local / home — but explicit and env should fail.
        // We can't guarantee None here due to project-local or home checks, so just verify
        // the explicit and env paths didn't match.
        if let Some(ref p) = found {
            assert_ne!(p, &PathBuf::from("/nonexistent/path.toml"));
            assert_ne!(p, &PathBuf::from("/also/nonexistent.toml"));
        }
    }

    #[test]
    fn load_config_from_nonexistent_returns_default() {
        let config = load_config_from(Path::new("/nonexistent/config.toml"));
        assert!(config.connection.host.is_none());
    }

    #[test]
    fn config_error_display() {
        assert!(
            ConfigError::NoConfigDir
                .to_string()
                .contains("config directory")
        );

        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert!(ConfigError::Io(io_err).to_string().contains("denied"));

        let path = PathBuf::from("/tmp/test.toml");
        let msg = ConfigError::AlreadyExists(path).to_string();
        assert!(msg.contains("already exists"));
        assert!(msg.contains("/tmp/test.toml"));
    }

    #[test]
    fn config_serializes_to_json() {
        let config = ConfigFile::default();
        let resolved = resolve_config(&config, None);
        let json = serde_json::to_string(&resolved).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["connection"]["port"], 9222);
        assert_eq!(parsed["connection"]["host"], "127.0.0.1");
        assert_eq!(parsed["output"]["format"], "json");
    }
}
