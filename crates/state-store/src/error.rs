#![allow(unused_assignments)]
use aws_sdk_dynamodb::config::http::HttpResponse;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::get_item::GetItemError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::operation::query::QueryError;
use serde_dynamo::Error as SerdeDynamoError;
use snafu::{Location, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the state store helpers.
#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Environment variable {reason} is not set"))]
    MissingEnvVar { reason: String },
    #[snafu(display("Failed to serialize JSON: {error}"))]
    FailedToParseJson {
        #[snafu(source)]
        error: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Dynamodb get item error: {error}"))]
    DynamoDbGetItem {
        #[snafu(source(from(SdkError<GetItemError, HttpResponse>, Box::new)))]
        error: Box<SdkError<GetItemError, HttpResponse>>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Dynamodb put item error: {error}"))]
    DynamoDbPutItem {
        #[snafu(source(from(SdkError<PutItemError, HttpResponse>, Box::new)))]
        error: Box<SdkError<PutItemError, HttpResponse>>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Dynamodb delete item error: {error}"))]
    DynamoDbDeleteItem {
        #[snafu(source(from(SdkError<DeleteItemError, HttpResponse>, Box::new)))]
        error: Box<SdkError<DeleteItemError, HttpResponse>>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Dynamodb query error: {error}"))]
    DynamoDbQueryOutput {
        #[snafu(source(from(SdkError<QueryError, HttpResponse>, Box::new)))]
        error: Box<SdkError<QueryError, HttpResponse>>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Dynamodb query error: {error}"))]
    DynamoDbCredentialsError {
        #[snafu(source(from(aws_credential_types::provider::error::CredentialsError, Box::new)))]
        error: Box<aws_credential_types::provider::error::CredentialsError>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize DynamoDB item: {error}"))]
    FailedToSerializeDynamo {
        #[snafu(source)]
        error: SerdeDynamoError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize DynamoDB item: {error}"))]
    FailedToDeserializeDynamo {
        #[snafu(source)]
        error: SerdeDynamoError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("item not found"))]
    NotFound,
    #[snafu(display("data attribute missing from DynamoDB item"))]
    MissingData,
    #[snafu(display("invalid time: {value}"))]
    InvalidTime { value: String },
}
