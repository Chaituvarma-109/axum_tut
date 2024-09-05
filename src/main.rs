mod error;
mod web;
mod model;
mod ctx;
mod log;

use axum::{
    response::{Html, IntoResponse, Response},
    extract::{Path, Query},
    http::{Method, Uri},
    routing::{get, get_service},
    {middleware, Json, Router},
};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::web::{routes_login, routes_tickets, mw_auth};
use crate::error::{Error, Result};
use crate::model::ModelController;
use crate::log::log_request;
use crate::ctx::Ctx;

#[derive(Debug, Deserialize)]
struct HelloParams {
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mc = ModelController::new().await?;

    let routes_apis = routes_tickets::routes(mc.clone())
        .route_layer(middleware::from_fn(mw_auth::mw_require_auth));

    let routes_all = Router::new()
        .merge(routes_hello())
        .merge(routes_login::routes())
        .nest("/api", routes_apis)
        .layer(middleware::map_response(main_response_mapper))
        .layer(middleware::from_fn_with_state(
            mc.clone(),
            mw_auth::mw_ctx_resolver,
        ))
        .layer(CookieManagerLayer::new())
        .fallback_service(routes_static());

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("->> LISTENING on {:?}\n", listener.local_addr());
    axum::serve(listener, routes_all.into_make_service()).await.unwrap();

    Ok(())
}

async fn main_response_mapper(ctx: Option<Ctx>,
                              uri: Uri,
                              req_method: Method,
                              res: Response) -> Response {
    println!("->> {:<12} - main_response_mapper", "RES_MAPPER");
    let uuid = Uuid::new_v4();

    // -- Get the eventual response error.
    let service_error = res.extensions().get::<Error>();
    let client_status_error = service_error.map(|se| se.client_status_and_error());

    // -- If client error, build the new response.
    let error_response =
        client_status_error
            .as_ref()
            .map(|(status_code, client_error)| {
                let client_error_body = json!({
					"error": {
						"type": client_error.as_ref(),
						"req_uuid": uuid.to_string(),
					}
				});

                println!("    ->> client_error_body: {client_error_body}");

                // Build the new response from the client_error_body
                (*status_code, Json(client_error_body)).into_response()
            });

    // Build and log the server log line.
    let client_error = client_status_error.unzip().1;
    // TODO: Need to handler if log_request fail (but should not fail request)
    let _ =
        log_request(uuid, req_method, uri, ctx, service_error, client_error).await;

    println!();
    error_response.unwrap_or(res)
}

fn routes_static() -> Router {
    Router::new().nest_service("/", get_service(ServeDir::new("./")))
}

fn routes_hello() -> Router {
    Router::new()
        .route("/hello", get(handler_hello))
        .route("/hello2/:name", get(handler_hello2))
}

async fn handler_hello(Query(params): Query<HelloParams>) -> impl IntoResponse {
    println!("->> {:<12} - handler_hello - {params:?}", "HANDLER");
    let name = params.name.as_deref().unwrap_or("World!!");

    Html(format!("Hello <strong>{name}</strong>"))
}

async fn handler_hello2(Path(name): Path<String>) -> impl IntoResponse {
    println!("->> {:<12} - handler_hello2 - {name}", "HANDLER");
    Html(format!("Hello <strong>{name}</strong>"))
}
