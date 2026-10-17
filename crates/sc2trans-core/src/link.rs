//! SC2 文档与「同名 JSON」的关联。
//!
//! 约定：@@Maps/Stukov2.SC2Map@@ 对应 @@Maps/Stukov2.json@@ —— 同目录、同主文件名。
//! 这条约定 CLI 侧也要用（@@extract@@ 之后 @@apply@@ 就是它），所以放 core。

use std::path::{Path, PathBuf};

use serde::Serialize;

/// 文档该关联的 JSON 路径（不判断存不存在）。
pub fn sibling_json(document: &Path) -> PathBuf {
    document.with_extension("json")
}

/// 关联结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Link {
    /// 存在，且是我们的格式。
    Linked { path: PathBuf },
    /// 存在，但不是 @@sc2trans.keys.v1@@。
    ForeignSchema { path: PathBuf },
    /// 不存在。
    Missing { path: PathBuf },
}

impl Link {
    pub fn path(&self) -> &Path {
        match self {
            Link::Linked { path } | Link::ForeignSchema { path } | Link::Missing { path } => path,
        }
    }
}

/// 查一份文档的关联情况。
///
/// **不创建文件** —— 在用户目录里凭空多出东西是很烦的事，必须由用户点一下。
pub fn link(document: &Path) -> Link {
    let path = sibling_json(document);
    if !path.is_file() {
        return Link::Missing { path };
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let ours = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("schema").and_then(|s| s.as_str()).map(|s| s.to_string()))
                .map(|s| s.starts_with("sc2trans.keys"))
                .unwrap_or(false);
            if ours {
                Link::Linked { path }
            } else {
                Link::ForeignSchema { path }
            }
        }
        Err(_) => Link::ForeignSchema { path },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_is_same_stem() {
        assert_eq!(
            sibling_json(Path::new("D:/m/Maps/Stukov2.SC2Map")),
            PathBuf::from("D:/m/Maps/Stukov2.json")
        );
        assert_eq!(sibling_json(Path::new("a.SC2Mod")), PathBuf::from("a.json"));
    }

    #[test]
    fn missing_when_no_file() {
        let d = std::env::temp_dir().join("sc2trans_link_test_missing");
        let _ = std::fs::create_dir_all(&d);
        let doc = d.join("X.SC2Map");
        std::fs::write(&doc, b"x").unwrap();
        assert!(matches!(link(&doc), Link::Missing { .. }));
    }

    #[test]
    fn linked_when_schema_matches() {
        let d = std::env::temp_dir().join("sc2trans_link_test_ok");
        let _ = std::fs::create_dir_all(&d);
        let doc = d.join("Y.SC2Map");
        std::fs::write(&doc, b"x").unwrap();
        std::fs::write(d.join("Y.json"), r#"{"schema":"sc2trans.keys.v1","entries":[]}"#).unwrap();
        assert!(matches!(link(&doc), Link::Linked { .. }));
    }

    #[test]
    fn foreign_schema_is_flagged_not_silently_used() {
        let d = std::env::temp_dir().join("sc2trans_link_test_foreign");
        let _ = std::fs::create_dir_all(&d);
        let doc = d.join("Z.SC2Map");
        std::fs::write(&doc, b"x").unwrap();
        std::fs::write(d.join("Z.json"), r#"{"name":"someone elses config"}"#).unwrap();
        assert!(matches!(link(&doc), Link::ForeignSchema { .. }));
    }

    #[test]
    fn link_never_creates_the_file() {
        let d = std::env::temp_dir().join("sc2trans_link_test_create");
        let _ = std::fs::create_dir_all(&d);
        let doc = d.join("W.SC2Map");
        std::fs::write(&doc, b"x").unwrap();
        let _ = link(&doc);
        assert!(!d.join("W.json").exists(), "关联检查不该创建文件");
    }
}
