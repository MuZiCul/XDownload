import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// 启动画面淡出移除：React 首次提交完成后给遮罩加隐藏 class，动画结束再删除。
// 不能放内联 <script>（CSP script-src 'self'），必须写在这里。
const splash = document.getElementById("boot-splash");
if (splash) {
  setTimeout(() => {
    splash.classList.add("boot-splash-hide");
    setTimeout(() => splash.remove(), 350);
  }, 0);
}
