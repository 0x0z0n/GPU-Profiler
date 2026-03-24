#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::fs;
use std::io::Write;
use std::fs::OpenOptions;

// --- [ TELEMETRY & HARDWARE STATE ] ---
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

#[tauri::command]
fn get_ac_state() -> bool {
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(ps_type) = fs::read_to_string(path.join("type")) {
                if ps_type.trim() == "Mains" {
                    if let Ok(online) = fs::read_to_string(path.join("online")) { return online.trim() == "1"; }
                }
            }
        }
    }
    true 
}

#[tauri::command]
fn append_ledger(line: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/z0n_telemetry_ledger.csv", home);
    let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    writeln!(file, "{}", line).map_err(|e| e.to_string())?;
    Ok(())
}

// --- [ KERNEL SCRAPER ] ---
#[tauri::command]
fn get_gpu_processes() -> Vec<serde_json::Value> {
    let mut processes = Vec::new();
    if let Ok(output) = Command::new("nvidia-smi").args(["--query-compute-apps=pid,process_name,used_memory", "--format=csv,noheader,nounits"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(", ").collect();
            if parts.len() == 3 {
                let pid = parts[0];
                let name = parts[1].split('/').last().unwrap_or(parts[1]);
                let memory = parts[2];
                
                // Read raw bytes to bypass Linux null-byte string crashes
                let mut full_cmd = name.to_string();
                if let Ok(bytes) = fs::read(format!("/proc/{}/cmdline", pid)) {
                    let parsed = String::from_utf8_lossy(&bytes).replace('\0', " ").trim().to_string();
                    if !parsed.is_empty() { full_cmd = parsed; }
                }

                processes.push(serde_json::json!({
                    "pid": pid, 
                    "name": name, 
                    "full_cmd": full_cmd,
                    "memory": memory
                }));
            }
        }
    }
    processes
}

#[tauri::command]
fn kill_process(pid: String) -> Result<String, String> {
    let out = Command::new("pkexec").args(["kill", "-9", &pid]).output();
    if let Ok(output) = out {
        if output.status.success() { return Ok(format!("SUCCESS: TERMINATED PID {}", pid)); }
    }
    let cmd = format!("zenity --password --title='z0n // ROOT REQUIRED TO KILL PID' | sudo -S kill -9 {}", pid);
    let fallback = Command::new("sh").arg("-c").arg(&cmd).output().map_err(|e| e.to_string())?;
    if fallback.status.success() { Ok(format!("SUCCESS: TERMINATED PID {}", pid)) } else { Err("ERR: AUTHENTICATION DISMISSED".to_string()) }
}

#[tauri::command]
fn set_power_limit(watts: u32) -> Result<String, String> {
    let out = Command::new("pkexec").args(["nvidia-smi", "-pl", &watts.to_string()]).output();
    if let Ok(output) = out {
        if output.status.success() { return Ok(format!("SUCCESS: TGP LOCKED AT {}W", watts)); }
    }
    let cmd = format!("zenity --password --title='z0n // ROOT REQUIRED FOR HARDWARE OVERRIDE' | sudo -S nvidia-smi -pl {}", watts);
    let fallback = Command::new("sh").arg("-c").arg(&cmd).output().map_err(|e| e.to_string())?;
    if fallback.status.success() { Ok(format!("SUCCESS: TGP LOCKED AT {}W", watts)) } else { Err("ERR: AUTHENTICATION DISMISSED".to_string()) }
}

#[tauri::command]
fn set_brightness(level: f32) -> Result<String, String> {
    let xrandr_out = Command::new("sh").arg("-c").arg("xrandr | grep ' connected'").output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&xrandr_out.stdout);
    let display = stdout.split_whitespace().next().unwrap_or("eDP");
    let output = Command::new("xrandr").args(["--output", display, "--brightness", &level.to_string()]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(format!("SUCCESS: BRIGHTNESS SET TO {}%", (level * 100.0) as i32)) } else { Err("ERR: XRANDR INJECTION FAILED".to_string()) }
}

fn force_spawn_terminal(inner_script: &str) -> Result<(), String> {
    let safe_script = inner_script.replace("\"", "\\\"");
    let cmd = format!(
        "qterminal -e bash -c \"{}\" || xfce4-terminal -e \"bash -c '{}'\" || x-terminal-emulator -e bash -c \"{}\"", 
        safe_script, safe_script, safe_script
    );
    Command::new("bash").arg("-c").arg(&cmd).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// --- [ AUTOMATED TMUX DASHBOARD ] ---
#[tauri::command]
fn launch_soc_monitor() -> Result<String, String> {
    // export TMUX= prevents nesting panics when launching from an existing Tmux session
    let tmux_script = "\
        export TMUX=; \
        tmux kill-session -t CORTEX 2>/dev/null; \
        tmux new-session -d -s CORTEX; \
        tmux send-keys -t CORTEX sudo Space journalctl Space -f C-m; \
        tmux split-window -v -t CORTEX; \
        tmux send-keys -t CORTEX:0.1 nvtop C-m; \
        tmux split-window -h -t CORTEX:0.1; \
        tmux send-keys -t CORTEX:0.2 watch Space -n Space 2 Space ss Space -tuln C-m; \
        tmux select-pane -t CORTEX:0.0; \
        tmux attach -t CORTEX\
    ";
    force_spawn_terminal(tmux_script)?;
    Ok("SUCCESS: TMUX CORTEX DASHBOARD SPAWNED".to_string())
}

#[tauri::command]
fn spawn_nvidia_terminal() -> Result<String, String> {
    let env_vars = "export __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia __VK_LAYER_NV_optimus=NVIDIA_only;";
    let script = format!("{} echo '>> z0n // NVIDIA GPU ENVIRONMENT ACTIVE'; exec bash", env_vars);
    force_spawn_terminal(&script)?;
    Ok("SUCCESS: PERSISTENT SHELL SPAWNED".to_string())
}

#[tauri::command]
fn launch_cmd(payload: String, env_vars: String) -> Result<String, String> {
    let script = format!("{}{} ; echo ; echo '>> SESSION TERMINATED'; read -p '>> PRESS ENTER TO CLOSE...'", env_vars, payload);
    force_spawn_terminal(&script)?;
    Ok("SUCCESS: TERMINAL HOOK ESTABLISHED".to_string())
}

#[tauri::command]
fn browse_file() -> Result<String, String> {
    let output = Command::new("zenity").args(["--file-selection", "--title=z0n // SELECT TARGET BINARY", "--filename=/usr/bin/", "--modal"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(String::from_utf8_lossy(&output.stdout).trim().to_string()) } else { Err("BROWSE ABORTED".to_string()) }
}

#[tauri::command]
fn launch_gui(target: String, env_vars: String) -> Result<String, String> {
    let cmd = format!("{} '{}'", env_vars, target);
    Command::new("bash").arg("-c").arg(&cmd).spawn().map_err(|e| e.to_string())?;
    Ok(format!("SUCCESS: SPAWNED {}", target))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_telemetry, get_ac_state, append_ledger, 
            get_gpu_processes, kill_process,
            set_power_limit, set_brightness, 
            browse_file, spawn_nvidia_terminal, launch_gui, launch_cmd, launch_soc_monitor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
