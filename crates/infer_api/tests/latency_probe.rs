//! 音声合成にかかる時間を計測する
//!
//! モデル一式を用意したうえで、次のように実行する
//!
//! ```sh
//! ORT_DYLIB_PATH=<libonnxruntime.so のパス> \
//!     cargo test -p infer_api --release --test latency_probe -- --ignored --nocapture
//! ```

use std::time::Instant;

use infer_api::Sbv2RustClient;

/// モデルを置いてあるフォルダ (既定はワークスペースのルート)
fn root() -> String {
    std::env::var("SONORUST_ROOT")
        .unwrap_or_else(|_| format!("{}/../..", env!("CARGO_MANIFEST_DIR")))
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn latency_probe() -> anyhow::Result<()> {
    let root = root();

    let mut client = Sbv2RustClient::new_from_model_folder(
        &format!("{root}/appdata/downloads/deberta.onnx"),
        &format!("{root}/appdata/downloads/tokenizer.json"),
        &format!("{root}/sbv2api_models"),
        Some(1),
    )
    .await?;

    let texts = [
        "こんにちは。",
        "今日はいい天気ですね。",
        "ボイスチャンネルに接続しました。よろしくお願いします。",
        "都会の暮らしに疲れ果て、田舎で憧れのスローライフを始めようとしていた男は、車に轢かれて異世界に転生した。",
        // 読み上げ上限の 100 文字
        "都会の暮らしに疲れ果て、田舎で憧れのスローライフを始めようとしていた男は、車に轢かれて異世界に転生した。目を覚ますとそこは見知らぬ森の中で、隣には喋る猫が座っていた。",
    ];

    // length: 1.0 は通常速度、0.7 は「長い文章の場合早めに読み上げる」が有効な時の速度
    // (0.7 は sonorust の FASTREAD_LENGTH と揃えている)
    for length in [1.0, 0.7] {
        println!("-- length {length} --");

        for text in texts.iter() {
            let now = Instant::now();
            let wav = client.infer(text, "", length, "").await?;
            let elapsed = now.elapsed();

            // 44.1kHz 32bit として音声の長さを求める
            let audio_secs = (wav.len() * 8) as f64 / (44100.0 * 32.0);

            println!(
                "{:>3}文字 | 合成 {:>6.2}秒 | 音声 {:>5.2}秒 | 実時間比 {:.2}x",
                text.chars().count(),
                elapsed.as_secs_f64(),
                audio_secs,
                elapsed.as_secs_f64() / audio_secs,
            );
        }
    }

    Ok(())
}
