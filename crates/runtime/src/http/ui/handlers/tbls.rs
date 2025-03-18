// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.
use crate::http::state::AppState;
use crate::http::{
    error::ErrorResponse,
    metastore::handlers::QueryParameters,
    ui::error::{UIError, UIResult},
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum::extract::Multipart;
use icebucket_metastore::error::MetastoreError;
use icebucket_metastore::models::{IceBucketSchema, IceBucketSchemaIdent};
use utoipa::OpenApi;
use icebucket_metastore::IceBucketTableIdent;
use crate::http::session::DFSessionId;
use crate::http::ui::models::tbls::TableUploadPayload;
#[utoipa::path(
    post,
    path = "/ui/databases/{databaseName}/schemas/{schemaName}/tables/{tableName}/upload",
    operation_id = "tableUpload",
    tags = ["tables"],
    params(
        ("databaseName" = String, description = "Database Name"),
        ("schemaName" = String, description = "Schema Name"),
        ("tableName" = String, description = "Table Name")
    ),
    request_body(
        content = TableUploadPayload,
        content_type = "multipart/form-data",
        description = "Upload data to the table in multipart/form-data format"
    ),
    responses(
        (status = 200, description = "Successful Response"),
        (status = 422, description = "Unprocessable entity", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(level = "debug", skip(state, multipart), err, ret(level = tracing::Level::TRACE))]
pub async fn upload_data_to_table(
    DFSessionId(session_id): DFSessionId,
    State(state): State<AppState>,
    Path((database_name, schema_name, table_name)): Path<(String, String, String)>,
    mut multipart: Multipart,
) -> UIResult<()> {
    loop {
        let next_field = multipart
            .next_field()
            .await?;
        match next_field {
            Some(field) => {
                if field.name().ok_or(UIError::Execution)? != "uploadFile" {
                    continue;
                }
                let file_name = field
                    .file_name()
                    .ok_or(UIError::Execution)?
                    .to_string();
                let data = field
                    .bytes()
                    .await?;
                //TODO: should check if table exists or not?
                let table_ident = IceBucketTableIdent::new(database_name.as_ref(), schema_name.as_ref(), table_name.as_ref());
                state
                    .execution_svc
                    .upload_data_to_table(
                        &session_id,
                        &table_ident,
                        data,
                        file_name,
                    )
                    .await?;
            }
            None => {
                break;
            }
        }
    }
    Ok(())
}