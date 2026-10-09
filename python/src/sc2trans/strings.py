"""SC2 本地化字符串表的解析与序列化。

对应归档内的 <locale>.SC2Data\\LocalizedData\\*.txt。

## 实测到的字节级事实

以官方战役地图 Maps/.../zchar01.SC2Map 与社区样本为证：

| 事实 | 观测 |
| --- | --- |
| 编码 | UTF-8；**BOM 有无都可能**（暴雪 zhCN 无 BOM，银河编辑器写出的有 BOM） |
| 换行 | CRLF（编辑器与暴雪产物均为 CRLF，实测 lf=0） |
| 语法 | key=value，一行一条；值里可以出现等号（如 <s val="X">），所以只能按**第一个**等号切分 |
| 注释 | 真实文件里没有注释行；仍把 // 开头的行原样保留 |
| 空行 | 真实文件里没有；仍原样保留 |
| 译文原文对照 | key=本地化文本 /// 英文原文 —— **暴雪自己就这么写**：zchar01 的 zhCN 有 48/50 行带 ///，游戏解析时丢弃 /// 之后的部分 |
| 空值 | 合法，例如 DocInfo/DescLong= |
| 键的空白 | 实测两侧无空白；解析时仍 strip 容错 |

设计原则是**逐字节保真**：解析时把无法识别的行原样记为 RawLine，写回时按原顺序输出，
只在被显式修改的 Pair 上做最小改动。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable, Iterator

BOM_BYTES = b"\xef\xbb\xbf"
BOM_TEXT = "\ufeff"

#: 译文与原文的分隔符。暴雪官方本地化文件即使用此约定。
ORIGINAL_SEP = " /// "

_CRLF = "\r\n"
_LF = "\n"

_UNSET = object()


class StringFileError(ValueError):
    """字符串表解析失败。"""


@dataclass
class RawLine:
    """无法/无需解析的行：注释、空行、畸形行。原样保留。"""

    text: str


@dataclass
class Pair:
    """一条 key=value；original 是 /// 之后的英文原文（可空）。"""

    key: str
    value: str
    original: str | None = None

    def render(self) -> str:
        if self.original is None:
            return self.key + "=" + self.value
        return self.key + "=" + self.value + ORIGINAL_SEP + self.original


Record = RawLine | Pair


@dataclass
class StringFile:
    """一张字符串表：有序记录列表 + 文件的编码细节。"""

    records: list[Record] = field(default_factory=list)
    bom: bool = True
    newline: str = _CRLF
    trailing_newline: bool = True
    source_name: str = ""

    # ---------------------------------------------------------------- 解析
    @classmethod
    def parse(cls, data: bytes, *, source_name: str = "") -> "StringFile":
        bom = data.startswith(BOM_BYTES)
        try:
            text = data.decode("utf-8-sig")
        except UnicodeDecodeError as exc:  # pragma: no cover - 实测样本都是 UTF-8
            raise StringFileError(
                (source_name or "<memory>") + " 不是 UTF-8 文本（SC2 字符串表应为 UTF-8）: " + str(exc)
            ) from exc

        newline = _CRLF if _CRLF in text else _LF
        trailing = text.endswith(_CRLF) or text.endswith(_LF)
        raw_lines = text.split(_CRLF) if newline == _CRLF else text.split(_LF)
        if trailing and raw_lines and raw_lines[-1] == "":
            raw_lines.pop()

        records: list[Record] = []
        for line in raw_lines:
            stripped = line.strip()
            if not stripped or stripped.startswith("//") or "=" not in line:
                records.append(RawLine(line))
                continue
            key, _, rest = line.partition("=")
            key = key.strip()
            if not key:
                records.append(RawLine(line))
                continue
            if ORIGINAL_SEP in rest:
                value, _, original = rest.partition(ORIGINAL_SEP)
                records.append(Pair(key, value, original))
            else:
                records.append(Pair(key, rest, None))

        return cls(records=records, bom=bom, newline=newline,
                   trailing_newline=trailing, source_name=source_name)

    # ---------------------------------------------------------------- 写出
    def to_bytes(self) -> bytes:
        lines = [r.render() if isinstance(r, Pair) else r.text for r in self.records]
        if not lines:
            text = ""
        else:
            text = self.newline.join(lines)
            if self.trailing_newline:
                text += self.newline
        payload = (BOM_TEXT if self.bom else "") + text
        return payload.encode("utf-8")

    # ---------------------------------------------------------------- 访问
    def pairs(self) -> list[Pair]:
        return [r for r in self.records if isinstance(r, Pair)]

    def keys(self) -> list[str]:
        return [p.key for p in self.pairs()]

    def as_dict(self) -> dict[str, str]:
        return {p.key: p.value for p in self.pairs()}

    def get(self, key: str) -> Pair | None:
        for r in self.records:
            if isinstance(r, Pair) and r.key == key:
                return r
        return None

    def __contains__(self, key: str) -> bool:
        return self.get(key) is not None

    def __len__(self) -> int:
        return sum(1 for r in self.records if isinstance(r, Pair))

    # ---------------------------------------------------------------- 修改
    def set(self, key: str, value: str, original=_UNSET) -> Pair:
        """写入/更新一条。original 不传时保留原有的 /// 原文。"""
        existing = self.get(key)
        if existing is not None:
            existing.value = value
            if original is not _UNSET:
                existing.original = original  # type: ignore[assignment]
            return existing
        orig = None if original is _UNSET else original
        pair = Pair(key, value, orig)  # type: ignore[arg-type]
        self.records.append(pair)
        return pair

    def remove(self, key: str) -> bool:
        for i, r in enumerate(self.records):
            if isinstance(r, Pair) and r.key == key:
                del self.records[i]
                return True
        return False

    def sort(self) -> None:
        """就地按 key 排序（只重排 Pair，RawLine 的位置不动）。"""
        idx = [i for i, r in enumerate(self.records) if isinstance(r, Pair)]
        ordered = sorted((self.records[i] for i in idx), key=lambda p: p.key)  # type: ignore[attr-defined]
        for i, rec in zip(idx, ordered):
            self.records[i] = rec

    def add_missing(self, items: Iterable[tuple[str, str]]) -> list[str]:
        """批量补键，返回真正新增的键（已存在的不动）。"""
        added: list[str] = []
        for key, value in items:
            if self.get(key) is None:
                self.records.append(Pair(key, value, None))
                added.append(key)
        return added

    def iter_pairs(self) -> Iterator[Pair]:
        for r in self.records:
            if isinstance(r, Pair):
                yield r
