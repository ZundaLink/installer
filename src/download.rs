use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;
use tokio::time::{interval, Duration};

const DOWNLOAD_THREADS_PER_FILE: usize = 16;
const MAX_CONCURRENT_FILES: usize = 5;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
    pub status: DownloadStatus,
    pub verify_progress: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Verifying,
    Completed,
    Failed(String),
    Skipped,
}

pub struct Downloader {
    temp_dir: String,
    progress_tx: mpsc::Sender<DownloadProgress>,
    skip_verify: Arc<AtomicBool>,
}

impl Downloader {
    pub fn new(temp_dir: &str, progress_tx: mpsc::Sender<DownloadProgress>) -> Self {
        Self {
            temp_dir: temp_dir.to_string(),
            progress_tx,
            skip_verify: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn new_with_skip_flag(temp_dir: &str, progress_tx: mpsc::Sender<DownloadProgress>, skip_verify: Arc<AtomicBool>) -> Self {
        Self {
            temp_dir: temp_dir.to_string(),
            progress_tx,
            skip_verify,
        }
    }

    pub fn set_skip_verify(&self, skip: bool) {
        self.skip_verify.store(skip, Ordering::Relaxed);
    }

    pub async fn download_file(
        &self,
        filename: &str,
        urls: &[String],
        expected_size: u64,
        expected_sha256: &str,
    ) -> Result<String> {
        let file_path = format!("{}/{}", self.temp_dir, filename);
        let temp_path = format!("{}.tmp", file_path);

        fs::create_dir_all(&self.temp_dir)?;

        // Handle empty file (size == 0)
        if expected_size == 0 {
            // Create empty file
            File::create(&file_path)?;
            self.send_progress(filename, 0, 0, DownloadStatus::Completed).await;
            return Ok(file_path);
        }

        // Check if file already exists and is valid
        if Path::new(&file_path).exists() {
            let metadata = fs::metadata(&file_path)?;
            if metadata.len() == expected_size {
                self.send_progress(filename, expected_size, expected_size, DownloadStatus::Verifying).await;
                if self.verify_sha256(&file_path, expected_sha256, filename, expected_size).await? {
                    self.send_progress(filename, expected_size, expected_size, DownloadStatus::Completed).await;
                    return Ok(file_path);
                }
            }
        }

        // Multi-threaded download
        self.send_progress(filename, 0, expected_size, DownloadStatus::Downloading).await;

        let mut last_error = None;
        for url in urls {
            match self.download_with_multi_threads(url, &temp_path, expected_size, filename).await {
                Ok(()) => {
                    self.send_progress(filename, expected_size, expected_size, DownloadStatus::Verifying).await;
                    if self.verify_sha256(&temp_path, expected_sha256, filename, expected_size).await? {
                        fs::rename(&temp_path, &file_path)?;
                        self.send_progress(filename, expected_size, expected_size, DownloadStatus::Completed).await;
                        return Ok(file_path);
                    } else {
                        fs::remove_file(&temp_path)?;
                        last_error = Some(anyhow!("SHA256 verification failed"));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        self.send_progress(filename, 0, expected_size, DownloadStatus::Failed(last_error.as_ref().map(|e| e.to_string()).unwrap_or_default())).await;
        Err(last_error.unwrap_or_else(|| anyhow!("All download URLs failed")))
    }

    async fn download_with_multi_threads(
        &self,
        url: &str,
        temp_path: &str,
        total_size: u64,
        filename: &str,
    ) -> Result<()> {
        let client = Arc::new(
            reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()?
        );

        // Shared atomic counter for total downloaded bytes
        let total_downloaded = Arc::new(AtomicU64::new(0));

        // Calculate chunk size for each thread
        let chunk_size = total_size / DOWNLOAD_THREADS_PER_FILE as u64;
        let mut tasks = JoinSet::new();
        let progress_tx = self.progress_tx.clone();
        let filename = filename.to_string();
        let url = url.to_string();
        let temp_path = temp_path.to_string();

        // Create temp directory for chunks
        let chunk_dir = format!("{}.chunks", temp_path);
        fs::create_dir_all(&chunk_dir)?;

        // Check for existing chunks and calculate already downloaded bytes
        let mut existing_downloaded: u64 = 0;
        for thread_id in 0..DOWNLOAD_THREADS_PER_FILE {
            let chunk_path = format!("{}/chunk_{}", chunk_dir, thread_id);
            if Path::new(&chunk_path).exists() {
                if let Ok(metadata) = fs::metadata(&chunk_path) {
                    existing_downloaded += metadata.len();
                }
            }
        }
        // Initialize the total downloaded counter with existing progress
        total_downloaded.store(existing_downloaded, Ordering::Relaxed);

        // Spawn a task to periodically report progress
        let progress_filename = filename.clone();
        let progress_total_downloaded = Arc::clone(&total_downloaded);
        let progress_sender = progress_tx.clone();
        let progress_task = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(100));
            loop {
                ticker.tick().await;
                let downloaded = progress_total_downloaded.load(Ordering::Relaxed);
                let _ = progress_sender.try_send(DownloadProgress {
                    filename: progress_filename.clone(),
                    downloaded,
                    total: total_size,
                    status: DownloadStatus::Downloading,
                    verify_progress: 0.0,
                });
            }
        });

        // Spawn download threads
        for thread_id in 0..DOWNLOAD_THREADS_PER_FILE {
            let start_byte = thread_id as u64 * chunk_size;
            let end_byte = if thread_id == DOWNLOAD_THREADS_PER_FILE - 1 {
                total_size - 1
            } else {
                (thread_id as u64 + 1) * chunk_size - 1
            };

            let client = Arc::clone(&client);
            let url = url.clone();
            let filename = filename.clone();
            let chunk_dir = chunk_dir.clone();
            let total_downloaded = Arc::clone(&total_downloaded);

            tasks.spawn(async move {
                download_chunk(
                    client,
                    &url,
                    &chunk_dir,
                    thread_id,
                    start_byte,
                    end_byte,
                    &filename,
                    total_downloaded,
                    total_size,
                ).await
            });
        }

        // Wait for all chunks to complete
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(chunk_info)) => results.push(chunk_info),
                Ok(Err(e)) => {
                    progress_task.abort();
                    return Err(e);
                }
                Err(e) => {
                    progress_task.abort();
                    return Err(anyhow!("Task join error: {}", e));
                }
            }
        }

        // Stop progress reporting
        progress_task.abort();

        // Send final progress
        let _ = self.progress_tx.send(DownloadProgress {
            filename: filename.clone(),
            downloaded: total_size,
            total: total_size,
            status: DownloadStatus::Downloading,
            verify_progress: 0.0,
        }).await;

        // Sort by thread_id and merge chunks
        results.sort_by_key(|(id, _)| *id);

        // Merge chunks into final file
        let mut output_file = File::create(&temp_path)?;
        for (thread_id, _) in results {
            let chunk_path = format!("{}/chunk_{}", chunk_dir, thread_id);
            let mut chunk_file = File::open(&chunk_path)?;
            let mut buffer = Vec::new();
            chunk_file.read_to_end(&mut buffer)?;
            output_file.write_all(&buffer)?;
            // Clean up chunk file
            let _ = fs::remove_file(&chunk_path);
        }

        // Clean up chunk directory
        let _ = fs::remove_dir(&chunk_dir);

        output_file.flush()?;
        Ok(())
    }

    async fn verify_sha256(&self, file_path: &str, expected_hash: &str, filename: &str, file_size: u64) -> Result<bool> {
        // Handle empty file (size == 0)
        if file_size == 0 {
            self.send_progress(filename, 0, 0, DownloadStatus::Completed).await;
            return Ok(true);
        }

        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        let mut verified_bytes: u64 = 0;

        loop {
            // Check if skip verification is requested
            if self.skip_verify.load(Ordering::Relaxed) {
                self.send_progress(filename, file_size, file_size, DownloadStatus::Skipped).await;
                return Ok(true); // Treat as success when skipped
            }

            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
            verified_bytes += bytes_read as u64;

            let verify_progress = if file_size > 0 {
                verified_bytes as f32 / file_size as f32
            } else {
                0.0
            };

            self.send_progress_with_verify(filename, file_size, file_size, DownloadStatus::Verifying, verify_progress).await;
        }

        let result = hasher.finalize();
        let computed_hash = hex::encode(result);

        Ok(computed_hash.to_lowercase() == expected_hash.to_lowercase())
    }

    async fn send_progress(&self, filename: &str, downloaded: u64, total: u64, status: DownloadStatus) {
        let _ = self.progress_tx.send(DownloadProgress {
            filename: filename.to_string(),
            downloaded,
            total,
            status,
            verify_progress: 0.0,
        }).await;
    }

    async fn send_progress_with_verify(&self, filename: &str, downloaded: u64, total: u64, status: DownloadStatus, verify_progress: f32) {
        let _ = self.progress_tx.send(DownloadProgress {
            filename: filename.to_string(),
            downloaded,
            total,
            status,
            verify_progress,
        }).await;
    }
}

async fn download_chunk(
    client: Arc<reqwest::Client>,
    url: &str,
    chunk_dir: &str,
    thread_id: usize,
    start_byte: u64,
    end_byte: u64,
    _filename: &str,
    total_downloaded: Arc<AtomicU64>,
    _total_size: u64,
) -> Result<(usize, u64)> {
    let chunk_path = format!("{}/chunk_{}", chunk_dir, thread_id);
    let expected_chunk_size = (end_byte - start_byte + 1) as u64;

    // Check if chunk already exists and is complete
    if Path::new(&chunk_path).exists() {
        let metadata = fs::metadata(&chunk_path)?;
        if metadata.len() == expected_chunk_size {
            // Chunk is complete, skip downloading
            return Ok((thread_id, metadata.len()));
        }
    }

    // Check for partial chunk and calculate resume position
    let existing_size = if Path::new(&chunk_path).exists() {
        fs::metadata(&chunk_path)?.len()
    } else {
        0
    };

    let resume_start = start_byte + existing_size;
    if resume_start > end_byte {
        // Already downloaded more than expected, restart from beginning
        let _ = fs::remove_file(&chunk_path);
    }

    let range_header = if existing_size > 0 && resume_start <= end_byte {
        // Resume from where we left off
        format!("bytes={}-{}", resume_start, end_byte)
    } else {
        // Download entire chunk
        format!("bytes={}-{}", start_byte, end_byte)
    };

    let response = client
        .get(url)
        .header("Range", &range_header)
        .header("Accept", "application/octet-stream,*/*")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() && status.as_u16() != 206 {
        return Err(anyhow!("HTTP error: {} for chunk {}", status, thread_id));
    }

    // Open file for appending if resuming, or create new if starting fresh
    let mut file = if existing_size > 0 && resume_start <= end_byte {
        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&chunk_path)?
    } else {
        File::create(&chunk_path)?
    };

    let mut stream = response.bytes_stream();
    let mut chunk_downloaded: u64 = existing_size;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        let chunk_len = chunk.len() as u64;
        chunk_downloaded += chunk_len;
        // Update total downloaded counter
        total_downloaded.fetch_add(chunk_len, Ordering::Relaxed);
    }

    file.flush()?;
    Ok((thread_id, chunk_downloaded))
}

pub async fn download_all_files(
    temp_dir: &str,
    files: &[crate::config::InstallFile],
    progress_tx: mpsc::Sender<DownloadProgress>,
    mut skip_verify_rx: mpsc::Receiver<bool>,
) -> Result<Vec<String>> {
    // Use semaphore to limit concurrent downloads to 5 files
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FILES));
    let mut tasks = JoinSet::new();

    // Shared skip verification flag
    let skip_verify = Arc::new(AtomicBool::new(false));

    // Spawn a task to listen for skip verification signal
    let skip_verify_listener = Arc::clone(&skip_verify);
    tokio::spawn(async move {
        if let Some(skip) = skip_verify_rx.recv().await {
            skip_verify_listener.store(skip, Ordering::Relaxed);
        }
    });

    for file in files {
        let permit = semaphore.clone().acquire_owned().await?;
        let temp_dir = temp_dir.to_string();
        let progress_tx = progress_tx.clone();
        let file = file.clone();
        let skip_verify_flag = Arc::clone(&skip_verify);

        tasks.spawn(async move {
            let _permit = permit; // Hold permit until task completes
            let downloader = Downloader::new_with_skip_flag(&temp_dir, progress_tx, skip_verify_flag);
            downloader.download_file(
                &file.filename,
                &file.url_list,
                file.size,
                &file.sha256,
            ).await
        });
    }

    let mut downloaded_files = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(path)) => downloaded_files.push(path),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow!("Task join error: {}", e)),
        }
    }

    Ok(downloaded_files)
}
