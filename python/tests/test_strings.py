"""字符串表的解析 / 序列化测试。判据是**逐字节保真**。"""

from __future__ import annotations

import pytest

from sc2trans.strings import RawLine, StringFile

CRLF_BOM = b"\xef\xbb\xbfParam/Value/AB=Hive Destroyed\r\nDocInfo/Name=Domination\r\n"
CRLF_NOBOM = ("DocInfo/Author=Blizzard Entertainment\r\n"
              "DocInfo/Name=\u7edf\u6cbb /// Domination\r\n").encode("utf-8")


def test_parse_reports_bom_and_newline():
    f = StringFile.parse(CRLF_BOM)
    assert f.bom is True
    assert f.newline == "\r\n"
    assert f.trailing_newline is True
    assert f.keys() == ["Param/Value/AB", "DocInfo/Name"]


def test_parse_without_bom_keeps_it_that_way():
    """暴雪官方 zhCN 文件没有 BOM —— 写回时必须保持没有。"""
    f = StringFile.parse(CRLF_NOBOM)
    assert f.bom is False
    assert f.to_bytes() == CRLF_NOBOM


def test_roundtrip_is_byte_exact():
    f = StringFile.parse(CRLF_BOM)
    assert f.to_bytes() == CRLF_BOM


def test_triple_slash_splits_translation_and_original():
    f = StringFile.parse(CRLF_NOBOM)
    pair = f.get("DocInfo/Name")
    assert pair is not None
    assert pair.value == "\u7edf\u6cbb"
    assert pair.original == "Domination"
    assert pair.render() == "DocInfo/Name=\u7edf\u6cbb /// Domination"


def test_value_may_contain_equals_sign():
    """<s val="X"> 这种值里带等号，只能按第一个等号切分。"""
    data = ('LoadingScreen/Title=<s val="VoidLoadingIntroScreenTitle">LEGACY OF THE VOID</s>\r\n').encode()
    f = StringFile.parse(data)
    pair = f.get("LoadingScreen/Title")
    assert pair is not None
    assert pair.value == '<s val="VoidLoadingIntroScreenTitle">LEGACY OF THE VOID</s>'
    assert f.to_bytes() == data


def test_empty_value_is_legal():
    data = b"DocInfo/DescLong=\r\n"
    f = StringFile.parse(data)
    pair = f.get("DocInfo/DescLong")
    assert pair is not None and pair.value == ""
    assert f.to_bytes() == data


def test_comments_and_blank_lines_survive():
    data = b"// a comment\r\n\r\nDocInfo/Name=X\r\n"
    f = StringFile.parse(data)
    assert any(isinstance(r, RawLine) for r in f.records)
    assert f.to_bytes() == data


def test_lf_file_stays_lf():
    data = b"DocInfo/Name=X\nDocInfo/Author=Y\n"
    f = StringFile.parse(data)
    assert f.newline == "\n"
    assert f.to_bytes() == data


def test_set_keeps_existing_original_by_default():
    f = StringFile.parse(CRLF_NOBOM)
    f.set("DocInfo/Name", "\u7edf\u6cbb\u8005")
    pair = f.get("DocInfo/Name")
    assert pair is not None
    assert pair.value == "\u7edf\u6cbb\u8005"
    assert pair.original == "Domination"


def test_set_can_override_original():
    f = StringFile.parse(CRLF_NOBOM)
    f.set("DocInfo/Name", "\u7edf\u6cbb\u8005", "Domination")
    pair = f.get("DocInfo/Name")
    assert pair is not None and pair.original == "Domination"


def test_new_key_appended_without_original():
    f = StringFile.parse(CRLF_NOBOM)
    f.set("New/Key", "value")
    assert f.keys()[-1] == "New/Key"
    pair = f.get("New/Key")
    assert pair is not None and pair.original is None


def test_add_missing_does_not_touch_existing():
    f = StringFile.parse(CRLF_NOBOM)
    added = f.add_missing([("DocInfo/Name", "SHOULD NOT WIN"), ("DocInfo/Website", "https://x")])
    assert added == ["DocInfo/Website"]
    pair = f.get("DocInfo/Name")
    assert pair is not None and pair.value == "\u7edf\u6cbb"


def test_sort_only_reorders_pairs():
    data = b"// head\r\nB=1\r\nA=2\r\n"
    f = StringFile.parse(data)
    f.sort()
    assert [p.key for p in f.pairs()] == ["A", "B"]
    assert isinstance(f.records[0], RawLine)
    assert f.to_bytes() == b"// head\r\nA=2\r\nB=1\r\n"


def test_crlf_preserved_on_new_file():
    f = StringFile()
    f.set("A", "1")
    f.set("B", "2")
    assert f.to_bytes() == b"\xef\xbb\xbfA=1\r\nB=2\r\n"


def test_malformed_line_is_preserved():
    data = b"this line has no equals\r\nA=1\r\n"
    f = StringFile.parse(data)
    assert f.to_bytes() == data


def test_bad_encoding_raises():
    from sc2trans.strings import StringFileError
    with pytest.raises(StringFileError):
        StringFile.parse(b"\xff\xfe\x00\x00bad")
