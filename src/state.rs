use std::env;

use axum::extract::FromRef;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub redis: redis::Client,
    pub pg: sqlx::postgres::PgPool,
}

impl AppState {
    pub async fn new() -> Self {
        Self {
            redis: redis_client(),
            pg: pg_pool().await,
        }
    }
}

fn redis_client() -> redis::Client {
    let password = env::var("REDIS_PASSWORD").unwrap();
    let addr = env::var("REDIS_ADDR").unwrap();

    let conn_str = format!("redis://default:{password}@{addr}");

    redis::Client::open(conn_str).unwrap()
}

async fn pg_pool() -> sqlx::postgres::PgPool {
    let url = env::var("POSTGRES_URL").unwrap();

    let pool = sqlx::postgres::PgPool::connect(&url).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    pool
}
