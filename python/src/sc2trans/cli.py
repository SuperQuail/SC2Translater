"""命令行入口。面向 agent：每个子命令都支持 --json，输出稳定的机器可读结构。

约定：

* **stdout 只放结果**（--json 时是一个 JSON 对象；否则是人读的摘要）。
* **stderr 放进度与告警**，agent 解析 stdout 不会被噪声干扰。
* 退出码：0 成功；1 业务失败；2 用法错误。
"""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any

from . import __version__, model, ops
from .archive import ArchiveError
from .locales import locale_code_ok, locale_display
from .strings import StringFileError


def _locale(value: str) -> str:
    """语言代码不限定取值：实测真实归档里出现过 enCN/enGB/enSG/enTW/jaJP/zhCH。"""
    if not locale_code_ok(value):
        raise argparse.ArgumentTypeError("语言代码形状不对（应为 2~4 个字母，如 zhCN）: " + value)
    return value


def _emit(args: argparse.Namespace, payload: dict[str, Any], text: str) -> None:
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(text)


def _warn(msg: str) -> None:
    print(msg, file=sys.stderr)


# ------------------------------------------------------------------ 子命令
def _cmd_info(args: argparse.Namespace) -> int:
    info = ops.inspect(args.archive)
    if args.json:
        _emit(args, info, "")
        return 0
    c = info["container"]
    lines = [
        "文件      " + info["path"],
        "大小      " + str(info["size"]) + " 字节",
        "SHA256    " + info["sha256"],
        "容器      format=" + str(c["format_version"]) + " header=" + str(c["header_size"])
        + " blocks=" + str(c["blocks"]) + " 组件=" + str(c["components"]),
        "索引      HET=" + str(c["has_het"]) + " BET=" + str(c["has_bet"])
        + " attributes=" + str(c["has_attributes"]) + " 头部摘要=" + str(c["header_md5_valid"]),
        "语言目录  " + (", ".join(l["locale"] for l in info["locales"]) or "（无）"),
    ]
    for loc in info["locales"]:
        for name, meta in loc["files"].items():
            lines.append("          " + loc["locale"] + " / " + name + "  " + str(meta["bytes"]) + " 字节")
    _emit(args, info, "\n".join(lines))
    return 0


def _cmd_extract(args: argparse.Namespace) -> int:
    doc = ops.extract(args.archive, source_locale=args.source, target_locale=args.target,
                      include_all_locales=args.all_locales)
    if args.out:
        model.dump(doc, args.out)
        _warn("已写出 " + args.out)
    if args.json:
        _emit(args, doc, "")
        return 0
    s = doc["stats"]
    lines = [
        "源语言  " + doc["source_locale"] + "   目标语言 " + doc["target_locale"],
        "文件    " + ", ".join(doc["files"]),
        "键总数  " + str(s["entries"]),
    ]
    for status, count in s["by_status"].items():
        lines.append("  " + status.ljust(12) + str(count) + "   " + model.STATUS_LABELS.get(status, ""))
    lines.append("")
    lines.append(doc["diagnosis"]["message"])
    _emit(args, doc, "\n".join(lines))
    return 0


def _cmd_audit(args: argparse.Namespace) -> int:
    report = ops.audit(args.archive, source_locale=args.source)
    if args.json:
        _emit(args, report, "")
        return 0
    lines = ["源语言 " + report["source_locale"] + "，共 " + str(report["source_keys"]) + " 个键", ""]
    lines.append("语言    期望    已有    缺失    覆盖率")
    for r in report["locales"]:
        lines.append("{:6}  {:6}  {:6}  {:6}  {:6.1f}%".format(
            r["locale"], r["keys_expected"], r["keys_present"], r["keys_missing"], r["coverage"] * 100))
    if report["locales_absent"]:
        lines.append("")
        lines.append("归档里没有这些语言目录：" + ", ".join(report["locales_absent"]))
    lines.append("")
    lines.append(report["diagnosis"]["message"])
    _emit(args, report, "\n".join(lines))
    return 0


def _cmd_apply(args: argparse.Namespace) -> int:
    data = model.load(args.json) if args.json_in else json.load(sys.stdin)
    result = ops.apply(args.archive, data, out=args.out, target_locale=args.target,
                       fill_missing=args.fill_missing, with_original=args.with_original,
                       sort=args.sort_keys, force=args.force)
    if args.json:
        _emit(args, result, "")
        return 0
    lines = ["已写出 " + result["output"], "目标语言 " + result["target_locale"]]
    for locale, files in result["components_written"].items():
        for filename, info in files.items():
            lines.append("  " + filename + ": 新增 " + str(info["added"]) + "，更新 " + str(info["updated"])
                         + "，共 " + str(info["total"]) + " 条")
    lines.append("回读校验 " + ("通过" if result["ok"] else "失败"))
    _emit(args, result, "\n".join(lines))
    return 0 if result["ok"] else 1


def _cmd_seed(args: argparse.Namespace) -> int:
    result = ops.seed(args.archive, source_locale=args.source, target_locale=args.target,
                      out=args.out, with_original=args.with_original, sort=args.sort_keys,
                      force=args.force)
    if args.json:
        _emit(args, result, "")
        return 0
    lines = [
        "已写出 " + result["output"],
        "把 " + result["source_locale"] + " 的文本补进 " + result["target_locale"]
        + "，补了 " + str(result["seeded"]) + " 个键",
        "现在用 " + result["target_locale"] + " 进入游戏不会再看到原始键路径（显示的是 "
        + result["source_locale"] + " 的原文，等你替换成译文）。",
        "回读校验 " + ("通过" if result["ok"] else "失败"),
    ]
    _emit(args, result, "\n".join(lines))
    return 0 if result["ok"] else 1


def _cmd_verify(args: argparse.Namespace) -> int:
    report = ops.verify(args.archive, deep=not args.shallow)
    if args.json:
        _emit(args, report, "")
        return 0
    lines = [
        "文件            " + report["path"],
        "头部摘要        " + str(report.get("header_md5_valid")),
        "HET 逐字节重建  " + str(report.get("het_roundtrip_exact")),
        "BET 逐字节重建  " + str(report.get("bet_roundtrip_exact")),
        "HET/经典表一致  " + ("是" if not report.get("het_vs_classic_mismatch") else "否"),
        "attributes      md5 ok=" + str(report.get("md5_ok")) + " bad=" + str(report.get("md5_bad"))
        + " crc ok=" + str(report.get("crc_ok")) + " bad=" + str(report.get("crc_bad")),
        "字符串表往返    " + str(report.get("string_tables_roundtrip_exact")),
    ]
    _emit(args, report, "\n".join(lines))
    return 0


def _cmd_list(args: argparse.Namespace) -> int:
    arc = ops.Archive(args.archive)
    tables = arc.locale_tables()
    if args.json:
        _emit(args, {"path": arc.path, "locales": {k: sorted(v) for k, v in tables.items()}}, "")
        return 0
    for locale in sorted(tables):
        print(locale + ": " + ", ".join(sorted(tables[locale])))
    return 0


# ------------------------------------------------------------------ 解析
def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="sc2trans-py",
        description="星际争霸 II 地图/模组翻译键提取与回填工具（Python 调研原型）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""常用流程：
  先看看缺什么       sc2trans-py audit 地图.SC2Map
  导出翻译键         sc2trans-py extract 地图.SC2Map -o keys.json
  （翻译 keys.json 里的 target 字段）
  写回地图           sc2trans-py apply 地图.SC2Map keys.json -o 中文.SC2Map
  没有译文时的兜底   sc2trans-py seed 地图.SC2Map --target zhCN
""")
    p.add_argument("--version", action="version", version="sc2trans-py " + __version__)
    p.add_argument("--json", action="store_true", default=False,
                   help="输出机器可读 JSON（agent 用）；放在子命令前或后都可以")
    sub = p.add_subparsers(dest="command", required=True)

    def add_common(sp: argparse.ArgumentParser) -> None:
        # 默认用 SUPPRESS：子命令后面没写 --json 时，不会把顶层 --json 的值冲掉。
        sp.add_argument("--json", action="store_true", default=argparse.SUPPRESS,
                        help="输出机器可读 JSON（agent 用）")

    sp = sub.add_parser("info", help="归档概览")
    sp.add_argument("archive")
    add_common(sp)
    sp.set_defaults(func=_cmd_info)

    sp = sub.add_parser("list", help="列出归档里的语言与字符串表")
    sp.add_argument("archive")
    add_common(sp)
    sp.set_defaults(func=_cmd_list)

    sp = sub.add_parser("extract", help="提取翻译键为 JSON")
    sp.add_argument("archive")
    sp.add_argument("-o", "--out", help="JSON 输出路径")
    sp.add_argument("--source", default="enUS", type=_locale, help="原文语言（默认 enUS）")
    sp.add_argument("--target", default="zhCN", type=_locale, help="目标语言（默认 zhCN）")
    sp.add_argument("--all-locales", action="store_true", help="把其它语言的键也并入结果")
    add_common(sp)
    sp.set_defaults(func=_cmd_extract)

    sp = sub.add_parser("audit", help="检查每种语言对原文键的覆盖率（定位「显示成路径」的原因）")
    sp.add_argument("archive")
    sp.add_argument("--source", default="enUS", type=_locale)
    add_common(sp)
    sp.set_defaults(func=_cmd_audit)

    sp = sub.add_parser("apply", help="把翻译 JSON 写回归档")
    sp.add_argument("archive")
    sp.add_argument("json_in", nargs="?", help="翻译 JSON；省略则读 stdin")
    sp.add_argument("-o", "--out", help="输出归档路径（默认 <原名>.<目标语言>.SC2Map）")
    sp.add_argument("--target", type=_locale, help="目标语言（默认取 JSON 里的 target_locale）")
    sp.add_argument("--fill-missing", action="store_true", help="target 为空时用 source 顶上")
    sp.add_argument("--with-original", action="store_true", help="写成「译文 /// 原文」的形式")
    sp.add_argument("--sort", dest="sort_keys", action="store_true", help="按 key 排序")
    sp.add_argument("--force", action="store_true", help="允许覆盖输入文件")
    add_common(sp)
    sp.set_defaults(func=_cmd_apply)

    sp = sub.add_parser("seed", help="把原文整份补进目标语言（没有译文时的兜底修复）")
    sp.add_argument("archive")
    sp.add_argument("-o", "--out", help="输出归档路径")
    sp.add_argument("--source", default="enUS", type=_locale)
    sp.add_argument("--target", default="zhCN", type=_locale)
    sp.add_argument("--with-original", action="store_true")
    sp.add_argument("--sort", dest="sort_keys", action="store_true")
    sp.add_argument("--force", action="store_true")
    add_common(sp)
    sp.set_defaults(func=_cmd_seed)

    sp = sub.add_parser("verify", help="容器自检 + 字符串表往返回读")
    sp.add_argument("archive")
    sp.add_argument("--shallow", action="store_true", help="跳过逐块 CRC/MD5 校验")
    add_common(sp)
    sp.set_defaults(func=_cmd_verify)

    return p


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except (ArchiveError, StringFileError, ValueError, KeyError, OSError) as exc:
        if getattr(args, "json", False):
            print(json.dumps({"ok": False, "error": type(exc).__name__, "message": str(exc)},
                             ensure_ascii=False), file=sys.stdout)
        else:
            print("错误: " + str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
