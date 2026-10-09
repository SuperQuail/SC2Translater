// 专有名词库的状态与动作。
//
// 匹配与检索放在前端做：术语表通常几百条，放内存里直接算比每次 IPC 往返快得多，
// 输入框敲一个字就查一次也不卡。Rust 侧只负责持久化与导入导出（那才是它该干的）。

import { computed, ref, watch } from "vue";

import { api } from "../api";
import type { Glossary, GlossaryBundle, Term, TermScope } from "../api/types";

/** 一段文本命中了哪些术语 —— 长的优先（Infested Terran 比 Terran 更具体）。 */
export function matchTerms(terms: Term[], text: string | null | undefined): Term[] {
  if (!text) return [];
  const hits = terms.filter((t) => {
    const src = t.source.trim();
    if (!src) return false;
    const hay = t.case_sensitive ? text : text.toLowerCase();
    const needle = t.case_sensitive ? src : src.toLowerCase();
    return hay.includes(needle);
  });
  return hits.sort((a, b) => b.source.length - a.source.length);
}

/** 检索：原文/译名/备注/领域都算，原文精确匹配排最前（与 Rust 侧同规则）。 */
export function searchTerms(terms: Term[], query: string): Term[] {
  const q = query.trim().toLowerCase();
  const rank = (t: Term): number => {
    if (!q) return 3;
    const src = t.source.toLowerCase();
    if (src === q) return 0;
    if (src.startsWith(q)) return 1;
    if (src.includes(q)) return 2;
    return 3;
  };
  return terms
    .filter((t) => {
      if (!q) return true;
      return (
        t.source.toLowerCase().includes(q) ||
        t.target.includes(q) ||
        (t.note ?? "").toLowerCase().includes(q) ||
        (t.domain ?? "").toLowerCase().includes(q)
      );
    })
    .sort((a, b) => rank(a) - rank(b) || a.source.toLowerCase().localeCompare(b.source.toLowerCase()));
}

export function useGlossary(getProjectFile: () => string | null) {
  const global = ref<Glossary>({ format: 1, name: "全局术语库", terms: [] });
  const project = ref<Glossary>({ format: 1, name: "项目术语库", terms: [] });
  const globalPath = ref("");
  const projectPath = ref<string | null>(null);

  const query = ref("");
  const scopeFilter = ref<"all" | TermScope>("all");
  const busy = ref(false);
  const status = ref("");
  const error = ref("");

  /** 合并视图：项目覆盖全局（同原文时项目那条胜出）。 */
  const merged = computed<Term[]>(() => {
    const bySource = new Map<string, Term>();
    for (const t of global.value.terms) bySource.set(t.source.trim().toLowerCase(), t);
    for (const t of project.value.terms) bySource.set(t.source.trim().toLowerCase(), t);
    return [...bySource.values()].sort((a, b) => a.source.toLowerCase().localeCompare(b.source.toLowerCase()));
  });

  const filtered = computed<Term[]>(() => {
    const scoped = scopeFilter.value === "all" ? merged.value : merged.value.filter((t) => t.scope === scopeFilter.value);
    return searchTerms(scoped, query.value);
  });

  const counts = computed(() => ({ global: global.value.terms.length, project: project.value.terms.length, merged: merged.value.length }));

  function fail(err: unknown) {
    error.value = err instanceof Error ? err.message : String(err);
  }

  async function refresh() {
    try {
      const bundle: GlossaryBundle = await api.glossaryLoad(getProjectFile());
      global.value = bundle.global ?? { format: 1, name: "全局术语库", terms: [] };
      project.value = bundle.project ?? { format: 1, name: "项目术语库", terms: [] };
      globalPath.value = bundle.global_path;
      projectPath.value = bundle.project_path ?? null;
      error.value = "";
    } catch (e) {
      fail(e);
    }
  }

  /** 保存一份（默认自动保存，见下面的 watch）。 */
  async function save(scope: TermScope) {
    status.value = "保存中…";
    try {
      const glossary = scope === "global" ? global.value : project.value;
      const res = await api.glossarySave(scope, glossary, getProjectFile());
      status.value = scope === "global" ? "全局术语库已保存（" + res.terms + " 条）" : "项目术语库已保存（" + res.terms + " 条）";
      error.value = "";
    } catch (e) {
      fail(e);
      status.value = "";
    }
  }

  /** 新增或更新一条。 */
  function upsert(term: Partial<Term> & { source: string; target: string; scope: TermScope }) {
    const target = term.scope === "global" ? global : project;
    const existing = target.value.terms.find((t) => t.id === term.id);
    if (existing) {
      Object.assign(existing, term);
    } else {
      // id 与 Rust 侧同规则：作用域 + 小写原文
      const id = term.id ?? makeLocalId(term.source, term.scope);
      const dup = target.value.terms.find((t) => t.id === id);
      if (dup) Object.assign(dup, term, { id });
      else
        target.value.terms.push({
          id,
          source: term.source,
          target: term.target,
          note: term.note ?? "",
          domain: term.domain ?? "",
          case_sensitive: term.case_sensitive ?? true,
          scope: term.scope,
        });
    }
    void save(term.scope);
  }

  function remove(id: string) {
    const inGlobal = global.value.terms.find((t) => t.id === id);
    if (inGlobal) {
      global.value.terms = global.value.terms.filter((t) => t.id !== id);
      void save("global");
      return;
    }
    project.value.terms = project.value.terms.filter((t) => t.id !== id);
    void save("project");
  }

  /** 把一条译文一键加进术语库（从详情面板调用）。 */
  function addFromEntry(source: string, target: string, scope: TermScope, domain = "") {
    if (!source.trim() || !target.trim()) return;
    upsert({ source, target, scope, domain, case_sensitive: true });
    status.value = "已加入" + (scope === "global" ? "全局" : "项目") + "术语库";
  }

  async function importFile(scope: TermScope) {
    busy.value = true;
    try {
      const path = await api.glossaryPick(false, "glossary.csv");
      if (!path) return;
      const res = await api.glossaryRead(path, scope);
      const bucket = scope === "global" ? global : project;
      for (const t of res.terms) {
        const term: Term = { ...t, scope };
        const i = bucket.value.terms.findIndex((x) => x.id === term.id);
        if (i >= 0) bucket.value.terms[i] = term;
        else bucket.value.terms.push(term);
      }
      await save(scope);
      status.value = "已导入 " + res.count + " 条（" + res.format + "）到" + (scope === "global" ? "全局" : "项目") + "术语库";
    } catch (e) {
      fail(e);
    } finally {
      busy.value = false;
    }
  }

  async function exportFile(scope: TermScope, asCsv: boolean) {
    busy.value = true;
    try {
      const name = scope === "global" ? "glossary.global" : "glossary.project";
      const path = await api.glossaryPick(true, name + (asCsv ? ".csv" : ".json"));
      if (!path) return;
      const glossary = scope === "global" ? global.value : project.value;
      await api.glossaryWrite(path, glossary);
      status.value = "已导出 " + glossary.terms.length + " 条到 " + path;
    } catch (e) {
      fail(e);
    } finally {
      busy.value = false;
    }
  }

  // 改了就自动存（800ms 去抖），免得用户忘了保存丢数据
  let timer: number | undefined;
  watch(
    () => [global.value.terms.length, project.value.terms.length],
    () => {
      if (timer) window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        void save("global");
        if (getProjectFile()) void save("project");
      }, 800);
    },
  );

  return {
    global,
    project,
    globalPath,
    projectPath,
    query,
    scopeFilter,
    busy,
    status,
    error,
    merged,
    filtered,
    counts,
    refresh,
    save,
    upsert,
    remove,
    addFromEntry,
    importFile,
    exportFile,
  };
}

/** 与 Rust 的 make_id 同规则的本地实现（用于前端先行生成 id）。 */
export function makeLocalId(source: string, scope: TermScope): string {
  const text = scope + "|" + source.trim().toLowerCase();
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193;
  for (let i = 0; i < text.length; i += 1) {
    const c = text.charCodeAt(i);
    h1 = Math.imul(h1 ^ c, 0x01000193) >>> 0;
    h2 = Math.imul(h2 + c, 0x85ebca6b) >>> 0;
  }
  return (h1.toString(16).padStart(8, "0") + h2.toString(16).padStart(8, "0")).slice(0, 16);
}

export type GlossaryApi = ReturnType<typeof useGlossary>;
