"""翻译键 JSON 结构：sc2trans.keys.v1。

设计目标有三个：

1. **给 agent 用**：字段名自解释，附 stats，便于一次调用就拿到结论。
2. **给翻译者用**：每条同时带 key / 原文 / 译文，翻译者只需要填 target。
3. **可回填**：apply 只依赖 (file, key, target) 三个字段，多余的字段不会破坏兼容。
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from typing import Any

SCHEMA_ID = "sc2trans.keys.v1"

#: 单条翻译的状态。
STATUS_MISSING = "missing"        # 源语言有，目标语言没有 -> 游戏会显示原始键路径
STATUS_IDENTICAL = "identical"    # 目标语言有，但与原文完全相同 -> 等于没翻译
STATUS_TRANSLATED = "translated"  # 目标语言有且与原文不同
STATUS_EXTRA = "extra"            # 只在目标语言里有（源语言没有）
STATUS_NO_SOURCE = "no-source"    # 只在其它语言里有

STATUS_LABELS = {
    STATUS_MISSING: "缺失（会显示原始键路径）",
    STATUS_IDENTICAL: "与原文相同（未翻译）",
    STATUS_TRANSLATED: "已翻译",
    STATUS_EXTRA: "仅目标语言存在",
    STATUS_NO_SOURCE: "无原文",
}


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


def new_document(*, archive: dict[str, Any], source_locale: str, target_locale: str,
                 files: list[str], entries: list[dict[str, Any]], tool: str,
                 extra_stats: dict[str, Any] | None = None) -> dict[str, Any]:
    stats: dict[str, Any] = {
        "entries": len(entries),
        "by_status": _count(entries, "status"),
        "by_file": _count(entries, "file"),
        "translatable": sum(1 for e in entries if e["status"] in (STATUS_MISSING, STATUS_IDENTICAL)),
    }
    if extra_stats:
        stats.update(extra_stats)
    return {
        "schema": SCHEMA_ID,
        "tool": tool,
        "generated_at": utc_now(),
        "archive": archive,
        "source_locale": source_locale,
        "target_locale": target_locale,
        "files": files,
        "entries": entries,
        "stats": stats,
    }


def _count(entries: list[dict[str, Any]], field: str) -> dict[str, int]:
    out: dict[str, int] = {}
    for e in entries:
        key = str(e.get(field, ""))
        out[key] = out.get(key, 0) + 1
    return dict(sorted(out.items()))


def load(path: str) -> dict[str, Any]:
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)


def dump(doc: dict[str, Any], path: str | None) -> str:
    text = json.dumps(doc, ensure_ascii=False, indent=2, sort_keys=False)
    if path:
        parent = os.path.dirname(os.path.abspath(path))
        if parent:
            os.makedirs(parent, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(text)
            fh.write("\n")
    return text


def normalize_input(raw: Any, *, default_file: str) -> tuple[list[dict[str, Any]], str | None, str | None]:
    """把任意输入 JSON 归一成 entries 列表。

    支持两种形态：

    * 本工具导出的完整结构（含 entries / source_locale / target_locale）；
    * 翻译者手写的扁平字典 {key: 译文}。
    """
    if isinstance(raw, dict) and isinstance(raw.get("entries"), list):
        entries = []
        for e in raw["entries"]:
            if not isinstance(e, dict) or "key" not in e:
                continue
            entries.append({
                "key": str(e["key"]),
                "file": str(e.get("file") or default_file),
                "source": e.get("source"),
                "target": e.get("target"),
                "status": e.get("status", ""),
                "origin": e.get("origin", []),
            })
        return entries, raw.get("source_locale"), raw.get("target_locale")

    if isinstance(raw, dict):
        entries = []
        for key, value in raw.items():
            if isinstance(value, dict):
                entries.append({"key": str(key), "file": str(value.get("file") or default_file),
                                "source": value.get("source"), "target": value.get("target"),
                                "status": "", "origin": []})
            else:
                entries.append({"key": str(key), "file": default_file, "source": None,
                                "target": None if value is None else str(value),
                                "status": "", "origin": []})
        return entries, None, None

    raise ValueError("无法识别的翻译 JSON：既不是 {entries:[...]} 也不是 {key: value}")
