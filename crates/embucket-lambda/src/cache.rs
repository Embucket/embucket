use crate::InitResult;
use crate::config::EnvConfig;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::io::{self, AsyncWriteExt};
use tracing::{info, warn};

#[derive(Clone, Debug)]
struct CacheSettings {
    bucket: String,
    key: String,
    download_path: PathBuf,
    extract_dir: PathBuf,
    marker_path: PathBuf,
}

impl CacheSettings {
    fn from_env(config: &EnvConfig) -> Option<Self> {
        let bucket = config.cache_s3_bucket.clone()?;
        let key = config.cache_s3_key.clone()?;
        let download_path = config.cache_tar_path.clone();
        let extract_dir = config.cache_extract_dir.clone();
        let marker_path = extract_dir.join(".cache_ready");

        Some(Self {
            bucket,
            key,
            download_path,
            extract_dir,
            marker_path,
        })
    }
}

pub async fn download_cache_if_needed(config: &EnvConfig) -> InitResult<()> {
    let Some(settings) = CacheSettings::from_env(config) else {
        return Ok(());
    };

    let started_at = Instant::now();

    if path_exists(&settings.marker_path).await? {
        info!(
            marker = %settings.marker_path.display(),
            elapsed_ms = elapsed_ms(&started_at),
            "Cache already extracted for this execution environment"
        );
        return Ok(());
    }

    info!(
        bucket = %settings.bucket,
        key = %settings.key,
        tar_path = %settings.download_path.display(),
        extract_dir = %settings.extract_dir.display(),
        "Attempting to hydrate cache from S3"
    );

    let download_started = Instant::now();
    let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let client = S3Client::new(&aws_config);

    let downloaded = download_archive(&client, &settings).await?;
    let download_elapsed_ms = elapsed_ms(&download_started);

    if !downloaded {
        info!(
            elapsed_ms = elapsed_ms(&started_at),
            download_ms = download_elapsed_ms,
            "Cache archive not found; skipping cache warmup"
        );
        return Ok(());
    }

    let extract_started = Instant::now();
    extract_archive(&settings.download_path, &settings.extract_dir).await?;
    tokio::fs::write(&settings.marker_path, b"ready").await?;
    let extract_elapsed_ms = elapsed_ms(&extract_started);

    info!(
        marker = %settings.marker_path.display(),
        total_ms = elapsed_ms(&started_at),
        download_ms = download_elapsed_ms,
        extract_ms = extract_elapsed_ms,
        "Cache download and extraction completed"
    );
    Ok(())
}

async fn download_archive(client: &S3Client, settings: &CacheSettings) -> InitResult<bool> {
    if let Some(parent) = settings.download_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let response = client
        .get_object()
        .bucket(&settings.bucket)
        .key(&settings.key)
        .send()
        .await;

    let output = match response {
        Ok(output) => output,
        Err(SdkError::ServiceError(service_err)) if is_not_found(service_err.err()) => {
            info!(
                bucket = %settings.bucket,
                key = %settings.key,
                "Cache archive not found in S3; skipping cache warmup"
            );
            return Ok(false);
        }
        Err(err) => return Err(Box::new(err)),
    };

    let mut body = output.body.into_async_read();
    let mut file = tokio::fs::File::create(&settings.download_path).await?;
    tokio::io::copy(&mut body, &mut file).await?;
    file.flush().await?;
    Ok(true)
}

async fn extract_archive(tar_path: &Path, extract_dir: &Path) -> InitResult<()> {
    if let Some(parent) = extract_dir.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::create_dir_all(extract_dir).await?;

    let tar_path = tar_path.to_owned();
    let extract_dir = extract_dir.to_owned();

    let join_result = tokio::task::spawn_blocking(move || {
        let tar_file = std::fs::File::open(&tar_path)?;
        let mut archive = tar::Archive::new(tar_file);
        archive.unpack(extract_dir)?;
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|err| {
        warn!(error = %err, "Cache extraction task failed to join");
        boxed_error(err)
    })?;

    join_result.map_err(boxed_error)?;

    Ok(())
}

fn elapsed_ms(start: &Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn is_not_found(err: &GetObjectError) -> bool {
    matches!(err.code(), Some("NoSuchKey" | "NotFound"))
}

async fn path_exists(path: &Path) -> io::Result<bool> {
    tokio::fs::try_exists(path).await
}

fn boxed_error<E>(err: E) -> Box<dyn std::error::Error + Send + Sync>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Box::new(err)
}
