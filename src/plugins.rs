use crate::*;
use anyhow::Result;
use std::boxed::Box;
use std::collections::HashMap;
use tcloud_library::Plugin;
use tokio::sync::{Mutex, OnceCell};

/// Container of every plugin
pub static PLUGINS: OnceCell<HashMap<String, Mutex<Box<dyn Plugin + Send + Sync>>>> =
    OnceCell::const_new();

/// Initializes every plugin, panics if it was initialized before
pub fn init() -> Result<()> {
    PLUGINS
        .set(HashMap::from([
            #[cfg(feature = "archive")]
            {
                set_plugin!(tcloud_archive::ArchivePlugin)
            },
        ]))
        .unwrap_or_else(|_| panic!("Plugins have already been initialized"));
    Ok(())
}
