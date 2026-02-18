#![allow(unused_imports)]

mod discovery;
mod error;
mod launcher;
mod platform;

pub use discovery::{
    BrowserVersion, TargetInfo, activate_target, discover_chrome, query_targets, query_version,
    read_devtools_active_port, read_devtools_active_port_from,
};
pub use error::ChromeError;
pub use launcher::{ChromeProcess, LaunchConfig, find_available_port, launch_chrome};
pub use platform::{Channel, default_user_data_dir, find_chrome_executable};
