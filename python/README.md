# sc2trans（Python 调研原型）

星际争霸 II 地图 / 模组的**翻译键提取与回填**工具。这一份是**参考实现**：
先用 Python 把 MPQ 容器与本地化文件的事实逐字节验证清楚，再用 Rust 重写成交付软件
（`crates/` 与 `src-tauri/`）。两边共用同一套 CLI 契约与 JSON 结构，便于差分测试。

## 它解决什么问题

英文地图只带 `enUS.SC2Data\LocalizedData\*.txt`。星际争霸 II **不会**在语言缺失时回退到英文，
它只加载与当前游戏语言同名的那一份。于是游戏语言设为中文时，地图里所有自定义键
（`DocInfo/Name`、`MapInfo/Player01/Name`、`Param/Value/XXXXXXXX` …）都查不到，
游戏就把**原始键路径**当成文本显示出来。

本工具做三件事：

1. `extract` —— 把地图里所有翻译键连同**英文原文**导成 JSON，翻译者只填 `target`；
2. `apply` —— 把 JSON 写回地图的 `zhCN.SC2Data`，重新封装成可直接打开的归档；
3. `seed` —— 还没有译文时的兜底：把英文整份补进目标语言，**先让游戏里不再出现键路径**。

## 环境

```
uv sync           # 创建 .venv 并安装（本项目零运行时依赖）
```

## 用法

```
# 1. 先看每种语言的覆盖率 —— 一眼看出「显示成路径」的原因
uv run sc2trans-py audit 地图.SC2Map

# 2. 导出翻译键（含英文原文）
uv run sc2trans-py extract 地图.SC2Map --source enUS --target zhCN -o keys.json

# 3. 翻译 keys.json 里的 target 字段（可以是任意 JSON 工具）

# 4. 写回地图
uv run sc2trans-py apply 地图.SC2Map keys.json -o 地图.zhCN.SC2Map

# 兜底：没有译文时先填英文
uv run sc2trans-py seed 地图.SC2Map --target zhCN -o 地图.zhCN.SC2Map
```

面向 agent 的用法：任何子命令加 `--json`，stdout 就是一个完整的 JSON 对象，
进度与告警全部走 stderr。

```
uv run sc2trans-py --json audit 地图.SC2Map
uv run sc2trans-py --json extract 地图.SC2Map
```

## JSON 结构（sc2trans.keys.v1）

```json
{
  "schema": "sc2trans.keys.v1",
  "source_locale": "enUS",
  "target_locale": "zhCN",
  "entries": [
    { "key": "DocInfo/Name", "file": "GameStrings.txt",
      "source": "Domination", "target": null,
      "status": "missing", "origin": ["enUS.SC2Data/GameStrings.txt"] }
  ],
  "stats": { "entries": 50, "by_status": { "missing": 48, "translated": 2 } },
  "diagnosis": { "severity": "warning", "message": "……" }
}
```

`status` 取值：`missing`（缺失，游戏显示键路径）、`identical`（与原文相同，等于没翻）、
`translated`（已翻）、`extra`（只在目标语言有）、`no-source`（只在别的语言有）。

`apply` 只依赖 `(file, key, target)` 三个字段，所以也接受**手写的扁平 JSON**：
`{"DocInfo/Name": "统治"}`。

## 目录

| 路径 | 说明 |
| --- | --- |
| `src/sc2trans/vendor/` | 从 SC2Diff 逐字节复制的 MPQ 容器层（对照物，**不要改**，见 NOTICE.md） |
| `src/sc2trans/strings.py` | 字符串表解析 / 序列化（BOM、CRLF、`///` 原文全部保真） |
| `src/sc2trans/ops.py` | extract / apply / seed / audit / verify 的业务实现 |
| `src/sc2trans/cli.py` | 命令行入口 |
| `tests/` | 单元测试与端到端测试 |

## 测试

```
uv run pytest
```
