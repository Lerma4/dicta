# Dicta implementation contract

Frontend and backend exchange snake_case JSON. All commands return Result<T, String>.

Settings: { language: string ('it' or 'auto'), model_id: string ('tiny' default), input_device: string|null, shortcut: string ('CommandOrControl+Shift+Space'), shortcut_mode: 'toggle'|'hold', history_enabled: boolean (false), retention_days: number (30), overlay_position: 'bottom'|'top', autostart: boolean (false), vad_threshold: number (0.01), sounds: boolean (false) }.

DictationSnapshot: { state: 'idle'|'preparing'|'recording'|'processing'|'inserting'|'completed'|'cancelled'|'error', session_id: string|null, elapsed_ms: number, level: number, speech: boolean, message: string|null, recovery_text: string|null }. Backend event 'dictation-state' sends full snapshot; 'audio-level' sends {level, speech, elapsed_ms}. Recovery text only in RAM regardless of history preference. Event 'model-progress': {id, downloaded, total, state, error: string|null}.

Commands owned by orchestrator: get_snapshot, start_dictation, stop_dictation, cancel_dictation, copy_recovery, get_settings, save_settings(settings), get_permissions, request_permission(kind: 'microphone'|'accessibility'), list_input_devices, list_models, download_model(id), cancel_model_download, delete_model(id), get_dashboard(filter), list_sessions(filter, search, limit, offset), delete_transcript(id), clear_data(kind: 'transcripts'|'metrics'|'all'), export_data(kind: 'sessions'|'transcripts', format: 'json'|'csv') returning file contents (UI offers local save), get_history_enabled.

Permissions {microphone: 'granted'|'denied'|'undetermined'|'unavailable', accessibility: boolean, platform: string, text_injection: boolean}.
ModelInfo {id,name,size_bytes,checksum,installed,downloading,description}. InputDevice {name,is_default}.
Session {id,started_at_utc,ended_at_utc,status,recorded_ms,inference_ms,insertion_ms,words_count,chars_count,language_code,model_id,error_category,input_mode,text: string|null}.
DashboardFilter {days: number (1,7,30,90),language: string|null,model_id:string|null,start_date:string|null,end_date:string|null,granularity:'day'|'week'|'month'} dates YYYY-MM-DD local inclusive.
Dashboard {started,completed,failed,cancelled,words,chars,recorded_ms,inference_ms,insertion_ms,mean_inference_ms,median_inference_ms,p95_inference_ms,wpm,throughput,active_days,previous_words,words_change_pct:number|null,top_language:string|null,top_model:string|null,timeline: [{date,sessions,words,inference_ms}],heatmap:[{weekday:number (Monday=0),hour,sessions,words}],duration_histogram:[{label,count}],models:[{model_id,sessions,mean_inference_ms}],errors:[{category,count}]}.

Rust module ownership:
- Persistence agent: domain/mod.rs (Settings, Session, Dashboard types), persistence/mod.rs repository migrations settings sessions aggregate export tests. Repository::open(path), settings(), save_settings(&Settings), save_session(&Session, history_enabled), dashboard(&DashboardFilter), list_sessions(&DashboardFilter, search:&str, limit:u32, offset:u32), delete_transcript(id), clear_data(kind), export_data(kind,format), recover_interrupted(). Methods Result<_,String>.
- Native agent: audio/mod.rs capture, inference/mod.rs warm Whisper and verified model downloads, platform/mod.rs macOS guarded injection + permissions. Defines structs/types local to each module and exposes ergonomic APIs documented in message to orchestrator. Do NOT write Cargo.toml/lib.rs. Give required dependencies ASAP.
- UI agent: all src frontend files (except root scaffold orchestrator creates), index.html/overlay.html, styles, tests. Use contract commands above; no fake production data. Own package.json and frontend tooling.
- Orchestrator: Rust Cargo/build/config/lib/main commands/runtime/state machine, scripts/docs/CI, integration and validation.

No agent commits; orchestrator stages coherent milestones. Preserve other agents' files.

Final extensions: Snapshot includes `recovery_pending: boolean`; `export_file(kind,format)` opens the native Save dialog and returns selected path or null (kind supports sessions/transcripts/settings, settings JSON only); `copy_text(text)` performs explicit main-window clipboard writes with a 1MB bound. Full clear removes catalog model files and restores native preferences. IPC defaults camelCase argument naming only where multiword args exist; current frontend arguments are single words and DTOs remain snake_case.
