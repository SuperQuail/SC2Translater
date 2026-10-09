"""业务动作：info / extract / audit / apply / seed / verify。

这些函数的签名与返回结构就是未来 Rust CLI 的契约，两边必须逐字段对齐
（tests/test_cli_parity.py 会对同一输入比较两边输出的 JSON）。
"""

from __future__ import annotations

import os
from typing import Any, Iterable

from . import __version__, model
from .archive import Archive, ArchiveError, Document
from .locales import (
    BASE,
    KNOWN_STRING_FILES,
    LOCALES,
    is_player_facing,
    locale_display,
)
from .strings import StringFile

TOOL_ID = "sc2trans-py " + __version__


# =============================================================== 只读：info
def inspect(path: str) -> dict[str, Any]:
    """归档概览：容器参数、组件数、存在的语言、字符串表清单。"""
    arc = Archive(path)
    tables = arc.locale_tables()
    locales = []
    for locale in sorted(tables, key=lambda l: (l != BASE, l)):
        files = tables[locale]
        locales.append({
            "locale": locale,
            "display": locale_display(locale),
            "files": {name: {"component": comp, "bytes": len(arc.read(comp))}
                      for name, comp in sorted(files.items())},
        })
    verify = arc.arc.verify(deep=False)
    return {
        "path": arc.path,
        "size": arc.size,
        "sha256": arc.sha256(),
        "container": {
            "format_version": arc.arc.format_version,
            "header_size": arc.arc.header_size,
            "block_size": arc.arc.block_size,
            "components": len(arc.names),
            "hash_entries": arc.arc.hash_count,
            "blocks": arc.arc.block_count,
            "has_het": arc.arc.het is not None,
            "has_bet": arc.arc.bet is not None,
            "has_attributes": arc.arc.attributes is not None,
            "header_md5_valid": verify.get("header_md5_valid"),
        },
        "locales": locales,
        "components": arc.names,
    }


# =============================================================== 只读：extract
def _load_tables(arc: Archive, locale: str) -> dict[str, StringFile]:
    out: dict[str, StringFile] = {}
    for filename, comp in arc.locale_tables().get(locale, {}).items():
        out[filename] = StringFile.parse(arc.read(comp), source_name=comp)
    return out


def extract(path: str, *, source_locale: str = "enUS", target_locale: str = "zhCN",
            files: Iterable[str] | None = None, include_all_locales: bool = False) -> dict[str, Any]:
    """提取翻译键：以 source_locale 为原文，报告 target_locale 的覆盖情况。"""
    arc = Archive(path)
    tables = arc.locale_tables()
    src_files = _load_tables(arc, source_locale)
    base_files = _load_tables(arc, BASE)
    tgt_files = _load_tables(arc, target_locale)

    wanted = set(files) if files else (set(src_files) | set(base_files) | set(tgt_files))

    other_files: dict[str, dict[str, StringFile]] = {}
    if include_all_locales:
        for locale in tables:
            if locale in (source_locale, target_locale, BASE):
                continue
            other_files[locale] = _load_tables(arc, locale)

    entries: list[dict[str, Any]] = []
    for filename in sorted(wanted):
        src = src_files.get(filename)
        base = base_files.get(filename)
        tgt = tgt_files.get(filename)
        order: list[str] = []
        seen: set[str] = set()
        for table in (src, base, tgt):
            if table is None:
                continue
            for key in table.keys():
                if key not in seen:
                    seen.add(key)
                    order.append(key)
        for locale, loc_files in sorted(other_files.items()):
            table = loc_files.get(filename)
            if table is None:
                continue
            for key in table.keys():
                if key not in seen:
                    seen.add(key)
                    order.append(key)

        for key in order:
            origin: list[str] = []
            source_value: str | None = None
            for locale, table in ((source_locale, src), (BASE, base)):
                if table is None:
                    continue
                pair = table.get(key)
                if pair is not None:
                    origin.append(locale + ".SC2Data/" + filename)
                    if source_value is None:
                        source_value = pair.value
            target_value: str | None = None
            if tgt is not None:
                pair = tgt.get(key)
                if pair is not None:
                    target_value = pair.value

            if target_value is None and source_value is None:
                status = model.STATUS_NO_SOURCE
            elif target_value is None:
                status = model.STATUS_MISSING
            elif source_value is not None and target_value == source_value:
                status = model.STATUS_IDENTICAL
            elif source_value is None:
                status = model.STATUS_EXTRA
            else:
                status = model.STATUS_TRANSLATED

            entries.append({
                "key": key,
                "file": filename,
                "source": source_value,
                "target": target_value,
                "status": status,
                "origin": origin,
            })

    present_locales = sorted(tables.keys())
    doc = model.new_document(
        archive={"path": arc.path, "size": arc.size, "sha256": arc.sha256()},
        source_locale=source_locale, target_locale=target_locale,
        files=sorted(wanted), entries=entries, tool=TOOL_ID,
        extra_stats={"locales_present": present_locales},
    )
    doc["diagnosis"] = _diagnose(doc)
    return doc


def _diagnose(doc: dict[str, Any]) -> dict[str, Any]:
    """把 stats 翻译成一句人能看懂的话（GUI 与 agent 都直接用这句）。"""
    stats = doc["stats"]["by_status"]
    missing = stats.get(model.STATUS_MISSING, 0) + stats.get(model.STATUS_IDENTICAL, 0)
    target = doc["target_locale"]
    src = doc["source_locale"]
    if doc["stats"]["entries"] == 0:
        return {"severity": "info", "message": "归档里没有找到任何本地化字符串表。"}
    if missing == 0:
        return {"severity": "ok",
                "message": target + " 已覆盖全部 " + str(doc["stats"]["entries"]) + " 个键，游戏内不会出现原始键路径。"}
    return {
        "severity": "warning",
        "message": (locale_display(target) + "（" + target + "）缺少 " + str(missing) + " / "
                    + str(doc["stats"]["entries"]) + " 个键：游戏语言设为 " + target
                    + " 时这些文本会显示成原始键路径，而不是 " + src + " 里写的文本。"),
        "missing_in_target": missing,
        "actionable": "sc2trans seed 或 sc2trans apply 可以把这些键补进 " + target + "。",
    }


# =============================================================== 只读：audit
def audit(path: str, *, source_locale: str = "enUS") -> dict[str, Any]:
    """对归档里**每个**已存在的语言算一遍覆盖率，用于回答「为什么显示成路径」。"""
    arc = Archive(path)
    tables = arc.locale_tables()
    # 按**文件**分别统计：GameStrings / ObjectStrings 是玩家看得见的文本，
    # TriggerStrings 只喂给银河编辑器的触发器界面。混在一起算覆盖率会误导人
    # —— 实测暴雪官方地图的非英文语言目录里根本没有 TriggerStrings，那不是缺陷。
    source_by_file: dict[str, set[str]] = {}
    for filename, comp in tables.get(source_locale, {}).items():
        source_by_file.setdefault(filename, set()).update(StringFile.parse(arc.read(comp)).keys())
    for filename, comp in tables.get(BASE, {}).items():
        source_by_file.setdefault(filename, set()).update(StringFile.parse(arc.read(comp)).keys())
    source_keys: set[str] = set()
    for keys in source_by_file.values():
        source_keys |= keys

    report: list[dict[str, Any]] = []
    for locale in sorted(tables):
        if locale == BASE:
            continue
        # 一个语言目录下可能有 GameStrings / ObjectStrings / TriggerStrings 多张表，
        # 必须先把它们并成一个键集合再算覆盖率，否则同一批键会被重复计数。
        present_keys: set[str] = set()
        for comp in tables[locale].values():
            present_keys |= set(StringFile.parse(arc.read(comp)).keys())
        total = len(source_keys)
        present = len(source_keys & present_keys)
        coverage = 0.0 if total == 0 else present / total
        report.append({
            "locale": locale,
            "display": locale_display(locale),
            "keys_expected": total,
            "keys_present": present,
            "keys_missing": total - present,
            "keys_extra": len(present_keys - source_keys),
            "coverage": round(coverage, 4),
            "files": sorted(tables[locale]),
        })
    by_file: dict[str, Any] = {}
    for filename, keys in sorted(source_by_file.items()):
        rows: list[dict[str, Any]] = []
        for locale in sorted(tables):
            if locale == BASE:
                continue
            comp = tables[locale].get(filename)
            present = len(keys & set(StringFile.parse(arc.read(comp)).keys())) if comp else 0
            rows.append({
                "locale": locale,
                "present": present,
                "expected": len(keys),
                "coverage": round(present / len(keys), 4) if keys else 0.0,
            })
        by_file[filename] = {
            "player_facing": is_player_facing(filename),
            "source_keys": len(keys),
            "locales": rows,
        }

    missing_locales = [l for l in LOCALES if l not in tables]
    return {
        "path": arc.path,
        "source_locale": source_locale,
        "source_keys": len(source_keys),
        "locales": report,
        "by_file": by_file,
        "locales_absent": missing_locales,
        "diagnosis": _audit_diagnosis(report, by_file, source_locale),
    }


def _audit_diagnosis(report: list[dict[str, Any]], by_file: dict[str, Any],
                     source_locale: str) -> dict[str, Any]:
    """只拿**面向玩家**的表来下结论；编辑器专用的表缺了不影响游戏体验。"""
    player_files = [f for f, meta in by_file.items() if meta["player_facing"]]
    if not report:
        return {"severity": "info", "message": "归档里没有任何 <locale>.SC2Data 语言目录。"}
    if not player_files:
        return {
            "severity": "info",
            "message": ("归档里只有编辑器专用字符串表（" + "、".join(sorted(by_file)) + "），"
                        "玩家在游戏里不会看到它们。"),
        }

    worst: list[tuple[str, float, int]] = []
    for filename in player_files:
        for row in by_file[filename]["locales"]:
            if row["expected"] and row["coverage"] < 1.0:
                worst.append((row["locale"], row["coverage"], row["expected"] - row["present"]))
    if not worst:
        return {"severity": "ok",
                "message": "所有已存在的语言都完整覆盖了 " + source_locale + " 的玩家可见文本。"}
    worst.sort(key=lambda item: item[1])
    parts = [loc + " " + str(round(cov * 100, 1)) + "%" for loc, cov, _ in worst[:4]]
    return {
        "severity": "warning",
        "message": ("以下语言的**玩家可见**文本覆盖率不足：" + "、".join(parts)
                    + "。用这些语言进入游戏时，未覆盖的键会显示成原始键路径，而不是文本。"),
        "worst": [loc for loc, _, _ in worst[:4]],
        "player_files": player_files,
    }


# =============================================================== 写入：apply
def _target_path(src: str, out: str | None, target_locale: str) -> str:
    if out:
        return os.path.abspath(out)
    stem, ext = os.path.splitext(src)
    return stem + "." + target_locale + ext


def apply(path: str, data: Any, *, out: str | None = None, target_locale: str | None = None,
          fill_missing: bool = False, with_original: bool = False, sort: bool = False,
          default_file: str = KNOWN_STRING_FILES[0], force: bool = False) -> dict[str, Any]:
    """把翻译 JSON 写回归档。

    只依赖 (file, key, target) 三个字段；target 为空且 fill_missing=True 时用 source 顶上
    —— 这就是「先填英文，至少不再显示键路径」的那一步。
    """
    entries, src_locale_in_json, tgt_locale_in_json = model.normalize_input(data, default_file=default_file)
    target = target_locale or tgt_locale_in_json
    if not target:
        raise ValueError("必须指定目标语言：JSON 里没有 target_locale，命令行也没给 --target")

    dest = _target_path(path, out, target)
    if os.path.abspath(path) == dest and not force:
        raise ArchiveError("输出路径与输入相同，会覆盖原文件；确认请加 --force")

    doc = Document(path)
    # 源语言文件的编码习惯（BOM / 换行）用来初始化新建的目标文件
    src_tables = _load_tables(Archive(path), src_locale_in_json or "enUS")

    grouped: dict[str, list[dict[str, Any]]] = {}
    for e in entries:
        value = e.get("target")
        if value in (None, ""):
            if not fill_missing:
                continue
            value = e.get("source")
            if value in (None, ""):
                continue
        grouped.setdefault(e["file"], []).append({"key": e["key"], "value": str(value),
                                                  "source": e.get("source")})

    written: dict[str, dict[str, Any]] = {}
    for filename, items in sorted(grouped.items()):
        comp = _component(target, filename)
        raw = doc.try_read(comp)
        if raw is not None:
            table = StringFile.parse(raw, source_name=comp)
        else:
            template = src_tables.get(filename)
            table = StringFile(bom=template.bom if template else True,
                               newline=template.newline if template else "\r\n",
                               trailing_newline=template.trailing_newline if template else True,
                               source_name=comp)
        before = set(table.keys())
        for item in items:
            original = item["source"] if (with_original and item["source"]) else None
            table.set(item["key"], item["value"], original)
        if sort:
            table.sort()
        after = set(table.keys())
        doc.write(comp, table.to_bytes())
        written.setdefault(target, {})[filename] = {
            "component": comp,
            "added": len(after - before),
            "updated": len(after & before),
            "total": len(after),
        }

    saved = doc.save(dest)

    # 回读校验，确保写出去的文件真的能被自己读回来
    check = Archive(dest)
    readback: dict[str, Any] = {}
    for filename, info in written[target].items():
        raw = check.try_read(info["component"])
        if raw is None:
            readback[filename] = {"ok": False, "reason": "组件不存在"}
            continue
        table = StringFile.parse(raw, source_name=info["component"])
        wanted_keys = {i["key"] for i in grouped[filename]}
        got = {k: table.get(k).value for k in wanted_keys if table.get(k) is not None}  # type: ignore[union-attr]
        readback[filename] = {"ok": len(got) == len(wanted_keys), "keys": len(got),
                              "expected": len(wanted_keys)}

    return {
        "input": os.path.abspath(path),
        "output": dest,
        "target_locale": target,
        "source_locale": src_locale_in_json,
        "components_written": written,
        "container": saved,
        "readback": readback,
        "ok": all(r["ok"] for r in readback.values()) if readback else False,
        "bytes_in": os.path.getsize(path),
        "bytes_out": os.path.getsize(dest),
    }


def _component(locale: str, filename: str) -> str:
    from .locales import component_name
    return component_name(locale, filename)


# =============================================================== 写入：seed
def seed(path: str, *, source_locale: str = "enUS", target_locale: str = "zhCN",
         out: str | None = None, with_original: bool = False, sort: bool = False,
         force: bool = False) -> dict[str, Any]:
    """快捷修复：把源语言的文本整份补进目标语言，游戏内不再出现原始键路径。

    这是「没有汉化文本时」的兜底：显示英文总好过显示 Param/Value/xxxx。
    """
    doc = extract(path, source_locale=source_locale, target_locale=target_locale)
    result = apply(path, doc, out=out, target_locale=target_locale, fill_missing=True,
                   with_original=with_original, sort=sort, force=force)
    result["seeded"] = doc["stats"]["by_status"].get(model.STATUS_MISSING, 0) + \
        doc["stats"]["by_status"].get(model.STATUS_IDENTICAL, 0)
    result["diagnosis_before"] = doc.get("diagnosis")
    return result


# =============================================================== 只读：verify
def verify(path: str, *, deep: bool = True) -> dict[str, Any]:
    """容器自检 + 字符串表往返回读。"""
    arc = Archive(path)
    report = arc.arc.verify(deep=deep)
    report["sha256"] = arc.sha256()
    report["size"] = arc.size
    tables: dict[str, Any] = {}
    for locale, files in sorted(arc.locale_tables().items()):
        tables[locale] = {}
        for filename, comp in sorted(files.items()):
            raw = arc.read(comp)
            table = StringFile.parse(raw, source_name=comp)
            again = table.to_bytes()
            tables[locale][filename] = {
                "bytes": len(raw),
                "keys": len(table),
                "bom": table.bom,
                "newline": "CRLF" if table.newline == "\r\n" else "LF",
                "roundtrip_exact": again == raw,
                "utf8": True,
            }
    report["string_tables"] = tables
    report["string_tables_roundtrip_exact"] = all(
        info["roundtrip_exact"] for files in tables.values() for info in files.values()
    )
    return report
