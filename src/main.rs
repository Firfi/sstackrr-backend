//! ```not_rust
//! cargo run
//! ```
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel_derive_newtype;
#[macro_use]
extern crate diesel_derive_enum;
extern crate dotenv;
use dotenv::dotenv;
mod game;
mod graphql;
mod db;
mod db_schema;
mod broker;
mod adversary;
mod adversary_minimax;
mod db_schema_macro;



use std::env;

use crate::adversary::run_subscribe_bots;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use crate::db::run_embed_migrations;
use crate::graphql::{GraphQlSchema, MutationRoot, QueryRoot, SubscriptionRoot};

//async fn graphql_handler(schema: Extension<OrderBookSchema>, req: GraphQLRequest) -> GraphQLResponse {
async fn graphql_handler(schema: Extension<GraphQlSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
//
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/").subscription_endpoint("/ws")))
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    run_embed_migrations();
    let port = env::var("PORT").unwrap_or("3000".to_string());

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .finish();

    let app = Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(Extension(schema))
        .layer(CorsLayer::new()
                   .allow_origin(Any)
                   .allow_methods(Any)
                   .allow_headers(Any),
        );

    tokio::join!(run_subscribe_bots(), axum::Server::bind(&format!("0.0.0.0:{}", &port).parse().unwrap())
        .serve(app.into_make_service()));

}