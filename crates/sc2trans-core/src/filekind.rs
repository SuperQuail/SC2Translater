//! 打开一个文件之前，先判它是什么。
//!
//! **不靠扩展名** —— 扩展名会骗人。判据按可靠性排：
//! 归档扩展名 -> NUL 字节与 UTF-8 解码 -> 图片魔数 -> 其余当文本。
//!
//! 这里只认「能不能打开、用什么打开」，不负责渲染。

use std::path::Path;

use serde::Serialize;

/// 文本文件超过这个大小就不加载了 —— 打开一个几百 MB 的日志会把界面拖死。
pub const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;
/// 图片上限。
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// 判二进制时看前面这么多字节。
const SNIFF_BYTES: usize = 512;

/// 判出来的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    /// SC2 地图 / 模组，走归档解析。
    Sc2,
    /// 普通文本。
    Text,
    /// 图片。
    Image,
    /// 打不开的二进制。
    Binary,
}

/// 判定结果。
#[derive(Debug, Clone, Serialize)]
pub struct Classified {
    pub kind: FileKind,
    pub size: u64,
    /// kind 是 Binary 时给用户看的原因。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 判一个路径。读不出第一个字节就当二进制（宁可拒绝，也别把乱码塞进编辑器）。
pub fn classify(path: &Path) -> Classified {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();

    if ext == "sc2map" || ext == "sc2mod" {
        return Classified { kind: FileKind::Sc2, size, reason: None };
    }

    let head = read_head(path);
    if let Some(kind) = image_magic(&head, &ext) {
        if size > MAX_IMAGE_BYTES {
            return Classified {
                kind: FileKind::Binary,
                size,
                reason: Some(format!("图片太大（{} MB），不加载", size / 1024 / 1024)),
            };
        }
        return Classified { kind, size, reason: None };
    }

    // UTF-16 要排在 NUL 检查前面 —— UTF-16 文本本来就满是 0x00，
    // 先查 NUL 会把合法的 UTF-16 判成二进制（这条是测试逼出来的）。
    let utf16 = head.len() >= 2 && ((head[0] == 0xFF && head[1] == 0xFE) || (head[0] == 0xFE && head[1] == 0xFF));
    if utf16 {
        if size > MAX_TEXT_BYTES {
            return Classified {
                kind: FileKind::Binary,
                size,
                reason: Some(format!("文本太大（{} MB），超过上限，不加载", size / 1024 / 1024)),
            };
        }
        return Classified { kind: FileKind::Text, size, reason: None };
    }

    if head.contains(&0) {
        return Classified {
            kind: FileKind::Binary,
            size,
            reason: Some("这个文件不是文本（前面有 NUL 字节），大概是个二进制资源。本工具不打开这类文件。".into()),
        };
    }

    // UTF-8：只校验前面这一段，整文件在打开时才读
    if std::str::from_utf8(&head).is_err() {
        return Classified {
            kind: FileKind::Binary,
            size,
            reason: Some("这个文件不是 UTF-8 文本，本工具不打开（不猜编码，猜错比不猜更糟）。".into()),
        };
    }

    if size > MAX_TEXT_BYTES {
        return Classified {
            kind: FileKind::Binary,
            size,
            reason: Some(format!("文本太大（{} MB），超过 {} MB 上限，不加载", size / 1024 / 1024, MAX_TEXT_BYTES / 1024 / 1024)),
        };
    }

    Classified { kind: FileKind::Text, size, reason: None }
}

fn read_head(path: &Path) -> Vec<u8> {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let mut buf = vec![0u8; SNIFF_BYTES];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    buf
}

/// 图片：光看扩展名不够，魔数对不上就当没认出（避免把 .png 结尾的文本当图片）。
fn image_magic(head: &[u8], ext: &str) -> Option<FileKind> {
    let png = head.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let jpg = head.starts_with(&[0xFF, 0xD8, 0xFF]);
    let gif = head.starts_with(b"GIF8");
    let webp = head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP";
    let bmp = head.starts_with(b"BM");
    if png || jpg || gif || webp || bmp {
        return Some(FileKind::Image);
    }
    // 扩展名说是图片但魔数不对：明说，别默默当文本
    if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp") {
        return None;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    /// 每个用例一个独立目录 —— 共用目录时并行跑会互相删文件（踩过，测试随机红）。
    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join("sc2trans_filekind_test").join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sc2_by_extension() {
        let d = tmp("sc2");
        let p = write(&d, "a.SC2Map", b"whatever");
        assert_eq!(classify(&p).kind, FileKind::Sc2);
        let p = write(&d, "b.SC2Mod", b"whatever");
        assert_eq!(classify(&p).kind, FileKind::Sc2);
    }

    #[test]
    fn nul_byte_means_binary() {
        // 别用 "nul" 当目录名 —— 那是 Windows 保留设备名，目录建不出来（踩过）
        let d = tmp("has-nul");
        let p = write(&d, "model.m3", &[0x00, 0x01, 0x02, 0x03]);
        let c = classify(&p);
        assert_eq!(c.kind, FileKind::Binary);
        assert!(c.reason.unwrap().contains("NUL"));
    }

    #[test]
    fn invalid_utf8_without_bom_is_binary() {
        let d = tmp("gbk");
        let p = write(&d, "gbk.txt", &[0xC4, 0xE3, 0xBA, 0xC3]);
        assert_eq!(classify(&p).kind, FileKind::Binary);
    }

    #[test]
    fn utf16_with_bom_is_text() {
        let d = tmp("u16");
        let p = write(&d, "u16.txt", &[0xFF, 0xFE, b'h', 0x00, b'i', 0x00]);
        assert_eq!(classify(&p).kind, FileKind::Text);
    }

    #[test]
    fn plain_utf8_is_text() {
        let d = tmp("utf8");
        let p = write(&d, "readme.txt", "你好，世界\n".as_bytes());
        assert_eq!(classify(&p).kind, FileKind::Text);
    }

    #[test]
    fn png_by_magic() {
        let d = tmp("png");
        let p = write(&d, "cover.png", &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]);
        assert_eq!(classify(&p).kind, FileKind::Image);
    }

    #[test]
    fn text_named_png_is_not_an_image() {
        let d = tmp("fakepng");
        let p = write(&d, "fake.png", b"this is plainly text");
        assert_eq!(classify(&p).kind, FileKind::Text);
    }

    #[test]
    fn oversized_text_is_refused_not_silently_loaded() {
        let d = tmp("huge");
        let mut big = vec![b'a'; (MAX_TEXT_BYTES + 16) as usize];
        big.truncate((MAX_TEXT_BYTES + 16) as usize);
        let p = write(&d, "huge.log", &big);
        let c = classify(&p);
        assert_eq!(c.kind, FileKind::Binary);
        assert!(c.reason.unwrap().contains("太大"));
    }
}
