mod error;
pub mod file_utils;
mod java_install;
pub mod json;
pub mod print;

pub use error::{IoError, JsonDownloadError, JsonFileError};
pub use file_utils::RequestError;
use futures::StreamExt;
pub use java_install::{get_java_binary, JavaInstallError, JavaInstallProgress};

/// Limit on how many files to download concurrently.
const JOBS: usize = 64;

pub async fn do_jobs<ResultType>(
    results: impl Iterator<Item = impl std::future::Future<Output = ResultType>>,
) -> Vec<ResultType> {
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut outputs = Vec::new();

    for result in results {
        tasks.push(result);
        if tasks.len() > JOBS {
            if let Some(task) = tasks.next().await {
                outputs.push(task);
            }
        }
    }

    while let Some(task) = tasks.next().await {
        outputs.push(task);
    }
    outputs
}
