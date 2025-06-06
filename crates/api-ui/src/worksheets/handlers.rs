use crate::error::ErrorResponse;
use crate::state::AppState;
use crate::worksheets::{
    GetWorksheetsParams, SortBy, SortOrder, Worksheet, WorksheetCreatePayload,
    WorksheetCreateResponse, WorksheetResponse, WorksheetUpdatePayload, WorksheetsResponse,
    error::{ListSnafu, WorksheetUpdateError, WorksheetsAPIError, WorksheetsResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use core_history::WorksheetId;
use snafu::ResultExt;
use std::convert::From;
use tracing;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        worksheets,
        worksheet,
        create_worksheet,
        delete_worksheet,
        update_worksheet,
    ),
    components(schemas(
        ErrorResponse,
        WorksheetCreatePayload,
        WorksheetUpdatePayload,
        WorksheetCreateResponse,
        WorksheetResponse,
        WorksheetsResponse,
    )),
    tags(
        (name = "worksheets", description = "Worksheets endpoints"),
    )
)]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/ui/worksheets",
    operation_id = "getWorksheets",
    tags = ["worksheets"],
    params(GetWorksheetsParams),
    responses(
        (status = 200, description = "Get list of worksheets", body = WorksheetsResponse),
        (status = 401,
         description = "Unauthorized",
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = ErrorResponse),
        (status = 400, description = "Unknown", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::worksheets", level = "info", skip(state), err)]
pub async fn worksheets(
    State(state): State<AppState>,
    Query(GetWorksheetsParams {
        sort_order,
        sort_by,
    }): Query<GetWorksheetsParams>,
) -> WorksheetsResult<Json<WorksheetsResponse>> {
    let history_worksheets = state
        .history_store
        .get_worksheets()
        .await
        .context(ListSnafu)?;

    let mut items = history_worksheets
        .into_iter()
        .map(Worksheet::from)
        .collect::<Vec<Worksheet>>();

    let sort_order = sort_order.unwrap_or_default();
    let sort_by = sort_by.unwrap_or_default();

    items.sort_by(|w1, w2| {
        let cmp_res = match sort_by {
            SortBy::Name => w1.name.clone().cmp(&w2.name),
            SortBy::CreatedAt => w1
                .created_at
                .timestamp_millis()
                .cmp(&w2.created_at.timestamp_millis()),
            SortBy::UpdatedAt => w1
                .updated_at
                .timestamp_millis()
                .cmp(&w2.updated_at.timestamp_millis()),
        };
        match sort_order {
            SortOrder::Ascending => cmp_res,
            SortOrder::Descending => cmp_res.reverse(),
        }
    });

    Ok(Json(WorksheetsResponse { items }))
}

#[utoipa::path(
    post,
    path = "/ui/worksheets",
    operation_id = "createWorksheet",
    tags = ["worksheets"],
    request_body(
        content(
            (
                WorksheetCreatePayload = "application/json", 
                examples (
                    ("with name" = (
                        value = json!(WorksheetCreatePayload {
                            name: "worksheet1".to_string(), 
                            content: "select 1".to_string(),
                        })
                    )),
                    ("empty name" = (
                        value = json!(WorksheetCreatePayload {
                            name: String::new(),
                            content: "select 1".to_string(),
                        })
                    )),
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Created worksheet", body = WorksheetCreateResponse),
        (status = 401,
         description = "Unauthorized",
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = ErrorResponse),
        (status = 409, description = "Already Exists", body = ErrorResponse),
        (status = 422, description = "Unprocessable Entity"), // Failed to deserialize payload
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::create_worksheet", level = "info", skip(state, payload), err, ret(level = tracing::Level::TRACE))]
pub async fn create_worksheet(
    State(state): State<AppState>,
    Json(payload): Json<WorksheetCreatePayload>,
) -> WorksheetsResult<Json<WorksheetCreateResponse>> {
    let name = if payload.name.is_empty() {
        Utc::now().to_string()
    } else {
        payload.name
    };

    let history_worksheet = core_history::Worksheet::new(name, payload.content);

    let worksheet = state
        .history_store
        .add_worksheet(history_worksheet)
        .await
        .map_err(|e| WorksheetsAPIError::Create { source: e })?
        .into();

    Ok(Json(WorksheetCreateResponse(worksheet)))
}

#[utoipa::path(
    get,
    path = "/ui/worksheets/{worksheetId}",
    operation_id = "getWorksheet",
    tags = ["worksheets"],
    params(
        ("worksheetId" = WorksheetId, Path, description = "Worksheet id")
    ),
    responses(
        (status = 200, description = "Returns worksheet", body = WorksheetResponse),
        (status = 401,
         description = "Unauthorized",
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = ErrorResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),        
        (status = 404, description = "Worksheet not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::worksheet", level = "info", skip(state), err, ret(level = tracing::Level::TRACE))]
pub async fn worksheet(
    State(state): State<AppState>,
    Path(worksheet_id): Path<WorksheetId>,
) -> WorksheetsResult<Json<WorksheetResponse>> {
    let history_worksheet = state
        .history_store
        .get_worksheet(worksheet_id)
        .await
        .map_err(|e| WorksheetsAPIError::Get { source: e })?;

    Ok(Json(WorksheetResponse(Worksheet::from(history_worksheet))))
}

#[utoipa::path(
    delete,
    path = "/ui/worksheets/{worksheetId}",
    operation_id = "deleteWorksheet",
    tags = ["worksheets"],
    params(
        ("worksheetId" = WorksheetId, Path, description = "Worksheet id")
    ),
    responses(
        (status = 200, description = "Worksheet deleted"),
        (status = 401,
         description = "Unauthorized",
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = ErrorResponse),
        (status = 404, description = "Worksheet not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[tracing::instrument(name = "api_ui::delete_worksheet", level = "info", skip(state), err)]
pub async fn delete_worksheet(
    State(state): State<AppState>,
    Path(worksheet_id): Path<WorksheetId>,
) -> WorksheetsResult<()> {
    state
        .history_store
        .delete_worksheet(worksheet_id)
        .await
        .map_err(|e| WorksheetsAPIError::Delete { source: e })
}

#[utoipa::path(
    patch,
    path = "/ui/worksheets/{worksheetId}",
    operation_id = "updateWorksheet",
    tags = ["worksheets"],
    params(
        ("worksheetId" = WorksheetId, Path, description = "Worksheet id")
    ),
    request_body(
        content(
            (
                WorksheetUpdatePayload = "application/json", 
                examples (
                    ("rename" = (
                        value = json!(WorksheetUpdatePayload {
                            name: Some("new-worksheet".into()),
                            content: None,
                        })
                    )),
                    ("update content" = (
                        value = json!(WorksheetUpdatePayload {
                            name: None,
                            content: Some("SELECT * from test;".into()),
                        })
                    )),
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Worksheet updated"),
        (status = 401,
         description = "Unauthorized",
         headers(
            ("WWW-Authenticate" = String, description = "Bearer authentication scheme with error details")
         ),
         body = ErrorResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "Worksheet not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[tracing::instrument(
    name = "api_ui::update_worksheet",
    level = "info",
    skip(state, payload),
    err
)]
pub async fn update_worksheet(
    State(state): State<AppState>,
    Path(worksheet_id): Path<WorksheetId>,
    Json(payload): Json<WorksheetUpdatePayload>,
) -> WorksheetsResult<()> {
    if payload.name.is_none() && payload.content.is_none() {
        return Err(WorksheetsAPIError::Update {
            source: WorksheetUpdateError::NothingToUpdate,
        });
    }

    let mut worksheet = state
        .history_store
        .get_worksheet(worksheet_id)
        .await
        .map_err(|e| WorksheetsAPIError::Update {
            source: WorksheetUpdateError::Store { source: e },
        })?;

    if let Some(name) = payload.name {
        worksheet.set_name(name);
    }

    if let Some(content) = payload.content {
        worksheet.set_content(content);
    }

    state
        .history_store
        .update_worksheet(worksheet)
        .await
        .map_err(|e| WorksheetsAPIError::Update {
            source: WorksheetUpdateError::Store { source: e },
        })
}
