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

use arrow::ipc::Int;
use axum::Json;
use crate::http::metastore::error::MetastoreAPIError;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use snafu::prelude::*;
use icebucket_history::WorksheetsStoreError;
use icebucket_metastore::error::MetastoreError;
use crate::execution::error::ExecutionError;
use crate::http::error::ErrorResponse;

// #[derive(Debug, Snafu)]
// #[snafu(visibility(pub(crate)))]
// pub enum UIApiError {
//     #[snafu(display("Create {type_name} error: {source}"))]
//     Create { type_name: String, source: ExecutionError },
//     #[snafu(display("Get {type_name} error: {source}"))]
//     Get { type_name: String, source: ExecutionError },
//     #[snafu(display("Delete {type_name} error: {source}"))]
//     Delete { type_name: String, source: ExecutionError },
//     #[snafu(display("Update {type_name} error: {source}"))]
//     Update { type_name: String, source: ExecutionError },
//     #[snafu(display("Get {type_name} list error: {source}"))]
//     List { type_name: String, source: ExecutionError },
//     #[snafu(display("Query execution error: {source}"))]
//     QueryExecution { source: ExecutionError },
//     #[snafu(display("Query worksheet error: {source}"))]
//     QueryWorksheet { source: WorksheetsStoreError },
//     #[snafu(display("Query history error: {source}"))]
//     QueryHistory { source: WorksheetsStoreError },
// }

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum UIApiError {
    #[snafu(display("UI API error: {0}"))]
    Metastore(CRUDErrorType<MetastoreError>),
    #[snafu(display("UI API error: {0}"))]
    Query(QueryErrorType),
    #[snafu(display("UI API error: {0}"))]
    Worksheet(CRUDErrorType<WorksheetsStoreError>),
    //Just in case we need it is exposed
    #[snafu(display("UI API error: {0}"))]
    Execution(ExecutionError),
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum CRUDErrorType<E> {
    #[snafu(display("Create error: {0}"))]
    Create(E),
    #[snafu(display("Get error: {0}"))]
    Get(E),
    #[snafu(display("Delete error: {0}"))]
    Delete(E),
    #[snafu(display("Update error: {0}"))]
    Update(E),
    #[snafu(display("Get list error: {0}"))]
    List(E),
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum QueryErrorType {
    #[snafu(display("Query execution error: {0}"))]
    Execute(ExecutionError),
    #[snafu(display("Query worksheet error: {0}"))]
    Worksheet(WorksheetsStoreError),
    #[snafu(display("Query history error: {0}"))]
    History(WorksheetsStoreError),
}

impl IntoStatusCode for UIApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            UIApiError::Metastore(error) => error.status_code(),
            UIApiError::Query(error) => error.status_code(),
            UIApiError::Worksheet(error) => error.status_code(),
            UIApiError::Execution(_error) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoStatusCode for CRUDErrorType<MetastoreError> {
    fn status_code(&self) -> StatusCode {
        match self {
            CRUDErrorType::Create(error) => match error {
                MetastoreError::VolumeAlreadyExists { .. }
                | MetastoreError::DatabaseAlreadyExists { .. }
                | MetastoreError::SchemaAlreadyExists { .. }
                | MetastoreError::ObjectAlreadyExists { .. } => StatusCode::CONFLICT,
                MetastoreError::VolumeNotFound { .. }
                | MetastoreError::DatabaseNotFound { .. }
                | MetastoreError::Validation { .. } => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::Get(error) => match error {
                MetastoreError::UtilSlateDB { .. }
                | MetastoreError::DatabaseNotFound { .. }
                | MetastoreError::SchemaNotFound { .. }
                | MetastoreError::ObjectNotFound { .. } => {
                    StatusCode::NOT_FOUND
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::Delete(_error) => StatusCode::INTERNAL_SERVER_ERROR,
            CRUDErrorType::Update(error) => match error {
                MetastoreError::ObjectNotFound { .. }
                | MetastoreError::DatabaseNotFound { .. }
                | MetastoreError::SchemaNotFound { .. }
                | MetastoreError::VolumeNotFound { .. } => StatusCode::NOT_FOUND,
                MetastoreError::Validation { .. } => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::List(_error) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoStatusCode for CRUDErrorType<WorksheetsStoreError> {
    fn status_code(&self) -> StatusCode {
        match self {
            CRUDErrorType::Create(error) => match error {
                WorksheetsStoreError::WorksheetAdd { .. } => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::Get(error) => match error {
                WorksheetsStoreError::WorksheetNotFound { .. } => StatusCode::NOT_FOUND,
                WorksheetsStoreError::BadKey { .. } => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::Delete(error) => match error {
                WorksheetsStoreError::WorksheetNotFound { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::Update(error) => match error {
                WorksheetsStoreError::BadKey { .. }
                | WorksheetsStoreError::WorksheetUpdate { .. } => StatusCode::BAD_REQUEST,
                WorksheetsStoreError::WorksheetNotFound { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
            CRUDErrorType::List(error) => match error {
                WorksheetsStoreError::WorksheetsList { .. } => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}
impl IntoStatusCode for QueryErrorType {
    fn status_code(&self) -> StatusCode {
        match self {
            QueryErrorType::Execute(_error) => StatusCode::UNPROCESSABLE_ENTITY,
            QueryErrorType::Worksheet(error) => match error {
                WorksheetsStoreError::WorksheetNotFound { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::BAD_REQUEST,
            }
            QueryErrorType::History(error) => match error {
                WorksheetsStoreError::HistoryGet { .. }
                | WorksheetsStoreError::WorksheetNotFound { .. }
                | WorksheetsStoreError::BadKey { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}

pub type UIResult<T> = Result<T, UIApiError>;

pub(crate) trait IntoStatusCode {
    fn status_code(&self) -> StatusCode;
}

impl IntoResponse for UIApiError {
    fn into_response(&self) -> Response {
        let code = self.status_code();
        let error = ErrorResponse {
            message: self.to_string(),
            status_code: code.as_u16(),
        };
        (code, Json(error)).into_response()
    }
}

