import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { SettingsApp } from "./settings/SettingsApp";
import { Widget } from "./widget/Widget";

// 两个窗口共用一个前端产物，按 hash 路由区分（见 tauri.conf.json 的窗口 url）
const isWidget = window.location.hash.startsWith("#/widget");

// 屏蔽 WebView 默认右键菜单（触屏长按同样触发，菜单含「检查」会打开调试工具）。
// 设置窗口的输入框保留右键菜单，方便粘贴授权码/代理地址。
document.addEventListener("contextmenu", (e) => {
  const editable = (e.target as HTMLElement | null)?.closest(
    "input, textarea, [contenteditable='true']",
  );
  if (isWidget || !editable) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isWidget ? <Widget /> : <SettingsApp />}</StrictMode>,
);
