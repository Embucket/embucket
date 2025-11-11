use error::Result;
use snafu::ResultExt;
use tokio::runtime::Builder;

#[allow(clippy::module_inception)]
pub mod catalog;
pub mod catalog_list;
pub mod catalogs;
pub mod df_error;
pub mod error;
pub mod information_schema;
pub mod schema;
pub mod table;

// TBD: Should we move this into a separate crate? As this is duplicate implementation
// of what we have in the functions crate.
pub fn block_in_new_runtime<F, R>(future: F) -> Result<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    std::thread::spawn(move || {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .context(error::CreateTokioRuntimeSnafu)
            .map(|rt| rt.block_on(future))
    })
    .join()
    .unwrap_or_else(|_| error::ThreadPanickedWhileExecutingFutureSnafu.fail()?)
}
