use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use minify_js::{minify, Session, TopLevelMode};
use std::env;
use std::fs;
use std::path::PathBuf;

fn handle_file(file: PathBuf, out_dir: PathBuf) {
    let path_str = file.to_str().expect("Invalid path UTF-8");
    let minified: String = if let Ok(file_content) = fs::read_to_string(&file) {
        if path_str.ends_with(".css") {
            let mut stylesheet = StyleSheet::parse(&file_content, ParserOptions::default())
                .expect(&format!("Invalid CSS file {}, cannot parse it", path_str));
            stylesheet
                .minify(MinifyOptions::default())
                .expect(&format!("Cannot minify {} CSS file", path_str));
            stylesheet
                .to_css(PrinterOptions {
                    minify: true,
                    ..PrinterOptions::default()
                })
                .expect(&format!("Cannot get minified CSS of {}", path_str))
                .code
        } else if path_str.ends_with(".js") {
            let session = Session::new();
            let mut out = Vec::new();
            minify(
                &session,
                TopLevelMode::Global,
                file_content.as_bytes(),
                &mut out,
            )
            .expect(&format!("Failed to minify {} JS file", path_str));
            String::from_utf8(out)
                .expect(&format!("Minified JS file {} is not valid UTF-8", path_str))
        } else {
            return;
        }
    } else {
        return;
    };
    let mut new_file_path = out_dir.clone();
    new_file_path.push(&file);
    fs::write(&new_file_path, minified).expect(&format!(
        "Failed to write minified file {}",
        new_file_path.to_str().unwrap_or(path_str)
    ));
}

fn handle_directory(directory: PathBuf, out_dir: PathBuf) {
    let mut new_dir = out_dir.clone();
    let path_str = directory.to_str().unwrap_or("directory");
    new_dir.push(&directory);
    fs::create_dir_all(&new_dir).expect(&format!("Failed to create {}", path_str));
    for direntry in
        fs::read_dir(&directory).expect(&format!("Failed to read files of {}", path_str))
    {
        if let Ok(direntry) = direntry {
            if let Ok(file_type) = direntry.file_type() {
                if file_type.is_dir() {
                    handle_directory(PathBuf::from(direntry.path()), out_dir.clone());
                } else if file_type.is_file() {
                    handle_file(PathBuf::from(direntry.path()), out_dir.clone());
                }
            }
        }
    }
}

fn main() {
    let out_dir =
        PathBuf::from(env::var_os("OUT_DIR").expect("Failed to get OUT_DIR env variable"));
    handle_directory(PathBuf::from("assets"), out_dir);

    println!("cargo:rerun-if-changed=assets");
}
