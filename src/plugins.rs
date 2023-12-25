use crate::*;
use anyhow::Result;
use std::boxed::Box;
use std::collections::HashMap;
use tcloud_library::Plugin;
use tokio::sync::{Mutex, OnceCell};

/// Container of every plugin
pub static PLUGINS: OnceCell<HashMap<String, Mutex<Box<dyn Plugin + Send + Sync>>>> =
    OnceCell::const_new();

/// Initializes every plugin, panics if it has been initialized before
pub fn init() -> Result<()> {
    PLUGINS
        .set(HashMap::from([
            // Add and/or remove plugins from here.
            // Use the set_plugin! macro with the path to your plugin
            // (If your plugin is in another crate you have to specify it in Cargo.toml)
            set_plugin!(tcloud_archive::ArchivePlugin),
        ]))
        .unwrap_or_else(|_| panic!("Plugins have already been initialized"));
    Ok(())
}
