use serde::Serialize;
use tauri::{Manager, PhysicalPosition, Window};

#[tauri::command]
fn start_drag(window: Window) -> Result<(), String> {
  window.start_dragging().map_err(|error| error.to_string())
}

#[derive(Serialize)]
struct WindowGeometry {
  x: i32,
  y: i32,
  width: u32,
  height: u32,
  monitor_x: i32,
  monitor_y: i32,
  monitor_width: u32,
  monitor_height: u32,
}

#[tauri::command]
fn get_window_geometry(window: Window) -> Result<WindowGeometry, String> {
  let position = window.outer_position().map_err(|error| error.to_string())?;
  let size = window.outer_size().map_err(|error| error.to_string())?;

  let monitor = if let Some(current) = window.current_monitor().map_err(|error| error.to_string())? {
    Some(current)
  } else {
    window.primary_monitor().map_err(|error| error.to_string())?
  };

  let (monitor_x, monitor_y, monitor_width, monitor_height) = if let Some(monitor) = monitor {
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    (
      monitor_position.x,
      monitor_position.y,
      monitor_size.width,
      monitor_size.height,
    )
  } else {
    (0, 0, 1920, 1080)
  };

  Ok(WindowGeometry {
    x: position.x,
    y: position.y,
    width: size.width,
    height: size.height,
    monitor_x,
    monitor_y,
    monitor_width,
    monitor_height,
  })
}

#[tauri::command]
fn set_window_position(window: Window, x: i32, y: i32) -> Result<(), String> {
  window
    .set_position(PhysicalPosition::new(x, y))
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn exit_app(window: Window) -> Result<(), String> {
  window.app_handle().exit(0);
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      start_drag,
      get_window_geometry,
      set_window_position,
      exit_app
    ])
    .on_page_load(|window, payload| {
      println!("[speaki] loaded url: {} on {}", payload.url(), window.label());
    })
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
