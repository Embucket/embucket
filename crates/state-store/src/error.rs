use aws_sdk_dynamodb::config::http::HttpResponse;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::get_item::GetItemError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
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
    #[snafu(display("item not found"))]
    NotFound,
    #[snafu(display("data attribute missing from DynamoDB item"))]
    MissingData,
    #[snafu(display("Error: {name} not implemented"))]
    NotImplemented {
        name: String,
        #[snafu(implicit)]
        location: Location,
    },
}
