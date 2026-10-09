//! # sc2trans-core —— 星际争霸 II 汉化工具的领域核心
//!
//! 这一层只做「翻译键」这件事，不含任何 GUI / CLI 依赖，便于被 CLI、Tauri 壳与测试共用。
//!
//! ## 要解决的问题
//!
//! 英文地图只带 enUS.SC2Data。星际争霸 II **不会**在语言缺失时回退到英文，它只加载与
//! 当前游戏语言同名的那一份。于是游戏语言设为中文时，地图里所有自定义键
//! （DocInfo/Name、MapInfo/Player01/Name、Param/Value/XXXXXXXX …）都查不到，
//! 游戏就把**原始键路径**当成文本显示出来。
//!
//! 三个动作解决它：
//!
//! * [`ops::extract`] —— 导出所有翻译键及英文原文（JSON）；
//! * [`ops::apply`]   —— 把 JSON（含译文）写回归档的 zhCN.SC2Data；
//! * [`ops::seed`]    —— 没有译文时的兜底：先用英文把键补满，至少不再显示键路径。
//!
//! ## 模块
//!
//! * [`project`] —— 项目模型（一组文档 + 语言对 + 最近项目）
//! * [`glossary`] —— 专有名词库（全局 / 项目两个作用域，检索与导入导出）
//! * [`mt`]       —— 机翻参考（离线术语拼装 / 任意 OpenAI 兼容接口，带缓存）
//! * [`vcs`]      —— 内置 SC2Diff 的静默安装与 git 形状的版本管理
//! * [`locales`] —— locale 常量与归档内组件命名规则
//! * [`strings`] —— 本地化字符串表的解析 / 序列化（BOM、CRLF、\`/// 原文\` 全部保真）
//! * [`model`]   —— JSON 结构（schema = sc2trans.keys.v1）
//! * [`ops`]     —— info / extract / audit / apply / seed / verify

pub mod filekind;
pub mod glossary;
pub mod link;
pub mod locales;
pub mod mt;
pub mod model;
pub mod project;
pub mod ops;
pub mod strings;
pub mod timeutil;
pub mod vcs;

/// 工具标识，写进导出 JSON 的 tool 字段。
pub const TOOL_ID: &str = concat!("sc2trans ", env!("CARGO_PKG_VERSION"));

/// JSON 结构标识。
pub const SCHEMA_ID: &str = "sc2trans.keys.v1";

/// 领域错误。
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("容器错误: {0}")]
    Mpq(#[from] sc2mpq::MpqError),
    #[error("字符串表错误: {0}")]
    StringFile(#[from] strings::StringFileError),
    #[error("参数错误: {0}")]
    Invalid(String),
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
