use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use sqlx::Row;
use std::env;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, error};
use uuid::Uuid;

pub mod items_proto {
    tonic::include_proto!("items");
}

use items_proto::{
    items_server::{Items, ItemsServer},
    CreateItemRequest, DeleteItemRequest, DeleteItemResponse, GetItemRequest, Item,
    ItemResponse, ListItemsRequest, ListItemsResponse, UpdateItemRequest,
};

#[derive(Debug)]
struct ItemsService {
    pool: PgPool,
}

impl ItemsService {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl Items for ItemsService {
    async fn create_item(
        &self,
        request: Request<CreateItemRequest>,
    ) -> Result<Response<ItemResponse>, Status> {
        let req = request.into_inner();
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "INSERT INTO items (id, name, description, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                info!(id = %id, name = %req.name, "Item created");
                Ok(Response::new(ItemResponse {
                    item: Some(Item {
                        id,
                        name: req.name,
                        description: req.description,
                        created_at: now.clone(),
                        updated_at: now,
                    }),
                }))
            }
            Err(e) => {
                error!(error = %e, "Failed to create item");
                Err(Status::internal(format!("Database error: {}", e)))
            }
        }
    }

    async fn get_item(
        &self,
        request: Request<GetItemRequest>,
    ) -> Result<Response<ItemResponse>, Status> {
        let req = request.into_inner();

        let result = sqlx::query(
            "SELECT id, name, description, created_at, updated_at FROM items WHERE id = $1",
        )
        .bind(&req.id)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(row)) => Ok(Response::new(ItemResponse {
                item: Some(Item {
                    id: row.try_get("id").unwrap_or_default(),
                    name: row.try_get("name").unwrap_or_default(),
                    description: row.try_get::<Option<String>, _>("description").unwrap_or(None).unwrap_or_default(),
                    created_at: row.try_get("created_at").unwrap_or_default(),
                    updated_at: row.try_get("updated_at").unwrap_or_default(),
                }),
            })),
            Ok(None) => Err(Status::not_found(format!("Item {} not found", req.id))),
            Err(e) => {
                error!(error = %e, "Failed to get item");
                Err(Status::internal(format!("Database error: {}", e)))
            }
        }
    }

    async fn list_items(
        &self,
        _request: Request<ListItemsRequest>,
    ) -> Result<Response<ListItemsResponse>, Status> {
        let result = sqlx::query(
            "SELECT id, name, description, created_at, updated_at FROM items ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await;

        match result {
            Ok(rows) => {
                let items = rows
                    .into_iter()
                    .map(|row| Item {
                        id: row.try_get("id").unwrap_or_default(),
                        name: row.try_get("name").unwrap_or_default(),
                        description: row.try_get::<Option<String>, _>("description").unwrap_or(None).unwrap_or_default(),
                        created_at: row.try_get("created_at").unwrap_or_default(),
                        updated_at: row.try_get("updated_at").unwrap_or_default(),
                    })
                    .collect();
                Ok(Response::new(ListItemsResponse { items }))
            }
            Err(e) => {
                error!(error = %e, "Failed to list items");
                Err(Status::internal(format!("Database error: {}", e)))
            }
        }
    }

    async fn update_item(
        &self,
        request: Request<UpdateItemRequest>,
    ) -> Result<Response<ItemResponse>, Status> {
        let req = request.into_inner();
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE items SET name = $2, description = $3, updated_at = $4 WHERE id = $1 RETURNING id, name, description, created_at, updated_at",
        )
        .bind(&req.id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(row)) => {
                info!(id = %req.id, "Item updated");
                Ok(Response::new(ItemResponse {
                    item: Some(Item {
                        id: row.try_get("id").unwrap_or_default(),
                        name: row.try_get("name").unwrap_or_default(),
                        description: row.try_get::<Option<String>, _>("description").unwrap_or(None).unwrap_or_default(),
                        created_at: row.try_get("created_at").unwrap_or_default(),
                        updated_at: row.try_get("updated_at").unwrap_or_default(),
                    }),
                }))
            }
            Ok(None) => Err(Status::not_found(format!("Item {} not found", req.id))),
            Err(e) => {
                error!(error = %e, "Failed to update item");
                Err(Status::internal(format!("Database error: {}", e)))
            }
        }
    }

    async fn delete_item(
        &self,
        request: Request<DeleteItemRequest>,
    ) -> Result<Response<DeleteItemResponse>, Status> {
        let req = request.into_inner();

        let result = sqlx::query("DELETE FROM items WHERE id = $1")
            .bind(&req.id)
            .execute(&self.pool)
            .await;

        match result {
            Ok(res) => {
                let success = res.rows_affected() > 0;
                if success {
                    info!(id = %req.id, "Item deleted");
                }
                Ok(Response::new(DeleteItemResponse { success }))
            }
            Err(e) => {
                error!(error = %e, "Failed to delete item");
                Err(Status::internal(format!("Database error: {}", e)))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let loki_url = env::var("LOKI_URL").unwrap_or_else(|_| "http://loki:3100".to_string());

    let (loki_layer, task) = tracing_loki::builder()
        .label("service", "db-service")?
        .extra_field("pod", env::var("HOSTNAME").unwrap_or_default())?
        .build_url(url::Url::parse(&loki_url)?)?;

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(loki_layer)
        .init();

    tokio::spawn(task);

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/experiproduct".to_string());

    info!("Connecting to database: {}", database_url);
    let pool = loop {
        match PgPool::connect(&database_url).await {
            Ok(pool) => {
                info!("Database connected");
                break pool;
            }
            Err(e) => {
                error!(error = %e, "Database not ready, retrying in 2s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    };

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;
    info!("Database initialized");

    let addr = "0.0.0.0:50051".parse()?;
    info!("db-service listening on {}", addr);

    Server::builder()
        .add_service(ItemsServer::new(ItemsService::new(pool)))
        .serve(addr)
        .await?;

    Ok(())
}
