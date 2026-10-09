# AGENTS.md — SC2Translater 项目约定

> 本文件是本仓库的**项目约定**，面向 AI 编码代理与人类协作者。
> **动手改代码前请先完整阅读本文档。**

---

## 1. 项目标识

| 项 | 值 |
| --- | --- |
| 项目名 | **SC2Translater** |
| 定位 | 《星际争霸 II》地图 / 模组的**汉化工具**（翻译键提取与回填） |
| 许可证 | MIT |
| 语言 | Rust（交付） + Python（调研原型） + TypeScript（GUI 前端） |
| 目标平台 | Windows 优先 |
| CLI 名 | `sc2trans` |
| GUI | Tauri 2.11 + Vue 3.5 + Vite 8 |

---

## 2. 非目标

* 不提供、不分发任何暴雪版权资源（游戏本体、官方地图、语音等）。
* 不做破解、不改游戏授权校验。
* 不自动启动银河编辑器或游戏（验收时人工做一次，见 `docs/design/验收清单.md`）。
* **不按键以外的规则匹配文本**。8 位十六进制键（如 `Param/Value/0F5D0539`）是编辑器分配的
  随机 Id，**跨地图不可复用** —— 同一段文本在不同地图里键不同，按值匹配会张冠李戴。

---

## 3. 技术栈与依赖方向

| 层 | 选型 |
| --- | --- |
| Rust | 1.94.0，edition 2024（`rust-toolchain.toml` 固定版本） |
| 错误处理 | `thiserror`（库层） + `anyhow`（CLI 边界） |
| 序列化 | `serde` / `serde_json` |
| 压缩 | `flate2` / `bzip2` / `lzma-rs`（可关掉 `bzip2-backend` feature） |
| GUI | Tauri 2 + Vue 3 + TypeScript + Vite，pnpm 包管理 |
| GUI 控件 | **VS Code 官方控件**：`@vscode-elements/elements`（MIT，38 个 Web Components）+ `@vscode/codicons`（官方图标字体）+ VS Code Dark Modern 主题令牌 |
| Python 原型 | uv 管理，放在 `python/` 子目录 |

```
依赖方向（不可违反）：
  sc2mpq  <-  sc2trans-core  <-  sc2trans-cli
                            <-  src-tauri  <-  ui
```

* `sc2mpq` 只认字节，不知道「汉化」是什么。
* `sc2trans-core` 不依赖 GUI、不弹窗、不读配置目录（路径由调用方传入）。
* `src-tauri` 只做状态持有与命令转发，业务逻辑一律放 core。

---

## 4. 领域知识：改代码前必须知道的 SC2 事实

详细证据见 `docs/research/sc2-localization.md` 与 `docs/research/mpq-spec.md`。

### 4.1 多语言机制（这是本工具存在的理由）

* 地图的文本是「键 -> 文本」查表，**键由地图自己携带**。
* 归档里 `<locale>.SC2Data` 只有**当前游戏语言**那一份会被加载；`Base.SC2Data` 任何语言都加载。
* **不存在跨语言回退**。地图只带 enUS 而游戏是中文时，引擎查不到键，就把键本身当文本渲染
  —— 界面上于是出现 `Param/Value/0F5D0539` 这种东西。
* 语言代码**不止官方那 12 个**：实测还出现过 `enCN / enGB / enSG / enTW / jaJP / zhCH`。
  任何地方都**不要**把 `LOCALES` 当白名单用（只做形状校验）。

### 4.2 字符串表格式（逐字节保真，别自作聪明）

| 事实 | 约定 |
| --- | --- |
| 编码 | UTF-8；BOM 有无都可能。**随原文件**，新建写 BOM |
| 换行 | CRLF / LF 都可能。**随原文件**，新建用 CRLF |
| 切分 | **只按第一个等号**（值里可以含等号，例如 `<s val=...>` 这种标记） |
| 转义 | **没有**。反斜杠 n 不是换行，换行用 `<n/>`；内联标记整体保留 |
| 注释 | 以 `//` 开头的整行 |
| 三斜杠 | `译文 /// 英文原文` 是**暴雪自己**在用的约定。本工具默认**不新增**它 |
| 重复键 | 真实存在；只改第一条、保序 |

### 4.3 容器（MPQ）

* SC2 写的是 **208 字节头 + formatVersion=3 + v4 的 size64 字段布局**。按 `headerSize` 判字段，别按版本号。
* **`dwRawChunkSize` 必须写 0**。写 16384（暴雪原件的值）会打开 System_Mopaq 的
  raw-chunk MD5 校验，编辑器就会对**任何**内容改动报 `e_fileCorrupt`。
* 头部 0x70..0xCF 有 6 个 MD5 摘要，**0xC0 是自哈希，必须最后算**。缺失与写错同罪。
* **HET 是权威索引**，但 HET 条目只存名字哈希高 8 位，命中后必须用 BET 的 64 位哈希复核，
  且不匹配要**继续探测**（不是直接失败）。
* BET 的 flag 数组在磁盘上**小端**；照抄 StormLib 的 BSWAP 会得到废值。
* **BET unknown08 沿用原件**（实测样本恒为 7888），不要写 StormLib 的 0x10。
* 经典哈希表**必须大于条目数**（至少留一个空槽），否则读取器会判定「表没解密」而退回原始字节解析。

---

## 5. 常用命令

```powershell
# 构建与测试
cargo build --release            # CLI 产物 target/release/sc2trans.exe
cargo test                       # 单元 + 真实归档验收（无样本自动跳过）
cargo clippy --all-targets       # 必须无警告

# GUI
pnpm -C ui install
python scripts/gen-vscode-theme.py   # 重新生成 VS Code 官方主题令牌（只在换主题色板时需要）
pnpm -C ui dev                   # 浏览器调试（自动演示模式）
pnpm -C ui build                 # 生成 ui/dist
cargo build --release            # 桌面壳（custom-protocol 已是默认特性，不用再带 --features）
pnpm -C ui test                  # vitest

# Python 原型（对照实现）
cd python; uv sync; uv run pytest; uv run sc2trans-py audit <map>

# 手工验收
target/release/sc2trans.exe verify <产物.SC2Map>
python ../SC2Diff/tools/editor_accept.py <原件.SC2Map> <产物.SC2Map>   # 必须 LOADED
```

---

## 6. 硬性约束（踩过的坑）

1. **GUI 必须内嵌前端**。custom-protocol 已经是**默认特性**，所以 `cargo build --release` 直接就能用；
   需要连 dev server 调试时显式关掉：`cargo run --no-default-features -p sc2trans-gui`。
   验证方法：在 exe 里搜一下当前 dist 的 `assets/index-*.js` 名字，搜不到就是没内嵌。
2. **别让 bin 与 lib 的输出重名**。`sc2trans-gui` 会被 cargo 归一成 `sc2trans_gui`，
   与 lib 的 pdb 撞车；所以 bin 显式命名为 `SC2Translater`。
3. **改字符串表必须逐字节保真**。写入前先跑 `verify`，它有一项 `string_tables_roundtrip_exact`。
4. **不要用整归档校验来判成败**。`(listfile)` 与 `(attributes)` 的 MD5 约定是**全零**、
   后者的 CRC 约定是 **0**，所以 `md5_bad == 2 / crc_bad == 1` 才是**正确**的。
5. **改动 core 的 JSON 结构要同步三处**：`crates/sc2trans-core/src/model.rs`、
   `ui/src/api/types.ts`、以及 Python 的 `python/src/sc2trans/model.py`。
6. **PowerShell 脚本里别写中文**：以无 BOM 的 UTF-8 写盘后，PS 会按 ANSI 解读而解析失败（踩过）。
7. **重活必须 `spawn_blocking`**：解一个 44MB 的模组要几秒，放 UI 线程就是「点了没反应」。
8. **改了写侧就一定要重跑编辑器验收**。容器层的自检全绿、StormLib 也认，都不代表 SC2 编辑器会接受
   —— 历史上就差一个 `dwRawChunkSize` 字段。见 `docs/design/验收记录.md`。
9. **Vue 里别把 ref 塞进 `reactive()`**：reactive 会深层解包 ref，
   于是 s.archive 直接变成值、s.archive.value 变成非法访问。用普通对象 + 显式 .value。
10. **UI 一律用 VS Code 官方控件，不要自己画**。按钮 = vscode-button、输入 = vscode-textfield、
    下拉 = vscode-single-select、勾选 = vscode-checkbox、单选 = vscode-radio-group、
    标签页 = vscode-tabs、树 = vscode-tree、右键菜单 = vscode-context-menu、
    分隔栏 = vscode-split-layout、图标 = vscode-icon。自定义元素在 Vue 模板里要加
    @ts-expect-error 注释，否则 vue-tsc 会当成未知标签报错。
11. **index.html 里那个 id 为 vscode-codicon-stylesheet 的 link 不能删**。vscode-icon 要在
    shadow root 里再用一次字体表，靠的就是这个固定 id（VS Code 的 webview 宿主就是这么注入的）。
    删掉之后所有图标会变成豆腐块或汉字兜底字形。
12. **不要用「补默认值」的方式覆盖官方主题令牌**。scripts/gen-vscode-theme.py 的补缺表现在会
    跳过官方已有的键 —— 早先没有跳过，把 button-secondaryBackground 从透明的 #00000000 盖成了
    灰底，secondary 按钮立刻就不是 VS Code 的样子了。

---

## 7. 验收清单（改完必跑）

* [ ] `cargo test` 全绿（含真实归档的 HET/BET 逐字节重生成）
* [ ] `cargo clippy --all-targets` 无警告
* [ ] `cd python; uv run pytest` 全绿
* [ ] `pnpm -C ui build` 与 `pnpm -C ui test` 通过
* [ ] 在一张真实地图上跑 `audit -> fill/seed -> verify`：
      `dwRawChunkSize == 0` 且 `healthy == true`，且**未改动的组件逐字节不变**
* [ ] 产物过一遍 StormLib（`../SC2Diff/tools/stormlib_verify.exe`）
* [ ] **银河编辑器打开产物不崩溃** —— 这是 MPQ 改动的唯一合格判据，
      用 `../SC2Diff/tools/editor_accept.py <原件作对照> <产物>`，判定必须为 `LOADED`。
      实测记录见 `docs/design/验收记录.md`。

---

## 8. 与 Python 原型的关系

`python/` 里的实现是**对照实现（oracle）**，不是废弃代码：

* 事实先在 Python 侧逐字节验证清楚，再落到 Rust；
* 两边的 CLI 契约与 JSON 结构必须一致，便于**差分测试**；
* `python/src/sc2trans/vendor/sc2mpq.py` 是从上游 SC2Diff **逐字节复制**的，
  **不要改**（要改先改上游，再重新复制并更新 SHA256）。
