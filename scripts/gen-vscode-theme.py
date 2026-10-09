
import json, re, urllib.request, os

URL = "https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/dark_modern.json"
raw = urllib.request.urlopen(URL, timeout=60).read().decode("utf-8")
raw = re.sub(r"(?m)^\s*//.*$", "", raw)
raw = re.sub(r",(\s*[}\]])", r"\1", raw)
data = json.loads(raw)
colors = data.get("colors", {})

out = [
    "/* 由 VS Code 官方 Dark Modern 主题生成 —— 不要手改，跑 scripts/gen-vscode-theme.py 重新生成。",
    " *",
    " * @vscode-elements/* 的组件全部读 --vscode-* 变量（真实 webview 里由 VS Code 宿主注入）。",
    " * 桌面版没有宿主注入，所以把官方主题的 colors 原样搬过来 ——",
    " * 这是「直接接入 VS Code 原生外观」最正统的做法，不是照着仿。",
    " */",
    ":root {",
]
for k in sorted(colors):
    out.append("  --vscode-%s: %s;" % (k.replace(".", "-"), colors[k]))
out.append("}")
out.append("")

# 只补「主题文件里确实没有」的令牌。**不要**覆盖已有的 ——
# 覆盖会改变组件外观（例如 button-secondaryBackground 官方是 #00000000 全透明，
# 覆盖成灰底之后 secondary 按钮就不是 VS Code 的样子了）。
missing = {
    "widget-shadow": "rgba(0, 0, 0, 0.36)",
    "menu-border": "#454545",
    "menu-selectionBackground": "#0078D4",
    "menu-selectionForeground": "#FFFFFF",
    "menu-separatorBackground": "#454545",
    "scrollbar-shadow": "#000000",
    "textLink-foreground": "#4DAAFC",
    "textLink-activeForeground": "#4DAAFC",
    "editorWidget-background": "#202020",
    "editorWidget-border": "#313131",
    "editorWidget-foreground": "#CCCCCC",
    "toolbar-hoverBackground": "rgba(90, 93, 94, 0.31)",
    "toolbar-activeBackground": "rgba(90, 93, 94, 0.31)",
    "progressBar-background": "#0078D4",
    "editorWarning-foreground": "#CCA700",
    "editorError-foreground": "#F14C4C",
    "testing-iconPassed": "#73C991",
    "editor-selectionBackground": "#264F78",
    "editor-inactiveSelectionBackground": "#3A3D41",
    "editor-lineHighlightBorder": "#282828",
    "list-dropBackground": "#0078D4",
    "activityBar-dropBorder": "#D7D7D7",
    "sideBar-dropBackground": "#0078D4",
    "breadcrumb-foreground": "#CCCCCC",
    "breadcrumb-background": "#1F1F1F",
    "breadcrumb-focusForeground": "#E0E0E0",
    "breadcrumb-activeSelectionForeground": "#E0E0E0",
    "breadcrumbPicker-background": "#212121",
    "icon-foreground": "#CCCCCC",
}
# 注意：比较要用「转换后的变量名」，不能拿原始 key 比 ——
# 官方 key 是 editorWidget.background，补缺表里写的是 editorWidget-background，
# 直接 k in colors 判不出来，会写出重复定义把官方值盖掉。
existing = {k.replace(".", "-") for k in colors}
added = 0
out.append("/* 主题文件里没有、但组件会读的几个，用 VS Code 运行时的默认值补上（不覆盖已有的） */")
out.append(":root {")
for k in sorted(missing):
    if k in existing:
        continue
    added += 1
    out.append("  --vscode-%s: %s;" % (k, missing[k]))
out.append("}")

target = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "ui", "src", "styles", "vscode-theme.css")
open(target, "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
print("theme:", data.get("name"), "| official:", len(colors), "| 补缺:", added, "| bytes:", os.path.getsize(target))
