// Global Invoke Bridge for Vanilla JS
const invoke = window.__TAURI__.core ? window.__TAURI__.core.invoke : window.__TAURI__.tauri.invoke;

// --- UI ELEMENTS ---
const consoleEl = document.getElementById("console");
const powerSlider = document.getElementById("power-slider");
const powerStatus = document.getElementById("power-status");
const brightSlider = document.getElementById("bright-slider");
const brightStatus = document.getElementById("bright-status");
const appCombo = document.getElementById("app-combo");

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

// --- BUTTONS ---
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
  const payload = document.getElementById("cmd-entry").value.strip ? document.getElementById("cmd-entry").value.strip() : document.getElementById("cmd-entry").value;
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

// --- SLIDERS ---
powerSlider.addEventListener("input", (e) => {
  const mode = powerModes[e.target.value];
  powerStatus.innerText = `[ ${mode.name} ] // ${mode.watts}W`;
  powerStatus.className = `status-large ${mode.class}`;
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

// --- TELEMETRY ---
async function updateTelemetry() {
  try {
    const data = await invoke("get_telemetry");
    document.getElementById("nv-pct").innerText = `${data.nvidia.usage}%`;
    document.getElementById("nv-bar").style.width = `${data.nvidia.usage}%`;
    document.getElementById("nv-stats").innerText = `${data.nvidia.temp}°C | ${data.nvidia.power.toFixed(1)}W`;
    document.getElementById("amd-pct").innerText = `${data.amd.usage}%`;
    document.getElementById("amd-bar").style.width = `${data.amd.usage}%`;
  } catch (err) { console.error(err); }
}

log("SYS.CORE INITIATED");
setInterval(updateTelemetry, 1000);