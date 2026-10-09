"""SC2 的 locale 常量，以及归档内「本地化组件」的命名规则。

事实来源（实测，见 docs/research/sc2-localization.md）：

* 归档内的组件名用反斜杠分隔，形如 zhCN.SC2Data/LocalizedData/GameStrings.txt（反斜杠）。
* 与语言无关的部分放在 Base.SC2Data/... ，任何语言都会加载。
* 与语言相关的部分放在 <locale>.SC2Data/... ，**只有当前游戏语言对应的那个会被加载**。
  这就是「英文地图在中文环境下键显示成路径」的根因：地图只带了 enUS，而游戏要的是 zhCN。
"""

from __future__ import annotations

import re

#: SC2 正式支持的 12 个语言目录。
LOCALES: tuple[str, ...] = (
    "enUS", "deDE", "esES", "esMX", "frFR", "itIT",
    "koKR", "plPL", "ptBR", "ruRU", "zhCN", "zhTW",
)

#: 语言无关组件集的名字（任何语言都会加载）。
BASE = "Base"

#: 语言数据目录的后缀与本地化子目录名。
LOCALE_DATA_SUFFIX = ".SC2Data"
LOCALIZED_DIR = "LocalizedData"

#: 已知的字符串表文件名（按编辑器实际写出的顺序）。
KNOWN_STRING_FILES: tuple[str, ...] = (
    "GameStrings.txt",
    "ObjectStrings.txt",
    "TriggerStrings.txt",
)

#: 语言的显示名，用于人类可读输出。
LOCALE_NAMES: dict[str, str] = {
    "enUS": "英语（美国）",
    "deDE": "德语",
    "esES": "西班牙语（西班牙）",
    "esMX": "西班牙语（墨西哥）",
    "frFR": "法语",
    "itIT": "意大利语",
    "koKR": "韩语",
    "plPL": "波兰语",
    "ptBR": "葡萄牙语（巴西）",
    "ruRU": "俄语",
    "zhCN": "简体中文",
    "zhTW": "繁体中文",
    "Base": "语言无关（Base）",
}

_BACKSLASH = chr(92)
#: 正则里表示「一个字面反斜杠」的写法。re.escape 给出的两个反斜杠，正好是正则需要的形式。
_BS_RE = re.escape(_BACKSLASH)

#: 组件名匹配：<locale>.SC2Data / LocalizedData / <something>.txt（分隔符是反斜杠）
_STRING_COMPONENT_RE = re.compile(
    "^(?P<locale>[A-Za-z]{2}[A-Za-z]{2}|Base)"
    + "[.]SC2Data"
    + _BS_RE + "LocalizedData"
    + _BS_RE + "(?P<file>[^" + _BS_RE + "]+[.]txt)$",
    re.IGNORECASE,
)


#: 面向玩家的字符串表 —— 决定游戏里会不会出现原始键路径。
PLAYER_FACING_FILES: tuple[str, ...] = ("GameStrings.txt", "ObjectStrings.txt")

#: 只喂给银河编辑器触发器界面的字符串表 —— 玩家在游戏里看不到，缺了不影响游戏体验。
#: 实测依据：暴雪官方战役地图的非英文语言目录里根本没有 TriggerStrings.txt。
EDITOR_FACING_FILES: tuple[str, ...] = ("TriggerStrings.txt",)


def is_player_facing(filename: str) -> bool:
    return filename in PLAYER_FACING_FILES


def locale_code_ok(code: str) -> bool:
    """归档里出现的语言前缀**不止**官方那 12 个。

    实测（本机 183 个真实归档抽样）：还出现过 enCN / enGB / enSG / enTW / jaJP / zhCH。
    所以这里只校验形状，不限定取值；LOCALES 仅用于显示名与「常见语言」列表。
    """
    return bool(re.fullmatch(r"[A-Za-z]{2,4}", code or ""))


def normalize_component(name: str) -> str:
    """统一组件名分隔符为反斜杠，去掉首尾空白与多余分隔符。"""
    return name.replace("/", _BACKSLASH).strip().strip(_BACKSLASH)


def locale_data_dir(locale: str) -> str:
    """zhCN -> zhCN.SC2Data"""
    return locale + LOCALE_DATA_SUFFIX


def component_name(locale: str, filename: str) -> str:
    """构造归档内的组件名，例如 component_name('zhCN', 'GameStrings.txt')。"""
    return _BACKSLASH.join((locale_data_dir(locale), LOCALIZED_DIR, filename))


def parse_string_component(name: str) -> tuple[str, str] | None:
    """解析组件名，返回 (locale, filename)；不是本地化字符串表则返回 None。"""
    m = _STRING_COMPONENT_RE.match(normalize_component(name))
    if not m:
        return None
    raw_locale = m.group("locale")
    canon = {loc.lower(): loc for loc in LOCALES}
    locale = canon.get(raw_locale.lower(), BASE if raw_locale.lower() == "base" else raw_locale)
    return locale, m.group("file")


def locale_display(locale: str) -> str:
    return LOCALE_NAMES.get(locale, locale)
