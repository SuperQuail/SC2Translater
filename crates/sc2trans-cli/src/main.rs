//! sc2trans —— 星际争霸 II 地图/模组的翻译键提取与回填（CLI，面向 agent）。
//!
//! 约定：
//!
//! * **stdout 只放结果**（--json 时是一个完整 JSON 对象；否则是人读的摘要）；
//! * **stderr 放进度与告警**，agent 解析 stdout 不会被噪声干扰；
//! * --json 放在子命令前或后都可以；
//! * 退出码：0 成功；1 业务失败；2 用法错误（clap 自己给）。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sc2trans_core::link::{self, Link};
use sc2trans_core::locales::{locale_code_ok, locale_display};
use sc2trans_core::model::status_label;
use sc2trans_core::ops::{self, ApplyOptions};
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(
    name = "sc2trans",
    version,
    about = "星际争霸 II 地图/模组翻译键提取与回填工具",
    long_about = None,
    after_help = "常用流程：\n          先看缺什么        sc2trans audit 地图.SC2Map\n          导出翻译键        sc2trans extract 地图.SC2Map -o keys.json\n          （翻译 keys.json 里的 target 字段）\n          写回地图          sc2trans apply 地图.SC2Map keys.json -o 中文.SC2Map\n          没有译文时的兜底  sc2trans seed 地图.SC2Map --target zhCN\n\n        agent 用法：任何子命令加 --json，stdout 就是一个完整 JSON 对象。"
)]
struct Cli {
    /// 输出机器可读 JSON（agent 用）；放在子命令前后都可以
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 查一份文档该关联哪份同名 JSON（不做任何写操作）
    Link { archive: PathBuf },
    /// 归档概览：容器参数、组件数、存在的语言
    Info { archive: PathBuf },

    /// 列出归档里的语言与字符串表
    List { archive: PathBuf },

    /// 提取翻译键为 JSON
    Extract {
        archive: PathBuf,
        /// JSON 输出路径
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// 原文语言（默认 enUS）
        #[arg(long, default_value = "enUS")]
        source: String,
        /// 目标语言（默认 zhCN）
        #[arg(long, default_value = "zhCN")]
        target: String,
        /// 只处理这些字符串表（可重复）
        #[arg(long = "file")]
        files: Vec<String>,
        /// 把其它语言的键也并入结果
        #[arg(long)]
        all_locales: bool,
    },

    /// 检查每种语言对原文键的覆盖率（定位「显示成路径」的原因）
    Audit {
        archive: PathBuf,
        #[arg(long, default_value = "enUS")]
        source: String,
    },

    /// 把翻译 JSON 写回归档
    Apply {
        archive: PathBuf,
        /// 翻译 JSON；省略则读 stdin
        json_in: Option<PathBuf>,
        /// 输出归档路径（默认 <原名>.<目标语言>.SC2Map）
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// 目标语言（默认取 JSON 里的 target_locale）
        #[arg(long)]
        target: Option<String>,
        /// target 为空时用 source 顶上
        #[arg(long)]
        fill_missing: bool,
        /// 写成「译文 /// 原文」的形式
        #[arg(long)]
        with_original: bool,
        /// 按 key 排序
        #[arg(long = "sort")]
        sort_keys: bool,
        /// 允许覆盖输入文件
        #[arg(long)]
        force: bool,
    },

    /// 把原文整份补进目标语言（没有译文时的兜底修复）
    Seed {
        archive: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "enUS")]
        source: String,
        #[arg(long, default_value = "zhCN")]
        target: String,
        #[arg(long)]
        with_original: bool,
        #[arg(long = "sort")]
        sort_keys: bool,
        #[arg(long)]
        force: bool,
    },

    /// 补全默认语言：用指定语言的文本覆盖那些「没有语言文件」的语言
    Fill {
        archive: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// 填充来源语言
        #[arg(long, default_value = "enUS")]
        source: String,
        /// 要补哪些语言：all（全部）或逗号分隔，如 zhCN,zhTW
        #[arg(long, default_value = "all")]
        locales: String,
        /// 用源语言整份覆盖（默认只补缺的键，不动已有译文）
        #[arg(long)]
        overwrite: bool,
        /// 写成「译文 /// 原文」的形式
        #[arg(long)]
        with_original: bool,
        #[arg(long)]
        force: bool,
    },

    /// 容器自检 + 字符串表往返回读
    Verify {
        archive: PathBuf,
        /// 跳过逐块 CRC/MD5 校验
        #[arg(long)]
        shallow: bool,
    },
}

fn check_locale(code: &str) -> Result<()> {
    if !locale_code_ok(code) {
        anyhow::bail!("语言代码形状不对（应为 2~4 个字母，如 zhCN）: {code}");
    }
    Ok(())
}

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if cli.json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": format!("{err:#}"),
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
            } else {
                eprintln!("错误: {err:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Info { archive } => {
            let v = ops::inspect(archive).with_context(|| format!("读取 {}", archive.display()))?;
            if cli.json {
                return print_json(&v);
            }
            let c = &v["container"];
            println!("文件      {}", v["path"].as_str().unwrap_or(""));
            println!("大小      {} 字节", v["size"].as_u64().unwrap_or(0));
            println!("SHA256    {}", v["sha256"].as_str().unwrap_or(""));
            println!(
                "容器      format={} header={} 组件={} 块={}",
                c["format_version"], c["header_size"], c["components"], c["blocks"]
            );
            println!(
                "索引      HET={} BET={} attributes={} 头部摘要={} dwRawChunkSize={}",
                c["has_het"], c["has_bet"], c["has_attributes"], c["header_md5_valid"], c["raw_chunk_size"]
            );
            let empty = Vec::new();
            let locales = v["locales"].as_array().unwrap_or(&empty);
            let names: Vec<&str> = locales.iter().filter_map(|l| l["locale"].as_str()).collect();
            println!("语言目录  {}", if names.is_empty() { "（无）".to_string() } else { names.join(", ") });
            for loc in locales {
                if let Some(files) = loc["files"].as_object() {
                    for (name, meta) in files {
                        println!(
                            "          {} / {}  {} 字节",
                            loc["locale"].as_str().unwrap_or(""),
                            name,
                            meta["bytes"].as_u64().unwrap_or(0)
                        );
                    }
                }
            }
            Ok(())
        }

        Command::List { archive } => {
            let v = ops::inspect(archive)?;
            let empty = Vec::new();
            let locales = v["locales"].as_array().unwrap_or(&empty);
            for loc in locales {
                let files: Vec<&str> =
                    loc["files"].as_object().map(|m| m.keys().map(|s| s.as_str()).collect()).unwrap_or_default();
                println!("{}: {}", loc["locale"].as_str().unwrap_or(""), files.join(", "));
            }
            if cli.json {
                print_json(&v)?;
            }
            Ok(())
        }

        Command::Extract { archive, out, source, target, files, all_locales } => {
            check_locale(source)?;
            check_locale(target)?;
            let only = if files.is_empty() { None } else { Some(files.clone()) };
            let doc = ops::extract(archive, source, target, only, *all_locales)?;
            let value = serde_json::to_value(&doc)?;
            if let Some(path) = out {
                std::fs::write(path, format!("{}
", serde_json::to_string_pretty(&value)?))
                    .with_context(|| format!("写出 {}", path.display()))?;
                eprintln!("已写出 {}", path.display());
            }
            if cli.json {
                return print_json(&value);
            }
            println!("源语言  {}   目标语言 {}", doc.source_locale, doc.target_locale);
            println!("文件    {}", doc.files.join(", "));
            println!("键总数  {}", doc.stats.entries);
            for (status, count) in &doc.stats.by_status {
                println!("  {:<12} {:<6} {}", status, count, status_label(status));
            }
            println!();
            if let Some(d) = &doc.diagnosis {
                println!("{}", d.message);
            }
            Ok(())
        }

        Command::Link { archive } => {
            let result = link::link(archive);
            let state = match &result {
                Link::Linked { .. } => "linked",
                Link::ForeignSchema { .. } => "foreign_schema",
                Link::Missing { .. } => "missing",
            };
            if cli.json {
                print_json(&serde_json::json!({
                    "document": archive.display().to_string(),
                    "json": result.path().display().to_string(),
                    "state": state,
                }))?;
            } else {
                println!("文档  {}", archive.display());
                println!("同名  {}", result.path().display());
                match &result {
                    Link::Linked { .. } => println!("状态  已关联（是一份 sc2trans.keys.v1）"),
                    Link::ForeignSchema { .. } => {
                        println!("状态  同名 JSON 存在，但不是翻译键格式（schema 不是 sc2trans.keys.*）")
                    }
                    Link::Missing { .. } => println!("状态  不存在（本工具不会自动创建）"),
                }
            }
            Ok(())
        }
        Command::Audit { archive, source } => {
            check_locale(source)?;
            let v = ops::audit(archive, source)?;
            if cli.json {
                return print_json(&v);
            }
            println!("源语言 {}，共 {} 个键
", v["source_locale"].as_str().unwrap_or(""), v["source_keys"]);
            println!("{:10} {:>14} {:>10} {:>10}  覆盖率", "语言", "期望", "已有", "缺失");
            for row in v["locales"].as_array().unwrap_or(&Vec::new()) {
                let coverage = row["coverage"].as_f64().unwrap_or(0.0) * 100.0;
                println!(
                    "{:10} {:>14} {:>10} {:>10}  {:.1}%",
                    row["locale"].as_str().unwrap_or(""),
                    row["keys_expected"],
                    row["keys_present"],
                    row["keys_missing"],
                    coverage
                );
            }
            if let Some(absent) = v["locales_absent"].as_array().filter(|a| !a.is_empty()) {
                let list: Vec<&str> = absent.iter().filter_map(|x| x.as_str()).collect();
                println!("
归档里没有这些语言目录：{}", list.join(", "));
            }
            println!();
            println!("{}", v["diagnosis"]["message"].as_str().unwrap_or(""));
            Ok(())
        }

        Command::Apply {
            archive,
            json_in,
            out,
            target,
            fill_missing,
            with_original,
            sort_keys,
            force,
        } => {
            if let Some(t) = target {
                check_locale(t)?;
            }
            let raw: Value = match json_in {
                Some(path) => serde_json::from_str(
                    &std::fs::read_to_string(path).with_context(|| format!("读取 {}", path.display()))?,
                )
                .with_context(|| format!("解析 {}", path.display()))?,
                None => serde_json::from_reader(std::io::stdin()).context("从 stdin 解析翻译 JSON")?,
            };
            let opts = ApplyOptions {
                out: out.clone(),
                target_locale: target.clone(),
                fill_missing: *fill_missing,
                with_original: *with_original,
                sort: *sort_keys,
                force: *force,
                ..Default::default()
            };
            let result = ops::apply(archive, &raw, &opts)?;
            let value = serde_json::to_value(&result)?;
            if cli.json {
                print_json(&value)?;
            } else {
                println!("已写出 {}", result.output);
                println!("目标语言 {}", result.target_locale);
                for files in result.components_written.values() {
                    for (filename, info) in files {
                        println!(
                            "  {}: 新增 {}，更新 {}，共 {} 条",
                            filename, info.added, info.updated, info.total
                        );
                    }
                }
                println!("回读校验 {}", if result.ok { "通过" } else { "失败" });
            }
            if !result.ok {
                anyhow::bail!("回读校验失败");
            }
            Ok(())
        }

        Command::Seed { archive, out, source, target, with_original, sort_keys, force } => {
            check_locale(source)?;
            check_locale(target)?;
            let opts = ApplyOptions {
                out: out.clone(),
                with_original: *with_original,
                sort: *sort_keys,
                force: *force,
                ..Default::default()
            };
            let result = ops::seed(archive, source, target, &opts)?;
            let value = serde_json::to_value(&result)?;
            if cli.json {
                print_json(&value)?;
            } else {
                println!("已写出 {}", result.output);
                println!(
                    "把 {} 的文本补进 {}，补了 {} 个键",
                    source,
                    locale_display(target),
                    result.seeded.unwrap_or(0)
                );
                println!(
                    "现在用 {target} 进入游戏不会再看到原始键路径（显示的是 {} 的原文，等你替换成译文）。",
                    source
                );
                println!("回读校验 {}", if result.ok { "通过" } else { "失败" });
            }
            if !result.ok {
                anyhow::bail!("回读校验失败");
            }
            Ok(())
        }

        Command::Fill { archive, out, source, locales, overwrite, with_original, force } => {
            check_locale(source)?;
            let targets = ops::FillTargets::parse(locales)?;
            let result = ops::fill_locales(
                archive,
                source,
                &targets,
                !overwrite,
                *with_original,
                out.as_deref(),
                *force,
            )?;
            let value = serde_json::to_value(&result)?;
            if cli.json {
                print_json(&value)?;
            } else {
                println!("已写出 {}", result.output);
                println!(
                    "用 {} 补全了 {} 种语言：",
                    source,
                    result.components_written.len()
                );
                for (locale, files) in &result.components_written {
                    let added: usize = files.values().map(|f| f.added).sum();
                    let names: Vec<&str> = files.keys().map(|s| s.as_str()).collect();
                    println!("  {:<6} 新增 {:>5} 条   {}", locale, added, names.join(", "));
                }
                println!(
                    "共新增 {} 条；回读校验{}",
                    result.seeded.unwrap_or(0),
                    if result.ok { "通过" } else { "失败" }
                );
                println!(
                    "体积 {} -> {} 字节",
                    result.bytes_in, result.bytes_out
                );
            }
            if !result.ok {
                anyhow::bail!("回读校验失败");
            }
            Ok(())
        }

        Command::Verify { archive, shallow } => {
            let v = ops::verify(archive, !shallow)?;
            if cli.json {
                return print_json(&v);
            }
            println!("文件            {}", v["path"].as_str().unwrap_or(""));
            println!("dwRawChunkSize  {}  (必须为 0)", v["raw_chunk_size"]);
            println!("头部自哈希      {}", v["header_md5_valid"]);
            println!("HET 逐字节重建  {}", v["het_roundtrip_exact"]);
            println!("BET 逐字节重建  {}", v["bet_roundtrip_exact"]);
            println!("HET/经典表一致  {}", v["het_vs_classic_mismatch"].as_array().is_none_or(|a| a.is_empty()));
            println!("attributes      md5 ok={} bad={}  crc ok={} bad={}", v["md5_ok"], v["md5_bad"], v["crc_ok"], v["crc_bad"]);
            println!("字符串表往返    {}", v["string_tables_roundtrip_exact"]);
            println!("整体健康        {}", v["healthy"]);
            Ok(())
        }
    }
}
