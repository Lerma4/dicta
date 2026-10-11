pub mod application;
pub mod audio;
pub mod commands;
pub mod domain;
pub mod inference;
pub mod persistence;
pub mod platform;
pub mod state;

pub fn run() {
    use application::{Control, Runtime};
    use std::sync::Arc;
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
        Manager,
    };
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                .args(["--hidden"])
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| commands::shortcut_event(app, event))
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::start_dictation,
            commands::stop_dictation,
            commands::cancel_dictation,
            commands::copy_recovery,
            commands::copy_text,
            commands::get_settings,
            commands::save_settings,
            commands::get_history_enabled,
            commands::get_permissions,
            commands::request_permission,
            commands::list_input_devices,
            commands::list_models,
            commands::download_model,
            commands::cancel_model_download,
            commands::delete_model,
            commands::get_dashboard,
            commands::list_sessions,
            commands::delete_transcript,
            commands::clear_data,
            commands::export_data,
            commands::export_file
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let model_dir = data_dir.join("models");
            std::fs::create_dir_all(&model_dir)?;
            let model_dir = model_dir.canonicalize()?;
            let repository = persistence::Repository::open(data_dir.join("dicta.sqlite3"))?;
            repository.recover_interrupted()?;
            let settings = repository.settings()?;
            let runtime = Runtime::new(repository, model_dir, app.handle().clone());
            app.manage(runtime);
            if let Err(e) = app.global_shortcut().register(settings.shortcut.as_str()) {
                use tauri::Emitter;
                let runtime = app.state::<Arc<Runtime>>();
                let mut snapshot = runtime.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                snapshot.message = Some(format!(
                    "Scorciatoia non disponibile: {e}. Modificala nelle impostazioni."
                ));
                let _ = app.emit("dictation-state", &*snapshot);
            }
            let open = MenuItem::with_id(app, "open", "Apri Dicta", true, None::<&str>)?;
            let toggle = MenuItem::with_id(
                app,
                "toggle",
                "Avvia / termina dettatura",
                true,
                None::<&str>,
            )?;
            let cancel = MenuItem::with_id(app, "cancel", "Annulla dettatura", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Esci da Dicta", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &cancel, &quit])?;
            // Tiny monochrome microphone template; no asset/runtime dependencies.
            let mut pixels = vec![0u8; 32 * 32 * 4];
            for y in 5..27usize {
                for x in 8..24usize {
                    if (12..20).contains(&x) && y < 19
                        || (x == 8 || x == 23) && (12..21).contains(&y)
                        || y == 21 && (9..23).contains(&x)
                        || (15..17).contains(&x) && y > 21
                        || y == 26 && (11..21).contains(&x)
                    {
                        pixels[(y * 32 + x) * 4 + 3] = 255;
                    }
                }
            }
            TrayIconBuilder::with_id("dicta")
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .icon_as_template(true)
                .tooltip("Dicta — dettatura locale")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let runtime = app.state::<Arc<Runtime>>();
                    match event.id.as_ref() {
                        "open" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "toggle" => {
                            let _ = runtime.send(
                                if runtime.snapshot().state == state::Phase::Recording {
                                    Control::Stop
                                } else {
                                    Control::Start
                                },
                            );
                        }
                        "cancel" => {
                            let _ = runtime.send(Control::Cancel);
                        }
                        "quit" => {
                            let _ = runtime.send(Control::Shutdown);
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                if let Some(overlay) = app.get_webview_window("overlay") {
                    unsafe {
                        platform::configure_overlay(overlay.ns_window()?)?;
                    }
                }
                let events = platform::subscribe_system_events()?;
                let runtime = app.state::<Arc<Runtime>>().inner().clone();
                std::thread::spawn(move || {
                    while let Ok(event) = events.recv() {
                        if matches!(
                            event,
                            platform::SystemEvent::WillSleep
                                | platform::SystemEvent::ScreensDidSleep
                        ) {
                            runtime
                                .shortcut_down
                                .store(false, std::sync::atomic::Ordering::SeqCst);
                            let _ = runtime.send(Control::Cancel);
                        }
                    }
                });
            }
            if std::env::args().any(|arg| arg == "--hidden") {
                if let Some(main) = app.get_webview_window("main") {
                    main.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Dicta could not start");
}
