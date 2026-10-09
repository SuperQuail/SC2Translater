//! SC2 的 locale 常量，以及归档内「本地化组件」的命名规则。
//!
//! 事实来源：docs/research/sc2-localization.md。
//!
//! * 归档内组件名用**反斜杠**分隔，形如 zhCN.SC2Data\\LocalizedData\\GameStrings.txt。
//! * 与语言无关的部分放在 Base.SC2Data，任何语言都会加载。
//! * 与语言相关的部分放在 <locale>.SC2Data，**只有当前游戏语言对应的那一份会被加载**。
//!   这就是「英文地图在中文环境下显示键路径」的根因。

/// SC2 常见语言目录（用于显示名与「常见语言」提示，**不是**取值白名单）。
pub const LOCALES: [&str; 12] = [
    "enUS", "deDE", "esES", "esMX", "frFR", "itIT", "koKR", "plPL", "ptBR", "ruRU", "zhCN", "zhTW",
];

/// 语言无关组件集的名字（任何语言都会加载）。
pub const BASE: &str = "Base";

pub const LOCALE_DATA_SUFFIX: &str = ".SC2Data";
pub const LOCALIZED_DIR: &str = "LocalizedData";

/// 已知的字符串表文件名。
pub const KNOWN_STRING_FILES: [&str; 3] = ["GameStrings.txt", "ObjectStrings.txt", "TriggerStrings.txt"];

/// **面向玩家**的字符串表 —— 决定游戏里会不会出现原始键路径。
pub const PLAYER_FACING_FILES: [&str; 2] = ["GameStrings.txt", "ObjectStrings.txt"];

/// 只喂给银河编辑器触发器界面的表 —— 玩家在游戏里看不到。
///
/// 实测依据：暴雪官方战役地图的非英文语言目录里根本没有 TriggerStrings.txt。
pub const EDITOR_FACING_FILES: [&str; 1] = ["TriggerStrings.txt"];

/// 语言代码的显示名。
pub fn locale_display(locale: &str) -> &str {
    match locale {
        "enUS" => "英语（美国）",
        "deDE" => "德语",
        "esES" => "西班牙语（西班牙）",
        "esMX" => "西班牙语（墨西哥）",
        "frFR" => "法语",
        "itIT" => "意大利语",
        "koKR" => "韩语",
        "plPL" => "波兰语",
        "ptBR" => "葡萄牙语（巴西）",
        "ruRU" => "俄语",
        "zhCN" => "简体中文",
        "zhTW" => "繁体中文",
        "Base" => "语言无关（Base）",
        other => other,
    }
}

/// 这张表是否面向玩家。
pub fn is_player_facing(filename: &str) -> bool {
    PLAYER_FACING_FILES.iter().any(|f| f.eq_ignore_ascii_case(filename))
}

/// 语言代码只校验形状。
///
/// 实测（本机 183 个真实归档抽样）：归档里还出现过 enCN / enGB / enSG / enTW / jaJP / zhCH，
/// 所以不能拿官方那 12 个当白名单。
pub fn locale_code_ok(code: &str) -> bool {
    let n = code.len();
    (2..=4).contains(&n) && code.bytes().all(|b| b.is_ascii_alphabetic())
}

/// 统一组件名分隔符为反斜杠。
pub fn normalize_component(name: &str) -> String {
    name.replace('/', "\\").trim_matches('\\').to_string()
}

/// zhCN -> zhCN.SC2Data
pub fn locale_data_dir(locale: &str) -> String {
    format!("{locale}{LOCALE_DATA_SUFFIX}")
}

/// 构造归档内组件名。
pub fn component_name(locale: &str, filename: &str) -> String {
    format!("{}{}{}{}{}", locale_data_dir(locale), '\\', LOCALIZED_DIR, '\\', filename)
}

/// 解析组件名，返回 (locale, filename)；不是本地化字符串表则返回 None。
pub fn parse_string_component(name: &str) -> Option<(String, String)> {
    let normalized = normalize_component(name);
    let parts: Vec<&str> = normalized.split('\\').collect();
    if parts.len() != 3 {
        return None;
    }
    let dir = parts[0];
    if !parts[1].eq_ignore_ascii_case(LOCALIZED_DIR) {
        return None;
    }
    if !parts[2].to_ascii_lowercase().ends_with(".txt") {
        return None;
    }
    let lower = dir.to_ascii_lowercase();
    if !lower.ends_with(&LOCALE_DATA_SUFFIX.to_ascii_lowercase()) {
        return None;
    }
    let raw_locale = &dir[..dir.len() - LOCALE_DATA_SUFFIX.len()];
    let canon = LOCALES
        .iter()
        .find(|l| l.eq_ignore_ascii_case(raw_locale))
        .copied()
        .unwrap_or(if raw_locale.eq_ignore_ascii_case(BASE) {
            BASE
        } else if locale_code_ok(raw_locale) {
            raw_locale
        } else {
            return None;
        });
    Some((canon.to_string(), parts[2].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locale_components() {
        assert_eq!(
            parse_string_component("zhCN.SC2Data\\LocalizedData\\GameStrings.txt"),
            Some(("zhCN".to_string(), "GameStrings.txt".to_string()))
        );
        // 门径两种分隔符都要认
        assert_eq!(
            parse_string_component("zhCN.SC2Data/LocalizedData/ObjectStrings.txt"),
            Some(("zhCN".to_string(), "ObjectStrings.txt".to_string()))
        );
        // 实测出现过的非官方 12 语言代码
        assert_eq!(
            parse_string_component("zhCH.SC2Data\\LocalizedData\\GameStrings.txt").map(|x| x.0),
            Some("zhCH".to_string())
        );
        assert_eq!(
            parse_string_component("Base.SC2Data\\LocalizedData\\GameStrings.txt").map(|x| x.0),
            Some("Base".to_string())
        );
    }

    #[test]
    fn rejects_non_locale_components() {
        assert!(parse_string_component("Base.SC2Data\\GameData\\UnitData.xml").is_none());
        assert!(parse_string_component("MapInfo").is_none());
        assert!(parse_string_component("enUS.SC2Data\\LocalizedData\\Hotkeys.txt").is_some());
    }

    #[test]
    fn builds_component_names() {
        assert_eq!(
            component_name("zhCN", "GameStrings.txt"),
            "zhCN.SC2Data\\LocalizedData\\GameStrings.txt"
        );
    }
}
