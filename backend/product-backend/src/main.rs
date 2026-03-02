use actix_cors::Cors;
use actix_web::{
    middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracing::{error, info};

pub mod items_proto {
    tonic::include_proto!("items");
}

pub mod cache_proto {
    tonic::include_proto!("cache");
}

use items_proto::items_client::ItemsClient;
use items_proto::{
    CreateItemRequest, DeleteItemRequest, GetItemRequest, ListItemsRequest, UpdateItemRequest,
};
use cache_proto::cache_client::CacheClient;
use cache_proto::{ListKeysRequest, SetKeyRequest};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    email: String,
    exp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleAuthRequest {
    id_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleTokenInfo {
    sub: String,
    email: String,
    aud: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateItemBody {
    name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateItemBody {
    name: String,
    description: String,
}

#[derive(Clone)]
struct AppState {
    items_client: Arc<Mutex<ItemsClient<Channel>>>,
    cache_client: Arc<Mutex<CacheClient<Channel>>>,
    jwt_secret: String,
    google_client_id: String,
    google_auth_enabled: bool,
    http_client: reqwest::Client,
}

fn extract_jwt(req: &HttpRequest, jwt_secret: &str) -> Result<Claims, HttpResponse> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({"error": "Missing token"})));
    }

    let token = &auth_header[7..];
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|e| {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": format!("Invalid token: {}", e)}))
    })
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn dev_auth(state: web::Data<AppState>) -> impl Responder {
    if state.google_auth_enabled {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Not available"}));
    }
    let exp = Utc::now().timestamp() + 86400;
    let claims = Claims {
        sub: "dev-user".to_string(),
        email: "dev@local".to_string(),
        exp,
    };
    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(token) => {
            info!("Dev login issued");
            HttpResponse::Ok().json(AuthResponse { token })
        }
        Err(e) => {
            error!(error = %e, "Failed to encode dev JWT");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Token generation failed"}))
        }
    }
}

async fn google_auth(
    state: web::Data<AppState>,
    body: web::Json<GoogleAuthRequest>,
) -> impl Responder {
    if !state.google_auth_enabled {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Not available"}));
    }
    let token_info_url = format!(
        "https://oauth2.googleapis.com/tokeninfo?id_token={}",
        body.id_token
    );

    let result = state.http_client.get(&token_info_url).send().await;

    match result {
        Ok(resp) => {
            if !resp.status().is_success() {
                return HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Invalid Google token"}));
            }
            match resp.json::<GoogleTokenInfo>().await {
                Ok(info) => {
                    if info.aud != state.google_client_id {
                        return HttpResponse::Unauthorized()
                            .json(serde_json::json!({"error": "Token audience mismatch"}));
                    }
                    let exp = Utc::now().timestamp() + 86400;
                    let claims = Claims {
                        sub: info.sub.clone(),
                        email: info.email.clone(),
                        exp,
                    };
                    match encode(
                        &Header::default(),
                        &claims,
                        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
                    ) {
                        Ok(token) => {
                            info!(email = %info.email, "User authenticated via Google");
                            HttpResponse::Ok().json(AuthResponse { token })
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to encode JWT");
                            HttpResponse::InternalServerError()
                                .json(serde_json::json!({"error": "Token generation failed"}))
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to parse Google token info");
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error": "Failed to verify token"}))
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Failed to call Google tokeninfo");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Auth service unavailable"}))
        }
    }
}

async fn list_items(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let mut client = state.items_client.lock().await;
    match client.list_items(ListItemsRequest {}).await {
        Ok(response) => {
            let items = response.into_inner().items;
            info!(count = items.len(), "Listed items");
            HttpResponse::Ok().json(items)
        }
        Err(e) => {
            error!(error = %e, "Failed to list items");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn create_item(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateItemBody>,
) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let mut client = state.items_client.lock().await;
    match client
        .create_item(CreateItemRequest {
            name: body.name.clone(),
            description: body.description.clone(),
        })
        .await
    {
        Ok(response) => {
            let item = response.into_inner().item;
            info!(name = %body.name, "Item created");
            HttpResponse::Created().json(item)
        }
        Err(e) => {
            error!(error = %e, "Failed to create item");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn get_item(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let id = path.into_inner();
    let mut client = state.items_client.lock().await;
    match client.get_item(GetItemRequest { id: id.clone() }).await {
        Ok(response) => {
            let item = response.into_inner().item;
            HttpResponse::Ok().json(item)
        }
        Err(e) if e.code() == tonic::Code::NotFound => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"}))
        }
        Err(e) => {
            error!(error = %e, id = %id, "Failed to get item");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn update_item(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateItemBody>,
) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let id = path.into_inner();
    let mut client = state.items_client.lock().await;
    match client
        .update_item(UpdateItemRequest {
            id: id.clone(),
            name: body.name.clone(),
            description: body.description.clone(),
        })
        .await
    {
        Ok(response) => {
            let item = response.into_inner().item;
            info!(id = %id, "Item updated");
            HttpResponse::Ok().json(item)
        }
        Err(e) if e.code() == tonic::Code::NotFound => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"}))
        }
        Err(e) => {
            error!(error = %e, id = %id, "Failed to update item");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn delete_item(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let id = path.into_inner();
    let mut client = state.items_client.lock().await;
    match client.delete_item(DeleteItemRequest { id: id.clone() }).await {
        Ok(response) => {
            let success = response.into_inner().success;
            if success {
                info!(id = %id, "Item deleted");
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"}))
            }
        }
        Err(e) => {
            error!(error = %e, id = %id, "Failed to delete item");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn list_redis(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    if let Err(e) = extract_jwt(&req, &state.jwt_secret) {
        return e;
    }

    let mut client = state.cache_client.lock().await;
    match client
        .list_keys(ListKeysRequest {
            pattern: "*".to_string(),
        })
        .await
    {
        Ok(response) => {
            let pairs = response.into_inner().pairs;
            HttpResponse::Ok().json(pairs)
        }
        Err(e) => {
            error!(error = %e, "Failed to list Redis keys");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.message()}))
        }
    }
}

async fn background_redis_writer(state: AppState) {
    let mut counter: u64 = 0;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        counter += 1;
        let key = format!("heartbeat:{}", counter);
        let value = format!("alive at {}", Utc::now().to_rfc3339());

        let mut client = state.cache_client.lock().await;
        match client
            .set_key(SetKeyRequest {
                key: key.clone(),
                value,
                ttl_seconds: 180,
            })
            .await
        {
            Ok(_) => info!(key = %key, "Heartbeat written to Redis"),
            Err(e) => error!(error = %e, "Failed to write heartbeat to Redis"),
        }
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    let loki_url = env::var("LOKI_URL").unwrap_or_else(|_| "http://loki:3100".to_string());

    let (loki_layer, task) = tracing_loki::builder()
        .label("service", "product-backend")?
        .extra_field("pod", env::var("HOSTNAME").unwrap_or_default())?
        .build_url(url::Url::parse(&loki_url)?)?;

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(loki_layer)
        .init();

    tokio::spawn(task);

    let db_service_url = env::var("DB_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());
    let cache_service_url = env::var("CACHE_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:50052".to_string());
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".to_string());
    let google_client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let google_auth_enabled = env::var("GOOGLE_AUTH_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Use lazy channels — connections are established on first use, not at startup.
    // This prevents crashes when sibling pods aren't ready yet.
    info!("Configuring db-service channel: {}", db_service_url);
    let items_channel = Channel::from_shared(db_service_url)?
        .connect_lazy();
    let items_client = Arc::new(Mutex::new(ItemsClient::new(items_channel)));

    info!("Configuring cache-service channel: {}", cache_service_url);
    let cache_channel = Channel::from_shared(cache_service_url)?
        .connect_lazy();
    let cache_client = Arc::new(Mutex::new(CacheClient::new(cache_channel)));

    info!(google_auth_enabled, "Auth configuration");
    let state = AppState {
        items_client,
        cache_client,
        jwt_secret,
        google_client_id,
        google_auth_enabled,
        http_client: reqwest::Client::new(),
    };

    let bg_state = state.clone();
    tokio::spawn(async move {
        background_redis_writer(bg_state).await;
    });

    info!("product-backend listening on 0.0.0.0:8080");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(state.clone()))
            .route("/api/health", web::get().to(health))
            .route("/api/auth/dev", web::post().to(dev_auth))
            .route("/api/auth/google", web::post().to(google_auth))
            .route("/api/items", web::get().to(list_items))
            .route("/api/items", web::post().to(create_item))
            .route("/api/items/{id}", web::get().to(get_item))
            .route("/api/items/{id}", web::put().to(update_item))
            .route("/api/items/{id}", web::delete().to(delete_item))
            .route("/api/redis", web::get().to(list_redis))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
