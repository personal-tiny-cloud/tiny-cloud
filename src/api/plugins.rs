use crate::plugins::PLUGINS;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use std::path::Path;
use tcloud_library::error::PluginError;

/// Handles plugins
pub async fn handler(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let plugin_name = path.into_inner();
    let plugins = PLUGINS.get().expect("Plugins haven't been initialized");
    let plugin = match plugins.get(&plugin_name) {
        Some(plugin) => plugin,
        None => {
            return HttpResponse::NotFound().body(format!("Plugin `{}` not found", plugin_name))
        }
    };
    let mut plugin = plugin.lock().await;
    match plugin
        .process_api_request("test".to_string(), &Path::new("./test"))
        .await
    {
        Ok(resp) => HttpResponse::Ok().body(resp),
        Err(err) => match err {
            PluginError::IOError(err) => {
                log::error!("Internal IO Error: {:?}", err);
                HttpResponse::InternalServerError().body(format!(
                    "An IO error occurred while using `{}` plugin",
                    plugin_name
                ))
            }
            PluginError::InvalidRequest(resp) => HttpResponse::BadRequest().body(resp),
            PluginError::RequestFailed(code, resp) => {
                HttpResponse::build(StatusCode::from_u16(code).expect(&format!(
                    "Invalid HTTP Status Code from `{}` plugin",
                    plugin_name
                )))
                .body(resp)
            }
            _ => panic!(
                "Invalid error returned by `{}` plugin while handling request",
                plugin_name
            ),
        },
    }
}
