use crate::SqlState;
use crate::models::{JsonResponse, ResponseData, RowSet};
use crate::server::error::{self as api_snowflake_rest_error, Error, Result};
use base64;
use base64::engine::general_purpose::STANDARD as engine_base64;
use base64::prelude::*;
use datafusion::arrow::ipc::MetadataVersion;
use datafusion::arrow::ipc::writer::{IpcWriteOptions, StreamWriter};
use datafusion::arrow::json::{StructMode, WriterBuilder, writer::JsonArray};
use datafusion::arrow::record_batch::RecordBatch;
use executor::models::QueryResult;
use executor::utils::{
    DataSerializationFormat, convert_record_batches, convert_struct_to_timestamp,
};
use snafu::ResultExt;
use tracing;
use uuid::Uuid;

// https://arrow.apache.org/docs/format/Columnar.html#buffer-alignment-and-padding
// Buffer Alignment and Padding: Implementations are recommended to allocate memory
// on aligned addresses (multiple of 8- or 64-bytes) and pad (overallocate) to a
// length that is a multiple of 8 or 64 bytes. When serializing Arrow data for interprocess
// communication, these alignment and padding requirements are enforced.
// For more info see issue #115
const ARROW_IPC_ALIGNMENT: usize = 8;

fn records_to_arrow_string(recs: &[RecordBatch]) -> std::result::Result<String, Error> {
    let mut buf = Vec::new();
    let options = IpcWriteOptions::try_new(ARROW_IPC_ALIGNMENT, false, MetadataVersion::V5)
        .context(api_snowflake_rest_error::ArrowSnafu)?;
    if !recs.is_empty() {
        let mut writer =
            StreamWriter::try_new_with_options(&mut buf, recs[0].schema_ref(), options)
                .context(api_snowflake_rest_error::ArrowSnafu)?;
        for rec in recs {
            writer
                .write(rec)
                .context(api_snowflake_rest_error::ArrowSnafu)?;
        }
        writer
            .finish()
            .context(api_snowflake_rest_error::ArrowSnafu)?;
        drop(writer);
    }
    Ok(engine_base64.encode(buf))
}

fn records_to_json_string(recs: &[RecordBatch]) -> std::result::Result<String, Error> {
    let recs = recs.iter().collect::<Vec<_>>();

    let buf = Vec::new();
    let mut writer = WriterBuilder::new()
        .with_struct_mode(StructMode::ListOnly)
        .with_explicit_nulls(true)
        .build::<_, JsonArray>(buf);

    writer
        .write_batches(&recs)
        .context(api_snowflake_rest_error::ArrowSnafu)?;
    writer
        .finish()
        .context(api_snowflake_rest_error::ArrowSnafu)?;
    let buf = writer.into_inner();
    // it is expected to be cheap, as no allocations just reuses underlying buffer
    String::from_utf8(buf).context(api_snowflake_rest_error::Utf8Snafu)
}

#[tracing::instrument(
    name = "handle_query_ok_result",
    level = "debug",
    err,
    ret(level = tracing::Level::TRACE)
)]
pub fn handle_query_ok_result(
    sql_text: &str,
    query_id: Uuid,
    query_result: QueryResult,
    ser_fmt: DataSerializationFormat,
) -> Result<JsonResponse> {
    // Convert the QueryResult to RecordBatches using the specified serialization format
    // Add columns dbt metadata to each field
    let records = convert_record_batches(&query_result, ser_fmt)?;
    let total_rows = records
        .iter()
        .map(|batch| i64::try_from(batch.num_rows()).unwrap_or(i64::MAX))
        .sum();

    let row_set = if ser_fmt == DataSerializationFormat::Json {
        // Convert struct timestamp columns to string representation
        let records = convert_struct_to_timestamp(&records)?;
        let serialized_rowset = records_to_json_string(&records)?;
        Some(RowSet::Raw(serialized_rowset))
    } else {
        None
    };
    let row_set_base_64 = if ser_fmt == DataSerializationFormat::Arrow {
        Option::from(records_to_arrow_string(&records)?)
    } else {
        None
    };
    let returned_rows = total_rows;

    let json_resp = JsonResponse {
        data: Option::from(ResponseData {
            row_type: query_result
                .column_info()
                .into_iter()
                .map(Into::into)
                .collect(),
            query_result_format: Some(ser_fmt.to_string().to_lowercase()),
            row_set,
            row_set_base_64,
            total: Some(total_rows),
            returned: Some(returned_rows),
            query_id: Some(query_id.to_string()),
            error_code: None,
            sql_state: Some(SqlState::Success.to_string()),
        }),
        success: true,
        message: Option::from("successfully executed".to_string()),
        error_stack_trace: None,
        debug_error: None,
        code: None,
    };
    Ok(json_resp)
}
