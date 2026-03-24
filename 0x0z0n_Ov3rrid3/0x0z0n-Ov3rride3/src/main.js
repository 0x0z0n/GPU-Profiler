// Global Invoke Bridge for Vanilla JS
const invoke = window.__TAURI__.core ? window.__TAURI__.core.invoke : window.__TAURI__.tauri.invoke;

// --- UI ELEMENTS ---
const consoleEl = document.getElementById("console");
const powerSlider = document.getElementById("power-slider");
const powerStatus = document.getElementById("power-status");
const brightSlider = document.getElementById("bright-slider");
const brightStatus = document.getElementById("bright-status");
const appCombo = document.getElementById("app-combo");
const processList = document.getElementById("process-list");
const acWarning = document.getElementById("ac-warning");

const powerModes = [
  { name: "ECO_STEALTH", watts: 45, class: "text-green" },
  { name: "NOMINAL_YIELD", watts: 70, class: "text-cyan" },
  { name: "KINETIC_BURST", watts: 95, class: "text-orange" },
  { name: "OVERRIDE_MAX", watts: 115, class: "text-crimson" }
];

// --- LOGGING ---
function log(msg, isError = false) {
  const color = isError ? "var(--crimson-neon)" : "var(--success-green)";
  consoleEl.innerHTML += `<div style="color: ${color}; margin-bottom: 4px;">> ${msg}</div>`;
  consoleEl.scrollTop = consoleEl.scrollHeight;
}

// --- ENV BUILDER ---
function getEnvPrefix() {
  const useNvidia = document.querySelector('input[name="gpu"]:checked').value === "nvidia";
  let envVars = useNvidia ? "__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia __VK_LAYER_NV_optimus=NVIDIA_only " : "";
  if (document.getElementById("chk-gamemode").checked) envVars += "gamemoderun ";
  if (document.getElementById("chk-mangohud").checked) envVars += "mangohud ";
  return envVars;
}

// --- [MODIFIED] 3-TIER WEAPONIZED PROFILES ---
document.getElementById("btn-profile-stealth").addEventListener("click", async () => {
  log("EXECUTING PROFILE: STEALTH");
  powerSlider.value = 0; powerSlider.dispatchEvent(new Event('input')); // ECO_STEALTH (45W)
  brightSlider.value = 3; brightSlider.dispatchEvent(new Event('change'));
  document.querySelector('input[value="integrated"]').checked = true;
  try { await invoke("set_power_limit", { watts: 45 }); } catch (err) { log(err, true); }
});

document.getElementById("btn-profile-light").addEventListener("click", async () => {
  log("EXECUTING PROFILE: LIGHT");
  powerSlider.value = 1; powerSlider.dispatchEvent(new Event('input')); // NOMINAL (70W)
  brightSlider.value = 6; brightSlider.dispatchEvent(new Event('change'));
  document.querySelector('input[value="nvidia"]').checked = true;
  try { await invoke("set_power_limit", { watts: 70 }); } catch (err) { log(err, true); }
});

document.getElementById("btn-profile-boost").addEventListener("click", async () => {
  log("EXECUTING PROFILE: BOOST");
  powerSlider.value = 3; powerSlider.dispatchEvent(new Event('input')); // OVERRIDE (115W)
  brightSlider.value = 10; brightSlider.dispatchEvent(new Event('change'));
  document.querySelector('input[value="nvidia"]').checked = true;
  try {
    await invoke("set_power_limit", { watts: 115 });
    await invoke("spawn_nvidia_terminal");
  } catch (err) { log(err, true); }
});

// --- EXECUTION BUTTONS ---
document.getElementById("btn-browse").addEventListener("click", async () => {
  try {
    const path = await invoke("browse_file");
    appCombo.innerHTML = `<option value="${path}">${path}</option>`;
    log(`TARGET LOCKED: ${path}`);
  } catch (err) { log(err, true); }
});

document.getElementById("btn-launch-gui").addEventListener("click", async () => {
  const target = appCombo.value;
  if (!target || target === "None Selected") return log("ERR: NO BINARY SELECTED", true);
  try {
    const res = await invoke("launch_gui", { target, envVars: getEnvPrefix() });
    log(res);
  } catch (err) { log(`FATAL: ${err}`, true); }
});

document.getElementById("btn-launch-cmd").addEventListener("click", async () => {
  const payloadStr = document.getElementById("cmd-entry").value;
  const payload = payloadStr.trim ? payloadStr.trim() : payloadStr;
  if (!payload) return log("ERR: NO PAYLOAD PROVIDED", true);
  try {
    const res = await invoke("launch_cmd", { payload, envVars: getEnvPrefix() });
    log(res);
  } catch (err) { log(`FATAL: ${err}`, true); }
});

document.getElementById("btn-spawn-shell").addEventListener("click", async () => {
  try {
    log("INITIATING PERMANENT NVIDIA SHELL...");
    const res = await invoke("spawn_nvidia_terminal");
    log(res);
  } catch (err) { log(err, true); }
});

// --- [MODIFIED] SOC MONITOR ---
document.getElementById("btn-soc-monitor").addEventListener("click", async () => {
  try {
    log("LAUNCHING EXTERNAL MONITOR TOOL...");
    const res = await invoke("launch_soc_monitor");
    log(res);
  } catch (err) { log(err, true); }
});

// --- SLIDERS ---
powerSlider.addEventListener("input", (e) => {
  const mode = powerModes[e.target.value];
  powerStatus.innerText = `[ ${mode.name} ] // ${mode.watts}W`;
  powerStatus.className = `status-large font-mono bold ${mode.class}`;
});

document.getElementById("btn-apply-power").addEventListener("click", async () => {
  const watts = powerModes[powerSlider.value].watts;
  log(`INJECTING HARDWARE LIMIT -> ${watts}W`);
  try {
    const res = await invoke("set_power_limit", { watts });
    log(res);
  } catch (err) { log(err, true); }
});

brightSlider.addEventListener("change", async (e) => {
  const level = parseInt(e.target.value) / 10.0;
  brightStatus.innerText = `[ BRIGHTNESS ] // ${e.target.value * 10}%`;
  try { await invoke("set_brightness", { level }); } 
  catch (err) { log(err, true); }
});

// --- [NEW] PROCESS TERMINATOR ---
window.killPid = async function(pid) {
  try { 
    log(`ATTEMPTING TO KILL PID: ${pid}...`);
    const res = await invoke("kill_process", { pid: String(pid) });
    log(res);
  } catch (err) { log(err, true); }
};

// --- [MODIFIED] TELEMETRY LOOP & CORTEX EDR ---
async function updateTelemetry() {
  try {
    // 1. AC State Warning
    const isPlugged = await invoke("get_ac_state");
    acWarning.style.display = isPlugged ? "none" : "block";

    // 2. Hardware Stats
    const data = await invoke("get_telemetry");
    document.getElementById("nv-pct").innerText = `${data.nvidia.usage}%`;
    document.getElementById("nv-bar").style.width = `${data.nvidia.usage}%`;
    document.getElementById("nv-stats").innerText = `${data.nvidia.temp}°C | ${data.nvidia.power.toFixed(1)}W | ${data.nvidia.vram_used}MB`;
    document.getElementById("amd-pct").innerText = `${data.amd.usage}%`;
    document.getElementById("amd-bar").style.width = `${data.amd.usage}%`;

    // 3. Ledger Recording
    if (document.getElementById("chk-ledger").checked) {
      const timestamp = new Date().toISOString();
      const csvLine = `${timestamp},${data.nvidia.temp}C,${data.nvidia.power}W,${data.nvidia.usage}%,${data.nvidia.vram_used}MB`;
      await invoke("append_ledger", { line: csvLine });
    }

    // 4. CORTEX EDR PROCESS RENDERER (Dual-Line Kernel Extraction)
    const procs = await invoke("get_gpu_processes");
    if (procs.length === 0) {
      processList.innerHTML = `<div class="muted small-text font-mono" style="text-align:center; padding-top: 35px; letter-spacing: 1px;">[ CORTEX ] ZERO ACTIVE PAYLOADS DETECTED.</div>`;
    } else {
      let html = `
        <div style="display: flex; justify-content: space-between; font-size: 9px; color: var(--muted-text); font-family: var(--font-mono); padding-bottom: 6px; border-bottom: 1px dashed var(--border-color); margin-bottom: 6px; text-transform: uppercase; letter-spacing: 1px;">
          <span style="width: 50px;">PID</span>
          <span class="flex-grow">PAYLOAD_SIGNATURE / EXECUTION_PATH</span>
          <span style="width: 70px; text-align: right; margin-right: 15px;">FOOTPRINT</span>
          <span style="width: 40px; text-align: center;">ACTION</span>
        </div>
      `;
      
      html += procs.map(p => {
        let memVal = parseInt(p.memory);
        // THREAT LOGIC: > 2GB = Red, > 500MB = Orange, Normal = Cyan
        let threatColor = memVal > 2000 ? 'var(--crimson-neon)' : (memVal > 500 ? 'var(--warning-orange)' : 'var(--cyan-neon)');
        
        return `
        <div style="display: flex; justify-content: space-between; align-items: center; padding: 6px 0; border-bottom: 1px solid rgba(255,255,255,0.03); font-size: 11px;">
          <span class="font-mono bold" style="width: 50px; color: ${threatColor};">${p.pid}</span>
          
          <div class="flex-grow" style="display: flex; flex-direction: column; overflow: hidden; margin-right: 10px;">
            <span class="text-main font-mono bold" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${p.name}</span>
            <span class="muted font-mono" style="font-size: 9px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px;" title="${p.full_cmd}">${p.full_cmd}</span>
          </div>

          <span class="font-mono bold" style="width: 70px; text-align: right; margin-right: 15px; color: ${threatColor};">${p.memory} MiB</span>
          <button class="btn-kill" style="width: 40px; border: 1px solid ${threatColor}; color: ${threatColor};" onmouseover="this.style.background='${threatColor}'; this.style.color='#000';" onmouseout="this.style.background='transparent'; this.style.color='${threatColor}';" onclick="killPid(${p.pid})">KILL</button>
        </div>
      `}).join('');
      processList.innerHTML = html;
    }
  } catch (err) { 
    processList.innerHTML = `<div class="text-crimson bold font-mono" style="text-align:center; padding-top: 10px;">[!] BACKEND DESYNC: ${err}</div>`;
  }
}

// --- [NEW] GLOBAL HOTKEYS ---
window.addEventListener('keydown', async (e) => {
  if (e.ctrlKey && e.shiftKey && e.code === 'KeyO') {
    log("HOTKEY DETECTED: Spawning Persistent Shell...");
    try { await invoke("spawn_nvidia_terminal"); } catch(err) { log(err, true); }
  }
  if (e.ctrlKey && e.shiftKey && e.code === 'KeyP') {
    log("HOTKEY DETECTED: Toggling Hardware Limit...");
    powerSlider.value = powerSlider.value == 0 ? 3 : 0;
    powerSlider.dispatchEvent(new Event('input'));
    document.getElementById("btn-apply-power").click();
  }
});

log("SYS.CORE INITIATED // TACTICAL MODULES ACTIVE");
setInterval(updateTelemetry, 1000);