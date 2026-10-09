import { createApp } from "vue";

// VS Code 官方控件：@vscode-elements/elements（Lit 写的 Web Components，框架无关）。
// bundled 会把 38 个组件一次性注册成自定义元素。
import "@vscode-elements/elements/dist/bundled.js";
// VS Code 官方图标字体在 index.html 里以 <link id="vscode-codicon-stylesheet"> 引入 ——
// 这是 <vscode-icon> 约定的固定 id，删了图标就变豆腐块。
// VS Code 官方 Dark Modern 主题色（由 scripts/gen-vscode-theme.py 生成）。
// 真实 webview 里这些 --vscode-* 变量由宿主注入，桌面版没有宿主，所以要把主题搬进来。
import "./styles/vscode-theme.css";
import "./styles/tokens.css";
import "./styles/base.css";
// JetBrains 皮肤：只在 <html data-skin="jb"> 时生效，把 --vscode-* 换一套值。
import "./styles/jb-skin.css";
import "./styles/jb-tokens.css";

import App from "./App.vue";

createApp(App).mount("#app");
