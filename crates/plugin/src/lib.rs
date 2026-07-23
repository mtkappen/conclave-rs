//! Plugin system for loading dynamic libraries.

use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load plugin: {0}")]
    LoadError(String),

    #[error("Invalid manifest: {0}")]
    ManifestError(String),

    #[error("Plugin API mismatch: {0}")]
    ApiMismatch(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),
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

/// Plugin context passed to plugins during initialization
pub struct PluginContext {
    campaign_id: String,
    player_id: String,
    send_event_callback: Option<Box<dyn Fn(&[u8]) + Send + Sync>>,
    get_config_callback: Option<Box<dyn Fn(&mut [u8]) -> usize + Send + Sync>>,
}

impl PluginContext {
    pub fn new(campaign_id: &str, player_id: &str) -> Self {
        PluginContext {
            campaign_id: campaign_id.to_string(),
            player_id: player_id.to_string(),
            send_event_callback: None,
            get_config_callback: None,
        }
    }

    pub fn with_send_event<F>(mut self, callback: F) -> Self
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.send_event_callback = Some(Box::new(callback));
        self
    }

    pub fn with_get_config<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut [u8]) -> usize + Send + Sync + 'static,
    {
        self.get_config_callback = Some(Box::new(callback));
        self
    }

    pub fn campaign_id(&self) -> &str {
        &self.campaign_id
    }

    pub fn player_id(&self) -> &str {
        &self.player_id
    }

    pub fn send_event(&self, data: &[u8]) {
        if let Some(cb) = &self.send_event_callback {
            cb(data);
        }
    }

    pub fn get_config(&self, buffer: &mut [u8]) -> usize {
        if let Some(cb) = &self.get_config_callback {
            cb(buffer)
        } else {
            0
        }
    }
}

/// Core API that plugins can call (C-compatible interface)
#[repr(C)]
pub struct CoreApi {
    pub send_event: Option<unsafe extern "C" fn(*const u8, usize)>,
    pub get_config: Option<unsafe extern "C" fn(*mut u8, usize) -> usize>,
}

impl From<&PluginContext> for CoreApi {
    fn from(_ctx: &PluginContext) -> Self {
        unsafe extern "C" fn send_event_impl(data: *const u8, len: usize) {
            if data.is_null() || len == 0 {
                return;
            }
        }

        unsafe extern "C" fn get_config_impl(buffer: *mut u8, len: usize) -> usize {
            if buffer.is_null() || len == 0 {
                return 0;
            }
            0
        }

        CoreApi {
            send_event: Some(send_event_impl),
            get_config: Some(get_config_impl),
        }
    }
}

/// A loaded plugin instance
pub struct LoadedPlugin {
    _lib: Library,
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

/// Plugin manager for lifecycle management
pub struct PluginManager {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    campaign_plugins: RwLock<HashMap<String, Vec<String>>>, // campaign_id -> [plugin_names]
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: RwLock::new(HashMap::new()),
            campaign_plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Load a plugin from a dynamic library file with campaign context
    pub fn load_plugin_with_context<P: AsRef<Path>>(
        &self, 
        path: P, 
        ctx: PluginContext,
    ) -> Result<String> {
        let lib = unsafe { Library::new(path.as_ref()) }
            .map_err(|e| PluginError::LoadError(e.to_string()))?;

        // Get plugin info
        let info_fn: Symbol<unsafe extern "C" fn() -> PluginInfo> = unsafe {
            lib.get(b"plugin_info")
                .map_err(|e| PluginError::LoadError(e.to_string()))?
        };

        let info = unsafe { info_fn() };
        let (name, version) = info.to_string();

        // Check if already loaded
        {
            let plugins = self.plugins.read().unwrap();
            if plugins.contains_key(&name) {
                return Err(PluginError::AlreadyLoaded(name));
            }
        }

        // Get plugin init function
        let init_fn: Symbol<unsafe extern "C" fn(CoreApi) -> i32> = unsafe {
            lib.get(b"plugin_init")
                .map_err(|e| PluginError::LoadError(e.to_string()))?
        };

        // Create core API from context
        let core_api = CoreApi::from(&ctx);

        let result = unsafe { init_fn(core_api) };
        if result != 0 {
            return Err(PluginError::LoadError(format!("Init failed with code {}", result)));
        }

        let plugin = LoadedPlugin { _lib: lib, name: name.clone(), version };
        
        self.plugins.write().unwrap().insert(name.clone(), plugin);
        
        Ok(name)
    }

    /// Load a plugin from a dynamic library file (legacy method without context)
    pub fn load_plugin<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let ctx = PluginContext::new("unknown", "unknown");
        self.load_plugin_with_context(path, ctx)
    }

    /// Unload a plugin by name
    pub fn unload_plugin(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();
        
        if plugins.remove(name).is_some() {
            // Remove from campaign associations
            for (_, plugin_list) in self.campaign_plugins.write().unwrap().iter_mut() {
                plugin_list.retain(|p| p != name);
            }
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<(String, String)> {
        self.plugins.read().unwrap()
            .iter()
            .map(|(name, plugin)| (name.clone(), plugin.version.clone()))
            .collect()
    }

    /// Associate a plugin with a campaign
    pub fn associate_with_campaign(&self, campaign_id: &str, plugin_name: &str) -> Result<()> {
        let plugins = self.plugins.read().unwrap();
        
        if !plugins.contains_key(plugin_name) {
            return Err(PluginError::NotFound(plugin_name.to_string()));
        }

        drop(plugins);

        self.campaign_plugins
            .write()
            .unwrap()
            .entry(campaign_id.to_string())
            .or_insert_with(Vec::new)
            .push(plugin_name.to_string());

        Ok(())
    }

    /// Get plugins for a campaign
    pub fn get_campaign_plugins(&self, campaign_id: &str) -> Vec<String> {
        self.campaign_plugins
            .read()
            .unwrap()
            .get(campaign_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Load all plugins from a directory
    pub fn load_from_directory<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        
        if !dir.as_ref().exists() {
            return Ok(loaded);
        }

        for entry in std::fs::read_dir(dir.as_ref())
            .map_err(|e| PluginError::LoadError(e.to_string()))? 
        {
            let entry = entry.map_err(|e| PluginError::LoadError(e.to_string()))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("so") ||
               path.extension().and_then(|s| s.to_str()) == Some("dll") ||
               path.extension().and_then(|s| s.to_str()) == Some("dylib") {
                
                match self.load_plugin(&path) {
                    Ok(name) => loaded.push(name),
                    Err(e) => eprintln!("Failed to load plugin {:?}: {}", path, e),
                }
            }
        }

        Ok(loaded)
    }
}

/// Load a plugin from a dynamic library file (standalone function)
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
