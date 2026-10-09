"""端到端：造一个真实 MPQ 归档 -> extract -> seed/apply -> 回读校验。

判据有三条：
1. 补完之后目标语言不再缺键（游戏里不会再显示原始键路径）；
2. 没被改动的组件在产物里**逐字节相同**；
3. 产出的归档能被自己的读取器完整打开并自洽（HET/BET/attributes 全部校验通过）。
"""

from __future__ import annotations

import json
import os

import pytest

from sc2trans import model, ops
from sc2trans.archive import Archive, ArchiveError
from sc2trans.strings import StringFile
from sc2trans.vendor import sc2mpq

EN_GAME = [
    ("DocInfo/Name", "Domination"),
    ("DocInfo/Author", "Blizzard Entertainment"),
    ("MapInfo/Player01/Name", "Kerrigan's Brood"),
    ("Param/Value/0F5D0539", "Hive Destroyed"),
    ("LoadingScreen/Title", '<s val="X">LEGACY OF THE VOID</s>'),
]
EN_OBJECT = [("Unit/EditorSuffix/Purifier", "Mothership")]
ZH_GAME = [("DocInfo/Name", "统治")]
BINARY = bytes(range(256)) * 40


def build_fixture(path: str) -> None:
    en = StringFile()
    for key, value in EN_GAME:
        en.set(key, value)
    en_obj = StringFile()
    for key, value in EN_OBJECT:
        en_obj.set(key, value)
    zh = StringFile()
    for key, value in ZH_GAME:
        zh.set(key, value)
    entries = [
        ("Base.SC2Data\\GameData\\UnitData.xml", BINARY),
        ("MapInfo", b"<MapInfo/>"),
        ("enUS.SC2Data\\LocalizedData\\GameStrings.txt", en.to_bytes()),
        ("enUS.SC2Data\\LocalizedData\\ObjectStrings.txt", en_obj.to_bytes()),
        ("zhCN.SC2Data\\LocalizedData\\GameStrings.txt", zh.to_bytes()),
    ]
    sc2mpq.build_document(entries, path)


@pytest.fixture()
def fixture(tmp_path):
    src = tmp_path / "fixture.SC2Map"
    build_fixture(str(src))
    return str(src)


def test_fixture_is_readable(fixture):
    arc = Archive(fixture)
    assert "enUS.SC2Data\\LocalizedData\\GameStrings.txt" in arc.names
    assert sorted(arc.locale_tables()) == ["enUS", "zhCN"]
    report = arc.arc.verify(deep=True)
    assert report["header_md5_valid"] is True
    assert report["het_roundtrip_exact"] is True
    assert report["bet_roundtrip_exact"] is True


def test_extract_reports_missing_keys(fixture):
    doc = ops.extract(fixture, source_locale="enUS", target_locale="zhCN")
    assert doc["schema"] == model.SCHEMA_ID
    by_status = doc["stats"]["by_status"]
    assert by_status["missing"] == 5      # enUS GameStrings 5 条 + ObjectStrings 1 条里 zhCN 都没有
    assert by_status["translated"] == 1   # DocInfo/Name 已有中文
    # 每条都带英文原文，翻译者不用回去翻归档
    entry = next(e for e in doc["entries"] if e["key"] == "Param/Value/0F5D0539")
    assert entry["source"] == "Hive Destroyed"
    assert entry["target"] is None
    assert entry["status"] == "missing"


def test_extract_diagnosis_is_actionable(fixture):
    doc = ops.extract(fixture, source_locale="enUS", target_locale="zhCN")
    assert doc["diagnosis"]["severity"] == "warning"
    assert "原始键路径" in doc["diagnosis"]["message"]


def test_audit_shows_low_coverage(fixture):
    report = ops.audit(fixture, source_locale="enUS")
    zhcn = next(r for r in report["locales"] if r["locale"] == "zhCN")
    assert zhcn["keys_expected"] == 6
    assert zhcn["keys_present"] == 1
    assert zhcn["coverage"] == pytest.approx(1 / 6, abs=1e-4)


def test_seed_fills_target_and_preserves_other_components(fixture, tmp_path):
    out = str(tmp_path / "seeded.SC2Map")
    result = ops.seed(fixture, source_locale="enUS", target_locale="zhCN", out=out)
    assert result["ok"] is True
    assert os.path.getsize(out) > 0

    after = ops.extract(out, source_locale="enUS", target_locale="zhCN")
    assert after["stats"]["by_status"].get("missing", 0) == 0
    assert after["stats"]["by_status"].get("identical", 0) == 5
    assert after["stats"]["by_status"].get("translated", 0) == 1

    # 未改动的组件必须逐字节相同
    src_arc, dst_arc = Archive(fixture), Archive(out)
    for name in ("Base.SC2Data\\GameData\\UnitData.xml", "MapInfo"):
        assert src_arc.read(name) == dst_arc.read(name)
    # 已有中文的那条不能被英文覆盖
    zh = StringFile.parse(dst_arc.read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt"))
    pair = zh.get("DocInfo/Name")
    assert pair is not None and pair.value == "统治"


def test_seed_creates_missing_locale_component(fixture, tmp_path):
    """ObjectStrings 在 zhCN 下原本不存在，seed 后必须被创建出来。"""
    out = str(tmp_path / "seeded2.SC2Map")
    ops.seed(fixture, source_locale="enUS", target_locale="zhCN", out=out)
    arc = Archive(out)
    assert "zhCN.SC2Data\\LocalizedData\\ObjectStrings.txt" in arc.names
    table = StringFile.parse(arc.read("zhCN.SC2Data\\LocalizedData\\ObjectStrings.txt"))
    pair = table.get("Unit/EditorSuffix/Purifier")
    assert pair is not None and pair.value == "Mothership"
    # 新建文件沿用源语言的编码习惯（BOM + CRLF）
    assert table.bom is True
    assert table.newline == "\r\n"


def test_seed_roundtrips_through_apply_with_real_translation(fixture, tmp_path):
    keys = ops.extract(fixture, source_locale="enUS", target_locale="zhCN")
    for entry in keys["entries"]:
        if entry["status"] in ("missing", "identical"):
            entry["target"] = "译:" + (entry["source"] or "")
    out = str(tmp_path / "applied.SC2Map")
    result = ops.apply(fixture, keys, out=out, target_locale="zhCN", with_original=True)
    assert result["ok"] is True
    table = StringFile.parse(Archive(out).read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt"))
    pair = table.get("Param/Value/0F5D0539")
    assert pair is not None
    assert pair.value == "译:Hive Destroyed"
    assert pair.original == "Hive Destroyed"


def test_apply_accepts_flat_json(fixture, tmp_path):
    out = str(tmp_path / "flat.SC2Map")
    result = ops.apply(fixture, {"DocInfo/Name": "统治", "DocInfo/Website": "https://example.org"},
                       out=out, target_locale="zhCN")
    assert result["ok"] is True
    table = StringFile.parse(Archive(out).read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt"))
    assert table.get("DocInfo/Website").value == "https://example.org"  # type: ignore[union-attr]


def test_apply_refuses_to_overwrite_input(fixture):
    with pytest.raises(ArchiveError):
        ops.apply(fixture, {"DocInfo/Name": "x"}, out=fixture, target_locale="zhCN")


def test_apply_can_overwrite_with_force(fixture):
    ops.apply(fixture, {"DocInfo/Name": "覆盖"}, out=fixture, target_locale="zhCN", force=True)
    table = StringFile.parse(Archive(fixture).read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt"))
    assert table.get("DocInfo/Name").value == "覆盖"  # type: ignore[union-attr]


def test_verify_reports_string_table_roundtrip(fixture):
    report = ops.verify(fixture)
    assert report["string_tables_roundtrip_exact"] is True
    # (listfile) 与 (attributes) 的 MD5 按约定是全零，(attributes) 的 CRC 是 0；
    # 所以「坏人」恒为 2 / 1，这是 SC2Diff 实测出来的簿记项约定，不是错误。
    assert report["md5_bad"] == 2
    assert report["crc_bad"] == 1


def test_dump_and_load_roundtrip(fixture, tmp_path):
    doc = ops.extract(fixture, source_locale="enUS", target_locale="zhCN")
    path = str(tmp_path / "keys.json")
    model.dump(doc, path)
    again = model.load(path)
    assert again["stats"] == doc["stats"]
    assert json.dumps(again, ensure_ascii=False) == json.dumps(doc, ensure_ascii=False)
