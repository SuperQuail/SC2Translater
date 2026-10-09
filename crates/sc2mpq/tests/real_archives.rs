//! 用**真实归档**做验收。
//!
//! 判据来自 SC2Diff 的实测结论：
//!
//! * HET / BET 从模型重新序列化后必须与磁盘字节**逐字节相同**（最强证据：模型无信息损失）；
//! * 头部六个 MD5 摘要全部有效；
//! * HET 查找与经典哈希表查找的结果逐名一致（含高 8 位哈希碰撞的名字）；
//! * BET 的 (pos,size,cmp) 与经典块表逐条相等，flag 索引也一致；
//! * (listfile)/(attributes) 的 MD5 全零约定导致 md5_bad 恒为 2、crc_bad 恒为 1。
//!
//! 找不到样本时**跳过**，这样在没有游戏/样本的机器上也能跑测试。

use std::path::{Path, PathBuf};

use sc2mpq::{Archive, Document};

fn sample_dir() -> PathBuf {
    std::env::var("SC2TRANS_TESTDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"D:\Code\Rust\SC2Diff\testdata\samples"))
}

fn samples() -> Vec<PathBuf> {
    let dir = sample_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            if ext == "sc2map" || ext == "sc2mod" {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn real_archives_parse_and_verify() {
    let samples = samples();
    if samples.is_empty() {
        eprintln!("跳过：在 {} 下找不到 .SC2Map/.SC2Mod 样本", sample_dir().display());
        return;
    }
    for path in &samples {
        let arc = Archive::open(path).expect("应能打开");
        assert_eq!(arc.format_version, 3, "{}", path.display());
        assert_eq!(arc.header_size, 208, "{}", path.display());
        assert!(arc.het.is_some(), "SC2 文档必须有 HET: {}", path.display());
        assert!(arc.bet.is_some(), "SC2 文档必须有 BET: {}", path.display());

        let r = arc.verify(true);
        assert!(r.header_md5_valid, "头部自哈希应当有效: {}", path.display());
        assert!(r.block_table_md5_valid, "{}", path.display());
        assert!(r.hash_table_md5_valid, "{}", path.display());
        assert!(r.het_md5_valid, "{}", path.display());
        assert!(r.bet_md5_valid, "{}", path.display());
        assert!(r.het_roundtrip_exact, "HET 应能逐字节重生成: {}", path.display());
        assert!(r.bet_roundtrip_exact, "BET 应能逐字节重生成: {}", path.display());
        assert!(r.het_vs_classic_mismatch.is_empty(), "HET 与经典表结果应当一致: {:?}", r.het_vs_classic_mismatch);
        assert!(r.bet_vs_blocktable_mismatch.is_empty(), "BET 与块表应当一致");
        assert!(r.bet_flag_mismatch.is_empty(), "BET flag 索引应当一致");
        // (listfile) 与 (attributes) 的 MD5 约定为全零 -> bad 恒为 2；后者 CRC 约定为 0 -> bad 恒为 1
        assert_eq!(r.md5_bad, 2, "{}", path.display());
        assert_eq!(r.crc_bad, 1, "{}", path.display());
        // 暴雪原件在 0x6C 写的是 v4 才有的 16384 —— 这正是「编辑器拒收任何改动」的根因
        assert_eq!(arc.raw_chunk_size, 16384, "原件应当带这个自相矛盾的值: {}", path.display());

        // 每个列出的名字都要能读出来，且长度与块表声明一致
        for name in arc.component_names() {
            let data = arc.read(&name).unwrap_or_else(|e| panic!("读 {name} 失败: {e}"));
            let idx = arc.find_block(&name).unwrap();
            assert_eq!(data.len() as u32, arc.block_table[idx as usize].file_size);
        }
    }
    eprintln!("已验收 {} 个真实归档", samples.len());
}

#[test]
fn rebuild_is_healthy_and_preserves_components() {
    let samples = samples();
    let Some(first) = samples.first().cloned() else {
        eprintln!("跳过：没有样本");
        return;
    };
    rebuild_one(&first);
}

fn rebuild_one(src: &Path) {
    let dir = std::env::temp_dir().join("sc2mpq_rebuild");
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("rebuilt.SC2Map");

    let before = Archive::open(src).unwrap();
    const NEW_COMPONENT: &str = "sc2trans-probe\\Probe.bin";
    const LOCALE_COMPONENT: &str = "zhCN.SC2Data\\LocalizedData\\GameStrings.txt";
    // UTF-8 BOM + 「DocInfo/Name=测试」+ CRLF（字节字面量不能写 unicode 转义）
    let zhcn_payload = b"\xef\xbb\xbfDocInfo/Name=\xe6\xb5\x8b\xe8\xaf\x95\r\n".to_vec();

    assert!(!before.has(NEW_COMPONENT), "探针组件不该已经存在");

    let mut doc = Document::open(src).unwrap();
    doc.set(NEW_COMPONENT, b"probe".to_vec());
    doc.set(LOCALE_COMPONENT, zhcn_payload.clone());
    doc.save(&dest).unwrap();

    let after = Archive::open(&dest).unwrap();
    let report = after.verify(true);
    assert!(report.healthy(), "重建件应当完全自洽: {report:?}");
    assert_eq!(after.raw_chunk_size, 0, "写回必须把 dwRawChunkSize 置 0");
    assert!(after.has("(listfile)"));
    assert_eq!(after.read(NEW_COMPONENT).unwrap(), b"probe");
    assert_eq!(after.read(LOCALE_COMPONENT).unwrap(), zhcn_payload);

    // 未被改动的组件必须逐字节相同（改动过的两个单独比对）
    let mut checked = 0usize;
    for name in before.component_names() {
        if name == NEW_COMPONENT || name == LOCALE_COMPONENT {
            continue;
        }
        let a = before.read(&name).unwrap();
        let b = after.read(&name).unwrap();
        assert_eq!(a, b, "组件 {name} 不应被改动");
        checked += 1;
    }
    assert!(checked > 0, "至少要检查到一个组件");
    assert_eq!(after.component_names().len(), before.component_names().len() + 1);

    let _ = std::fs::remove_file(&dest);
    eprintln!("重建验收通过：{} 个组件原样保留", checked);
}
