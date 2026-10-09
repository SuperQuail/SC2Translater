//! 领域层的端到端测试：造一个真实 MPQ 归档 -> extract -> seed/apply -> 回读校验。
//!
//! 端到端验收：在真实归档上跑完整流程。

use std::path::{Path, PathBuf};

use sc2mpq::{Archive, BuildOptions, build_document};
use sc2trans_core::model::{STATUS_IDENTICAL, STATUS_MISSING, STATUS_TRANSLATED};
use sc2trans_core::ops::{self, ApplyOptions};
use sc2trans_core::strings::{Newline, StringFile};
use serde_json::json;

const EN_GAME: [(&str, &str); 5] = [
    ("DocInfo/Name", "Domination"),
    ("DocInfo/Author", "Blizzard Entertainment"),
    ("MapInfo/Player01/Name", "Kerrigan's Brood"),
    ("Param/Value/0F5D0539", "Hive Destroyed"),
    ("LoadingScreen/Title", "<s val=\"X\">LEGACY OF THE VOID</s>"),
];
const EN_OBJECT: [(&str, &str); 1] = [("Unit/EditorSuffix/Purifier", "Mothership")];
const ZH_GAME: [(&str, &str); 1] = [("DocInfo/Name", "统治")];
const BINARY_LEN: usize = 10 * 1024;

fn build_fixture(path: &Path) {
    let mut en = StringFile::default();
    for (k, v) in EN_GAME {
        en.set(k, v);
    }
    let mut en_obj = StringFile::default();
    for (k, v) in EN_OBJECT {
        en_obj.set(k, v);
    }
    let mut zh = StringFile::default();
    for (k, v) in ZH_GAME {
        zh.set(k, v);
    }
    let binary: Vec<u8> = (0..BINARY_LEN).map(|i| (i % 251) as u8).collect();
    let entries = vec![
        ("Base.SC2Data\\GameData\\UnitData.xml".to_string(), binary),
        ("MapInfo".to_string(), b"<MapInfo/>".to_vec()),
        ("enUS.SC2Data\\LocalizedData\\GameStrings.txt".to_string(), en.to_bytes()),
        ("enUS.SC2Data\\LocalizedData\\ObjectStrings.txt".to_string(), en_obj.to_bytes()),
        ("zhCN.SC2Data\\LocalizedData\\GameStrings.txt".to_string(), zh.to_bytes()),
    ];
    let info = build_document(&entries, path, &BuildOptions::default()).expect("构造夹具失败");
    assert!(info.bytes > 0);
}

fn fixture(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join("fixture.SC2Map");
    build_fixture(&path);
    path
}

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join("sc2trans_core_e2e").join(name)
}

#[test]
fn extract_reports_missing_keys_with_source_text() {
    let src = fixture(&tmp("extract"));
    let doc = ops::extract(&src, "enUS", "zhCN", None, false).unwrap();
    assert_eq!(doc.schema, "sc2trans.keys.v1");
    let by_status = &doc.stats.by_status;
    // enUS 的 5 条 GameStrings + 1 条 ObjectStrings 里，zhCN 只有 DocInfo/Name
    assert_eq!(by_status.get(STATUS_MISSING).copied().unwrap_or(0), 5);
    assert_eq!(by_status.get(STATUS_TRANSLATED).copied().unwrap_or(0), 1);

    let entry = doc
        .entries
        .iter()
        .find(|e| e.key == "Param/Value/0F5D0539")
        .expect("应当有这个键");
    assert_eq!(entry.source.as_deref(), Some("Hive Destroyed"));
    assert!(entry.target.is_none());
    assert_eq!(entry.status, STATUS_MISSING);

    let diagnosis = doc.diagnosis.unwrap();
    assert_eq!(diagnosis.severity, "warning");
    assert!(diagnosis.message.contains("原始键路径"));
}

#[test]
fn audit_shows_low_coverage_for_target() {
    let src = fixture(&tmp("audit"));
    let report = ops::audit(&src, "enUS").unwrap();
    assert_eq!(report["source_keys"], 6);
    let rows = report["locales"].as_array().unwrap();
    let zhcn = rows.iter().find(|r| r["locale"] == "zhCN").expect("应当有 zhCN 行");
    assert_eq!(zhcn["keys_expected"], 6);
    assert_eq!(zhcn["keys_present"], 1);
    assert_eq!(zhcn["keys_missing"], 5);
    assert_eq!(report["diagnosis"]["severity"], "warning");
}

#[test]
fn seed_completes_coverage_and_preserves_other_components() {
    let src = fixture(&tmp("seed"));
    let out = tmp("seed").join("seeded.SC2Map");
    let result = ops::seed(&src, "enUS", "zhCN", &ApplyOptions { out: Some(out.clone()), ..Default::default() })
        .unwrap();
    assert!(result.ok, "回读校验应当通过: {result:?}");

    // 补完之后：zhCN 不再缺键，且已有的中文不被英文覆盖
    let after = ops::extract(&out, "enUS", "zhCN", None, false).unwrap();
    assert_eq!(after.stats.by_status.get(STATUS_MISSING).copied().unwrap_or(0), 0);
    assert_eq!(after.stats.by_status.get(STATUS_IDENTICAL).copied().unwrap_or(0), 5);
    assert_eq!(after.stats.by_status.get(STATUS_TRANSLATED).copied().unwrap_or(0), 1);

    // 未改动的组件必须逐字节相同
    let before_arc = Archive::open(&src).unwrap();
    let after_arc = Archive::open(&out).unwrap();
    for name in ["Base.SC2Data\\GameData\\UnitData.xml", "MapInfo"] {
        assert_eq!(
            before_arc.read(name).unwrap(),
            after_arc.read(name).unwrap(),
            "组件 {name} 不应被改动"
        );
    }

    // zhCN.ObjectStrings 原本不存在，seed 应当把它建出来，并沿用源文件的 BOM + CRLF
    let created = after_arc.read("zhCN.SC2Data\\LocalizedData\\ObjectStrings.txt").unwrap();
    let table = StringFile::parse(&created, "zhCN ObjectStrings").unwrap();
    assert!(table.bom);
    assert_eq!(table.newline, Newline::Crlf);
    assert_eq!(table.get("Unit/EditorSuffix/Purifier").unwrap().value, "Mothership");

    // 中文标题不能被覆盖
    let zh = StringFile::parse(&after_arc.read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt").unwrap(), "zh").unwrap();
    assert_eq!(zh.get("DocInfo/Name").unwrap().value, "统治");
}

#[test]
fn apply_writes_translations_and_keeps_original_comment() {
    let src = fixture(&tmp("apply"));
    let mut doc = ops::extract(&src, "enUS", "zhCN", None, false).unwrap();
    for entry in doc.entries.iter_mut() {
        if entry.status == STATUS_MISSING || entry.status == STATUS_IDENTICAL {
            entry.target = Some(format!("译:{}", entry.source.clone().unwrap_or_default()));
        }
    }
    let out = tmp("apply").join("applied.SC2Map");
    let raw = serde_json::to_value(&doc).unwrap();
    let result = ops::apply(
        &src,
        &raw,
        &ApplyOptions { out: Some(out.clone()), target_locale: Some("zhCN".into()), with_original: true, ..Default::default() },
    )
    .unwrap();
    assert!(result.ok);

    let table = StringFile::parse(
        &Archive::open(&out).unwrap().read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt").unwrap(),
        "zh",
    )
    .unwrap();
    let pair = table.get("Param/Value/0F5D0539").unwrap();
    assert_eq!(pair.value, "译:Hive Destroyed");
    assert_eq!(pair.original.as_deref(), Some("Hive Destroyed"));
}

#[test]
fn apply_accepts_flat_json() {
    let src = fixture(&tmp("flat"));
    let out = tmp("flat").join("flat.SC2Map");
    let raw = json!({ "DocInfo/Name": "统治", "DocInfo/Website": "https://example.org" });
    let result = ops::apply(
        &src,
        &raw,
        &ApplyOptions { out: Some(out.clone()), target_locale: Some("zhCN".into()), ..Default::default() },
    )
    .unwrap();
    assert!(result.ok);
    let table = StringFile::parse(
        &Archive::open(&out).unwrap().read("zhCN.SC2Data\\LocalizedData\\GameStrings.txt").unwrap(),
        "zh",
    )
    .unwrap();
    assert_eq!(table.get("DocInfo/Website").unwrap().value, "https://example.org");
}

#[test]
fn apply_refuses_to_overwrite_input_without_force() {
    let src = fixture(&tmp("refuse"));
    let raw = json!({ "DocInfo/Name": "x" });
    let opts = ApplyOptions { out: Some(src.clone()), target_locale: Some("zhCN".into()), ..Default::default() };
    assert!(ops::apply(&src, &raw, &opts).is_err(), "不应当允许静默覆盖输入文件");
    let opts_force = ApplyOptions { force: true, ..opts };
    assert!(ops::apply(&src, &raw, &opts_force).is_ok(), "加了 force 就应当允许");
}

#[test]
fn verify_reports_healthy_container_and_roundtrip() {
    let src = fixture(&tmp("verify"));
    let report = ops::verify(&src, true).unwrap();
    assert_eq!(report["raw_chunk_size"], 0);
    assert_eq!(report["header_md5_valid"], true);
    assert_eq!(report["het_roundtrip_exact"], true);
    assert_eq!(report["bet_roundtrip_exact"], true);
    assert_eq!(report["string_tables_roundtrip_exact"], true);
    assert_eq!(report["healthy"], true);
    // 簿记项约定：(listfile) 与 (attributes) 的 MD5 全零、后者 CRC 为 0
    assert_eq!(report["md5_bad"], 2);
    assert_eq!(report["crc_bad"], 1);
}

#[test]
fn inspect_lists_locale_tables() {
    let src = fixture(&tmp("inspect"));
    let info = ops::inspect(&src).unwrap();
    assert_eq!(info["container"]["format_version"], 3);
    assert_eq!(info["container"]["header_size"], 208);
    let locales: Vec<&str> = info["locales"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["locale"].as_str())
        .collect();
    assert!(locales.contains(&"enUS"));
    assert!(locales.contains(&"zhCN"));
}
