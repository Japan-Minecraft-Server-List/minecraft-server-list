#![allow(unused)]
use std::sync::Arc;
use std::collections::HashMap;
use serde_json;
use axum::{
    Json, Router,
    body::Body,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub mod types;

#[allow(unused)]
pub async fn serve<T: crate::types::API>(api: T, address: &str) -> Result<(), std::io::Error> {
    let api = Arc::new(api);
    let mut router = Router::new();

    let api1 = api.clone();
    router = router.route("/api/get_server_list", axum::routing::get(async move |Query(__query): Query<HashMap<String, String>>, | {
        let api = api1;
        let ordering = __query.get("ordering").map(|str| serde_json::from_str(str).ok()).flatten();
        let Some(ordering) = ordering else { return Response::builder().status(StatusCode::BAD_REQUEST).body(Body::empty()).unwrap(); };
        let result = <T as crate::types::API>::get_server_list(&api, ordering, ).await;
        (StatusCode::OK, Json(result)).into_response()
    }));

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}