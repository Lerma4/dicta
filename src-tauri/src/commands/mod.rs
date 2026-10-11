use crate::{
    application::{Control, Runtime},
    audio,
    domain::{Dashboard, DashboardFilter, Session, Settings},
    inference, platform,
    state::Snapshot,
};
use std::sync::{atomic::Ordering, Arc};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        Err("Operazione disponibile soltanto nella finestra principale.".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, Arc<Runtime>>) -> Snapshot {
    state.snapshot()
}
#[tauri::command]
pub fn start_dictation(state: State<'_, Arc<Runtime>>) -> Result<(), String> {
    state.send(Control::Start)
}
#[tauri::command]
pub fn stop_dictation(state: State<'_, Arc<Runtime>>) -> Result<(), String> {
    state.send(Control::Stop)
}
#[tauri::command]
pub fn cancel_dictation(state: State<'_, Arc<Runtime>>) -> Result<(), String> {
    state.send(Control::Cancel)
}
#[tauri::command]
pub fn copy_recovery(state: State<'_, Arc<Runtime>>) -> Result<(), String> {
    platform::copy_recovery_text(
        state
            .snapshot()
            .recovery_text
            .as_deref()
            .ok_or("Nessun testo da copiare.")?,
    )?;
    state
        .snapshot
        .lock()
        .map_err(|_| "Stato non disponibile.")?
        .recovery_pending = false;
    Ok(())
}
#[tauri::command]
pub fn get_settings(state: State<'_, Arc<Runtime>>) -> Result<Settings, String> {
    state.settings()
}

#[tauri::command]
pub fn copy_text(window: WebviewWindow, text: String) -> Result<(), String> {
    main_only(&window)?;
    if text.len() > 1_000_000 {
        return Err("Testo troppo grande per gli appunti.".into());
    }
    platform::copy_recovery_text(&text)
}
#[tauri::command]
pub fn get_history_enabled(state: State<'_, Arc<Runtime>>) -> Result<bool, String> {
    Ok(state.settings()?.history_enabled)
}

#[tauri::command]
pub fn save_settings(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<Runtime>>,
    settings: Settings,
) -> Result<(), String> {
    main_only(&window)?;
    if state.snapshot().state.active() {
        return Err("Termina la dettatura prima di modificare le impostazioni.".into());
    }
    settings.validate()?;
    if !inference::list_models(&state.model_dir)
        .iter()
        .any(|m| m.id == settings.model_id)
    {
        return Err("Modello sconosciuto.".into());
    }
    let old = state.settings()?;
    let shortcut_changed = old.shortcut != settings.shortcut;
    if shortcut_changed {
        app.global_shortcut()
            .register(settings.shortcut.as_str())
            .map_err(|_| "Scorciatoia non valida o già occupata.")?;
    }
    let autostart = app.autolaunch();
    let result = (|| {
        if old.autostart != settings.autostart {
            if settings.autostart {
                autostart.enable()
            } else {
                autostart.disable()
            }
            .map_err(|e| e.to_string())?;
        }
        state
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.")?
            .save_settings(&settings)
    })();
    if let Err(e) = result {
        // A privacy purge can fail after its settings transaction committed.
        // Keep native preferences aligned with the committed values in that case.
        if state.settings().is_ok_and(|saved| saved == settings) {
            if shortcut_changed {
                let _ = app.global_shortcut().unregister(old.shortcut.as_str());
            }
            let _ = app.emit("settings-changed", &settings);
            return Err(format!(
                "Preferenze salvate, ma la cancellazione dei dati non è stata confermata: {e}"
            ));
        }
        if shortcut_changed {
            let _ = app.global_shortcut().unregister(settings.shortcut.as_str());
        }
        if old.autostart != settings.autostart {
            let _ = if old.autostart {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        return Err(e);
    }
    if shortcut_changed {
        let _ = app.global_shortcut().unregister(old.shortcut.as_str());
    }
    let _ = app.emit("settings-changed", &settings);
    if settings.model_id != old.model_id {
        state.send(Control::Unload)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_permissions() -> platform::Permissions {
    platform::permissions()
}
#[tauri::command]
pub fn request_permission(window: WebviewWindow, kind: String) -> Result<(), String> {
    main_only(&window)?;
    platform::request_permission(&kind)
}
#[tauri::command]
pub fn list_input_devices() -> Result<Vec<audio::InputDevice>, String> {
    audio::list_input_devices()
}
#[tauri::command]
pub fn list_models(state: State<'_, Arc<Runtime>>) -> Vec<inference::ModelInfo> {
    let mut models = inference::list_models(&state.model_dir);
    let active = state.downloading.lock().unwrap_or_else(|e| e.into_inner());
    for model in &mut models {
        model.downloading = active.as_deref() == Some(&model.id);
    }
    models
}
#[tauri::command]
pub fn download_model(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<Runtime>>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    if !inference::list_models(&state.model_dir)
        .iter()
        .any(|m| m.id == id)
    {
        return Err("Modello sconosciuto.".into());
    }
    let mut downloading = state
        .downloading
        .lock()
        .map_err(|_| "Gestore download non disponibile.")?;
    if downloading.is_some() {
        return Err("Un download è già in corso.".into());
    }
    *downloading = Some(id.clone());
    state.download_cancel.store(false, Ordering::SeqCst);
    let runtime = state.inner().clone();
    std::thread::spawn(move || {
        let result = inference::download_model(
            &id,
            &runtime.model_dir,
            &runtime.download_cancel,
            |progress| {
                let _ = app.emit("model-progress", progress);
            },
        );
        *runtime
            .downloading
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        #[derive(Clone, serde::Serialize)]
        struct FinalProgress {
            id: String,
            downloaded: u64,
            total: u64,
            state: String,
            error: Option<String>,
        }
        let size = result
            .as_ref()
            .ok()
            .and_then(|p| p.metadata().ok())
            .map(|m| m.len())
            .unwrap_or(0);
        let status = if result.is_ok() {
            "completed"
        } else if runtime.download_cancel.load(Ordering::SeqCst) {
            "cancelled"
        } else {
            "error"
        };
        let _ = app.emit(
            "model-progress",
            FinalProgress {
                id,
                downloaded: size,
                total: size,
                state: status.into(),
                error: result.err(),
            },
        );
    });
    Ok(())
}
#[tauri::command]
pub fn cancel_model_download(
    window: WebviewWindow,
    state: State<'_, Arc<Runtime>>,
) -> Result<(), String> {
    main_only(&window)?;
    state.download_cancel.store(true, Ordering::SeqCst);
    Ok(())
}
#[tauri::command]
pub fn delete_model(
    window: WebviewWindow,
    state: State<'_, Arc<Runtime>>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    if state.snapshot().state.active()
        || state
            .downloading
            .lock()
            .map_err(|_| "Gestore download non disponibile.")?
            .is_some()
    {
        return Err("Termina dettatura e download prima di eliminare un modello.".into());
    }
    if !inference::list_models(&state.model_dir)
        .iter()
        .any(|m| m.id == id)
    {
        return Err("Modello sconosciuto.".into());
    }
    let path = state.model_dir.join(format!("ggml-{id}.bin"));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    state.send(Control::Unload)
}

#[tauri::command]
pub async fn get_dashboard(
    window: WebviewWindow,
    state: State<'_, Arc<Runtime>>,
    filter: DashboardFilter,
) -> Result<Dashboard, String> {
    main_only(&window)?;
    let runtime = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.".to_string())?
            .dashboard(&filter)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn list_sessions(
    window: WebviewWindow,
    state: State<'_, Arc<Runtime>>,
    filter: DashboardFilter,
    search: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<Session>, String> {
    main_only(&window)?;
    let runtime = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.".to_string())?
            .list_sessions(&filter, &search, limit, offset)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn delete_transcript(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<Runtime>>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    state
        .repository
        .lock()
        .map_err(|_| "Database non disponibile.")?
        .delete_transcript(&id)?;
    let _ = app.emit("sessions-changed", ());
    Ok(())
}
#[tauri::command]
pub fn clear_data(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<Runtime>>,
    kind: String,
) -> Result<(), String> {
    main_only(&window)?;
    if state.snapshot().state.active() {
        return Err("Termina la dettatura prima di cancellare i dati.".into());
    }
    if !matches!(kind.as_str(), "transcripts" | "metrics" | "all") {
        return Err("Tipo di cancellazione sconosciuto.".into());
    }
    if kind == "all" {
        if state
            .downloading
            .lock()
            .map_err(|_| "Gestore download non disponibile.")?
            .is_some()
        {
            return Err("Termina il download prima di cancellare tutti i dati.".into());
        }
        // Restore native preferences first so persisted defaults and registered shortcut agree.
        save_settings(
            window.clone(),
            app.clone(),
            state.clone(),
            Settings::default(),
        )?;
        state.send(Control::Unload)?;
        for model in inference::list_models(&state.model_dir) {
            let path = state.model_dir.join(format!("ggml-{}.bin", model.id));
            if path.symlink_metadata().is_ok() {
                std::fs::remove_file(path).map_err(|e| format!("Impostazioni ripristinate, ma eliminazione del modello non completata: {e}"))?;
            }
        }
    }
    state
        .repository
        .lock()
        .map_err(|_| "Database non disponibile.")?
        .clear_data(&kind)?;
    {
        let mut s = state
            .snapshot
            .lock()
            .map_err(|_| "Stato non disponibile.")?;
        s.recovery_text = None;
        s.recovery_pending = false;
    }
    let _ = app.emit("sessions-changed", ());
    let _ = app.emit("dictation-state", state.snapshot());
    Ok(())
}
#[tauri::command]
pub async fn export_data(
    window: WebviewWindow,
    state: State<'_, Arc<Runtime>>,
    kind: String,
    format: String,
) -> Result<String, String> {
    main_only(&window)?;
    let runtime = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.".to_string())?
            .export_data(&kind, &format)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_file(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Arc<Runtime>>,
    kind: String,
    format: String,
) -> Result<Option<String>, String> {
    main_only(&window)?;
    let runtime = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let contents = runtime
            .repository
            .lock()
            .map_err(|_| "Database non disponibile.")?
            .export_data(&kind, &format)?;
        let chosen = app
            .dialog()
            .file()
            .set_title("Esporta i dati locali di Dicta")
            .set_file_name(format!("dicta-{kind}.{format}"))
            .add_filter("Dati Dicta", &[format.as_str()])
            .blocking_save_file();
        let Some(file) = chosen else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| e.to_string())?;
        // The path comes only from the native save dialog, never from renderer IPC.
        std::fs::write(&path, contents).map_err(|e| e.to_string())?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn shortcut_event(app: &AppHandle, event: tauri_plugin_global_shortcut::ShortcutEvent) {
    let runtime = app.state::<Arc<Runtime>>();
    let pressed = event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed;
    if pressed {
        if runtime.shortcut_down.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Ok(settings) = runtime.settings() {
            let command = if matches!(
                runtime.snapshot().state,
                crate::state::Phase::Preparing | crate::state::Phase::Recording
            ) && settings.shortcut_mode == "toggle"
            {
                Some(Control::Stop)
            } else if !runtime.snapshot().state.active() {
                Some(Control::Start)
            } else {
                None
            };
            if let Some(command) = command {
                let _ = runtime.send(command);
            }
        }
    } else {
        if !runtime.shortcut_down.swap(false, Ordering::SeqCst) {
            return;
        }
        if runtime.settings().is_ok_and(|s| s.shortcut_mode == "hold") {
            let _ = runtime.send(Control::Stop);
        }
    }
}
