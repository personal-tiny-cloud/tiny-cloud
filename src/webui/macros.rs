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
