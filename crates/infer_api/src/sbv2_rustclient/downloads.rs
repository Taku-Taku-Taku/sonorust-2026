use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt as _};
use uuid::Uuid;
use zip::ZipArchive;

pub struct Sbv2RustDownloads {
    multi_progress: Arc<MultiProgress>,
}

impl Sbv2RustDownloads {
    pub fn new() -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
        }
    }

    async fn download_file<P>(
        &self,
        download_displayname: &str,
        download_url: &str,
        download_to_path: P,
    ) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let client = Client::new();

        // set download, temp path
        let download_to_path = download_to_path.as_ref();
        let tmp_file = download_to_path
            .parent()
            .ok_or_else(|| anyhow!("parent is None"))?
            .join(format!("tmp-{}", Uuid::new_v4()));
        let mut file = File::create(&tmp_file).await?;

        let mut responce = client.get(download_url).send().await?;
        let total_size = responce.content_length().unwrap_or(0);

        // progress bar
        let progress_bar = ProgressBar::new(total_size);
        progress_bar.set_style(
            ProgressStyle::default_bar()
            .template(&format!("{{spinner:.green}} Downloading: {download_displayname} [{{elapsed_precise}}] [{{wide_bar:.green/white}}] {{bytes}}/{{total_bytes}} ({{eta}})")).unwrap()
            .progress_chars("->-")
        );

        let progress_bar = self.multi_progress.add(progress_bar);

        // download
        let mut downloaded = 0;
        while let Some(chunk) = responce.chunk().await? {
            file.write_all(&chunk).await?;

            downloaded = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
            progress_bar.set_position(downloaded);
        }

        tokio::fs::rename(&tmp_file, &download_to_path).await?;

        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "✔ Download Complete: {download_displayname} [{{elapsed_precise}}]"
                ))
                .unwrap()
                .progress_chars("->-"),
        );
        progress_bar.finish();

        Ok(())
    }

    pub async fn download_debertaonnx<P>(&self, download_to_folder: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = download_to_folder.as_ref().join("deberta.onnx");
        if path.exists() {
            return Ok(());
        }

        self.download_file("deberta.onnx","https://huggingface.co/googlefan/sbv2_onnx_models/resolve/main/deberta.onnx?download=true", path).await?;
        Ok(())
    }

    pub async fn download_tokenizer<P>(&self, download_to_folder: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = download_to_folder.as_ref().join("tokenizer.json");
        if path.exists() {
            return Ok(());
        }

        self.download_file("tokenizer.json","https://huggingface.co/googlefan/sbv2_onnx_models/resolve/main/tokenizer.json?download=true", path).await?;
        Ok(())
    }

    async fn extract_zip<P>(&self, zip_path: P, output_dir: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let zip_pathbuf = zip_path.as_ref().to_owned();
        let output_dir = output_dir.as_ref().to_owned();

        let arc = self.multi_progress.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let zip_file = std::fs::File::open(zip_pathbuf)?;
            let mut archive = ZipArchive::new(zip_file)?;

            // spinner
            let spinner = arc.add(ProgressBar::new_spinner());

            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            spinner.set_message("Zip extracting...");
            spinner.enable_steady_tick(Duration::from_millis(50));

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = output_dir.join(file.name());

                if file.name().ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }

                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }

            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("✔ Zip extracted.")
                    .unwrap(),
            );
            spinner.finish();

            Ok(())
        })
        .await??;

        tokio::fs::remove_file(zip_path).await?;

        Ok(())
    }

    async fn extract_targz<P>(&self, targz_path: P, output_dir: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let targz_pathbuf = targz_path.as_ref().to_owned();
        let output_dir = output_dir.as_ref().to_owned();

        let arc = self.multi_progress.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let targz_file = std::fs::File::open(targz_pathbuf)?;
            let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(targz_file));

            // spinner
            let spinner = arc.add(ProgressBar::new_spinner());

            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            spinner.set_message("TarGz extracting...");
            spinner.enable_steady_tick(Duration::from_millis(50));

            std::fs::create_dir_all(&output_dir)?;
            archive.unpack(&output_dir)?;

            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("✔ TarGz extracted.")
                    .unwrap(),
            );
            spinner.finish();

            Ok(())
        })
        .await??;

        tokio::fs::remove_file(targz_path).await?;

        Ok(())
    }

    /// x86_64 の Windows と Linux に対応
    pub async fn download_and_set_onnxruntime<P>(
        &self,
        download_to_folder: P,
        is_gpu_version: bool,
    ) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let download_to_folder = download_to_folder.as_ref();

        let is_x86_win = cfg!(target_os = "windows") && cfg!(target_arch = "x86_64");
        let is_x86_linux = cfg!(target_os = "linux") && cfg!(target_arch = "x86_64");

        // (ダウンロード元, 展開後のフォルダ名, 動的ライブラリのフォルダ内での位置)
        let (download_url, extracted_dir_name, dylib_relative_path) =
            match (is_x86_win, is_x86_linux, is_gpu_version) {
                (true, _, false) => (
                    "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-1.20.1.zip",
                    "onnxruntime-win-x64-1.20.1",
                    "lib/onnxruntime.dll",
                ),
                (true, _, true) => (
                    "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-gpu-1.20.1.zip",
                    "onnxruntime-win-x64-gpu-1.20.1",
                    "lib/onnxruntime.dll",
                ),
                (_, true, false) => (
                    "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-linux-x64-1.20.1.tgz",
                    "onnxruntime-linux-x64-1.20.1",
                    "lib/libonnxruntime.so",
                ),
                (_, true, true) => (
                    "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-linux-x64-gpu-1.20.1.tgz",
                    "onnxruntime-linux-x64-gpu-1.20.1",
                    "lib/libonnxruntime.so",
                ),

                _ => bail!("Not Supported Os"),
            };

        let output_path = download_to_folder.join("ONNXRuntime");
        let ort_dylib_folder_path = output_path.join(extracted_dir_name);
        let archive_path = download_to_folder.join("download-onnxruntime");

        // 存在しない場合のみダウンロード
        if !ort_dylib_folder_path.exists() {
            self.download_file("ONNXRuntime", download_url, &archive_path)
                .await?;

            match is_x86_win {
                true => {
                    self.extract_zip(archive_path.as_path(), output_path.as_path())
                        .await?
                }
                false => {
                    self.extract_targz(archive_path.as_path(), output_path.as_path())
                        .await?
                }
            }
        }

        // 環境変数に設定
        let ort_dylib_str = std::env::current_dir()?
            .join(&ort_dylib_folder_path)
            .join(dylib_relative_path)
            .to_string_lossy()
            .replace("\\", "/");

        if !Path::new(&ort_dylib_str).exists() {
            bail!("ONNXRuntime library not found: {ort_dylib_str}");
        }

        std::env::set_var("ORT_DYLIB_PATH", ort_dylib_str);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::fs::create_dir_all;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_download() -> anyhow::Result<()> {
        let download_to_folder = "appdata/downloads";
        create_dir_all(download_to_folder).await?;

        let download_client = Sbv2RustDownloads::new();

        let (r1, r2) = tokio::join!(
            download_client.download_debertaonnx(download_to_folder),
            download_client.download_tokenizer(download_to_folder),
        );

        r1?;
        r2?;

        download_client
            .download_and_set_onnxruntime(download_to_folder, false)
            .await?;

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_onnxruntime_download() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;

        let download_client = Sbv2RustDownloads::new();
        download_client
            .download_and_set_onnxruntime("appdata", false)
            .await?;

        Ok(())
    }
}
