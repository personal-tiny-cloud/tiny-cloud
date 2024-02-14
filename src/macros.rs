/// Sets a plugin during initialization
#[macro_export]
macro_rules! set_plugin {
    ($plugin:ty) => {{
        let (name, plugin) = match <$plugin>::new() {
            Ok((name, plugin)) => (name, Box::new(plugin) as Box<dyn Plugin + Send + Sync>),
            Err(err) => return Err(anyhow::format_err!("`{}`: {}", stringify!($plugin), err)),
        };
        (name, Mutex::new(plugin))
    }};
}

/// Gets a config's value
#[macro_export]
macro_rules! config {
    ($( $config:ident ).* ) => {{
        use crate::config::CONFIG;
        &CONFIG.get().expect("Config hasn't been opened yet")$(.$config)*
    }};
}

/// Gets a CSS/JS file from assets
#[macro_export]
macro_rules! web_file {
    ($filename:expr) => {
        PreEscaped(include_str!(concat!(
            env!("OUT_DIR"),
            "/assets/",
            $filename
        )))
    };
}
