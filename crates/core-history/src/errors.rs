use error_stack_trace;
use slatedb::SlateDBError;
use snafu::Location;
use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu)]
#[snafu(visibility(pub(crate)))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Error using key: {error}"))]
    BadKey {
        #[snafu(source)]
        error: std::str::Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error adding worksheet: {source}"))]
    WorksheetAdd {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error getting worksheet: {source}"))]
    WorksheetGet {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error getting worksheets: {source}"))]
    WorksheetsList {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error deleting worksheet: {source}"))]
    WorksheetDelete {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error updating worksheet: {source}"))]
    WorksheetUpdate {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error adding query record: {source}"))]
    QueryAdd {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Can't locate query record by key: {key}"))]
    QueryNotFound {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error adding query record reference: {source}"))]
    QueryReferenceAdd {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error getting query history: {source}"))]
    QueryGet {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Can't locate worksheet by key: {message}"))]
    WorksheetNotFound {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Bad query record reference key: {key}"))]
    QueryReferenceKey {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error getting worksheet queries: {source}"))]
    GetWorksheetQueries {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Error adding query inverted key: {source}"))]
    QueryInvertedKeyAdd {
        source: core_utils::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Query item seek error: {error}"))]
    Seek {
        #[snafu(source)]
        error: SlateDBError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Deserialize error: {error}"))]
    DeserializeValue {
        #[snafu(source)]
        error: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Query execution error: {message}"))]
    ExecutionResult {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}
