use crate::external_models::{
    AuthResponse, DatabaseCreatePayload, DatabaseCreateResponse, DatabasePayload,
    QueryCreateResponse, SchemaCreatePayload, SchemaCreateResponse, VolumeCreatePayload,
    VolumeCreateResponse, VolumePayload,
};
use crate::requests::service_client::{ServiceClient, BasicAuthClient};
use crate::requests::error::HttpRequestResult;
use http::Method;
use std::net::SocketAddr;

pub struct RestClient {
    pub client: BasicAuthClient,
}

#[async_trait::async_trait]
pub trait RestApiClient {
    async fn login(&mut self, user: &str, password: &str) -> HttpRequestResult<AuthResponse>;
    async fn create_volume(
        &mut self,
        volume: VolumePayload,
    ) -> HttpRequestResult<VolumeCreateResponse>;
    async fn create_database(
        &mut self,
        volume: &str,
        database: &str,
    ) -> HttpRequestResult<DatabaseCreateResponse>;
    async fn create_schema(
        &mut self,
        database: &str,
        schema: &str,
    ) -> HttpRequestResult<SchemaCreateResponse>;
    async fn create_table(
        &mut self,
        database: &str,
        schema: &str,
        table: &str,
        columns: &[(String, String)],
    ) -> HttpRequestResult<QueryCreateResponse>;
    // async fn upload_to_table(&self, table_name: String, payload: TableUploadPayload) -> HttpRequestResult<TableUploadResponse>;
}

impl RestClient {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            client: BasicAuthClient::new(addr),
        }
    }
}

#[async_trait::async_trait]
impl RestApiClient for RestClient {
    async fn login(&mut self, user: &str, password: &str) -> HttpRequestResult<AuthResponse> {
        self.client.login(user, password).await
    }

    async fn create_volume(
        &mut self,
        volume: VolumePayload,
    ) -> HttpRequestResult<VolumeCreateResponse> {
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
    ) -> HttpRequestResult<DatabaseCreateResponse> {
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
    ) -> HttpRequestResult<SchemaCreateResponse> {
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
    ) -> HttpRequestResult<QueryCreateResponse> {
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

    // async fn upload_to_table(&self, database: &str, schema: &str, table: &str) -> HttpRequestResult<TableUploadResponse> {
    //     self.client.generic_request::<TableUploadPayload, TableUploadResponse>(
    //         Method::POST, format!("/ui/databases/{database}/schemas/{schema}/tables/{table}/rows"),
    //         &TableUploadPayload { upload_file:  },
    //     ).await
    // }
}
