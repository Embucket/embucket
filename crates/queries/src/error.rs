#![allow(unused_assignments)]
use error_stack_trace;
use snafu::Location;
use snafu::prelude::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu)]
#[snafu(visibility(pub))]
#[error_stack_trace::debug]
pub enum Error {
    #[snafu(display("Failed to get connection from pool: {error}"))]
    Pool {
        #[snafu(source)]
        error: deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Generic error: {error}"))]
    Generic {
        #[snafu(source)]
        error: Box<dyn std::error::Error + 'static + Send + Sync>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Diesel error: {error}"))]
    Diesel {
        #[snafu(source)]
        error: diesel::result::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
