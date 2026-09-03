// Claude Usage — 창 쪽 코드. 코어(Rust)가 보내는 sys:update / usage:update 두 이벤트와
// ui:settings(네이티브 메뉴 결과)만 받아 그린다. 토큰·파일·OS API는 만지지 않는다 (ARCHITECTURE §3).
"use strict";
const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const N = 40;                                   // 링 버퍼 표본 수 (README: 40개, 1초 간격)
const LEN = { small: Math.PI * 40, big: Math.PI * 62, ring: 2 * Math.PI * 22 }; // 125.7 / 194.8 / 138.2
const state = {
  usage: null,                                  // usage:update 그대로
  sys: null,                                    // sys:update 그대로
  hist: { cpu: [], mem: [], gpu: [] },
  ui: { layout: "2a", theme: "system", alarm: "pulse", demo: false },
};

// ---------- 파생값 (렌더 시 계산, 상태로 두지 않음) ----------
const clamp = (v) => Math.max(0, Math.min(100, v));
const level = (p) => (p == null ? "none" : p >= 90 ? "crit" : p >= 75 ? "warn" : "ok");
const levelColor = (p) => ({ crit: "var(--crit)", warn: "var(--warn)" }[level(p)] || "var(--line1)");
const pad2 = (n) => String(n).padStart(2, "0");

function minutesLeft(iso) { return iso ? Math.max(0, Math.round((new Date(iso) - Date.now()) / 60000)) : null; }
function resetLong(iso) { const m = minutesLeft(iso); return m == null ? "" : `${Math.floor(m / 60)}시간 ${m % 60}분 후 초기화`; }
function resetShort(iso) { const m = minutesLeft(iso); if (m == null) return ""; const h = Math.floor(m / 60); return h ? `${h}시간 ${m % 60}분` : `${m}분`; }
function daysLeft(iso) { const m = minutesLeft(iso); return m == null ? null : Math.max(1, Math.round(m / 1440)); }

const STATUS_MSG = {
  no_source: "설정에서 연결 방법을 고르세요",
  no_token: "Claude Code 로그인 필요",
  auth_expired: "로그인 갱신 필요",
  rate_limited: "5분 뒤 재시도",
  shape_changed: "조회 불가 — 앱 업데이트 필요",
};
function ageMsg(u) { const m = Math.max(1, Math.round((Date.now() - new Date(u.fetched_at)) / 60000)); return `${m}분 전 값`; }

function derive() {
  const u = state.usage, s = state.sys;
  const hasVals = u && u.five_hour && ["ok", "stale", "unreachable", "rate_limited", "auth_expired"].includes(u.status);
  const five = hasVals ? Math.round(clamp(u.five_hour.used_pct)) : null;
  const week = hasVals && u.seven_day ? Math.round(clamp(u.seven_day.used_pct)) : null;
  const opus = hasVals && u.seven_day_opus ? Math.round(clamp(u.seven_day_opus.used_pct)) : null;
  let rootState = u && u.status === "ok" ? level(five) : hasVals ? "stale" : "none";
  if (rootState === "none") rootState = "ok";
  let reset = "", resetS = "";
  if (!u || u.status === "no_source") reset = resetS = STATUS_MSG.no_source;
  else if (u.status === "ok") { reset = resetLong(u.five_hour.resets_at); resetS = resetShort(u.five_hour.resets_at); }
  else if (u.status === "stale" || u.status === "unreachable") reset = resetS = ageMsg(u);
  else reset = resetS = STATUS_MSG[u.status] || u.status;
  const wd = week != null && u.seven_day ? daysLeft(u.seven_day.resets_at) : null;
  const memPct = s ? Math.round((s.mem.used_gb / s.mem.total_gb) * 100) : null;
  const now = new Date();
  return {
    five, week, opus, rootState,
    fiveT: five == null ? "–" : five, weekT: week == null ? "–" : week, opusT: opus == null ? "–" : opus,
    reset, resetShort: resetS,
    weekReset: wd == null ? "" : `${wd}일 후 초기화`, weekResetShort: wd == null ? "" : `${wd}일`,
    weekOpus: `${week == null ? "–" : week + "%"} / ${opus == null ? "–" : opus + "%"}`,
    source: (u && u.source ? u.source : "").toUpperCase() || "—",
    clock: `${pad2(now.getHours())}:${pad2(now.getMinutes())}`,
    cpu: s ? Math.round(s.cpu_pct) : null, mem: memPct,
    memGb: s ? `${s.mem.used_gb.toFixed(1)}GB` : "–",
    gpu: s && s.gpu ? Math.round(s.gpu.util_pct) : null,
    vram: s && s.gpu && s.gpu.mem_used_gb != null ? `${s.gpu.mem_used_gb.toFixed(1)}GB` : "",
    gpuName: s && s.gpu ? s.gpu.name : "",
    hasGpu: !!(s && s.gpu), hasOpus: opus != null,
  };
}

// ---------- 스파크라인 점열 ----------
function pts(arr, W, H, padY = 1.6) {
  if (!arr.length) return "";
  const step = W / (N - 1), off = N - arr.length;
  return arr.map((v, i) => `${((off + i) * step).toFixed(1)},${(padY + (1 - clamp(v) / 100) * (H - 2 * padY)).toFixed(2)}`).join(" ");
}
function area(arr, W, H, padY) { if (!arr.length) return ""; const off = N - arr.length; const x0 = (off * (W / (N - 1))).toFixed(1); return `${pts(arr, W, H, padY)} ${W},${H} ${x0},${H}`; }

// ---------- 공통 조각 ----------
const corners = '<i class="corner tl"></i><i class="corner tr"></i><i class="corner bl"></i><i class="corner br"></i>';
const gaugeS = (key, cls = "") => `<svg class="gauge" viewBox="0 0 96 56" style="width:96px;height:56px"><path class="trk" d="M8 50 A40 40 0 0 1 88 50" fill="none" stroke-width="8"/><path class="arc ${cls}" d="M8 50 A40 40 0 0 1 88 50" fill="none" stroke-width="8" data-arc="${key}:small"/></svg>`;
const spark = (key, W, H, withArea) => `<svg class="spark" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none">${withArea ? `<polygon class="area" data-area="${key}:${W}:${H}"/>` : ""}<polyline class="line" data-pts="${key}:${W}:${H}"/></svg>`;
const sysRow = (key, label, valKey, W, H, withArea, cls = "r") =>
  `<div class="${cls}" ${key === "gpu" ? "data-gpu" : ""}><span class="lbl row">${label}</span>${spark(key, W, H, withArea)}` +
  `<span class="val" data-t="${valKey}"${valKey === "memGb" ? "" : ' data-suffix="%"'}></span></div>`;
const num = (cls = "", extra = "") => `<span class="num five-num ${cls}" data-t="fiveT"></span><span class="num pct ${extra}">%</span>`;

// ---------- 레이아웃 7종 (README 표 · 시안 인라인 값) ----------
const LAYOUTS = {
  "2a": () => `
    <div class="c1 drag" data-tauri-drag-region>
      <div class="head" data-tauri-drag-region><span class="lbl" data-tauri-drag-region>5H LIMIT</span><span class="badge"><i></i><span data-t="source"></span></span></div>
      <div class="g" data-tauri-drag-region>${gaugeS("five", "st")}</div>
      <div class="ov" data-tauri-drag-region>${num()}</div>
      <div class="note rs" data-t="reset"></div>
    </div>
    <div class="c2">
      <div class="kv"><span>주간</span><b data-t="weekT" data-suffix="%"></b></div>
      <div class="bar" style="height:6px"><i data-w="week" data-lc="week"></i></div>
      <div class="kv" data-opus><span>Opus</span><b data-t="opusT" data-suffix="%"></b></div>
      <div class="bar" style="height:6px" data-opus><i data-w="opus" style="background:var(--line2)"></i></div>
      <div class="wr" data-t="weekReset"></div>
    </div>
    <div class="c3">
      ${sysRow("cpu", "CPU", "cpu", 120, 20, true)}${sysRow("mem", "MEM", "memGb", 120, 20, true)}${sysRow("gpu", "GPU", "gpu", 120, 20, true)}
    </div>`,

  "2b": () => `
    <div class="head drag" data-tauri-drag-region><span class="lbl" data-tauri-drag-region>CLAUDE LIMITS · SYSTEM</span><span class="clk"><span data-t="clock"></span> · <span data-t="source"></span></span></div>
    <div class="body">
      <div class="gc">${gaugeS("five", "st")}<div class="num five-num" style="font-size:26px"><span data-t="fiveT"></span><span class="pct">%</span></div><div class="lbl s">5H</div><div class="note" data-t="resetShort"></div></div>
      <div class="gc">${gaugeS("week")}<div class="num" style="font-size:26px"><span data-t="weekT"></span><span class="pct">%</span></div><div class="lbl s">WEEK</div><div class="note" data-t="weekResetShort"></div></div>
      <div class="gc" data-opus>${gaugeS("opus").replace('class="arc "', 'class="arc" style="stroke:var(--line2)"')}<div class="num" style="font-size:26px"><span data-t="opusT"></span><span class="pct">%</span></div><div class="lbl s">OPUS</div><div class="note" data-t="weekResetShort"></div></div>
      <div class="sys">${sysRow("cpu", "CPU", "cpu", 120, 18, false)}${sysRow("mem", "MEM", "mem", 120, 18, false)}${sysRow("gpu", "GPU", "gpu", 120, 18, false)}</div>
    </div>`,

  "2c": () => `
    <div class="row5 drag" data-tauri-drag-region>
      <div class="big" data-tauri-drag-region>${num("acc")}</div>
      <div class="meta" data-tauri-drag-region><div class="lbl s">5H LIMIT</div><div class="note" data-t="resetShort" data-suffix=" 남음"></div><div class="note d">주간 <span data-t="weekT"></span>%</div></div>
      <div class="sc"><div class="h"><span class="lbl s">CPU</span><span class="val" data-t="cpu" data-suffix="%"></span></div>${spark("cpu", 120, 20, false)}</div>
      <div class="sc"><div class="h"><span class="lbl s">MEM</span><span class="val" data-t="mem" data-suffix="%"></span></div>${spark("mem", 120, 20, false)}</div>
      <div class="sc" data-gpu><div class="h"><span class="lbl s">GPU</span><span class="val" data-t="gpu" data-suffix="%"></span></div>${spark("gpu", 120, 20, false)}</div>
    </div>
    <div class="bar foot"><i class="st" data-w="five"></i></div>`,

  "2d": () => `
    <div class="c1 drag" data-tauri-drag-region>
      <div class="lbl" data-tauri-drag-region>5H LIMIT</div>
      <div class="big" data-tauri-drag-region>${num("acc")}</div>
      <div class="bar b6"><i class="st" data-w="five"></i></div>
      <div class="note rs" data-t="reset"></div>
      <div class="kv"><span>주간 / Opus</span><b data-t="weekOpus"></b></div>
      <div class="bar b4"><i data-w="week" data-lc="week"></i></div>
    </div>
    <div class="c2">
      <div class="head"><span class="lbl">SYSTEM · 40초</span><span class="leg">
        <span><i style="background:var(--line1)"></i>CPU <b data-t="cpu" data-suffix="%"></b></span>
        <span data-gpu><i style="background:var(--line3)"></i>GPU <b data-t="gpu" data-suffix="%"></b></span>
        <span>MEM <b data-t="memGb"></b></span></span></div>
      <svg class="hist" viewBox="0 0 240 54" preserveAspectRatio="none">
        <polygon class="gA" data-area="gpu:240:54" data-gpu/><polyline class="gL" data-pts="gpu:240:54" data-gpu/>
        <polygon class="cA" data-area="cpu:240:54"/><polyline class="cL" data-pts="cpu:240:54"/>
        <polyline class="mL" data-pts="mem:240:54"/>
      </svg>
    </div>`,

  "2e": () => `
    <div class="head drag" data-tauri-drag-region><span class="lbl" data-tauri-drag-region>PANEL · 0–100%</span><span class="clk"><span data-t="resetShort"></span> → 5H RESET</span></div>
    <div class="cols">
      <div class="col"><span class="num five-num acc" style="font-size:17px" data-t="fiveT"></span><div class="vbar"><i class="st" data-hh="five"></i></div><span class="lbl s">5H</span></div>
      <div class="col"><span class="num" style="font-size:17px" data-t="weekT"></span><div class="vbar"><i data-hh="week" data-lc="week"></i></div><span class="lbl s">WEEK</span></div>
      <div class="col"><span class="num" style="font-size:17px" data-t="cpu"></span><div class="vbar"><i data-hh="cpu"></i></div><span class="lbl s">CPU</span></div>
      <div class="col"><span class="num" style="font-size:17px" data-t="mem"></span><div class="vbar"><i data-hh="mem"></i></div><span class="lbl s">MEM</span></div>
      <div class="col" data-gpu><span class="num" style="font-size:17px" data-t="gpu"></span><div class="vbar"><i data-hh="gpu"></i></div><span class="lbl s">GPU</span></div>
    </div>`,

  "2f": () => `
    <div class="c1 drag" data-tauri-drag-region>
      <div class="head" data-tauri-drag-region><span class="lbl" data-tauri-drag-region>5H LIMIT</span><span class="note" data-t="reset"></span></div>
      <div class="g" data-tauri-drag-region><span class="num five-num acc" style="font-size:40px"><span data-t="fiveT"></span><span class="pct">%</span></span><div class="dots"><i data-w="five"></i></div></div>
      <div class="foot"><span>주간 <b data-t="weekT" data-suffix="%"></b></span><span data-opus>Opus <b data-t="opusT" data-suffix="%"></b></span><span><span data-t="clock"></span> · <span data-t="source"></span></span></div>
    </div>
    <div class="c2">
      <div class="r"><span class="lbl row">CPU</span><div class="bar"><i data-w="cpu"></i></div><span class="val" data-t="cpu" data-suffix="%"></span></div>
      <div class="r"><span class="lbl row">MEM</span><div class="bar"><i data-w="mem"></i></div><span class="val" data-t="mem" data-suffix="%"></span></div>
      <div class="r" data-gpu><span class="lbl row">GPU</span><div class="bar"><i data-w="gpu"></i></div><span class="val" data-t="gpu" data-suffix="%"></span></div>
      <div class="gn" data-gpu><span data-t="gpuName"></span> · <span data-t="vram"></span></div>
    </div>`,

  "2g": () => `
    <div class="c1 drag" data-tauri-drag-region>
      <div class="head" data-tauri-drag-region><span class="lbl" data-tauri-drag-region>5H LIMIT</span><span class="badge"><i></i><span data-t="source"></span></span></div>
      <div class="gw" data-tauri-drag-region>
        <svg class="gauge" viewBox="0 0 148 78"><path class="trk" d="M12 72 A62 62 0 0 1 136 72" fill="none" stroke-width="9"/><path class="arc st" d="M12 72 A62 62 0 0 1 136 72" fill="none" stroke-width="9" data-arc="five:big"/></svg>
        <div class="ov">${num()}</div>
      </div>
      <div class="kv"><span><span data-t="resetShort"></span> 남음</span><span>주간 <span data-t="weekT"></span>%</span></div>
    </div>
    <div class="rings">
      ${ring("cpu", "CPU %", "cpu", "var(--line1)")}${ring("mem", "", "memGb", "var(--line2)")}${ring("gpu", "GPU %", "gpu", "var(--line3)")}
    </div>`,
};
function ring(key, label, labelKey, color) {
  return `<div class="ring" ${key === "gpu" ? "data-gpu" : ""}><div class="rw"><svg viewBox="0 0 52 52"><circle class="trk" cx="26" cy="26" r="22" fill="none" stroke-width="5"/><circle class="arc" cx="26" cy="26" r="22" fill="none" stroke-width="5" style="stroke:${color}" data-arc="${key}:ring" transform="rotate(-90 26 26)"/></svg><div class="cv" data-t="${key}"></div></div><span class="lbl s">${label || `<span data-t="${labelKey}"></span>`}</span></div>`;
}

// ---------- 렌더 ----------
const root = document.getElementById("root");
let mounted = null;
function mount() {
  const id = state.ui.layout in LAYOUTS ? state.ui.layout : "2a";
  root.innerHTML = `<div class="panel l${id}">${corners}${LAYOUTS[id]()}</div>`;
  mounted = id;
}
function applyTheme() {
  const t = state.ui.theme === "system" ? (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : state.ui.theme;
  document.documentElement.dataset.theme = t;
  document.documentElement.dataset.pulse = state.ui.alarm === "pulse" ? "on" : "off";
}
function render() {
  if (mounted !== state.ui.layout) mount();
  const d = derive();
  const panel = root.firstElementChild;
  document.documentElement.dataset.state = d.rootState;
  panel.classList.toggle("no-gpu", !d.hasGpu);
  panel.classList.toggle("no-opus", !d.hasOpus);
  panel.querySelectorAll("[data-gpu]").forEach((el) => (el.hidden = !d.hasGpu));
  panel.querySelectorAll("[data-opus]").forEach((el) => (el.hidden = !d.hasOpus));
  panel.querySelectorAll("[data-t]").forEach((el) => {
    const v = d[el.dataset.t];
    el.textContent = v == null ? "–" : v + (v === "–" ? "" : el.dataset.suffix || "");
  });
  const pct = (k) => (d[k] == null ? 0 : d[k]);
  panel.querySelectorAll("[data-w]").forEach((el) => (el.style.width = pct(el.dataset.w) + "%"));
  panel.querySelectorAll("[data-hh]").forEach((el) => (el.style.height = pct(el.dataset.hh) + "%"));
  panel.querySelectorAll("[data-lc]").forEach((el) => (el.style.background = levelColor(d[el.dataset.lc])));
  panel.querySelectorAll("[data-arc]").forEach((el) => {
    const [k, kind] = el.dataset.arc.split(":"), L = LEN[kind];
    el.setAttribute("stroke-dasharray", `${((L * pct(k)) / 100).toFixed(2)} ${L.toFixed(2)}`);
  });
  panel.querySelectorAll("[data-pts]").forEach((el) => { const [k, W, H] = el.dataset.pts.split(":"); el.setAttribute("points", pts(state.hist[k], +W, +H)); });
  panel.querySelectorAll("[data-area]").forEach((el) => { const [k, W, H] = el.dataset.area.split(":"); el.setAttribute("points", area(state.hist[k], +W, +H)); });
}

// ---------- 이벤트 ----------
function push(k, v) { const a = state.hist[k]; a.push(v == null ? 0 : v); if (a.length > N) a.shift(); }
listen("sys:update", ({ payload: s }) => {
  state.sys = s;
  push("cpu", s.cpu_pct); push("mem", (s.mem.used_gb / s.mem.total_gb) * 100); push("gpu", s.gpu ? s.gpu.util_pct : 0);
  if (state.ui.demo) demoTick();
  render();
});
listen("usage:update", ({ payload: u }) => { if (!state.ui.demo) { state.usage = u; render(); } });
listen("ui:settings", ({ payload: ui }) => { Object.assign(state.ui, ui); applyTheme(); if (state.ui.demo) demoTick(); else if (state.usage && state.usage.source === "demo") state.usage = null; render(); });
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => { applyTheme(); render(); });

// 예시 값 (검증용, 메뉴에서 켬): README의 74 / 47 / 12 — 임계 색을 보려면 5H를 올린다
let demoFive = 74, demoDir = 1;
function demoTick() {
  demoFive += 0.5 * demoDir; if (demoFive >= 96 || demoFive <= 60) demoDir *= -1;
  const t = Date.now();
  state.usage = { source: "demo", status: "ok", fetched_at: new Date(t).toISOString(),
    five_hour: { used_pct: demoFive, resets_at: new Date(t + 45 * 60000).toISOString() },
    seven_day: { used_pct: 47, resets_at: new Date(t + 4 * 86400000).toISOString() },
    seven_day_opus: { used_pct: 12, resets_at: new Date(t + 4 * 86400000).toISOString() } };
}

// 우클릭 → 네이티브 메뉴 (Rust가 띄움)
document.addEventListener("contextmenu", (e) => { e.preventDefault(); invoke("show_menu"); });

invoke("get_settings").then((ui) => { Object.assign(state.ui, ui); applyTheme(); if (state.ui.demo) demoTick(); render(); });
