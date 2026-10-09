"""MPQ 归档的读写门面。

容器层本身来自 vendor/sc2mpq.py（SC2Diff 的已验证实现），本模块只负责：

* 把「组件名」的两种分隔符统一成归档内用的反斜杠；
* 提供**只读、按需解压**的视图（extract/audit 用，避免为一个 44MB 的模组白白解压全部组件）；
* 提供**可写**的视图（apply/seed 用），并在写回时保留原归档的表参数与分块布局。
"""

from __future__ import annotations

import hashlib
import os

from .locales import component_name, normalize_component, parse_string_component
from .vendor import sc2mpq


class ArchiveError(RuntimeError):
    """归档操作失败。"""


def _sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class Archive:
    """只读视图：打开归档只解析表结构，读到某个组件时才解压它。"""

    def __init__(self, path: str):
        self.path = os.path.abspath(path)
        if not os.path.isfile(self.path):
            raise ArchiveError("找不到文件: " + self.path)
        try:
            self.arc = sc2mpq.MPQArchive(self.path)
        except Exception as exc:
            raise ArchiveError("不是可读的 MPQ 归档: " + self.path + " (" + str(exc) + ")") from exc
        self.names = [normalize_component(n) for n in self.arc.names()]

    # ------------------------------------------------------------- 基本信息
    @property
    def size(self) -> int:
        return os.path.getsize(self.path)

    def sha256(self) -> str:
        return _sha256_file(self.path)

    def components(self) -> list[str]:
        return list(self.names)

    def has(self, name: str) -> bool:
        return normalize_component(name) in self.names

    # ------------------------------------------------------------- 读取
    def try_read(self, name: str) -> bytes | None:
        norm = normalize_component(name)
        index = self.arc.find_block(norm)
        if index < 0:
            return None
        try:
            return self.arc.read_block(index, norm)
        except Exception as exc:
            raise ArchiveError("读取组件失败: " + norm + " (" + str(exc) + ")") from exc

    def read(self, name: str) -> bytes:
        data = self.try_read(name)
        if data is None:
            raise ArchiveError("归档里没有组件: " + normalize_component(name))
        return data

    # ------------------------------------------------------------- 本地化
    def locale_tables(self) -> dict[str, dict[str, str]]:
        """返回 {locale: {文件名: 组件名}}。只包含归档里真实存在的字符串表。"""
        found: dict[str, dict[str, str]] = {}
        for name in self.names:
            parsed = parse_string_component(name)
            if parsed is None:
                continue
            locale, filename = parsed
            found.setdefault(locale, {})[filename] = name
        return found

    def locales(self) -> list[str]:
        return sorted(self.locale_tables().keys())


class Document:
    """可写视图：把归档读成「组件名 -> 字节」的有序字典，改完再整体写回。

    写回走 sc2mpq.build_document，会自动重算 (listfile) / (attributes) / HET / BET /
    头部摘要块；同时沿用原归档的 hash 表大小、块大小、BET unknown08、attributes 标志
    与每个组件的分块方式，做到「只改该改的」。
    """

    def __init__(self, path: str):
        self.path = os.path.abspath(path)
        if not os.path.isfile(self.path):
            raise ArchiveError("找不到文件: " + self.path)
        try:
            self.doc = sc2mpq.Document.open(self.path)
        except Exception as exc:
            raise ArchiveError("打开归档失败: " + self.path + " (" + str(exc) + ")") from exc

    def names(self) -> list[str]:
        return [normalize_component(n) for n in self.doc.component_names()]

    def try_read(self, name: str) -> bytes | None:
        return self.doc.get(normalize_component(name))

    def read(self, name: str) -> bytes:
        data = self.try_read(name)
        if data is None:
            raise ArchiveError("归档里没有组件: " + normalize_component(name))
        return data

    def write(self, name: str, data: bytes) -> None:
        self.doc.set(normalize_component(name), data)

    def remove(self, name: str) -> bool:
        return self.doc.remove(normalize_component(name))

    def save(self, dest: str) -> dict:
        self._ensure_hash_headroom()
        dest = os.path.abspath(dest)
        parent = os.path.dirname(dest)
        if parent:
            os.makedirs(parent, exist_ok=True)
        return self.doc.save(dest)

    def _ensure_hash_headroom(self) -> None:
        """组件数变多时重算经典 hash 表大小。

        **实测踩过的坑**：sc2mpq 从源归档沿用 hash_count，边长为 8 的表装 8 个条目时，
        表被填满、一个空槽都不剩。而读取器（以及 StormLib 的惯例）要求表里**至少留一个空槽**
        才能判定「这张表是加密过的有效表」，否则会退化成把密文当明文解析 —— 表现就是新加组件
        之后连 (listfile) 都读不出来。这里按 **负载因子不超过 3/4** 预留余量，并且只在需要时
        才放大，没变化时仍沿用原表大小，保持“只改该改的”。
        """
        names = [normalize_component(n) for n in self.doc.component_names()]
        need = len(names) + 2  # 还要放 (listfile) 与 (attributes)
        hash_count = max(4, self.doc.hash_count)
        # 两条件：至少留一个空槽（hash_count >= need + 1），且负载不超过 7/8。
        # 满足时**沿用原表大小**，这样没加组件时产物与原件同尺寸，加了也只长一档
        # —— 实测 Stukov2.SC2Map 原件 50 条目 / 64 槽，加 3 个 zhCN 组件后仍是 64 槽。
        while hash_count < need + 1 or hash_count * 7 < need * 8:
            hash_count <<= 1
        self.doc.hash_count = hash_count


# ---------------------------------------------------------------- 工具函数
def string_component(locale: str, filename: str) -> str:
    return component_name(locale, filename)
