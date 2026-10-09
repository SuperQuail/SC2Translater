"""sc2trans —— 星际争霸 II 地图/模组的「翻译键」提取与回填工具（Python 调研原型）。

本包是**参考实现**：先用 Python 把 MPQ 容器与本地化文件的事实逐字节验证清楚，
再用 Rust 重写为交付软件。两边共用同一套 CLI 契约与 JSON 结构，便于差分测试。

模块划分：

* vendor/    —— 从 SC2Diff 逐字节复制的、已验证可用的 MPQ 容器层（对照物，不要改）
* locales.py —— SC2 的 locale 常量与归档内组件命名规则
* strings.py —— <locale>.SC2Data/LocalizedData/*.txt 的解析 / 序列化
* archive.py —— MPQ 归档的读写门面
* model.py   —— 翻译键 JSON 结构（sc2trans.keys.v1）
* ops.py     —— extract / apply / seed / audit / verify 等业务动作
* cli.py     —— 命令行入口（面向 agent，加 --json 输出机器可读结果）
"""

__version__ = "0.1.0"
SCHEMA_ID = "sc2trans.keys.v1"
