use anyhow::Result;
use redis::AsyncCommands;
use std::env;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, error};

pub mod cache_proto {
    tonic::include_proto!("cache");
}

use cache_proto::{
    cache_server::{Cache, CacheServer},
    DeleteKeyRequest, DeleteKeyResponse, GetKeyRequest, GetKeyResponse, KeyValuePair,
    ListKeysRequest, ListKeysResponse, SetKeyRequest, SetKeyResponse,
};

#[derive(Debug)]
struct CacheService {
    client: redis::Client,
}

impl CacheService {
    fn new(client: redis::Client) -> Self {
        Self { client }
    }

    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection, Status> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to get Redis connection");
                Status::internal(format!("Redis connection error: {}", e))
            })
    }
}

#[tonic::async_trait]
impl Cache for CacheService {
    async fn set_key(
        &self,
        request: Request<SetKeyRequest>,
    ) -> Result<Response<SetKeyResponse>, Status> {
        let req = request.into_inner();
        let mut conn = self.get_conn().await?;

        let result: Result<(), redis::RedisError> = if req.ttl_seconds > 0 {
            conn.set_ex(&req.key, &req.value, req.ttl_seconds as u64)
                .await
        } else {
            conn.set(&req.key, &req.value).await
        };

        match result {
            Ok(_) => {
                info!(key = %req.key, ttl = req.ttl_seconds, "Key set");
                Ok(Response::new(SetKeyResponse { success: true }))
            }
            Err(e) => {
                error!(error = %e, key = %req.key, "Failed to set key");
                Err(Status::internal(format!("Redis error: {}", e)))
            }
        }
    }

    async fn get_key(
        &self,
        request: Request<GetKeyRequest>,
    ) -> Result<Response<GetKeyResponse>, Status> {
        let req = request.into_inner();
        let mut conn = self.get_conn().await?;

        let result: Result<Option<String>, redis::RedisError> = conn.get(&req.key).await;

        match result {
            Ok(Some(value)) => Ok(Response::new(GetKeyResponse {
                value,
                found: true,
            })),
            Ok(None) => Ok(Response::new(GetKeyResponse {
                value: String::new(),
                found: false,
            })),
            Err(e) => {
                error!(error = %e, key = %req.key, "Failed to get key");
                Err(Status::internal(format!("Redis error: {}", e)))
            }
        }
    }

    async fn delete_key(
        &self,
        request: Request<DeleteKeyRequest>,
    ) -> Result<Response<DeleteKeyResponse>, Status> {
        let req = request.into_inner();
        let mut conn = self.get_conn().await?;

        let result: Result<i64, redis::RedisError> = conn.del(&req.key).await;

        match result {
            Ok(count) => {
                info!(key = %req.key, "Key deleted");
                Ok(Response::new(DeleteKeyResponse {
                    success: count > 0,
                }))
            }
            Err(e) => {
                error!(error = %e, key = %req.key, "Failed to delete key");
                Err(Status::internal(format!("Redis error: {}", e)))
            }
        }
    }

    async fn list_keys(
        &self,
        request: Request<ListKeysRequest>,
    ) -> Result<Response<ListKeysResponse>, Status> {
        let req = request.into_inner();
        let mut conn = self.get_conn().await?;

        let pattern = if req.pattern.is_empty() {
            "*".to_string()
        } else {
            req.pattern
        };

        let keys: Result<Vec<String>, redis::RedisError> = conn.keys(&pattern).await;

        match keys {
            Ok(keys) => {
                let mut pairs = Vec::new();
                for key in &keys {
                    let val: Result<Option<String>, _> = conn.get(key).await;
                    if let Ok(Some(value)) = val {
                        pairs.push(KeyValuePair {
                            key: key.clone(),
                            value,
                        });
                    }
                }
                Ok(Response::new(ListKeysResponse { pairs }))
            }
            Err(e) => {
                error!(error = %e, "Failed to list keys");
                Err(Status::internal(format!("Redis error: {}", e)))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let loki_url = env::var("LOKI_URL").unwrap_or_else(|_| "http://loki:3100".to_string());

    let (loki_layer, task) = tracing_loki::builder()
        .label("service", "cache-service")?
        .extra_field("pod", env::var("HOSTNAME").unwrap_or_default())?
        .build_url(url::Url::parse(&loki_url)?)?;

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(loki_layer)
        .init();

    tokio::spawn(task);

    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    info!("Connecting to Redis: {}", redis_url);
    let client = redis::Client::open(redis_url)?;

    let addr = "0.0.0.0:50052".parse()?;
    info!("cache-service listening on {}", addr);

    Server::builder()
        .add_service(CacheServer::new(CacheService::new(client)))
        .serve(addr)
        .await?;

    Ok(())
}
