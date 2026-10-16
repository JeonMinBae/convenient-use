import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import SettingsApp from "./settings/SettingsApp";
import ClipboardPopup from "./popup/ClipboardPopup";
import "./styles.css";

// 한 번들을 두 창이 공유하므로 창 label 로 화면을 고른다
const Root = getCurrentWindow().label === "clipboard-popup" ? ClipboardPopup : SettingsApp;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
