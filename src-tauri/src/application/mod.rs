use crate::{
    audio,
    domain::{Session, Settings},
    inference,
    persistence::Repository,
    platform,
    state::{Phase, Snapshot},
};
use chrono::Utc;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

pub enum Control {
    Start,
    Stop,
    Cancel,
    Reset(String),
    Unload,
    Shutdown,
}

pub struct Runtime {
    pub repository: Mutex<Repository>,
    pub snapshot: Mutex<Snapshot>,
    pub model_dir: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub download_cancel: Arc<AtomicBool>,
    pub downloading: Mutex<Option<String>>,
    pub shortcut_down: AtomicBool,
    pub controls: mpsc::Sender<Control>,
}

impl Runtime {
    pub fn new(repository: Repository, model_dir: PathBuf, app: AppHandle) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let runtime = Arc::new(Self {
            repository: Mutex::new(repository),
            snapshot: Mutex::new(Snapshot::default()),
            model_dir,
            cancel: Arc::new(AtomicBool::new(false)),
            download_cancel: Arc::new(AtomicBool::new(false)),
            downloading: Mutex::new(None),
            shortcut_down: AtomicBool::new(false),
            controls: tx,
        });
        let worker = runtime.clone();
        std::thread::spawn(move || run_worker(worker, rx, app));
        runtime
    }

    pub fn send(&self, control: Control) -> Result<(), String> {
        if matches!(control, Control::Cancel | Control::Shutdown) {
            self.cancel.store(true, Ordering::SeqCst);
        }
        self.controls
            .send(control)
            .map_err(|_| "Il servizio di dettatura non è disponibile.".into())
    }

    pub fn settings(&self) -> Result<Settings, String> {
        self.repository
            .lock()
            .map_err(|_| "Database non disponibile.")?
            .settings()
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn update(&self, app: &AppHandle, phase: Phase, message: Option<String>) {
        let snapshot = {
            let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            // State transitions are authoritative and duplicate events do not reinsert text.
            if s.state.transition(phase).is_err() {
                return;
            }
            s.state = phase;
            s.message = message;
            if phase != Phase::Recording {
                s.level = 0.0;
                s.speech = false;
            }
            s.clone()
        };
        let _ = app.emit("dictation-state", &snapshot);
        let delay = phase_delay(phase, snapshot.recovery_pending);
        if let Some(window) = app.get_webview_window("overlay") {
            if phase == Phase::Idle {
                let _ = window.hide();
            } else {
                let _ = window.show();
            }
        }
        if matches!(phase, Phase::Completed | Phase::Cancelled | Phase::Error) {
            if let Some(id) = snapshot.session_id {
                let tx = self.controls.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(delay));
                    let _ = tx.send(Control::Reset(id));
                });
            }
        }
    }

    fn persist(&self, app: &AppHandle, session: &Session) -> bool {
        let result = self
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.".to_string())
            .and_then(|db| {
                db.settings()
                    .and_then(|settings| db.save_session(session, settings.history_enabled))
            });
        if result.is_err() {
            let mut snapshot = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
            snapshot.message =
                Some("Le statistiche non sono state salvate. Il testo resta recuperabile.".into());
            let _ = app.emit("dictation-state", &*snapshot);
        }
        let _ = app.emit("sessions-changed", ());
        result.is_ok()
    }
}

fn phase_delay(phase: Phase, recovery_pending: bool) -> u64 {
    if phase == Phase::Error || recovery_pending {
        5000
    } else {
        1800
    }
}

struct Recording {
    capture: audio::CaptureHandle,
    session: Session,
    settings: Settings,
    target: Result<platform::FocusTarget, String>,
    started: Instant,
}

fn run_worker(runtime: Arc<Runtime>, rx: mpsc::Receiver<Control>, app: AppHandle) {
    let mut recording: Option<Recording> = None;
    let mut engine: Option<(PathBuf, inference::WhisperEngine)> = None;
    loop {
        let command = if recording.is_some() {
            match rx.recv_timeout(Duration::from_millis(40)) {
                Ok(c) => Some(c),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(_) => break,
            }
        } else {
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => break,
            }
        };
        match command {
            Some(Control::Start) => {
                if recording.is_some() || runtime.snapshot().state.active() {
                    continue;
                }
                if runtime.snapshot().recovery_pending {
                    show_start_error(&runtime, &app, "Copia prima il risultato precedente da Dicta: resterà in memoria finché non lo recuperi.".into());
                    continue;
                }
                let settings = match runtime.settings() {
                    Ok(s) => s,
                    Err(e) => {
                        show_start_error(&runtime, &app, e);
                        continue;
                    }
                };
                // Capture focus before showing any app window or starting lengthy model work.
                let target = platform::capture_target();
                let id = Uuid::new_v4().to_string();
                {
                    let mut s = runtime.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                    s.session_id = Some(id.clone());
                    s.elapsed_ms = 0;
                    s.message = None;
                    // Keep the last unsaved result until the next successful transcription replaces it.
                }
                runtime.cancel.store(false, Ordering::SeqCst);
                position_overlay(&app, &settings);
                runtime.update(
                    &app,
                    Phase::Preparing,
                    Some("Preparazione del microfono…".into()),
                );
                let mut session = Session {
                    id,
                    started_at_utc: Utc::now().to_rfc3339(),
                    ended_at_utc: None,
                    status: "started".into(),
                    recorded_ms: 0,
                    inference_ms: 0,
                    insertion_ms: 0,
                    words_count: 0,
                    chars_count: 0,
                    language_code: settings.language.clone(),
                    model_id: settings.model_id.clone(),
                    error_category: None,
                    input_mode: settings.shortcut_mode.clone(),
                    text: None,
                };
                if !runtime.persist(&app, &session) {
                    runtime.update(&app, Phase::Error, Some("Impossibile salvare la sessione. Controlla spazio libero e permessi del disco prima di registrare.".into()));
                    continue;
                }
                let result = (|| {
                    let model = inference::list_models(&runtime.model_dir)
                        .into_iter()
                        .find(|m| m.id == settings.model_id && m.installed)
                        .ok_or("Installa un modello nella sezione Modelli prima di dettare.")?;
                    let _ = model;
                    if platform::permissions().microphone != "granted" {
                        return Err("Concedi il permesso Microfono prima di dettare.".to_string());
                    }
                    if settings.sounds {
                        platform::play_sound();
                    }
                    audio::start_capture(
                        settings.input_device.as_deref(),
                        Duration::from_secs(300),
                        settings.vad_threshold as f32,
                    )
                })();
                match result {
                    Ok(capture) => {
                        runtime.update(
                            &app,
                            Phase::Recording,
                            Some("Ti ascolto. Il microfono si ferma al termine.".into()),
                        );
                        recording = Some(Recording {
                            capture,
                            session,
                            settings,
                            target,
                            started: Instant::now(),
                        });
                    }
                    Err(e) => {
                        finish_error(&runtime, &app, &mut session, "preparation", e);
                    }
                }
            }
            Some(Control::Stop) => {
                if let Some(active) = recording.take() {
                    finish_recording(&runtime, &app, active, &mut engine);
                }
            }
            Some(Control::Cancel) => {
                if let Some(active) = recording.take() {
                    let mut session = active.session;
                    session.recorded_ms = active.started.elapsed().as_millis() as u64;
                    active.capture.cancel();
                    session.status = "cancelled".into();
                    session.ended_at_utc = Some(Utc::now().to_rfc3339());
                    runtime.update(&app, Phase::Cancelled, Some("Dettatura annullata.".into()));
                    runtime.persist(&app, &session);
                }
            }
            Some(Control::Reset(id)) => {
                let s = runtime.snapshot();
                if s.session_id.as_deref() == Some(&id) && !s.state.active() {
                    runtime.update(&app, Phase::Idle, None);
                }
            }
            Some(Control::Unload) => {
                engine = None;
            }
            Some(Control::Shutdown) => {
                if let Some(active) = recording.take() {
                    active.capture.cancel();
                }
                break;
            }
            None => {}
        }
        let mut failure = None;
        let mut duration_limit = false;
        if let Some(active) = recording.as_ref() {
            while let Some(event) = active.capture.try_event() {
                match event {
                    audio::AudioEvent::Level {
                        level,
                        speech,
                        elapsed_ms,
                    } => {
                        let mut s = runtime.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                        s.level = level;
                        s.speech = speech;
                        s.elapsed_ms = elapsed_ms;
                        #[derive(Clone, Serialize)]
                        struct Level {
                            level: f32,
                            speech: bool,
                            elapsed_ms: u64,
                        }
                        let _ = app.emit(
                            "audio-level",
                            Level {
                                level,
                                speech,
                                elapsed_ms,
                            },
                        );
                    }
                    audio::AudioEvent::Error(e) => {
                        failure = Some(e);
                    }
                }
            }
            if active.started.elapsed() >= Duration::from_secs(300) {
                duration_limit = true;
            }
        }
        if let Some(error) = failure {
            if let Some(active) = recording.take() {
                let mut session = active.session;
                session.recorded_ms = active.started.elapsed().as_millis() as u64;
                active.capture.cancel();
                finish_error(&runtime, &app, &mut session, "audio", error);
            }
        } else if duration_limit {
            if let Some(active) = recording.take() {
                finish_recording(&runtime, &app, active, &mut engine);
            }
        }
    }
}

fn show_start_error(runtime: &Runtime, app: &AppHandle, message: String) {
    runtime
        .snapshot
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .session_id = Some(Uuid::new_v4().to_string());
    runtime.update(app, Phase::Preparing, None);
    runtime.update(app, Phase::Error, Some(message));
}

fn finish_error(
    runtime: &Runtime,
    app: &AppHandle,
    session: &mut Session,
    category: &str,
    message: String,
) {
    session.status = "failed".into();
    session.error_category = Some(category.into());
    session.ended_at_utc = Some(Utc::now().to_rfc3339());
    runtime.update(app, Phase::Error, Some(message));
    runtime.persist(app, session);
}

fn finish_recording(
    runtime: &Runtime,
    app: &AppHandle,
    active: Recording,
    engine: &mut Option<(PathBuf, inference::WhisperEngine)>,
) {
    let mut session = active.session;
    runtime.update(
        app,
        Phase::Processing,
        Some("Trascrizione sul tuo Mac…".into()),
    );
    let result = (|| {
        let audio = active.capture.stop()?;
        if active.settings.sounds {
            platform::play_sound();
        }
        session.recorded_ms = audio.recorded_ms;
        if !audio.speech_detected || audio.samples_16khz.is_empty() {
            return Err("Non ho rilevato voce. Prova ad avvicinarti al microfono.".to_string());
        }
        let path = runtime
            .model_dir
            .join(format!("ggml-{}.bin", active.settings.model_id));
        if engine.as_ref().is_none_or(|(loaded, _)| loaded != &path) {
            let path =
                inference::verified_model_path(&active.settings.model_id, &runtime.model_dir)?;
            *engine = Some((path.clone(), inference::WhisperEngine::load(&path)?));
        }
        let start = Instant::now();
        let text = engine
            .as_mut()
            .ok_or("Motore non disponibile.")?
            .1
            .transcribe(
                &audio.samples_16khz,
                &active.settings.language,
                runtime.cancel.clone(),
                Duration::from_secs(180),
            );
        session.inference_ms = start.elapsed().as_millis() as u64;
        text
    })();
    if runtime.cancel.load(Ordering::SeqCst) {
        session.status = "cancelled".into();
        session.ended_at_utc = Some(Utc::now().to_rfc3339());
        runtime.update(app, Phase::Cancelled, Some("Dettatura annullata.".into()));
        runtime.persist(app, &session);
        return;
    }
    match result {
        Ok(text) if !text.trim().is_empty() => {
            let text = text.trim().to_string();
            session.words_count = text
                .split_whitespace()
                .filter(|w| w.chars().any(char::is_alphanumeric))
                .count() as u64;
            session.chars_count = text.chars().count() as u64;
            runtime
                .snapshot
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .recovery_text = Some(text.clone());
            runtime.update(
                app,
                Phase::Inserting,
                Some("Inserimento nel campo originale…".into()),
            );
            if runtime.cancel.load(Ordering::SeqCst) {
                session.status = "cancelled".into();
                session.ended_at_utc = Some(Utc::now().to_rfc3339());
                runtime.update(
                    app,
                    Phase::Cancelled,
                    Some("Dettatura annullata prima dell’inserimento.".into()),
                );
                runtime.persist(app, &session);
                return;
            }
            let start = Instant::now();
            let inserted = active
                .target
                .and_then(|target| platform::insert_text(&target, &text));
            runtime
                .snapshot
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .recovery_pending = inserted.is_err();
            session.insertion_ms = start.elapsed().as_millis() as u64;
            session.status = "completed".into();
            session.ended_at_utc = Some(Utc::now().to_rfc3339());
            session.text = Some(text);
            let message = match inserted {
                Ok(()) => "Testo inserito.".into(),
                Err(error) => {
                    session.error_category = Some("insertion_recovery".into());
                    format!("Testo non inserito: {error}. Copialo da Dicta.")
                }
            };
            runtime.update(app, Phase::Completed, Some(message));
            runtime.persist(app, &session);
        }
        Ok(_) => finish_error(
            runtime,
            app,
            &mut session,
            "empty_transcript",
            "Nessun testo rilevato. Il modello non ha prodotto parole.".into(),
        ),
        Err(e) => finish_error(runtime, app, &mut session, "inference", e),
    }
}

fn position_overlay(app: &AppHandle, settings: &Settings) {
    if let Some(window) = app.get_webview_window("overlay") {
        // AppKit display bounds use top-left coordinates supplied by the native adapter.
        if let Ok(bounds) = platform::active_display_bounds() {
            let x = bounds.x + (bounds.width - 400.0) / 2.0;
            let y = bounds.y
                + if settings.overlay_position == "top" {
                    20.0
                } else {
                    bounds.height - 100.0 - 20.0
                };
            let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::phase_delay;
    use crate::state::Phase;

    #[test]
    fn keeps_recovery_visible_longer_than_success() {
        assert_eq!(phase_delay(Phase::Completed, false), 1800);
        assert_eq!(phase_delay(Phase::Completed, true), 5000);
        assert_eq!(phase_delay(Phase::Error, false), 5000);
    }
}
