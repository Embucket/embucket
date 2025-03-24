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
use icebucket_metastore::error::MetastoreError;
use icebucket_metastore::models::{IceBucketTable, IceBucketTableIdent};
use utoipa::OpenApi;
use icebucket_metastore::IceBucketSchema;
use crate::execution::query::{IceBucketQuery, IceBucketQueryContext};
use crate::http::session::DFSessionId;
use crate::http::ui::models::table::CreateTablePayload;

#[derive(OpenApi)]
#[openapi(
    paths(
        create_schema,
        delete_schema,
        get_schema,
        update_schema,
        list_schemas,
    ),
    components(
        schemas(
            IceBucketSchema,
            ErrorResponse,
        )
    ),
    tags(
        (name = "schemas", description = "Schemas management endpoints.")
    )
)]
pub struct ApiDoc;

#[utoipa::path(
    post,
    path = "/ui/databases/{databaseName}/schemas/{schemaName}/tables",
    operation_id = "createTable",
    tags = ["tables"],
    params(
        ("databaseName" = String, description = "Database Name"),
        ("schemaName" = String, description = "Schema Name")
    ),
    request_body = CreateTablePayload,
    responses(
        (status = 200, description = "Successful Response"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 422, description = "Unprocessable entity", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[tracing::instrument(level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn create_table(
    DFSessionId(session_id): DFSessionId,
    State(state): State<AppState>,
    Path((database_name, schema_name)): Path<(String, String)>,
    Json(payload): Json<CreateTablePayload>,
) -> UIResult<()> {
    let context = IceBucketQueryContext {
        database: Some(database_name),
        schema: Some(schema_name),
    };



    let _ = state
        .execution_svc
        .query(&session_id, "", context)
        .await
        .map_err(|e| UIError::Execution { source: e })?;

    Ok(())

}

#[utoipa::path(
    get,
    path = "/ui/databases/{databaseName}/schemas/{schemaName}",
    params(
        ("databaseName" = String, description = "Database Name"),
        ("schemaName" = String, description = "Schema Name")
    ),
    operation_id = "getSchema",
    tags = ["schemas"],
    responses(
        (status = 200, description = "Successful Response", body = IceBucketSchema),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 422, description = "Unprocessable entity", body = ErrorResponse),
    )
)]
#[tracing::instrument(level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn get_schema(
    State(state): State<AppState>,
    Path((database_name, schema_name)): Path<(String, String)>,
) -> UIResult<Json<IceBucketSchema>> {
    let schema_ident = IceBucketSchemaIdent {
        database: database_name.clone(),
        schema: schema_name.clone(),
    };
    match state.metastore.get_schema(&schema_ident).await {
        Ok(Some(schema)) => Ok(Json(schema.data)),
        Ok(None) => Err(MetastoreError::SchemaNotFound {
            db: database_name.clone(),
            schema: schema_name.clone(),
        }
            .into()),
        Err(e) => Err(e.into()),
    }
}

#[utoipa::path(
    delete,
    path = "/ui/databases/{databaseName}/schemas/{schemaName}",
    operation_id = "deleteSchema",
    tags = ["schemas"],
    params(
        ("databaseName" = String, description = "Database Name"),
        ("schemaName" = String, description = "Schema Name")
    ),
    responses(
        (status = 204, description = "Successful Response"),
        (status = 404, description = "Schema not found", body = ErrorResponse),
        (status = 422, description = "Unprocessable entity", body = ErrorResponse),
    )
)]
#[tracing::instrument(level = "debug", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn delete_schema(
    State(state): State<AppState>,
    Query(query): Query<QueryParameters>,
    Path((database_name, schema_name)): Path<(String, String)>,
) -> UIResult<()> {
    let schema_ident = IceBucketSchemaIdent::new(database_name, schema_name);
    state
        .metastore
        .delete_schema(&schema_ident, query.cascade.unwrap_or_default())
        .await
        .map_err(|e| UIError::Metastore { source: e })
}