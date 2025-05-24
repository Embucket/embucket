use crate::external_models::{
    AuthResponse, DatabaseCreatePayload, DatabaseCreateResponse, DatabasePayload,
    QueryCreateResponse, SchemaCreatePayload, SchemaCreateResponse, VolumeCreatePayload,
    VolumeCreateResponse, VolumePayload,
};
use crate::requests::client::BasicEmbucketClient;
use crate::requests::client::BasicHttpClient;
use crate::requests::error::HttpRequestError;
use http::Method;
use std::net::SocketAddr;

pub type ApiClientResult<T> = Result<T, HttpRequestError>;

pub struct RestClient {
    pub client: BasicHttpClient,
}

#[async_trait::async_trait]
pub trait RestApiClient {
    async fn login(&mut self, user: &str, password: &str) -> ApiClientResult<AuthResponse>;
    async fn create_volume(
        &mut self,
        volume: VolumePayload,
    ) -> ApiClientResult<VolumeCreateResponse>;
    async fn create_database(
        &mut self,
        volume: &str,
        database: &str,
    ) -> ApiClientResult<DatabaseCreateResponse>;
    async fn create_schema(
        &mut self,
        database: &str,
        schema: &str,
    ) -> ApiClientResult<SchemaCreateResponse>;
    async fn create_table(
        &mut self,
        database: &str,
        schema: &str,
        table: &str,
        columns: &[(String, String)],
    ) -> ApiClientResult<QueryCreateResponse>;
    // async fn upload_to_table(&self, table_name: String, payload: TableUploadPayload) -> ApiClientResult<TableUploadResponse>;
}

impl RestClient {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            client: BasicHttpClient::new(addr),
        }
    }
}

#[async_trait::async_trait]
impl RestApiClient for RestClient {
    async fn login(&mut self, user: &str, password: &str) -> ApiClientResult<AuthResponse> {
        self.client.login(user, password).await
    }

    async fn create_volume(
        &mut self,
        volume: VolumePayload,
    ) -> ApiClientResult<VolumeCreateResponse> {
        Ok(self
            .client
            .generic_request::<VolumeCreatePayload, VolumeCreateResponse>(
                Method::POST,
                &format!("http://{}/ui/volumes", self.client.addr()),
                &VolumeCreatePayload { data: volume },
            )
            .await?)
    }

    async fn create_database(
        &mut self,
        volume: &str,
        database: &str,
    ) -> ApiClientResult<DatabaseCreateResponse> {
        Ok(self
            .client
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
            .await?)
    }

    async fn create_schema(
        &mut self,
        database: &str,
        schema: &str,
    ) -> ApiClientResult<SchemaCreateResponse> {
        Ok(self
            .client
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
            .await?)
    }

    async fn create_table(
        &mut self,
        database: &str,
        schema: &str,
        table: &str,
        columns: &[(String, String)],
    ) -> ApiClientResult<QueryCreateResponse> {
        let table_columns = columns
            .iter()
            .map(|(name, col_type)| format!("{name} {col_type}"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(self
            .client
            .query(&format!(
                "CREATE TABLE {database}.{schema}.{table} ({table_columns});"
            ))
            .await?)
    }

    // async fn upload_to_table(&self, database: &str, schema: &str, table: &str) -> ApiClientResult<TableUploadResponse> {
    //     self.client.generic_request::<TableUploadPayload, TableUploadResponse>(
    //         Method::POST, format!("/ui/databases/{database}/schemas/{schema}/tables/{table}/rows"),
    //         &TableUploadPayload { upload_file:  },
    //     ).await
    // }
}
