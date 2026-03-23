#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::fs;

// [01] TELEMETRY
#[tauri::command]
fn get_telemetry() -> Result<serde_json::Value, String> {
    let nv_output = Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw", "--format=csv,noheader,nounits"])
        .output();
    
    let mut nv_data = serde_json::json!({"usage": 0, "vram_used": 0, "vram_total": 0, "temp": 0, "power": 0.0});
    
    if let Ok(output) = nv_output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split(", ").collect();
            if parts.len() == 5 {
                nv_data = serde_json::json!({
                    "usage": parts[0].parse::<u32>().unwrap_or(0),
                    "vram_used": parts[1].parse::<u32>().unwrap_or(0),
                    "vram_total": parts[2].parse::<u32>().unwrap_or(0),
                    "temp": parts[3].parse::<u32>().unwrap_or(0),
                    "power": parts[4].parse::<f32>().unwrap_or(0.0),
                });
            }
        }
    }

    let mut amd_usage = 0;
    let paths = ["/sys/class/drm/card0/device/gpu_busy_percent", "/sys/class/drm/card1/device/gpu_busy_percent"];
    for path in paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(usage) = content.trim().parse::<u32>() { amd_usage = usage; break; }
        }
    }

    Ok(serde_json::json!({ "nvidia": nv_data, "amd": {"usage": amd_usage} }))
}

// [02] POWER LIMITER
#[tauri::command]
fn set_power_limit(watts: u32) -> Result<String, String> {
    let output = Command::new("pkexec")
        .args(["nvidia-smi", "-pl", &watts.to_string()]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(format!("SUCCESS: TGP LOCKED AT {}W", watts)) } 
    else { Err(format!("ERR: {}", String::from_utf8_lossy(&output.stderr).trim())) }
}

// [03] DISPLAY OVERRIDE
#[tauri::command]
fn set_brightness(level: f32) -> Result<String, String> {
    let xrandr_out = Command::new("sh").arg("-c").arg("xrandr | grep ' connected'").output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&xrandr_out.stdout);
    let display = stdout.split_whitespace().next().unwrap_or("eDP");

    let output = Command::new("xrandr")
        .args(["--output", display, "--brightness", &level.to_string()]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(format!("SUCCESS: BRIGHTNESS SET TO {}%", (level * 100.0) as i32)) } 
    else { Err("ERR: XRANDR INJECTION FAILED".to_string()) }
}

// [04] FILE BROWSER
#[tauri::command]
fn browse_file() -> Result<String, String> {
    let output = Command::new("zenity")
        .args(["--file-selection", "--title=z0n // SELECT TARGET BINARY", "--filename=/usr/bin/", "--modal"])
        .output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).trim().to_string()) } 
    else { Err("BROWSE ABORTED".to_string()) }
}

// [05] GUI LAUNCHER
#[tauri::command]
fn launch_gui(target: &str, env_vars: &str) -> Result<String, String> {
    let cmd = format!("{} '{}'", env_vars, target);
    Command::new("bash").arg("-c").arg(&cmd).spawn().map_err(|e| e.to_string())?;
    Ok(format!("SUCCESS: SPAWNED {}", target))
}

// [06] TERMINAL LAUNCHER
#[tauri::command]
fn launch_cmd(payload: &str, env_vars: &str) -> Result<String, String> {
    let inner_cmd = format!("{}{}", env_vars, payload);
    let terminal_cmd = format!("x-terminal-emulator -e bash -c \"{}; echo; echo '>> SESSION TERMINATED'; exec bash\"", inner_cmd);
    Command::new("bash").arg("-c").arg(&terminal_cmd).spawn().map_err(|e| e.to_string())?;
    Ok("SUCCESS: TERMINAL HOOK ESTABLISHED".to_string())
}

// CORE BUILDER
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_telemetry, 
            set_power_limit, 
            set_brightness, 
            browse_file, 
            launch_gui, 
            launch_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
