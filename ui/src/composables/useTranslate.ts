// 界面状态与动作。组件只负责渲染，业务动作都收在这里，方便测试与替换。

import { computed, ref } from "vue";

import { api } from "../api";
import type { ApplyResult, AuditReport, VerifyReport } from "../api/types";

/** 常见语言，供下拉框使用；也允许手填（归档里实测出现过 zhCH/enGB 等）。 */
export const COMMON_LOCALES = [
  { code: "enUS", label: "英语（美国）" },
  { code: "zhCN", label: "简体中文" },
  { code: "zhTW", label: "繁体中文" },
  { code: "koKR", label: "韩语" },
  { code: "jaJP", label: "日语" },
  { code: "ruRU", label: "俄语" },
  { code: "deDE", label: "德语" },
  { code: "frFR", label: "法语" },
  { code: "esES", label: "西班牙语（西班牙）" },
  { code: "ptBR", label: "葡萄牙语（巴西）" },
];

export function useTranslate() {
  const archive = ref<string | null>(null);
  const source = ref("enUS");
  const target = ref("zhCN");
  const busy = ref(false);
  const status = ref("");
  const error = ref("");
  const logs = ref<string[]>([]);
  const audit = ref<AuditReport | null>(null);
  const result = ref<ApplyResult | null>(null);
  const verifyReport = ref<VerifyReport | null>(null);

  const ready = computed(() => Boolean(archive.value) && !busy.value);
  const diagnosis = computed(() => audit.value?.diagnosis ?? null);

  function log(message: string) {
    const stamp = new Date().toLocaleTimeString();
    logs.value = ["[" + stamp + "] " + message].concat(logs.value).slice(0, 200);
  }

  function fail(err: unknown) {
    const text = err instanceof Error ? err.message : String(err);
    error.value = text;
    log("失败：" + text);
  }

  async function guard<T>(label: string, fn: () => Promise<T>): Promise<T | null> {
    busy.value = true;
    error.value = "";
    status.value = label;
    try {
      return await fn();
    } catch (err) {
      fail(err);
      return null;
    } finally {
      busy.value = false;
      status.value = "";
    }
  }

  async function pickArchive() {
    const path = await guard("选择地图…", () => api.pickArchive());
    if (!path) return;
    archive.value = path;
    result.value = null;
    verifyReport.value = null;
    log("已选择 " + path);
    await refreshAudit();
  }

  async function refreshAudit() {
    if (!archive.value) return;
    const report = await guard("分析语言覆盖…", () => api.audit(archive.value as string, source.value));
    if (report) {
      audit.value = report;
      log(
        "共 " + report.source_keys + " 个键；" + report.diagnosis.message,
      );
    }
  }

  async function doExtract() {
    if (!archive.value) return;
    const name = await api.defaultJsonName(archive.value, target.value);
    const out = await api.pickJsonSave(name);
    if (!out) return;
    const doc = await guard("导出翻译键…", () =>
      api.extract(archive.value as string, source.value, target.value, out, false),
    );
    if (doc) {
      log("已导出 " + doc.stats.entries + " 个键到 " + out + "；把 target 字段填上译文后再「导入译文并写回」。");
    }
  }

  async function doSeed() {
    if (!archive.value) return;
    const name = await api.defaultOutputName(archive.value, target.value);
    const out = await api.pickArchiveSave(name);
    if (!out) return;
    const applied = await guard("兜底补齐…", () =>
      api.seed(archive.value as string, source.value, target.value, out, false, false),
    );
    if (applied) {
      result.value = applied;
      log(
        "已写出 " + applied.output + "；补了 " + (applied.seeded ?? 0) + " 个键，回读校验" +
          (applied.ok ? "通过" : "失败"),
      );
    }
  }

  async function doApply() {
    if (!archive.value) return;
    const jsonPath = await api.pickJsonOpen();
    if (!jsonPath) return;
    const name = await api.defaultOutputName(archive.value, target.value);
    const out = await api.pickArchiveSave(name);
    if (!out) return;
    const applied = await guard("写回译文…", () =>
      api.apply(archive.value as string, jsonPath, out, target.value, false, false, false),
    );
    if (applied) {
      result.value = applied;
      const counts = Object.values(applied.components_written[applied.target_locale] ?? {});
      const total = counts.reduce((sum, info) => sum + info.total, 0);
      log("已写出 " + applied.output + "；目标语言共 " + total + " 条，回读校验" + (applied.ok ? "通过" : "失败"));
    }
  }

  async function doVerify() {
    const path = result.value?.output ?? archive.value;
    if (!path) return;
    const report = await guard("容器自检…", () => api.verify(path, true));
    if (report) {
      verifyReport.value = report;
      log("自检 " + path + "：整体" + (report.healthy ? "健康" : "有问题"));
    }
  }

  async function revealOutput() {
    const path = result.value?.output;
    if (!path) return;
    await guard("打开目录…", () => api.reveal(path));
  }

  return {
    archive,
    source,
    target,
    busy,
    status,
    error,
    logs,
    audit,
    result,
    verifyReport,
    ready,
    diagnosis,
    pickArchive,
    refreshAudit,
    doExtract,
    doSeed,
    doApply,
    doVerify,
    revealOutput,
  };
}
