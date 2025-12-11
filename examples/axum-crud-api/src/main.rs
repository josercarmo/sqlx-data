use axum::{ extract::{ Path, State }, http::StatusCode, routing::{ get, post }, Json, Router };
use sqlx::postgres::PgPoolOptions;
use std::env;

mod user_repo;
use user_repo::{User, UserPayload, UserRepo, UserRepoImpl};

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().connect(&db_url).await.expect("Failed to connect to DB");
    sqlx::migrate!().run(&pool).await.expect("Migrations failed");
    let repo = UserRepoImpl { pool };

    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
        .with_state(repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("🚀 Server running on port 8000");
    axum::serve(listener, app).await.unwrap();
}

//Endpoint Handlers
//test endpoint
async fn root() -> &'static str {
    "Welcome to the User Management API!"
}

//GET ALL
async fn list_users(State(repo): State<UserRepoImpl>) -> Result<Json<Vec<User>>, StatusCode> {
    repo.list_users().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//CREATE USER
async fn create_user(
    State(repo): State<UserRepoImpl>,
    Json(payload): Json<UserPayload>
    ) -> Result<(StatusCode, Json<User>), StatusCode> {
    repo.create_user(payload.name, payload.email).await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//GET USER BY ID
async fn get_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>
    ) -> Result<Json<User>, StatusCode> {
    repo.get_user(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

//UPDATE USER
async fn update_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>,
    Json(payload): Json<UserPayload>
    ) -> Result<Json<User>, StatusCode> {
    repo.update_user(payload.name, payload.email, id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

//DELETE USER
async fn delete_user(
    State(repo): State<UserRepoImpl>,
    Path(id): Path<i32>
) -> Result<StatusCode, StatusCode> {
    let result = repo.delete_user(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}