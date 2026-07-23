//! Plugin system for loading dynamic libraries.

use libloading::{Library, Symbol};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load plugin: {0}")]
    LoadError(String),

    #[error("Invalid manifest: {0}")]
    ManifestError(String),

    #[error("Plugin API mismatch: {0}")]
    ApiMismatch(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;

/// Plugin information exported from dynamic library
#[repr(C)]
pub struct PluginInfo {
    pub name: *const libc::c_char,
    pub version: *const libc::c_char,
}

impl PluginInfo {
    fn to_string(&self) -> (String, String) {
        unsafe {
            let name = if self.name.is_null() {
                "unknown".to_string()
            } else {
                std::ffi::CStr::from_ptr(self.name).to_string_lossy().into_owned()
            };
            let version = if self.version.is_null() {
                "0.0.0".to_string()
            } else {
                std::ffi::CStr::from_ptr(self.version).to_string_lossy().into_owned()
            };
            (name, version)
        }
    }
}

/// Core API that plugins can call
#[repr(C)]
pub struct CoreApi {
    pub send_event: Option<unsafe extern "C" fn(*const u8, usize)>,
    pub get_config: Option<unsafe extern "C" fn(*mut u8, usize) -> usize>,
}

/// Load a plugin from a dynamic library file
pub fn load_plugin<P: AsRef<Path>>(path: P) -> Result<LoadedPlugin> {
    let lib = unsafe { Library::new(path.as_ref()) }
        .map_err(|e| PluginError::LoadError(e.to_string()))?;

    // Get plugin info
    let info_fn: Symbol<unsafe extern "C" fn() -> PluginInfo> = unsafe {
        lib.get(b"plugin_info")
            .map_err(|e| PluginError::LoadError(e.to_string()))?
    };

    let info = unsafe { info_fn() };
    let (name, version) = info.to_string();

    // Get plugin init function
    let init_fn: Symbol<unsafe extern "C" fn(CoreApi) -> i32> = unsafe {
        lib.get(b"plugin_init")
            .map_err(|e| PluginError::LoadError(e.to_string()))?
    };

    // Initialize with empty core API (TODO: implement real API)
    let core_api = CoreApi {
        send_event: None,
        get_config: None,
    };

    let result = unsafe { init_fn(core_api) };
    if result != 0 {
        return Err(PluginError::LoadError(format!("Init failed with code {}", result)));
    }

    Ok(LoadedPlugin { _lib: lib, name, version })
}

/// A loaded plugin instance
pub struct LoadedPlugin {
    _lib: Library, // Keep library alive
    pub name: String,
    pub version: String,
}

impl std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

/// Parse plugin manifest from TOML file
pub fn parse_manifest<P: AsRef<Path>>(path: P) -> Result<conclave_protocol::PluginManifest> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| PluginError::ManifestError(e.to_string()))?;

    toml::from_str(&content)
        .map_err(|e| PluginError::ManifestError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let manifest_toml = r#"
            name = "test-plugin"
            version = "1.0.0"
            core_version = "0.1.0"
            author = "Test Author"
            verified = true
        "#;

        std::fs::write("/tmp/test_plugin.toml", manifest_toml).unwrap();
        let manifest = parse_manifest("/tmp/test_plugin.toml").unwrap();
        
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
    }
}
