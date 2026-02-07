use serde::Serialize;
use std::{
  sync::{Mutex, OnceLock},
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, Window};

static PUMPKIN_DRAGGING: OnceLock<Mutex<bool>> = OnceLock::new();
static FOLLOW_PHASE: OnceLock<Mutex<u8>> = OnceLock::new();
static FOLLOW_WORKER_RUNNING: OnceLock<Mutex<bool>> = OnceLock::new();
static PUMPKIN_EATEN_PENDING: OnceLock<Mutex<bool>> = OnceLock::new();
static PUMPKIN_TIMEOUT_PENDING: OnceLock<Mutex<bool>> = OnceLock::new();
static PUMPKIN_SESSION_ID: OnceLock<Mutex<u64>> = OnceLock::new();
static PUMPKIN_CHASE_TIMED_OUT: OnceLock<Mutex<bool>> = OnceLock::new();

fn pumpkin_dragging_state() -> &'static Mutex<bool> {
  PUMPKIN_DRAGGING.get_or_init(|| Mutex::new(false))
}

fn follow_phase_state() -> &'static Mutex<u8> {
  FOLLOW_PHASE.get_or_init(|| Mutex::new(0))
}

fn follow_worker_running_state() -> &'static Mutex<bool> {
  FOLLOW_WORKER_RUNNING.get_or_init(|| Mutex::new(false))
}

fn pumpkin_eaten_pending_state() -> &'static Mutex<bool> {
  PUMPKIN_EATEN_PENDING.get_or_init(|| Mutex::new(false))
}

fn pumpkin_timeout_pending_state() -> &'static Mutex<bool> {
  PUMPKIN_TIMEOUT_PENDING.get_or_init(|| Mutex::new(false))
}

fn pumpkin_session_id_state() -> &'static Mutex<u64> {
  PUMPKIN_SESSION_ID.get_or_init(|| Mutex::new(0))
}

fn pumpkin_chase_timed_out_state() -> &'static Mutex<bool> {
  PUMPKIN_CHASE_TIMED_OUT.get_or_init(|| Mutex::new(false))
}

fn set_pumpkin_dragging_state(is_dragging: bool) {
  if let Ok(mut dragging) = pumpkin_dragging_state().lock() {
    *dragging = is_dragging;
  }
}

fn is_pumpkin_dragging() -> bool {
  pumpkin_dragging_state()
    .lock()
    .map(|dragging| *dragging)
    .unwrap_or(false)
}

fn set_pumpkin_chase_timed_out(is_timed_out: bool) {
  if let Ok(mut timed_out) = pumpkin_chase_timed_out_state().lock() {
    *timed_out = is_timed_out;
  }
}

fn is_pumpkin_chase_timed_out() -> bool {
  pumpkin_chase_timed_out_state()
    .lock()
    .map(|timed_out| *timed_out)
    .unwrap_or(false)
}

fn clear_pumpkin_timeout_pending() {
  if let Ok(mut pending) = pumpkin_timeout_pending_state().lock() {
    *pending = false;
  }
}

fn mark_pumpkin_timeout_pending() {
  if let Ok(mut pending) = pumpkin_timeout_pending_state().lock() {
    *pending = true;
  }
}

fn take_pumpkin_timeout_pending() -> bool {
  if let Ok(mut pending) = pumpkin_timeout_pending_state().lock() {
    let was_pending = *pending;
    *pending = false;
    return was_pending;
  }
  false
}

fn next_pumpkin_session_id() -> u64 {
  if let Ok(mut session_id) = pumpkin_session_id_state().lock() {
    *session_id = session_id.saturating_add(1);
    return *session_id;
  }
  0
}

fn current_pumpkin_session_id() -> u64 {
  pumpkin_session_id_state()
    .lock()
    .map(|session_id| *session_id)
    .unwrap_or(0)
}

fn invalidate_pumpkin_session() {
  let _ = next_pumpkin_session_id();
}

fn on_pumpkin_timeout() {
  set_pumpkin_dragging_state(false);
  if let Ok(mut phase) = follow_phase_state().lock() {
    *phase = 0;
  }
  set_pumpkin_chase_timed_out(true);
  mark_pumpkin_timeout_pending();
}

fn start_pumpkin_timeout_worker<R: tauri::Runtime>(app: tauri::AppHandle<R>, session_id: u64) {
  thread::spawn(move || {
    thread::sleep(Duration::from_secs(5));

    if current_pumpkin_session_id() != session_id {
      return;
    }
    if app.get_webview_window("pumpkin").is_none() {
      return;
    }

    on_pumpkin_timeout();
  });
}

fn begin_pumpkin_session<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
  if let Ok(mut pending) = pumpkin_eaten_pending_state().lock() {
    *pending = false;
  }
  clear_pumpkin_timeout_pending();
  set_pumpkin_chase_timed_out(false);
  let session_id = next_pumpkin_session_id();
  start_pumpkin_timeout_worker(app, session_id);
}

fn on_pumpkin_eaten() {
  invalidate_pumpkin_session();
  set_pumpkin_dragging_state(false);
  if let Ok(mut phase) = follow_phase_state().lock() {
    *phase = 0;
  }
  set_pumpkin_chase_timed_out(false);
  clear_pumpkin_timeout_pending();
  if let Ok(mut pending) = pumpkin_eaten_pending_state().lock() {
    *pending = true;
  }
}

fn take_pumpkin_eaten_pending() -> bool {
  if let Ok(mut pending) = pumpkin_eaten_pending_state().lock() {
    let was_pending = *pending;
    *pending = false;
    return was_pending;
  }
  false
}

fn try_start_follow_worker<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
  let should_start = if let Ok(mut running) = follow_worker_running_state().lock() {
    if *running {
      false
    } else {
      *running = true;
      true
    }
  } else {
    false
  };

  if !should_start {
    return;
  }

  thread::spawn(move || {
    loop {
      if !is_pumpkin_dragging() {
        break;
      }

      let Some(main_window) = app.get_webview_window("main") else {
        break;
      };
      let app_for_tick = app.clone();
      let _ = main_window.run_on_main_thread(move || {
        let Some(main_window) = app_for_tick.get_webview_window("main") else {
          return;
        };
        let Some(pumpkin_window) = app_for_tick.get_webview_window("pumpkin") else {
          return;
        };
        let _ = follow_main_toward_pumpkin_windows(&main_window, &pumpkin_window);
      });
      thread::sleep(Duration::from_millis(16));
    }

    if let Ok(mut running) = follow_worker_running_state().lock() {
      *running = false;
    }
  });
}

#[tauri::command]
fn start_drag(window: Window) -> Result<(), String> {
  window.start_dragging().map_err(|error| error.to_string())?;

  if window.label() == "pumpkin" {
    set_pumpkin_dragging_state(true);
    try_start_follow_worker(window.app_handle().clone());
  }

  Ok(())
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

fn monitor_bounds(window: &Window) -> Result<(i32, i32, u32, u32), String> {
  let monitor = if let Some(current) = window.current_monitor().map_err(|error| error.to_string())? {
    Some(current)
  } else {
    window.primary_monitor().map_err(|error| error.to_string())?
  };

  if let Some(monitor) = monitor {
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    Ok((
      monitor_position.x,
      monitor_position.y,
      monitor_size.width,
      monitor_size.height,
    ))
  } else {
    Ok((0, 0, 1920, 1080))
  }
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

fn close_pumpkin_if_touching(
  window: &Window,
  pumpkin_label: &str,
  main_x: i32,
  main_y: i32,
  main_w: i32,
  main_h: i32,
) -> bool {
  let Some(pumpkin_window) = window.app_handle().get_webview_window(pumpkin_label) else {
    return false;
  };

  let Ok(pumpkin_pos) = pumpkin_window.outer_position() else {
    return false;
  };
  let Ok(pumpkin_size) = pumpkin_window.outer_size() else {
    return false;
  };

  let touching = touching_or_overlapping(
    main_x,
    main_y,
    main_w,
    main_h,
    pumpkin_pos.x,
    pumpkin_pos.y,
    pumpkin_size.width as i32,
    pumpkin_size.height as i32,
  );

  if touching {
    consume_pumpkin_window(&pumpkin_window);
    return true;
  }

  false
}

fn consume_pumpkin_window<R: tauri::Runtime>(pumpkin_window: &tauri::WebviewWindow<R>) {
  let _ = pumpkin_window.hide();
  let _ = pumpkin_window.close();

  let app = pumpkin_window.app_handle().clone();
  let label = pumpkin_window.label().to_string();
  thread::spawn(move || {
    for _ in 0..24 {
      let Some(next_window) = app.get_webview_window(&label) else {
        break;
      };
      if next_window.close().is_ok() {
        break;
      }
      thread::sleep(Duration::from_millis(16));
    }
  });

  on_pumpkin_eaten();
}

fn close_dragging_pumpkin_if_touching<R: tauri::Runtime>(
  pumpkin_window: &tauri::WebviewWindow<R>,
  main_x: i32,
  main_y: i32,
  main_w: i32,
  main_h: i32,
) -> bool {
  let Ok(pumpkin_pos) = pumpkin_window.outer_position() else {
    return false;
  };
  let Ok(pumpkin_size) = pumpkin_window.outer_size() else {
    return false;
  };

  let touching = touching_or_overlapping(
    main_x,
    main_y,
    main_w,
    main_h,
    pumpkin_pos.x,
    pumpkin_pos.y,
    pumpkin_size.width as i32,
    pumpkin_size.height as i32,
  );

  if touching {
    consume_pumpkin_window(pumpkin_window);
    return true;
  }

  false
}

fn walk_window_to(
  window: &Window,
  start_x: i32,
  start_y: i32,
  target_x: i32,
  target_y: i32,
  main_width: i32,
  main_height: i32,
  initial_pumpkin_x: i32,
  initial_pumpkin_y: i32,
  pumpkin_label: &str,
) -> Result<bool, String> {
  if is_pumpkin_chase_timed_out() {
    return Ok(false);
  }

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
    if is_pumpkin_chase_timed_out() {
      return Ok(false);
    }
    if is_pumpkin_dragging() {
      return Ok(false);
    }

    if let Some(pumpkin_window) = window.app_handle().get_webview_window(pumpkin_label) {
      if let Ok(pumpkin_pos) = pumpkin_window.outer_position() {
        if (pumpkin_pos.x - initial_pumpkin_x).abs() > 2 || (pumpkin_pos.y - initial_pumpkin_y).abs() > 2 {
          return Ok(false);
        }
      }
    }

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

    if close_pumpkin_if_touching(window, pumpkin_label, x, y, main_width, main_height) {
      return Ok(true);
    }

    let edge_slow = (1.0 - (std::f64::consts::PI * t).sin()).max(0.0);
    let delay_ms = 8.0 + edge_slow * 4.0;
    thread::sleep(Duration::from_millis(delay_ms.round() as u64));
  }

  if is_pumpkin_chase_timed_out() {
    return Ok(false);
  }

  window
    .set_position(PhysicalPosition::new(target_x, target_y))
    .map_err(|error| error.to_string())?;

  if close_pumpkin_if_touching(window, pumpkin_label, target_x, target_y, main_width, main_height) {
    return Ok(true);
  }

  Ok(false)
}

fn start_walk_to_pumpkin_worker(
  window: Window,
  target_x: i32,
  target_y: i32,
  main_width: i32,
  main_height: i32,
  initial_pumpkin_x: i32,
  initial_pumpkin_y: i32,
  pumpkin_label: &'static str,
) {
  thread::spawn(move || {
    let Ok(start_position) = window.outer_position() else {
      return;
    };

    let _ = walk_window_to(
      &window,
      start_position.x,
      start_position.y,
      target_x,
      target_y,
      main_width,
      main_height,
      initial_pumpkin_x,
      initial_pumpkin_y,
      pumpkin_label,
    );
  });
}

#[tauri::command]
async fn spawn_pumpkin(window: Window) -> Result<bool, String> {
  const PUMPKIN_LABEL: &str = "pumpkin";
  const PUMPKIN_SIZE: f64 = 220.0;

  set_pumpkin_dragging_state(false);

  let app = window.app_handle();
  if let Some(existing) = app.get_webview_window(PUMPKIN_LABEL) {
    let _ = existing.close();
  }

  let (monitor_x, monitor_y, monitor_width, monitor_height) = monitor_bounds(&window)?;

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

  begin_pumpkin_session(window.app_handle().clone());

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

  start_walk_to_pumpkin_worker(
    window.clone(),
    target_x,
    target_y,
    main_width,
    main_height,
    pumpkin_x,
    pumpkin_y,
    PUMPKIN_LABEL,
  );

  Ok(false)
}

fn follow_main_toward_pumpkin_windows<R: tauri::Runtime>(
  main_window: &tauri::WebviewWindow<R>,
  pumpkin_window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
  const PUMPKIN_GAP: i32 = 0;
  const FOLLOW_RATIO: f64 = 0.18;
  const MAX_STEP: i32 = 8;

  let pumpkin_pos = pumpkin_window.outer_position().map_err(|error| error.to_string())?;
  let pumpkin_size = pumpkin_window.outer_size().map_err(|error| error.to_string())?;
  let main_pos = main_window.outer_position().map_err(|error| error.to_string())?;
  let main_size = main_window.outer_size().map_err(|error| error.to_string())?;

  let pumpkin_x = pumpkin_pos.x;
  let pumpkin_y = pumpkin_pos.y;
  let pumpkin_w = pumpkin_size.width as i32;
  let pumpkin_h = pumpkin_size.height as i32;
  let main_w = main_size.width as i32;
  let main_h = main_size.height as i32;

  if is_pumpkin_chase_timed_out() {
    let _ = close_dragging_pumpkin_if_touching(pumpkin_window, main_pos.x, main_pos.y, main_w, main_h);
    return Ok(());
  }

  if close_dragging_pumpkin_if_touching(pumpkin_window, main_pos.x, main_pos.y, main_w, main_h) {
    return Ok(());
  }

  let monitor = if let Some(current) = main_window.current_monitor().map_err(|error| error.to_string())? {
    Some(current)
  } else {
    main_window.primary_monitor().map_err(|error| error.to_string())?
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
  let left_candidate = pumpkin_x - main_w - PUMPKIN_GAP;
  let right_candidate = pumpkin_x + pumpkin_w + PUMPKIN_GAP;
  let centered_x = pumpkin_x + pumpkin_w / 2 - main_w / 2;
  let min_main_x = monitor_x;
  let max_main_x = monitor_x + (monitor_width as i32 - main_w).max(0);
  let target_x = if left_candidate >= min_main_x {
    left_candidate
  } else if right_candidate <= max_main_x {
    right_candidate
  } else {
    clamp_i32(centered_x, min_main_x, max_main_x)
  };

  let centered_y = pumpkin_y + pumpkin_h / 2 - main_h / 2;
  let min_main_y = monitor_y;
  let max_main_y = monitor_y + (monitor_height as i32 - main_h).max(0);
  let target_y = clamp_i32(centered_y, min_main_y, max_main_y);

  let delta_x = target_x - main_pos.x;
  let delta_y = target_y - main_pos.y;
  if delta_x.abs() <= 1 && delta_y.abs() <= 1 {
    let _ = close_dragging_pumpkin_if_touching(pumpkin_window, main_pos.x, main_pos.y, main_w, main_h);
    return Ok(());
  }

  let mut step_x = (delta_x as f64 * FOLLOW_RATIO).round() as i32;
  let mut step_y = (delta_y as f64 * FOLLOW_RATIO).round() as i32;
  if step_x == 0 && delta_x != 0 {
    step_x = delta_x.signum();
  }
  if step_y == 0 && delta_y != 0 {
    step_y = delta_y.signum();
  }
  step_x = clamp_i32(step_x, -MAX_STEP, MAX_STEP);
  step_y = clamp_i32(step_y, -MAX_STEP, MAX_STEP);

  let bob = if let Ok(mut phase) = follow_phase_state().lock() {
    *phase = (*phase + 1) % 4;
    match *phase {
      1 => 1,
      3 => -1,
      _ => 0,
    }
  } else {
    0
  };

  let next_x = clamp_i32(main_pos.x + step_x, min_main_x, max_main_x);
  let next_y = clamp_i32(main_pos.y + step_y + bob, min_main_y, max_main_y);
  main_window
    .set_position(PhysicalPosition::new(next_x, next_y))
    .map_err(|error| error.to_string())?;

  let _ = close_dragging_pumpkin_if_touching(pumpkin_window, next_x, next_y, main_w, main_h);
  Ok(())
}

#[tauri::command]
fn start_pumpkin_drag(window: Window) {
  set_pumpkin_dragging_state(true);
  try_start_follow_worker(window.app_handle().clone());
}

#[tauri::command]
fn stop_pumpkin_drag() {
  set_pumpkin_dragging_state(false);
  if let Ok(mut phase) = follow_phase_state().lock() {
    *phase = 0;
  }
}

#[tauri::command]
fn take_pumpkin_eaten_flag() -> bool {
  take_pumpkin_eaten_pending()
}

#[tauri::command]
fn take_pumpkin_timeout_flag() -> bool {
  take_pumpkin_timeout_pending()
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
      start_pumpkin_drag,
      stop_pumpkin_drag,
      take_pumpkin_eaten_flag,
      take_pumpkin_timeout_flag,
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
