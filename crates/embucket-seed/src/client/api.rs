use crate::client::requests::client::BasicHttpClient;
use crate::client::requests::client::EmbucketClient;
use crate::client::requests::error::HttpRequestError;
use crate::external_models::{
    AuthResponse, DatabasePayload, DatabaseCreatePayload, DatabaseCreateResponse, SchemaCreatePayload,
    SchemaCreateResponse, VolumePayload, VolumeCreatePayload, VolumeCreateResponse,
};
use http::Method;
use std::net::SocketAddr;

pub type ApiClientResult<T> = Result<T, HttpRequestError>;

pub struct DatabaseClient {
    pub client: BasicHttpClient,
}

#[async_trait::async_trait]
pub trait DatabaseClientApi {
    async fn login(&mut self, user: &str, password: &str) -> ApiClientResult<AuthResponse>;
    async fn create_volume(&mut self, volume: VolumePayload) -> ApiClientResult<()>;
    async fn create_database(&mut self, volume: &str, database: &str) -> ApiClientResult<()>;
    async fn create_schema(&mut self, database: &str, schema: &str) -> ApiClientResult<()>;
    // async fn upload_to_table(&self, table_name: String, payload: TableUploadPayload) -> ApiClientResult<TableUploadResponse>;
}

impl DatabaseClient {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            client: BasicHttpClient::new(addr),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseClientApi for DatabaseClient {
    async fn login(&mut self, user: &str, password: &str) -> ApiClientResult<AuthResponse> {
        self.client.login(user, password).await
    }

    async fn create_volume(&mut self, volume: VolumePayload) -> ApiClientResult<()> {
        self.client
            .generic_request::<VolumeCreatePayload, VolumeCreateResponse>(
                Method::POST,
                &format!("http://{}/ui/volumes", self.client.addr()),
                &VolumeCreatePayload { data: volume },
            )
            .await?;
        Ok(())
    }

    async fn create_database(&mut self, volume: &str, database: &str) -> ApiClientResult<()> {
        self.client
            .generic_request::<DatabaseCreatePayload, DatabaseCreateResponse>(
                Method::POST,
                &format!("http://{}/ui/databases", self.client.addr()),
                &DatabaseCreatePayload {
                    data: DatabasePayload {
                        name: database.to_string(),
                        volume: volume.to_string(),
                    },
                },
            )
            .await?;
        Ok(())
    }

    async fn create_schema(&mut self, database: &str, schema: &str) -> ApiClientResult<()> {
        self.client
            .generic_request::<SchemaCreatePayload, SchemaCreateResponse>(
                Method::POST,
                &format!(
                    "http://{}/ui/databases/{database}/schemas",
                    self.client.addr()
                ),
                &SchemaCreatePayload {
                    name: schema.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    // async fn upload_to_table(&self, database: &str, schema: &str, table: &str) -> ApiClientResult<TableUploadResponse> {
    //     self.client.generic_request::<TableUploadPayload, TableUploadResponse>(
    //         Method::POST, format!("/ui/databases/{database}/schemas/{schema}/tables/{table}/rows"),
    //         &TableUploadPayload { upload_file:  },
    //     ).await
    // }
}
