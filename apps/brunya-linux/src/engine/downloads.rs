use brunya_storage::{DownloadState, StorageManager};
use parking_lot::RwLock;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info};

pub struct ActiveDownload {
    pub id: i64,
    pub url: String,
    pub filename: String,
    pub target_path: PathBuf,
    pub cancel_flag: Arc<AtomicBool>,
    pub pause_flag: Arc<AtomicBool>,
}

pub struct DownloadManager {
    storage: Arc<StorageManager>,
    active_downloads: RwLock<Vec<ActiveDownload>>,
    default_download_dir: PathBuf,
}

impl DownloadManager {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        let download_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join("Downloads")
        } else {
            PathBuf::from("/tmp")
        };
        std::fs::create_dir_all(&download_dir).ok();

        Self {
            storage,
            active_downloads: RwLock::new(Vec::new()),
            default_download_dir: download_dir,
        }
    }

    pub fn download_directory(&self) -> &Path {
        &self.default_download_dir
    }

    pub fn start_download<F: Fn(i64, i64, i64, DownloadState) + Send + Sync + 'static>(
        &self,
        url: &str,
        suggested_name: Option<&str>,
        on_progress: F,
    ) -> Result<i64, String> {
        let filename = suggested_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::extract_filename_from_url(url));

        let target_path = self.default_download_dir.join(&filename);
        let target_str = target_path.to_string_lossy().to_string();

        let download_id = self
            .storage
            .add_download(url, &filename, &target_str, -1)
            .map_err(|e| e.to_string())?;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let pause_flag = Arc::new(AtomicBool::new(false));

        let active = ActiveDownload {
            id: download_id,
            url: url.to_string(),
            filename: filename.clone(),
            target_path: target_path.clone(),
            cancel_flag: cancel_flag.clone(),
            pause_flag: pause_flag.clone(),
        };

        self.active_downloads.write().push(active);

        let storage_clone = self.storage.clone();
        let url_owned = url.to_string();
        let target_owned = target_path.clone();
        let progress_cb = Arc::new(on_progress);

        // Disparar thread de streaming de download
        std::thread::spawn(move || {
            info!(
                "Iniciando download #{}: {} -> {}",
                download_id,
                url_owned,
                target_owned.display()
            );

            let config_res = ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(60)))
                .build();
            let agent = ureq::Agent::new_with_config(config_res);

            match agent.get(&url_owned).call() {
                Ok(mut response) => {
                    let total_bytes = response
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(-1);

                    let mut file = match File::create(&target_owned) {
                        Ok(f) => f,
                        Err(e) => {
                            error!("Falha ao criar arquivo de download: {}", e);
                            storage_clone
                                .update_download_progress(download_id, 0, DownloadState::Failed)
                                .ok();
                            progress_cb(download_id, 0, total_bytes, DownloadState::Failed);
                            return;
                        }
                    };

                    let mut reader = response.body_mut().as_reader();
                    let mut buffer = [0u8; 16384]; // Chunks de 16 KB
                    let mut downloaded: i64 = 0;

                    loop {
                        if cancel_flag.load(Ordering::SeqCst) {
                            info!("Download #{} cancelado pelo usuário", download_id);
                            storage_clone
                                .update_download_progress(
                                    download_id,
                                    downloaded,
                                    DownloadState::Cancelled,
                                )
                                .ok();
                            progress_cb(
                                download_id,
                                downloaded,
                                total_bytes,
                                DownloadState::Cancelled,
                            );
                            std::fs::remove_file(&target_owned).ok();
                            return;
                        }

                        match reader.read(&mut buffer) {
                            Ok(0) => break, // Fim da transmissão
                            Ok(n) => {
                                if let Err(e) = file.write_all(&buffer[..n]) {
                                    error!(
                                        "Erro de escrita em disco no download #{}: {}",
                                        download_id, e
                                    );
                                    storage_clone
                                        .update_download_progress(
                                            download_id,
                                            downloaded,
                                            DownloadState::Failed,
                                        )
                                        .ok();
                                    progress_cb(
                                        download_id,
                                        downloaded,
                                        total_bytes,
                                        DownloadState::Failed,
                                    );
                                    return;
                                }
                                downloaded += n as i64;
                                storage_clone
                                    .update_download_progress(
                                        download_id,
                                        downloaded,
                                        DownloadState::InProgress,
                                    )
                                    .ok();
                                progress_cb(
                                    download_id,
                                    downloaded,
                                    total_bytes,
                                    DownloadState::InProgress,
                                );
                            }
                            Err(e) => {
                                error!("Erro de rede no download #{}: {}", download_id, e);
                                storage_clone
                                    .update_download_progress(
                                        download_id,
                                        downloaded,
                                        DownloadState::Failed,
                                    )
                                    .ok();
                                progress_cb(
                                    download_id,
                                    downloaded,
                                    total_bytes,
                                    DownloadState::Failed,
                                );
                                return;
                            }
                        }
                    }

                    info!(
                        "Download #{} concluído com sucesso ({} bytes)",
                        download_id, downloaded
                    );
                    storage_clone
                        .update_download_progress(download_id, downloaded, DownloadState::Completed)
                        .ok();
                    progress_cb(
                        download_id,
                        downloaded,
                        total_bytes,
                        DownloadState::Completed,
                    );
                }
                Err(e) => {
                    error!("Erro HTTP ao iniciar download #{}: {}", download_id, e);
                    storage_clone
                        .update_download_progress(download_id, 0, DownloadState::Failed)
                        .ok();
                    progress_cb(download_id, 0, -1, DownloadState::Failed);
                }
            }
        });

        Ok(download_id)
    }

    pub fn cancel_download(&self, id: i64) -> bool {
        let downloads = self.active_downloads.read();
        if let Some(dl) = downloads.iter().find(|d| d.id == id) {
            dl.cancel_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    fn extract_filename_from_url(url_str: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url_str) {
            if let Some(mut segments) = parsed.path_segments() {
                if let Some(last) = segments.next_back() {
                    if !last.trim().is_empty() {
                        return last.to_string();
                    }
                }
            }
        }
        format!("download_{}.bin", chrono::Utc::now().timestamp())
    }
}
