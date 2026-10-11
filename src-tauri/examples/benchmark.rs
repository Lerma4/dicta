use dicta_lib::inference::{download_model, verified_model_path, WhisperEngine};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "Usage: benchmark <model-directory> <16kHz-mono-PCM16.wav> [--download] [language]"
                .into(),
        );
    }
    let dir = PathBuf::from(&args[1]);
    let cancel = Arc::new(AtomicBool::new(false));
    if args.iter().any(|s| s == "--download") {
        eprintln!("Explicit benchmark download: Whisper Tiny (77.7 MB), verified SHA-256.");
        download_model("tiny", &dir, &cancel, |_| {})?;
    }
    let mut reader = hound::WavReader::open(&args[2]).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.sample_rate != 16000
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err("Benchmark expects PCM16 mono 16kHz WAV.".into());
    }
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / 32768.0).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    if samples.is_empty() || samples.len() > 16_000 * 300 {
        return Err("Audio must contain 0–300 seconds.".into());
    }
    let language = args
        .iter()
        .skip(3)
        .find(|arg| arg.as_str() != "--download")
        .map(String::as_str)
        .unwrap_or("en");
    let start = Instant::now();
    let path = verified_model_path("tiny", &dir)?;
    let verification_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let engine = WhisperEngine::load(&path)?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut timings = Vec::new();
    let mut words = 0;
    for _ in 0..5 {
        let start = Instant::now();
        let text =
            engine.transcribe(&samples, language, cancel.clone(), Duration::from_secs(180))?;
        timings.push(start.elapsed().as_secs_f64() * 1000.0);
        words = text
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count();
    }
    let mut warm = timings[1..].to_vec();
    warm.sort_by(f64::total_cmp);
    let audio_ms = samples.len() as f64 / 16.0;
    println!(
        "{}",
        serde_json::json!({
            "model":"tiny", "platform":std::env::consts::OS, "arch":std::env::consts::ARCH,
            "language":language,"samples":samples.len(),"audio_ms":audio_ms,"words":words,
            "verification_ms":verification_ms,"model_load_ms":load_ms,"first_inference_ms":timings[0],
            "inference_ms":timings,"warm_p50_ms":(warm[1]+warm[2])/2.0,"warm_p95_ms":warm[3],
            "warm_max_ms":warm[3],"warm_rtf":((warm[1]+warm[2])/2.0)/audio_ms,
            "note":"5 trials, 4 warm; GPU attempted, CPU fallback; no UI/audio capture/insertion in this benchmark"
        })
    );
    Ok(())
}
