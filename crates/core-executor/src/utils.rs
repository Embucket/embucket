use super::models::QueryResult;
use crate::error::{ArrowSnafu, Result, SerdeParseSnafu, Utf8Snafu};
use arrow_schema::ArrowError;
use chrono::DateTime;
use core_history::QueryResultError;
use core_history::result_set::{Column, ResultSet, Row};
use core_metastore::SchemaIdent as MetastoreSchemaIdent;
use core_metastore::TableIdent as MetastoreTableIdent;
use datafusion::arrow::array::{
    Array, Decimal128Array, Int16Array, Int32Array, Int64Array, StringArray, StringBuilder,
    Time32MillisecondArray, Time32SecondArray, Time64MicrosecondArray, Time64NanosecondArray,
    TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
    TimestampSecondArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array, UnionArray,
};
use datafusion::arrow::array::{ArrayRef, Date32Array, Date64Array};
use datafusion::arrow::compute::cast;
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::datatypes::{Field, Schema, TimeUnit};
use datafusion::arrow::json::WriterBuilder;
use datafusion::arrow::json::writer::JsonArray;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::ScalarValue;
use datafusion_common::TableReference;
use datafusion_expr::{Expr, LogicalPlan};
use indexmap::IndexMap;
use serde_json::Value;
use snafu::ResultExt;
use sqlparser::ast::{Ident, ObjectName};
use std::collections::HashMap;
use std::sync::Arc;
use strum::{Display, EnumString};

#[derive(Clone)]
pub struct Config {
    pub embucket_version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            embucket_version: "0.1.0".to_string(),
        }
    }
}

impl Config {
    pub fn new() -> std::result::Result<Self, strum::ParseError> {
        Ok(Self {
            embucket_version: "0.1.0".to_string(),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, EnumString, Display, Default)]
#[strum(ascii_case_insensitive)]
pub enum DataSerializationFormat {
    Arrow,
    #[default]
    Json,
}

#[must_use]
pub fn is_logical_plan_effectively_empty(plan: &LogicalPlan) -> bool {
    match plan {
        LogicalPlan::EmptyRelation(e) => !e.produce_one_row,
        LogicalPlan::Projection(proj) => is_logical_plan_effectively_empty(&proj.input),
        LogicalPlan::SubqueryAlias(alias) => is_logical_plan_effectively_empty(&alias.input),
        LogicalPlan::Filter(filter) => {
            let is_false_predicate = matches!(
                filter.predicate,
                Expr::Literal(ScalarValue::Boolean(Some(false)))
            );
            is_false_predicate || is_logical_plan_effectively_empty(&filter.input)
        }
        _ => false,
    }
}

/*#[async_trait::async_trait]
pub trait S3ClientValidation: Send + Sync {
    async fn get_aws_bucket_acl(
        &self,
        request: GetBucketAclRequest,
    ) -> ControlPlaneResult<GetBucketAclOutput>;
}

#[async_trait::async_trait]
impl S3ClientValidation for S3Client {
    async fn get_aws_bucket_acl(
        &self,
        request: GetBucketAclRequest,
    ) -> ControlPlaneResult<GetBucketAclOutput> {
        self.client
            .get_bucket_acl(request)
            .await
            .map_err(ControlPlaneError::from)
    }
}

pub struct S3Client {
    client: ExternalS3Client,
}

impl S3Client {
    pub fn new(profile: &StorageProfile) -> ControlPlaneResult<Self> {
        if let Some(credentials) = profile.credentials.clone() {
            match credentials {
                Credentials::AccessKey(creds) => {
                    let profile_region = profile.region.clone().unwrap_or_default();
                    let credentials = StaticProvider::new_minimal(
                        creds.aws_access_key_id.clone(),
                        creds.aws_secret_access_key,
                    );
                    let region = Region::Custom {
                        name: profile_region.clone(),
                        endpoint: profile.endpoint.clone().unwrap_or_else(|| {
                            format!("https://s3.{profile_region}.amazonaws.com")
                        }),
                    };

                    let dispatcher =
                        HttpClient::new().context(crate::error::InvalidTLSConfigurationSnafu)?;
                    Ok(Self {
                        client: ExternalS3Client::new_with(dispatcher, credentials, region),
                    })
                }
                Credentials::Role(_) => Err(ControlPlaneError::UnsupportedAuthenticationMethod {
                    method: credentials.to_string(),
                }),
            }
        } else {
            Err(ControlPlaneError::InvalidCredentials)
        }
    }
}*/

#[must_use]
pub fn first_non_empty_type(union_array: &UnionArray) -> Option<(DataType, ArrayRef)> {
    for i in 0..union_array.type_ids().len() {
        let type_id = union_array.type_id(i);
        let child = union_array.child(type_id);
        if !child.is_empty() {
            return Some((child.data_type().clone(), Arc::clone(child)));
        }
    }
    None
}

#[allow(clippy::too_many_lines)]
pub fn convert_record_batches(
    query_result: QueryResult,
    data_format: DataSerializationFormat,
) -> Result<Vec<RecordBatch>> {
    let mut converted_batches = Vec::new();
    let column_infos = query_result.column_info();

    for batch in query_result.records {
        let mut columns = Vec::new();
        let mut fields = Vec::new();
        for (i, column) in batch.columns().iter().enumerate() {
            let metadata = column_infos[i].to_metadata();
            let field = batch.schema().field(i).clone();
            let converted_column = match field.data_type() {
                DataType::Union(..) => {
                    if let Some(union_array) = column.as_any().downcast_ref::<UnionArray>() {
                        if let Some((data_type, array)) = first_non_empty_type(union_array) {
                            fields.push(
                                Field::new(field.name(), data_type, field.is_nullable())
                                    .with_metadata(metadata),
                            );
                            array
                        } else {
                            fields.push(field.with_metadata(metadata));
                            Arc::clone(column)
                        }
                    } else {
                        fields.push(field.with_metadata(metadata));
                        Arc::clone(column)
                    }
                }
                DataType::Timestamp(unit, _) => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        Ok(convert_timestamp_to_struct(col, *unit, data_format))
                    })?
                }
                DataType::Date32 | DataType::Date64 => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        Ok(convert_date(col, data_format))
                    })?
                }
                DataType::Time32(unit) | DataType::Time64(unit) => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        Ok(convert_time(col, *unit, data_format))
                    })?
                }
                DataType::UInt64 | DataType::UInt32 | DataType::UInt16 | DataType::UInt8 => {
                    let column_info = &column_infos[i];
                    convert_uint_to_int_datatypes(
                        &mut fields,
                        &field,
                        column,
                        metadata,
                        data_format,
                        (
                            column_info.precision.unwrap_or(38),
                            column_info.scale.unwrap_or(0),
                        ),
                    )
                }
                DataType::BinaryView | DataType::Utf8View => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        cast(col, &DataType::Utf8).context(ArrowSnafu)
                    })?
                }
                DataType::Decimal128(_, _) => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        if data_format == DataSerializationFormat::Json {
                            Ok(cast(&col, &DataType::Utf8).context(ArrowSnafu)?)
                        } else {
                            Ok(Arc::clone(column))
                        }
                    })?
                }
                DataType::Boolean => {
                    convert_and_push(column, &field, metadata, &mut fields, |col| {
                        if data_format == DataSerializationFormat::Json {
                            Ok(to_utf8_array(col, true)?)
                        } else {
                            Ok(Arc::clone(column))
                        }
                    })?
                }
                _ => {
                    fields.push(field.clone().with_metadata(metadata));
                    Arc::clone(column)
                }
            };
            columns.push(converted_column);
        }
        let new_schema = Arc::new(Schema::new(fields));
        let converted_batch = RecordBatch::try_new(new_schema, columns).context(ArrowSnafu)?;
        converted_batches.push(converted_batch);
    }
    Ok(converted_batches)
}

fn convert_and_push(
    column: &ArrayRef,
    field: &Field,
    metadata: HashMap<String, String>,
    fields: &mut Vec<Field>,
    convert_fn: impl Fn(&ArrayRef) -> Result<ArrayRef>,
) -> Result<ArrayRef> {
    let converted = convert_fn(column)?;
    fields.push(
        Field::new(
            field.name(),
            converted.data_type().clone(),
            field.is_nullable(),
        )
        .with_metadata(metadata),
    );
    Ok(Arc::clone(&converted))
}

macro_rules! downcast_and_iter {
    ($column:expr, $array_type:ty) => {
        $column
            .as_any()
            .downcast_ref::<$array_type>()
            .unwrap()
            .into_iter()
    };
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
fn convert_timestamp_to_struct(
    column: &ArrayRef,
    unit: TimeUnit,
    data_format: DataSerializationFormat,
) -> ArrayRef {
    match data_format {
        DataSerializationFormat::Arrow => {
            let timestamps: Vec<_> = match unit {
                TimeUnit::Second => downcast_and_iter!(column, TimestampSecondArray).collect(),
                TimeUnit::Millisecond => {
                    downcast_and_iter!(column, TimestampMillisecondArray).collect()
                }
                TimeUnit::Microsecond => {
                    downcast_and_iter!(column, TimestampMicrosecondArray).collect()
                }
                TimeUnit::Nanosecond => {
                    downcast_and_iter!(column, TimestampNanosecondArray).collect()
                }
            };
            Arc::new(Int64Array::from(timestamps)) as ArrayRef
        }
        DataSerializationFormat::Json => {
            let timestamps: Vec<_> = match unit {
                TimeUnit::Second => downcast_and_iter!(column, TimestampSecondArray)
                    .map(|x| {
                        x.map(|ts| {
                            let ts = DateTime::from_timestamp(ts, 0).unwrap_or_default();
                            format!("{}", ts.timestamp())
                        })
                    })
                    .collect(),
                TimeUnit::Millisecond => downcast_and_iter!(column, TimestampMillisecondArray)
                    .map(|x| {
                        x.map(|ts| {
                            let ts = DateTime::from_timestamp_millis(ts).unwrap_or_default();
                            format!("{}.{}", ts.timestamp(), ts.timestamp_subsec_millis())
                        })
                    })
                    .collect(),
                TimeUnit::Microsecond => downcast_and_iter!(column, TimestampMicrosecondArray)
                    .map(|x| {
                        x.map(|ts| {
                            let ts = DateTime::from_timestamp_micros(ts).unwrap_or_default();
                            format!("{}.{}", ts.timestamp(), ts.timestamp_subsec_micros())
                        })
                    })
                    .collect(),
                TimeUnit::Nanosecond => downcast_and_iter!(column, TimestampNanosecondArray)
                    .map(|x| {
                        x.map(|ts| {
                            let ts = DateTime::from_timestamp_nanos(ts);
                            format!("{}.{}", ts.timestamp(), ts.timestamp_subsec_nanos())
                        })
                    })
                    .collect(),
            };
            Arc::new(StringArray::from(timestamps)) as ArrayRef
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
fn convert_date(column: &ArrayRef, data_format: DataSerializationFormat) -> ArrayRef {
    match data_format {
        DataSerializationFormat::Json => {
            let days: Vec<Option<i32>> = match column.data_type() {
                DataType::Date32 => downcast_and_iter!(column, Date32Array).collect(),
                DataType::Date64 => downcast_and_iter!(column, Date64Array)
                    .map(|ms| ms.map(|v| (v / 86_400_000) as i32))
                    .collect(),
                _ => return column.clone(),
            };
            Arc::new(Int32Array::from(days)) as ArrayRef
        }
        DataSerializationFormat::Arrow => column.clone(),
    }
}

#[allow(
    clippy::cast_lossless,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn convert_uint_to_int_datatypes(
    fields: &mut Vec<Field>,
    field: &Field,
    column: &ArrayRef,
    metadata: HashMap<String, String>,
    data_format: DataSerializationFormat,
    precision_scale: (i32, i32),
) -> ArrayRef {
    match data_format {
        DataSerializationFormat::Arrow => {
            match field.data_type() {
                DataType::UInt64 => {
                    fields.push(
                        Field::new(
                            field.name(),
                            DataType::Decimal128(precision_scale.0 as u8, precision_scale.1 as i8),
                            field.is_nullable(),
                        )
                        .with_metadata(metadata),
                    );
                    // converted_column
                    Arc::new(
                        Decimal128Array::from_unary(
                            column.as_any().downcast_ref::<UInt64Array>().unwrap(),
                            |x| x as i128,
                        )
                        .with_precision_and_scale(38, 0)
                        .unwrap(),
                    )
                }
                DataType::UInt32 => {
                    fields.push(
                        Field::new(field.name(), DataType::Int64, field.is_nullable())
                            .with_metadata(metadata),
                    );
                    // converted_column
                    Arc::new(Int64Array::from_unary(
                        column.as_any().downcast_ref::<UInt32Array>().unwrap(),
                        |x| x as i64,
                    ))
                }
                DataType::UInt16 => {
                    fields.push(
                        Field::new(field.name(), DataType::Int32, field.is_nullable())
                            .with_metadata(metadata),
                    );
                    // converted_column
                    Arc::new(Int32Array::from_unary(
                        column.as_any().downcast_ref::<UInt16Array>().unwrap(),
                        |x| x as i32,
                    ))
                }
                DataType::UInt8 => {
                    fields.push(
                        Field::new(field.name(), DataType::Int16, field.is_nullable())
                            .with_metadata(metadata),
                    );
                    // converted_column
                    Arc::new(Int16Array::from_unary(
                        column.as_any().downcast_ref::<UInt8Array>().unwrap(),
                        |x| x as i16,
                    ))
                }
                _ => {
                    fields.push(field.clone().with_metadata(metadata));
                    Arc::clone(column)
                }
            }
        }
        DataSerializationFormat::Json => {
            fields.push(field.clone().with_metadata(metadata));
            Arc::clone(column)
        }
    }
}
#[allow(clippy::as_conversions)]
fn convert_time(
    column: &ArrayRef,
    unit: TimeUnit,
    data_format: DataSerializationFormat,
) -> ArrayRef {
    match data_format {
        DataSerializationFormat::Json => {
            let time: Vec<_> = match unit {
                TimeUnit::Second => downcast_and_iter!(column, Time32SecondArray)
                    .map(|time| {
                        time.map(|ts| {
                            let ts = DateTime::from_timestamp(i64::from(ts), 0).unwrap_or_default();
                            //Snow sql expects value where `time = float(value[0 : -scale + 6])`
                            // `scale` for some reason by default is 9 (nanos)
                            // if we don't add this, time truncation is incorrect
                            // for any time function with seconds
                            format_time_string(ts.timestamp(), "000", 0)
                        })
                    })
                    .collect(),
                TimeUnit::Millisecond => downcast_and_iter!(column, Time32MillisecondArray)
                    .map(|time| {
                        time.map(|ts| {
                            let ts =
                                DateTime::from_timestamp_millis(i64::from(ts)).unwrap_or_default();
                            //If millis == 0, 4 zeroes after the `.` instead of 3
                            format_time_string(ts.timestamp(), ts.timestamp_subsec_millis(), 3)
                        })
                    })
                    .collect(),
                TimeUnit::Microsecond => downcast_and_iter!(column, Time64MicrosecondArray)
                    .map(|time| {
                        time.map(|ts| {
                            let ts = DateTime::from_timestamp_micros(ts).unwrap_or_default();
                            //If micros == 0, 7 zeroes after the `.` instead of 6
                            format_time_string(ts.timestamp(), ts.timestamp_subsec_micros(), 6)
                        })
                    })
                    .collect(),
                TimeUnit::Nanosecond => downcast_and_iter!(column, Time64NanosecondArray)
                    .map(|time| {
                        time.map(|ts| {
                            let ts = DateTime::from_timestamp_nanos(ts);
                            //If nanos == 0, 10 zeroes after the `.` instead of 9
                            format_time_string(ts.timestamp(), ts.timestamp_subsec_nanos(), 9)
                        })
                    })
                    .collect(),
            };
            Arc::new(StringArray::from(time)) as ArrayRef
        }
        DataSerializationFormat::Arrow => {
            let timestamps: Vec<_> = match unit {
                TimeUnit::Second => downcast_and_iter!(column, Time32SecondArray)
                    .map(|ts| ts.map(i64::from))
                    .collect(),
                TimeUnit::Millisecond => downcast_and_iter!(column, Time32MillisecondArray)
                    .map(|ts| ts.map(i64::from))
                    .collect(),
                TimeUnit::Microsecond => {
                    downcast_and_iter!(column, Time64MicrosecondArray).collect()
                }
                TimeUnit::Nanosecond => downcast_and_iter!(column, Time64NanosecondArray).collect(),
            };
            Arc::new(Int64Array::from(timestamps)) as ArrayRef
        }
    }
}

fn to_utf8_array(array: &ArrayRef, upper_case: bool) -> Result<ArrayRef> {
    let casted = cast(array, &DataType::Utf8).context(ArrowSnafu)?;
    let utf8_array = casted
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("expected Utf8 array".into()))
        .context(ArrowSnafu)?;

    let mut builder = StringBuilder::new();
    for i in 0..utf8_array.len() {
        if utf8_array.is_null(i) {
            builder.append_null();
        } else {
            let s = if upper_case {
                utf8_array.value(i).to_ascii_uppercase()
            } else {
                utf8_array.value(i).to_string()
            };
            builder.append_value(&s);
        }
    }
    Ok(Arc::new(builder.finish()))
}

/// Formats the timestamp and subsecond part into a string with the given scale.
/// `scale` is the number of digits to pad the subsecond value to.
fn format_time_string<T: std::fmt::Display>(timestamp: i64, subsecond: T, scale: usize) -> String {
    let sub_str = subsecond.to_string();
    let zeroes = scale.saturating_sub(sub_str.len());
    format!("{timestamp}.{:0>zeroes$}{sub_str}", "")
}

#[derive(Debug, Clone)]
pub struct NormalizedIdent(pub Vec<Ident>);

impl From<&NormalizedIdent> for String {
    fn from(ident: &NormalizedIdent) -> Self {
        ident
            .0
            .iter()
            .map(|i| i.value.clone())
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl From<NormalizedIdent> for MetastoreTableIdent {
    fn from(ident: NormalizedIdent) -> Self {
        let ident = ident.0;
        // TODO check len, return err. This code is just tmp
        Self {
            table: ident[2].value.clone(),
            schema: ident[1].value.clone(),
            database: ident[0].value.clone(),
        }
    }
}

impl From<NormalizedIdent> for MetastoreSchemaIdent {
    fn from(ident: NormalizedIdent) -> Self {
        let ident = ident.0;
        Self {
            schema: ident[1].value.clone(),
            database: ident[0].value.clone(),
        }
    }
}

impl From<NormalizedIdent> for ObjectName {
    fn from(ident: NormalizedIdent) -> Self {
        Self::from(ident.0)
    }
}

impl From<&NormalizedIdent> for TableReference {
    fn from(ident: &NormalizedIdent) -> Self {
        Self::parse_str(&String::from(ident))
    }
}

impl std::fmt::Display for NormalizedIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

pub fn query_result_to_history(
    result: &Result<QueryResult>,
) -> std::result::Result<ResultSet, QueryResultError> {
    match result {
        Ok(query_result) => {
            query_result_to_result_set(query_result).map_err(|err| QueryResultError {
                message: err.to_string(),
                diagnostic_message: format!("{err:?}"),
            })
        }
        Err(err) => Err(QueryResultError {
            message: err.to_string(),
            diagnostic_message: format!("{err:?}"),
        }),
    }
}

pub fn query_result_to_result_set(query_result: &QueryResult) -> Result<ResultSet> {
    let data_format = DataSerializationFormat::Arrow;

    // Convert the QueryResult to RecordBatches using the specified serialization format
    // Add columns dbt metadata to each field
    // Since we have to store already converted data to history
    let record_batches = convert_record_batches(query_result.clone(), data_format)?;
    let record_refs: Vec<&RecordBatch> = record_batches.iter().collect();

    // Serialize the RecordBatches into a JSON string using Arrow's Writer
    let buffer = Vec::new();
    let mut writer = WriterBuilder::new()
        .with_explicit_nulls(true)
        .build::<_, JsonArray>(buffer);

    writer.write_batches(&record_refs).context(ArrowSnafu)?;
    writer.finish().context(ArrowSnafu)?;

    let json_bytes = writer.into_inner();
    let json_str = String::from_utf8(json_bytes).context(Utf8Snafu)?;

    // Deserialize the JSON string into rows of values
    let raw_rows: Vec<IndexMap<String, Value>> =
        serde_json::from_str(&json_str).context(SerdeParseSnafu)?;
    let rows = raw_rows
        .into_iter()
        .map(|map| Row::new(map.into_values().collect()))
        .collect();

    // Extract column metadata from the original QueryResult
    let columns = query_result
        .column_info()
        .iter()
        .map(|ci| Column {
            name: ci.name.clone(),
            r#type: ci.r#type.clone(),
        })
        .collect();

    // Serialize original Schema into a JSON string
    let schema = serde_json::to_string(&query_result.schema).context(SerdeParseSnafu)?;
    Ok(ResultSet {
        columns,
        rows,
        data_format: data_format.to_string(),
        schema,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::as_conversions, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::models::ColumnInfo;
    use datafusion::arrow::array::{
        ArrayRef, BooleanArray, Float64Array, Int32Array, TimestampSecondArray, UInt64Array,
        UnionArray,
    };
    use datafusion::arrow::array::{BinaryViewArray, StringViewArray};
    use datafusion::arrow::buffer::ScalarBuffer;
    use datafusion::arrow::datatypes::{DataType, Field};
    use datafusion::arrow::record_batch::RecordBatch;
    use embucket_functions::datetime::timestamp_from_parts::make_time;
    use std::sync::Arc;

    #[test]
    fn test_first_non_empty_type() {
        let int_array = Int32Array::from(vec![Some(1), None, Some(34)]);
        let float_array = Float64Array::from(vec![None, Some(3.2), None]);
        let type_ids = [0_i8, 1, 0].into_iter().collect::<ScalarBuffer<i8>>();
        let union_fields = [
            (0, Arc::new(Field::new("A", DataType::Int32, false))),
            (1, Arc::new(Field::new("B", DataType::Float64, false))),
        ]
        .into_iter()
        .collect();

        let children = vec![Arc::new(int_array) as ArrayRef, Arc::new(float_array)];

        let union_array = UnionArray::try_new(union_fields, type_ids, None, children)
            .expect("Failed to create UnionArray");

        let result = first_non_empty_type(&union_array);
        assert!(result.is_some());
        let (data_type, array) = result.unwrap();
        assert_eq!(data_type, DataType::Int32);
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_convert_timestamp_to_struct() {
        let cases = [
            (
                TimeUnit::Second,
                Some(1_627_846_261),
                "1627846261",
                1_627_846_261,
            ),
            (
                TimeUnit::Millisecond,
                Some(1_627_846_261_233),
                "1627846261.233",
                1_627_846_261_233,
            ),
            (
                TimeUnit::Microsecond,
                Some(1_627_846_261_233_222),
                "1627846261.233222",
                1_627_846_261_233_222,
            ),
            (
                TimeUnit::Nanosecond,
                Some(1_627_846_261_233_222_111),
                "1627846261.233222111",
                1_627_846_261_233_222_111,
            ),
        ];
        for (unit, timestamp, expected_json, expected_arrow) in &cases {
            let values = vec![*timestamp, None];
            let timestamp_array = match unit {
                TimeUnit::Second => Arc::new(TimestampSecondArray::from(values)) as ArrayRef,
                TimeUnit::Millisecond => {
                    Arc::new(TimestampMillisecondArray::from(values)) as ArrayRef
                }
                TimeUnit::Microsecond => {
                    Arc::new(TimestampMicrosecondArray::from(values)) as ArrayRef
                }
                TimeUnit::Nanosecond => {
                    Arc::new(TimestampNanosecondArray::from(values)) as ArrayRef
                }
            };
            let result =
                convert_timestamp_to_struct(&timestamp_array, *unit, DataSerializationFormat::Json);
            let string_array = result.as_any().downcast_ref::<StringArray>().unwrap();
            assert_eq!(string_array.len(), 2);
            assert_eq!(string_array.value(0), *expected_json);
            assert!(string_array.is_null(1));
            let result = convert_timestamp_to_struct(
                &timestamp_array,
                *unit,
                DataSerializationFormat::Arrow,
            );
            let string_array = result.as_any().downcast_ref::<Int64Array>().unwrap();
            assert_eq!(string_array.len(), 2);
            assert_eq!(string_array.value(0), *expected_arrow);
            assert!(string_array.is_null(1));
        }
    }

    #[test]
    fn test_convert_time() {
        let cases = [
            (
                TimeUnit::Second,
                make_time(12, 54, 33, None) / 1_000_000_000,
                "46473.000",
                46_473i64,
            ),
            (
                TimeUnit::Millisecond,
                make_time(12, 54, 33, Some(333_000_000)) / 1_000_000,
                "46473.333",
                46_473_333i64,
            ),
            (
                TimeUnit::Millisecond,
                make_time(12, 54, 33, None) / 1_000_000,
                "46473.000",
                46_473_000i64,
            ),
            (
                TimeUnit::Microsecond,
                make_time(12, 54, 33, Some(333_000)) / 1_000,
                "46473.000333",
                46_473_000_333i64,
            ),
            (
                TimeUnit::Microsecond,
                make_time(12, 54, 33, None) / 1_000,
                "46473.000000",
                46_473_000_000i64,
            ),
            (
                TimeUnit::Nanosecond,
                make_time(12, 54, 33, Some(333)),
                "46473.000000333",
                46_473_000_000_333i64,
            ),
            (
                TimeUnit::Nanosecond,
                make_time(12, 54, 33, None),
                "46473.000000000",
                46_473_000_000_000i64,
            ),
        ];
        for (unit, timestamp, expected_json, expected_arrow) in &cases {
            let time_array = match unit {
                TimeUnit::Second => {
                    let values = vec![i32::try_from(*timestamp).ok(), None];
                    Arc::new(Time32SecondArray::from(values)) as ArrayRef
                }
                TimeUnit::Millisecond => {
                    let values = vec![i32::try_from(*timestamp).ok(), None];
                    Arc::new(Time32MillisecondArray::from(values)) as ArrayRef
                }
                TimeUnit::Microsecond => {
                    let values = vec![Some(*timestamp), None];
                    Arc::new(Time64MicrosecondArray::from(values)) as ArrayRef
                }
                TimeUnit::Nanosecond => {
                    let values = vec![Some(*timestamp), None];
                    Arc::new(Time64NanosecondArray::from(values)) as ArrayRef
                }
            };
            let result = convert_time(&time_array, *unit, DataSerializationFormat::Json);
            let string_array = result.as_any().downcast_ref::<StringArray>().unwrap();
            assert_eq!(string_array.len(), 2);
            assert_eq!(string_array.value(0), *expected_json);
            assert!(string_array.is_null(1));
            let result = convert_time(&time_array, *unit, DataSerializationFormat::Arrow);
            let string_array = result.as_any().downcast_ref::<Int64Array>().unwrap();
            assert_eq!(string_array.len(), 2);
            assert_eq!(string_array.value(0), *expected_arrow);
            assert!(string_array.is_null(1));
        }
    }

    #[test]
    fn test_convert_record_batches() {
        // helper to downcast to StringArray
        fn as_str_array(array: &ArrayRef) -> &StringArray {
            array.as_any().downcast_ref::<StringArray>().unwrap()
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("int_col", DataType::Int32, false),
            Field::new("ts_col", DataType::Timestamp(TimeUnit::Second, None), true),
            Field::new("binary_view", DataType::BinaryView, true),
            Field::new("binary_view", DataType::Utf8View, true),
            Field::new("boolean", DataType::Boolean, true),
        ]));
        let int_array = Arc::new(Int32Array::from(vec![1, 2, 3])) as ArrayRef;
        let timestamp_array = Arc::new(TimestampSecondArray::from(vec![
            Some(1_627_846_261),
            None,
            Some(1_627_846_262),
        ])) as ArrayRef;
        let binary_view_array = Arc::new(BinaryViewArray::from_iter_values(vec![
            b"hello" as &[u8],
            b"world",
            b"lulu",
        ]));
        let utf8_view_array = Arc::new(StringViewArray::from_iter_values(vec![
            "hello", "world", "lulu",
        ]));
        let bool_array = Arc::new(BooleanArray::from(vec![Some(false), Some(true), None]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                int_array,
                timestamp_array,
                binary_view_array,
                utf8_view_array,
                bool_array,
            ],
        )
        .unwrap();
        let result = QueryResult::new(vec![batch], schema, 0);
        let column_infos = result.column_info();

        // === JSON conversion ===
        let converted_batches =
            convert_record_batches(result.clone(), DataSerializationFormat::Json).unwrap();
        assert_eq!(converted_batches.len(), 1);

        let batch = &converted_batches[0];
        assert_eq!(batch.num_columns(), 5);
        assert_eq!(batch.num_rows(), 3);

        // timestamp → string
        let arr = as_str_array(batch.column(1));
        assert_eq!(arr.value(0), "1627846261");
        assert!(arr.is_null(1));
        assert_eq!(arr.value(2), "1627846262");

        // binary_view → string
        let arr = as_str_array(batch.column(2));
        assert_eq!(arr.value(0), "hello");
        assert_eq!(arr.value(1), "world");
        assert_eq!(arr.value(2), "lulu");

        // utf8_view → string
        let arr = as_str_array(batch.column(3));
        assert_eq!(arr.value(0), "hello");
        assert_eq!(arr.value(1), "world");
        assert_eq!(arr.value(2), "lulu");

        // boolean → "TRUE"/"FALSE"/""
        let arr = as_str_array(batch.column(4));
        assert_eq!(arr.value(0), "FALSE");
        assert_eq!(arr.value(1), "TRUE");
        assert_eq!(arr.value(2), ""); // null boolean to empty string

        // column info
        assert_eq!(column_infos[0].name, "int_col");
        assert_eq!(column_infos[0].r#type, "fixed");
        assert_eq!(column_infos[1].name, "ts_col");
        assert_eq!(column_infos[1].r#type, "timestamp_ntz");
        assert_eq!(column_infos[2].name, "binary_view");
        assert_eq!(column_infos[2].r#type, "binary");

        // === Arrow conversion ===
        let converted_batches =
            convert_record_batches(result, DataSerializationFormat::Arrow).unwrap();
        let converted_batch = &converted_batches[0];
        let arr = converted_batch
            .column(1)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(arr.value(0), 1_627_846_261);
        assert!(arr.is_null(1));
        assert_eq!(arr.value(2), 1_627_846_262);
    }

    #[allow(
        clippy::needless_pass_by_value,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    fn check_record_batches_uint_to_int(
        batches: Vec<RecordBatch>,
        converted_batches: Vec<RecordBatch>,
        column_infos: Vec<ColumnInfo>,
    ) -> i32 {
        assert_eq!(batches.len(), 1);
        assert_eq!(converted_batches.len(), 1);

        let batch = &batches[0];
        let converted_batch = &converted_batches[0];

        assert_eq!(converted_batch.num_columns(), batch.num_columns());
        assert_eq!(converted_batch.num_rows(), batch.num_rows());

        let mut fields_tested = 0;
        for (i, field) in batch.schema().fields.into_iter().enumerate() {
            let converted_field = converted_batch
                .schema_ref()
                .field_with_name(field.name())
                .unwrap();

            let column_info = &column_infos[i];
            let converted_column = &converted_batch.columns()[i];
            assert_eq!(column_infos[i].name, *converted_field.name());
            assert_eq!(
                column_infos[i].r#type.to_ascii_uppercase(),
                converted_field.metadata()["logicalType"]
            );
            // natively precision is u8, scale i8 but column_info is using i32
            let metadata_precision: i32 = converted_field.metadata()["precision"].parse().unwrap();
            let metadata_scale: i32 = converted_field.metadata()["scale"].parse().unwrap();
            assert_ne!(column_info.precision.unwrap(), 0);
            assert_eq!(column_info.precision.unwrap(), metadata_precision);
            assert_eq!(column_info.scale.unwrap(), metadata_scale);
            match field.data_type() {
                DataType::UInt64 => {
                    assert_eq!(
                        *converted_field.data_type(),
                        DataType::Decimal128(metadata_precision as u8, metadata_scale as i8)
                    );
                    let values: Decimal128Array = converted_column
                        .as_any()
                        .downcast_ref::<Decimal128Array>()
                        .unwrap()
                        .into_iter()
                        .collect();
                    assert_eq!(
                        values,
                        Decimal128Array::from(vec![0, 1, i128::from(u64::MAX)])
                    );
                    fields_tested += 1;
                }
                DataType::UInt32 => {
                    assert_eq!(*converted_field.data_type(), DataType::Int64);
                    let values: Int64Array = converted_column
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .unwrap()
                        .into_iter()
                        .collect();
                    assert_eq!(values, Int64Array::from(vec![0, 1, i64::from(u32::MAX)]));
                    fields_tested += 1;
                }
                DataType::UInt16 => {
                    assert_eq!(*converted_field.data_type(), DataType::Int32);
                    let values: Int32Array = converted_column
                        .as_any()
                        .downcast_ref::<Int32Array>()
                        .unwrap()
                        .into_iter()
                        .collect();
                    assert_eq!(values, Int32Array::from(vec![0, 1, i32::from(u16::MAX)]));
                    fields_tested += 1;
                }
                DataType::UInt8 => {
                    assert_eq!(*converted_field.data_type(), DataType::Int16);
                    let values: Int16Array = converted_column
                        .as_any()
                        .downcast_ref::<Int16Array>()
                        .unwrap()
                        .into_iter()
                        .collect();
                    assert_eq!(values, Int16Array::from(vec![0, 1, i16::from(u8::MAX)]));
                    fields_tested += 1;
                }
                _ => {
                    panic!("Bad DataType: {}", field.data_type());
                }
            }
        }
        fields_tested
    }

    #[test]
    fn test_convert_record_batches_uint() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("row_num_uint64", DataType::UInt64, false),
            Field::new("row_num_uint32", DataType::UInt32, false),
            Field::new("row_num_uint16", DataType::UInt16, false),
            Field::new("row_num_uint8", DataType::UInt8, false),
        ]));
        let record_batches = vec![
            RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(UInt64Array::from(vec![0, 1, u64::MAX])),
                    Arc::new(UInt32Array::from(vec![0, 1, u32::MAX])),
                    Arc::new(UInt16Array::from(vec![0, 1, u16::MAX])),
                    Arc::new(UInt8Array::from(vec![0, 1, u8::MAX])),
                ],
            )
            .unwrap(),
        ];
        let query_result = QueryResult::new(record_batches.clone(), schema, 0);
        let column_infos = query_result.column_info();
        let converted_batches =
            convert_record_batches(query_result, DataSerializationFormat::Arrow).unwrap();

        let fields_tested =
            check_record_batches_uint_to_int(record_batches, converted_batches, column_infos);
        assert_eq!(fields_tested, 4);
    }

    #[test]
    fn test_convert_record_batches_dates() {
        let date32_values = vec![Some(1), Some(2), None];
        let date64_values = vec![Some(86_400_000), Some(172_800_000), None]; // 1, 2 days in ms

        let date32_array = Arc::new(Date32Array::from(date32_values.clone())) as ArrayRef;
        let date64_array = Arc::new(Date64Array::from(date64_values.clone())) as ArrayRef;

        let schema = Arc::new(Schema::new(vec![
            Field::new("date32_col", DataType::Date32, true),
            Field::new("date64_col", DataType::Date64, true),
        ]));

        let record_batch =
            RecordBatch::try_new(schema.clone(), vec![date32_array, date64_array]).unwrap();
        let query_result = QueryResult::new(vec![record_batch], schema, 0);
        let converted_batches =
            convert_record_batches(query_result, DataSerializationFormat::Json).unwrap();
        let converted_batch = &converted_batches[0];

        let result_date32 = converted_batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        let result_date64 = converted_batch
            .column(1)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();

        for i in 0..3 {
            assert_eq!(result_date32.is_null(i), date32_values[i].is_none());
            assert_eq!(result_date64.is_null(i), date64_values[i].is_none());
            if let (Some(orig32), Some(orig64)) = (date32_values[i], date64_values[i]) {
                assert_eq!(result_date32.value(i), orig32);
                assert_eq!(
                    result_date64.value(i),
                    i32::try_from(orig64 / 86_400_000).unwrap()
                );
            }
        }
    }
}
