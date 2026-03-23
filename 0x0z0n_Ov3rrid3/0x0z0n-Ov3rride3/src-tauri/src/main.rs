// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::fs;

// [01] TELEMETRY PAYLOAD: Polls both the RTX 5060 and the AMD 780M
#[tauri::command]
fn get_telemetry() -> Result<serde_json::Value, String> {
    // 1. Fetch NVIDIA Data
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

    // 2. Fetch AMD Data (Direct from sysfs)
    let mut amd_usage = 0;
    let paths = ["/sys/class/drm/card0/device/gpu_busy_percent", "/sys/class/drm/card1/device/gpu_busy_percent"];
    for path in paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(usage) = content.trim().parse::<u32>() {
                amd_usage = usage;
                break;
            }
        }
    }

    // Package it up and send to the UI
    Ok(serde_json::json!({
        "nvidia": nv_data,
        "amd": {"usage": amd_usage}
    }))
}

// [02] HARDWARE LIMITER: Triggers pkexec for TGP control
#[tauri::command]
fn set_power_limit(watts: u32) -> Result<String, String> {
    let output = Command::new("pkexec")
        .args(["nvidia-smi", "-pl", &watts.to_string()])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(format!("SUCCESS: TGP LOCKED AT {}W", watts))
    } else {
        // If the user cancels the prompt or it fails
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("ERR/ABORTED: {}", err.trim()))
    }
}

// [03] DISPLAY OVERRIDE: Triggers xrandr
#[tauri::command]
fn set_brightness(display: &str, level: f32) -> Result<String, String> {
    let output = Command::new("xrandr")
        .args(["--output", display, "--brightness", &level.to_string()])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok("SUCCESS: BRIGHTNESS OVERRIDE APPLIED".to_string())
    } else {
        Err("ERR: XRANDR INJECTION FAILED".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        // Register your backend commands here so the frontend can see them
        .invoke_handler(tauri::generate_handler![get_telemetry, set_power_limit, set_brightness])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
