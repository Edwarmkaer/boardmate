use boardmate_core::{Command, Device, DeviceExecutor, DeviceReply};
use boardmate_providers::AgentReply;
use boardmate_runtime::{Runtime, RuntimeInfo};
use boardmate_transports::{BleEndpoint, ConnectionConfig};
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
fn runtime_info(runtime: State<'_, Arc<Runtime>>) -> RuntimeInfo {
    runtime.info()
}

#[tauri::command]
async fn list_devices(runtime: State<'_, Arc<Runtime>>) -> Result<Vec<Device>, String> {
    Ok(runtime.devices().await)
}

#[tauri::command]
async fn execute_command(
    runtime: State<'_, Arc<Runtime>>,
    command: Command,
) -> Result<DeviceReply, String> {
    runtime.execute(command).await
}

#[tauri::command]
async fn chat(runtime: State<'_, Arc<Runtime>>, message: String) -> Result<AgentReply, String> {
    runtime.chat(&message).await
}

#[tauri::command]
async fn connect_device(
    runtime: State<'_, Arc<Runtime>>,
    config: ConnectionConfig,
) -> Result<Device, String> {
    runtime.connect(config).await
}

#[tauri::command]
async fn disconnect_device(
    runtime: State<'_, Arc<Runtime>>,
    device_id: String,
) -> Result<(), String> {
    runtime.disconnect(&device_id).await
}

#[tauri::command]
async fn serial_ports() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(boardmate_transports::serial_ports)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn scan_ble() -> Result<Vec<BleEndpoint>, String> {
    boardmate_transports::scan_ble().await
}

#[tauri::command]
fn set_expanded(window: tauri::WebviewWindow, expanded: bool) -> Result<(), String> {
    window
        .set_min_size(Some(tauri::LogicalSize::new(88.0, 88.0)))
        .map_err(|error| error.to_string())?;
    window
        .set_resizable(expanded)
        .map_err(|error| error.to_string())?;
    let size = if expanded {
        tauri::LogicalSize::new(420.0, 680.0)
    } else {
        tauri::LogicalSize::new(88.0, 88.0)
    };
    window.set_size(size).map_err(|error| error.to_string())?;
    if expanded {
        window
            .set_min_size(Some(tauri::LogicalSize::new(320.0, 480.0)))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn run() {
    let _ = dotenvy::dotenv();
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Arc::new(
                Runtime::from_env().map_err(std::io::Error::other)?,
            ));
            let show = tauri::menu::MenuItem::with_id(
                app,
                "show",
                "Mostrar boardmate",
                true,
                None::<&str>,
            )?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;
            let mut tray = tauri::tray::TrayIconBuilder::new()
                .tooltip("boardmate")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            runtime_info,
            list_devices,
            execute_command,
            chat,
            connect_device,
            disconnect_device,
            serial_ports,
            scan_ble,
            set_expanded
        ])
        .run(tauri::generate_context!())
        .expect("could not run boardmate desktop");
}
