// 机翻参考的状态与动作。
//
// 铁律：**机翻结果永远不自动写进译文**。这里只产出「建议」，落到键里必须由用户在
// 详情面板上显式点一次「用作译文」——把机翻直接落库等于把错误固化进地图。

import { ref } from "vue";

import { api } from "../api";
import type { MtConfig, MtSuggestion } from "../api/types";

export function useMt(getProjectFile: () => string | null) {
  const config = ref<MtConfig>({
    provider: "offline",
    base_url: "https://api.openai.com/v1",
    api_key: "",
    model: "gpt-4o-mini",
    target_language: "简体中文",
    timeout_secs: 30,
  });
  const suggestion = ref<MtSuggestion | null>(null);
  const busy = ref(false);
  const error = ref("");
  const status = ref("");

  async function loadConfig() {
    try {
      config.value = await api.mtConfigGet();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function saveConfig(next: MtConfig) {
    try {
      config.value = await api.mtConfigSet(next);
      status.value = "机翻设置已保存";
      error.value = "";
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  /** 按需翻译一条（不会自动触发）。 */
  async function translate(text: string | null | undefined) {
    suggestion.value = null;
    error.value = "";
    if (!text || !text.trim()) {
      error.value = "这条没有原文";
      return;
    }
    busy.value = true;
    try {
      suggestion.value = await api.mtTranslate(text, getProjectFile());
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      busy.value = false;
    }
  }

  /** 批量：给一堆原文，返回「原文 -> 建议」的映射（不落键）。 */
  async function translateMany(texts: string[]) {
    if (!texts.length) return new Map<string, MtSuggestion>();
    busy.value = true;
    error.value = "";
    try {
      const res = await api.mtTranslateMany(texts, getProjectFile());
      const map = new Map<string, MtSuggestion>();
      res.results.forEach((r, i) => {
        if (r.ok && r.suggestion) map.set(texts[i], r.suggestion);
      });
      const failed = res.results.filter((r) => !r.ok).length;
      status.value = "已生成 " + map.size + " 条参考" + (failed ? "，失败 " + failed + " 条" : "");
      return map;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      return new Map<string, MtSuggestion>();
    } finally {
      busy.value = false;
    }
  }

  async function clearCache() {
    try {
      const res = await api.mtClearCache();
      status.value = "已清空 " + res.cleared + " 条缓存";
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  function clear() {
    suggestion.value = null;
    error.value = "";
  }

  return { config, suggestion, busy, error, status, loadConfig, saveConfig, translate, translateMany, clearCache, clear };
}

export type MtApi = ReturnType<typeof useMt>;
