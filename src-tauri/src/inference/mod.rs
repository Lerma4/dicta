use std::{
    collections::HashSet,
    ffi::{c_char, c_uint, c_void},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Once, OnceLock,
    },
    time::{Duration, Instant},
};

use reqwest::blocking::Client;
use serde::Serialize;
use sha2::{Digest, Sha256};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";
const MODELS: &[CatalogModel] = &[
    CatalogModel {
        id: "tiny",
        name: "Whisper Tiny",
        bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        description: "Smallest multilingual model",
        file: "ggml-tiny.bin",
    },
    CatalogModel {
        id: "base",
        name: "Whisper Base",
        bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        description: "Balanced multilingual model",
        file: "ggml-base.bin",
    },
    CatalogModel {
        id: "small",
        name: "Whisper Small",
        bytes: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        description: "Higher quality multilingual model",
        file: "ggml-small.bin",
    },
];

struct CatalogModel {
    id: &'static str,
    name: &'static str,
    bytes: u64,
    sha256: &'static str,
    description: &'static str,
    file: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub installed: bool,
    pub downloading: bool,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: u64,
    pub state: String,
    pub error: Option<String>,
}

fn active_downloads() -> &'static Mutex<HashSet<String>> {
    static ACTIVE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

fn catalog(id: &str) -> Result<&'static CatalogModel, String> {
    MODELS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("unknown model: {id}"))
}

pub fn list_models(dir: &Path) -> Vec<ModelInfo> {
    let active = active_downloads()
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    MODELS
        .iter()
        .map(|m| {
            let path = dir.join(m.file);
            ModelInfo {
                id: m.id.into(),
                name: m.name.into(),
                size_bytes: m.bytes,
                checksum: m.sha256.into(),
                installed: fs::symlink_metadata(&path)
                    .is_ok_and(|meta| meta.file_type().is_file() && meta.len() == m.bytes),
                downloading: active.contains(m.id),
                description: m.description.into(),
            }
        })
        .collect()
}

/// Rechecks the pinned SHA-256 on the inference worker before loading model bytes.
pub fn verified_model_path(id: &str, dir: &Path) -> Result<PathBuf, String> {
    let model = catalog(id)?;
    let root = dir.canonicalize().map_err(|e| e.to_string())?;
    let path = root.join(model.file);
    if verify_file(&path, model)? {
        Ok(path)
    } else {
        Err(format!(
            "model {id} is missing or failed SHA-256 verification"
        ))
    }
}

pub fn download_model(
    id: &str,
    dir: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(ModelProgress),
) -> Result<PathBuf, String> {
    let model = catalog(id)?;
    {
        let mut active = active_downloads()
            .lock()
            .map_err(|_| "download state poisoned")?;
        if !active.insert(id.to_owned()) {
            return Err("model download already in progress".into());
        }
    }
    struct DownloadGuard(String);
    impl Drop for DownloadGuard {
        fn drop(&mut self) {
            if let Ok(mut active) = active_downloads().lock() {
                active.remove(&self.0);
            }
        }
    }
    let _guard = DownloadGuard(id.to_owned());

    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let root = dir.canonicalize().map_err(|e| e.to_string())?;
    let destination = root.join(model.file);
    if verify_file(&destination, model)? {
        return Ok(destination);
    }
    if destination.exists() {
        fs::remove_file(&destination).map_err(|e| e.to_string())?;
    }
    let temp = root.join(format!(".{}.download", model.id));
    if let Ok(meta) = fs::symlink_metadata(&temp) {
        if !meta.file_type().is_file() {
            return Err("unsafe stale model download path".into());
        }
        fs::remove_file(&temp).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("cannot create model temp file: {e}"))?;
    let mut downloaded = 0u64;
    let mut last_progress = Instant::now() - Duration::from_secs(1);
    let result = (|| {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/{REVISION}/{}",
            model.file
        );
        let mut response = client
            .get(url)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("download cancelled".into());
            }
            let count = response.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            downloaded = downloaded
                .checked_add(count as u64)
                .ok_or("download size overflow")?;
            if downloaded > model.bytes {
                return Err("model download exceeded expected size".into());
            }
            file.write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
            hasher.update(&buffer[..count]);
            if last_progress.elapsed() >= Duration::from_millis(150) {
                progress(ModelProgress {
                    id: id.into(),
                    downloaded,
                    total: model.bytes,
                    state: "downloading".into(),
                    error: None,
                });
                last_progress = Instant::now();
            }
        }
        if downloaded != model.bytes {
            return Err(format!(
                "incomplete model download: {downloaded}/{} bytes",
                model.bytes
            ));
        }
        let checksum = format!("{:x}", hasher.finalize());
        if checksum != model.sha256 {
            return Err("model SHA-256 verification failed".into());
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("download cancelled".into());
        }
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temp, &destination).map_err(|e| e.to_string())?;
        Ok(destination.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    match &result {
        Ok(_) => progress(ModelProgress {
            id: id.into(),
            downloaded: model.bytes,
            total: model.bytes,
            state: "completed".into(),
            error: None,
        }),
        Err(error) => progress(ModelProgress {
            id: id.into(),
            downloaded,
            total: model.bytes,
            state: if error == "download cancelled" {
                "cancelled"
            } else {
                "error"
            }
            .into(),
            error: Some(error.clone()),
        }),
    }
    result
}

fn verify_file(path: &Path, model: &CatalogModel) -> Result<bool, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    if !meta.file_type().is_file() || meta.len() != model.bytes {
        return Ok(false);
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 128 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()) == model.sha256)
}

pub struct WhisperEngine {
    context: WhisperContext,
}

static WHISPER_LOG_SINK: Once = Once::new();

unsafe extern "C" fn discard_whisper_log(_: c_uint, _: *const c_char, _: *mut c_void) {}

struct AbortContext {
    cancel: Arc<AtomicBool>,
    deadline: Instant,
}

unsafe extern "C" fn abort_inference(user_data: *mut c_void) -> bool {
    let Some(context) = (unsafe { user_data.cast::<AbortContext>().as_ref() }) else {
        return true;
    };
    context.cancel.load(Ordering::Relaxed) || Instant::now() >= context.deadline
}

impl WhisperEngine {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Err("model file does not exist".into());
        }
        WHISPER_LOG_SINK.call_once(|| unsafe {
            // Whisper's debug build prints decoded tokens, so route its global C logger to a
            // non-panicking sink before any model operation can expose transcript text.
            whisper_rs::set_log_callback(Some(discard_whisper_log), std::ptr::null_mut());
        });
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .or_else(|_| {
                let mut cpu = WhisperContextParameters::default();
                cpu.use_gpu(false);
                WhisperContext::new_with_params(path, cpu)
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { context })
    }

    pub fn transcribe(
        &self,
        samples_16khz: &[f32],
        language: &str,
        cancel: Arc<AtomicBool>,
        timeout: Duration,
    ) -> Result<String, String> {
        if samples_16khz.is_empty() {
            return Err("cannot transcribe empty audio".into());
        }
        if timeout.is_zero() {
            return Err("inference timeout must be positive".into());
        }
        let deadline = Instant::now() + timeout;
        let mut state = self.context.create_state().map_err(|e| e.to_string())?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(
            std::thread::available_parallelism().map_or(2, |v| v.get().min(8)) as i32,
        );
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_language((language != "auto").then_some(language));
        let mut abort_context = AbortContext {
            cancel: Arc::clone(&cancel),
            deadline,
        };
        // SAFETY: whisper invokes this callback synchronously only inside `full`; the stack
        // context lives through that call, and the callback reads only immutable/atomic fields.
        unsafe {
            params.set_abort_callback(Some(abort_inference));
            params.set_abort_callback_user_data((&mut abort_context as *mut AbortContext).cast());
        }
        state.full(params, samples_16khz).map_err(|e| {
            if cancel.load(Ordering::Relaxed) {
                "inference cancelled".into()
            } else if Instant::now() >= deadline {
                "inference timed out".into()
            } else {
                e.to_string()
            }
        })?;
        let mut text = String::new();
        for index in 0..state.full_n_segments() {
            if let Some(segment) = state.get_segment(index) {
                text.push_str(segment.to_str().map_err(|e| e.to_string())?);
            }
        }
        Ok(text.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_pinned_and_rejects_arbitrary_ids() {
        assert!(catalog("tiny").is_ok());
        assert!(catalog("../../secret").is_err());
        assert!(MODELS
            .iter()
            .all(|m| m.sha256.len() == 64 && m.bytes > 1_000_000));
    }

    #[test]
    fn abort_callback_checks_cancel_timeout_and_null_user_data() {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut context = AbortContext {
            cancel: Arc::clone(&cancel),
            deadline: Instant::now() + Duration::from_secs(30),
        };
        let ptr = (&mut context as *mut AbortContext).cast();
        assert!(!unsafe { abort_inference(ptr) });
        cancel.store(true, Ordering::Relaxed);
        assert!(unsafe { abort_inference(ptr) });
        assert!(unsafe { abort_inference(std::ptr::null_mut()) });
        context.cancel.store(false, Ordering::Relaxed);
        context.deadline = Instant::now() - Duration::from_secs(1);
        assert!(unsafe { abort_inference(ptr) });
    }
}
