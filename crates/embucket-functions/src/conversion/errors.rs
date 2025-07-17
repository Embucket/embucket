use snafu::{Location, Snafu};

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Format must be a non-null scalar value"))]
    FormatMustBeNonNullScalarValue {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported input type: {data_type:?}"))]
    UnsupportedInputType {
        data_type: arrow_schema::DataType,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to decode hex string: {error}"))]
    FailedToDecodeHexString {
        error: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to decode base64 string: {error}"))]
    FailedToDecodeBase64String {
        error: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported format: {format}. Valid formats are HEX, BASE64, and UTF-8"))]
    UnsupportedFormat {
        format: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid boolean string: {v}"))]
    InvalidBooleanString {
        v: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Argument 2 needs to be integer"))]
    ArgumentTwoNeedsToBeInteger {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid value for function at position 2"))]
    InvalidValueForFunctionAtPositionTwo {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't cast to {v}"))]
    CantCastTo {
        v: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't parse timestamp"))]
    CantParseTimestamp {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't get timestamp"))]
    CantGetTimestamp {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't parse timezone"))]
    CantParseTimezone {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't create DateTime from timestamp"))]
    CantCreateDateTimeFromTimestamp {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Can't add local timezone"))]
    CantAddLocalTimezone {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid datatype"))]
    InvalidDataType {
        #[snafu(implicit)]
        location: Location,
    },
}

// Enum variants from this error return DataFusionError
// Following is made to preserve logical structure of error:
// DataFusionError::External
// |---- DataFusionInternalError::Conversion
//       |---- Error

impl From<Error> for datafusion_common::DataFusionError {
    fn from(value: Error) -> Self {
        Self::External(Box::new(crate::df_error::DFExternalError::Conversion {
            source: value,
        }))
    }
}

impl Default for Error {
    fn default() -> Self {
        UnsupportedInputTypeSnafu {
            data_type: arrow_schema::DataType::Boolean,
        }
        .build()
    }
}
