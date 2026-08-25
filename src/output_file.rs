use crate::error::AppError;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

// ponytail: 1ファイル64 MiBを上限とし、正当なデータが超える場合だけ設定化する。
const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) async fn atomic_write<I>(path: &Path, chunks: I) -> Result<(), AppError>
where
    I: IntoIterator<Item = String>,
{
    atomic_write_with_limit(path, chunks, MAX_OUTPUT_BYTES).await
}

async fn atomic_write_with_limit<I>(path: &Path, chunks: I, max_bytes: u64) -> Result<(), AppError>
where
    I: IntoIterator<Item = String>,
{
    let directory = path
        .parent()
        .map_or_else(|| Path::new("."), |parent| parent);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map_or("output", |name| name);
    let mut temporary_path = PathBuf::from(directory);
    temporary_path.push(format!(".{file_name}.tmp.{}", rand::random::<u64>()));

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary_path)
        .await?;
    let mut writer = BufWriter::new(file);

    let write_result: Result<(), AppError> = async {
        let mut total_bytes = 0u64;
        for chunk in chunks {
            let chunk_bytes = u64::try_from(chunk.len())
                .map_err(|_| AppError::Other("Output chunk length exceeds u64".into()))?;
            total_bytes = total_bytes
                .checked_add(chunk_bytes)
                .ok_or_else(|| AppError::Other("Output size overflow".into()))?;
            if total_bytes > max_bytes {
                return Err(AppError::Other(format!(
                    "Output too large ({total_bytes} bytes > {max_bytes} bytes)"
                )));
            }
            writer.write_all(chunk.as_bytes()).await?;
        }
        writer.flush().await?;
        writer.get_ref().sync_all().await?;
        Ok(())
    }
    .await;
    drop(writer);

    if let Err(error) = write_result {
        let _cleanup_result = fs::remove_file(&temporary_path).await;
        return Err(error);
    }

    if let Err(error) = fs::rename(&temporary_path, path).await {
        let _cleanup_result = fs::remove_file(&temporary_path).await;
        return Err(error.into());
    }

    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/output_file.rs"]
mod tests;
