/**
 * dsh-desktop 外壳页逻辑：
 *  - 轮询 boot_status，就绪后在 iframe 中加载 dsh Web GUI
 *  - 自定义标题栏窗口控制（关闭 / 最小化 / 最大化 / 双击标题栏 / 快捷键）
 */
const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

let appWindow;

/** 展示启动失败信息，并给出可读提示。 */
function showError(message) {
  document.getElementById("spinner").classList.add("hidden");
  const errorEl = document.getElementById("error");
  errorEl.textContent = message;
  errorEl.style.display = "block";
  document.getElementById("status").textContent = "启动失败";
  const hintEl = document.getElementById("hint");
  if (!hintEl) {
    const p = document.createElement("p");
    p.className = "hint";
    p.id = "hint";
    p.textContent = "提示：如反复失败，可在终端运行 dsh web 查看具体报错。";
    errorEl.after(p);
  }
}

/**
 * 轮询后端启动状态。
 *  - ready  → iframe 加载 GUI
 *  - failed → 展示错误
 *  - 其它   → 继续轮询
 */
async function poll() {
  try {
    const res = await invoke("boot_status");
    if (res.status === "ready") {
      document.getElementById("status").textContent = "服务已就绪，正在打开…";
      const guiEl = document.getElementById("gui");
      guiEl.src = res.url;
      guiEl.classList.add("visible");
      document.getElementById("loader").classList.add("hidden");
      return;
    }
    if (res.status === "failed") {
      showError(res.message || "未知错误");
      return;
    }
  } catch (e) {
    showError("无法连接本地进程：" + e);
    return;
  }
  setTimeout(poll, 300);
}

/** 绑定自定义标题栏的窗口控制。 */
function wireWindowControls() {
  appWindow = getCurrentWindow();
  document
    .getElementById("btn-close")
    .addEventListener("click", () => appWindow.close());
  document
    .getElementById("btn-min")
    .addEventListener("click", () => appWindow.minimize());
  document
    .getElementById("btn-max")
    .addEventListener("click", () => appWindow.toggleMaximize());

  // 双击标题栏空白处切换最大化（macOS 习惯）
  document.querySelector(".topbar").addEventListener("dblclick", (e) => {
    if (e.target.closest(".traffic-lights")) return;
    appWindow.toggleMaximize();
  });

  // 窗口失焦时按 macOS 习惯弱化三灯
  appWindow.onFocusChanged(({ payload: focused }) => {
    document.body.classList.toggle("window-unfocused", !focused);
  });

  // 常用窗口快捷键（无原生标题栏时保留手感）
  window.addEventListener("keydown", (e) => {
    if (!(e.metaKey || e.ctrlKey)) return;
    const key = e.key.toLowerCase();
    if (key === "w") {
      e.preventDefault();
      appWindow.close();
    } else if (key === "m") {
      e.preventDefault();
      appWindow.minimize();
    }
  });
}

window.addEventListener("DOMContentLoaded", () => {
  wireWindowControls();
  poll();
});
