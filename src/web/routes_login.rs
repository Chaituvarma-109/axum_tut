use axum::{Json, Router};
use axum::routing::post;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_cookies::{Cookie, Cookies};

use crate::{Error, Result};
use crate::web::AUTH_TOKEN;

#[derive(Debug, Deserialize)]
struct LoginPayLoad {
    username: String,
    pwd: String,
}

pub fn routes() -> Router {
    Router::new().route("/api/login", post(api_login))
}

async fn api_login(cookies: Cookies, payload: Json<LoginPayLoad>) -> Result<Json<Value>> {
    println!("->> {:<12} - api-login", "Handler");

    // TODO: Implement real auth/db login
    if payload.username != "demo1" || payload.pwd != "welcome" {
        return Err(Error::LoginFail);
    }

    // FIXME: Implement real auth-token generation/signature.
    cookies.add(Cookie::new(AUTH_TOKEN, "user-1.exp.sign"));

    // create success body
    let body = Json(json!({
        "result": {
            "success": true
        },
    }));

    Ok(body)
}
