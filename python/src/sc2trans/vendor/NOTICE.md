# vendor/ 说明

本目录下的 `sc2mpq.py` **不是本项目原创代码**，是从同作者的既有项目
**SC2Diff**（`https://github.com/SuperQuail/SC2Diff`，MIT）`tools/sc2mpq.py`
**逐字节复制**而来的 MPQ 容器层参考实现。

| 项 | 值 |
| --- | --- |
| 来源路径 | `D:\Code\Rust\SC2Diff\tools\sc2mpq.py` |
| 复制时 SHA256 | 见 `sc2mpq.py.sha256` |
| 复制日期 | 2026-10-09 |
| 许可证 | MIT（随 SC2Diff 仓库） |

## 为什么复制而不是引用
- 该实现已通过**逐字节可复现**判据（HET/BET 重生成）与 **StormLib 交叉验证**，
  是「已确认可用」的 MPQ 写入器。
- 本项目需要一份**自包含**的可运行原型，且未来要用 Rust 重写这一层；
  保留原样副本便于**差分测试**（Rust 产物 vs Python 产物）。

## 修改约定
- **不要直接改 `sc2mpq.py`**。它是对照物（oracle）。
  本项目自己的逻辑一律写在 `sc2trans/` 下的其它模块里。
- 若确实需要修 bug，请先改上游 SC2Diff，再重新复制并更新 SHA256。

## 顺带记录的上游关键结论（详见 SC2Diff/research/findings.md）
1. SC2 写的是 **208 字节头 + formatVersion=3 + v4 的 size64 字段布局**。
2. `dwRawChunkSize` 必须写 **0**；暴雪写入器写 16384 会导致编辑器对任何内容改动
   报 `e_fileCorrupt`（Blizzard `System_Mopaq` 的 raw-chunk MD5 校验路径）。
3. 头部 0x70..0xCF 的六个 MD5 摘要块是**硬性要求**，缺失或不匹配会被判
   `ERROR_FAKE_MPQ_HEADER`（StormLib 10009）。
4. HET 是权威索引，但 HET 条目只存名字哈希高 8 位，必须用 BET 的完整 64 位哈希复核，
   且不匹配时要**继续探测**而不是直接返回失败。
