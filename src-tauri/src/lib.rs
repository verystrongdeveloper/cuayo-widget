use serde::Serialize;
use std::{
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};
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

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
  value.max(min).min(max)
}

fn touching_or_overlapping(
  ax: i32,
  ay: i32,
  aw: i32,
  ah: i32,
  bx: i32,
  by: i32,
  bw: i32,
  bh: i32,
) -> bool {
  let a_left = ax;
  let a_right = ax + aw;
  let a_top = ay;
  let a_bottom = ay + ah;
  let b_left = bx;
  let b_right = bx + bw;
  let b_top = by;
  let b_bottom = by + bh;

  !(a_right < b_left || a_left > b_right || a_bottom < b_top || a_top > b_bottom)
}

fn walk_window_to(
  window: &Window,
  start_x: i32,
  start_y: i32,
  target_x: i32,
  target_y: i32,
  main_width: i32,
  main_height: i32,
  pumpkin_x: i32,
  pumpkin_y: i32,
  pumpkin_size: i32,
  pumpkin_label: &str,
) -> Result<bool, String> {
  let delta_x = target_x - start_x;
  let delta_y = target_y - start_y;
  let distance = ((delta_x as f64).powi(2) + (delta_y as f64).powi(2)).sqrt();

  if distance < 1.0 {
    return Ok(false);
  }

  let mut steps = (distance / 4.0).ceil() as i32;
  steps = clamp_i32(steps, 24, 320);
  let direction_x = delta_x as f64 / distance;
  let direction_y = delta_y as f64 / distance;
  let perp_x = -direction_y;
  let perp_y = direction_x;
  let gait_cycles = clamp_i32((distance / 34.0).round() as i32, 5, 20) as f64;
  let wobble_amp = if distance < 180.0 { 1.3 } else { 2.0 };

  for step in 1..=steps {
    let t = step as f64 / steps as f64;
    let eased_t = 0.5 - 0.5 * (std::f64::consts::PI * t).cos();
    let base_x = start_x as f64 + (delta_x as f64) * eased_t;
    let base_y = start_y as f64 + (delta_y as f64) * eased_t;
    let gait_wave = (2.0 * std::f64::consts::PI * gait_cycles * t).sin();
    let crawl_drop = gait_wave.abs() * 0.8;
    let x = (base_x + perp_x * gait_wave * wobble_amp).round() as i32;
    let y = (base_y + perp_y * gait_wave * wobble_amp + crawl_drop).round() as i32;

    window
      .set_position(PhysicalPosition::new(x, y))
      .map_err(|error| error.to_string())?;

    if touching_or_overlapping(
      x,
      y,
      main_width,
      main_height,
      pumpkin_x,
      pumpkin_y,
      pumpkin_size,
      pumpkin_size,
    ) {
      if let Some(pumpkin_window) = window.app_handle().get_webview_window(pumpkin_label) {
        let _ = pumpkin_window.close();
      }
      return Ok(true);
    }

    let edge_slow = (1.0 - (std::f64::consts::PI * t).sin()).max(0.0);
    let delay_ms = 8.0 + edge_slow * 4.0;
    thread::sleep(Duration::from_millis(delay_ms.round() as u64));
  }

  window
    .set_position(PhysicalPosition::new(target_x, target_y))
    .map_err(|error| error.to_string())?;

  if touching_or_overlapping(
    target_x,
    target_y,
    main_width,
    main_height,
    pumpkin_x,
    pumpkin_y,
    pumpkin_size,
    pumpkin_size,
  ) {
    if let Some(pumpkin_window) = window.app_handle().get_webview_window(pumpkin_label) {
      let _ = pumpkin_window.close();
    }
    return Ok(true);
  }

  Ok(false)
}

#[tauri::command]
async fn spawn_pumpkin(window: Window) -> Result<bool, String> {
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

  let pumpkin_x = random_between(time_seed ^ position_seed ^ 0x9E37_79B9_7F4A_7C15, monitor_x, max_x);
  let pumpkin_y = random_between(
    time_seed.rotate_left(17) ^ position_seed ^ 0xC2B2_AE3D_27D4_EB4F,
    monitor_y,
    max_y,
  );

  WebviewWindowBuilder::new(app, PUMPKIN_LABEL, WebviewUrl::App("pumpkin.html".into()))
    .title("Pumpkin")
    .position(pumpkin_x as f64, pumpkin_y as f64)
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

  let main_size = window.outer_size().map_err(|error| error.to_string())?;
  let main_width = main_size.width as i32;
  let main_height = main_size.height as i32;
  let pumpkin_size = PUMPKIN_SIZE.round() as i32;
  let gap = 0;

  let left_candidate = pumpkin_x - main_width - gap;
  let right_candidate = pumpkin_x + pumpkin_size + gap;
  let centered_x = pumpkin_x + pumpkin_size / 2 - main_width / 2;
  let min_main_x = monitor_x;
  let max_main_x = monitor_x + (monitor_width as i32 - main_width).max(0);
  let target_x = if left_candidate >= min_main_x {
    left_candidate
  } else if right_candidate <= max_main_x {
    right_candidate
  } else {
    clamp_i32(centered_x, min_main_x, max_main_x)
  };

  let centered_y = pumpkin_y + pumpkin_size / 2 - main_height / 2;
  let min_main_y = monitor_y;
  let max_main_y = monitor_y + (monitor_height as i32 - main_height).max(0);
  let target_y = clamp_i32(centered_y, min_main_y, max_main_y);

  let start_position = window.outer_position().map_err(|error| error.to_string())?;
  let touched = walk_window_to(
    &window,
    start_position.x,
    start_position.y,
    target_x,
    target_y,
    main_width,
    main_height,
    pumpkin_x,
    pumpkin_y,
    pumpkin_size,
    PUMPKIN_LABEL,
  )?;

  Ok(touched)
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
