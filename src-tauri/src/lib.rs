use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, Window};

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

fn random_between(seed: u64, min: i32, max: i32) -> i32 {
  if max <= min {
    return min;
  }

  let span = (max - min + 1) as u64;
  min + (seed % span) as i32
}

#[tauri::command]
async fn spawn_pumpkin(window: Window) -> Result<(), String> {
  const PUMPKIN_LABEL: &str = "pumpkin";
  const PUMPKIN_SIZE: f64 = 220.0;

  let app = window.app_handle();
  if let Some(existing) = app.get_webview_window(PUMPKIN_LABEL) {
    let _ = existing.close();
  }

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

  let pumpkin_size = PUMPKIN_SIZE.round() as i32;
  let max_x = monitor_x + (monitor_width as i32 - pumpkin_size).max(0);
  let max_y = monitor_y + (monitor_height as i32 - pumpkin_size).max(0);
  let time_seed = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_nanos() as u64)
    .unwrap_or(0);
  let position_seed = window
    .outer_position()
    .map(|position| ((position.x as i64 as u64) << 32) ^ (position.y as i64 as u64))
    .unwrap_or(0);

  let x = random_between(time_seed ^ position_seed ^ 0x9E37_79B9_7F4A_7C15, monitor_x, max_x) as f64;
  let y = random_between(
    time_seed.rotate_left(17) ^ position_seed ^ 0xC2B2_AE3D_27D4_EB4F,
    monitor_y,
    max_y,
  ) as f64;

  WebviewWindowBuilder::new(app, PUMPKIN_LABEL, WebviewUrl::App("pumpkin.html".into()))
    .title("Pumpkin")
    .position(x, y)
    .inner_size(PUMPKIN_SIZE, PUMPKIN_SIZE)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .focused(false)
    .build()
    .map_err(|error| error.to_string())?;

  Ok(())
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
      spawn_pumpkin,
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
