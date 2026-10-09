// 术语库检索与匹配的前端单测。
// 规则必须与 Rust 侧一致（sc2trans-core/src/glossary.rs 的同名逻辑），
// 否则「界面上搜得到、写回时查不到」这种不一致很难查。

import { describe, expect, it } from "vitest";

import type { Term } from "../src/api/types";
import { makeLocalId, matchTerms, searchTerms } from "../src/composables/useGlossary";

const terms: Term[] = [
  { id: "1", source: "Infested Terran", target: "被感染的陆战队员", case_sensitive: true, scope: "global", domain: "单位" },
  { id: "2", source: "Terran", target: "人类", case_sensitive: true, scope: "global", domain: "阵营" },
  { id: "3", source: "Hive", target: "主巢", case_sensitive: true, scope: "project", note: "虫族建筑" },
];

describe("术语匹配", () => {
  it("长的词优先（Infested Terran 比 Terran 更具体）", () => {
    const hits = matchTerms(terms, "An Infested Terran attacks the Hive");
    expect(hits.map((t) => t.source)).toEqual(["Infested Terran", "Terran", "Hive"]);
  });

  it("默认大小写敏感", () => {
    expect(matchTerms(terms, "a hive of scum")).toHaveLength(0);
  });

  it("关掉大小写敏感后小写也命中", () => {
    const loose: Term[] = [{ ...terms[2], case_sensitive: false }];
    expect(matchTerms(loose, "a hive")).toHaveLength(1);
  });

  it("空文本与空原文都不炸", () => {
    expect(matchTerms(terms, null)).toEqual([]);
    expect(matchTerms(terms, "")).toEqual([]);
  });
});

describe("术语检索", () => {
  it("原文精确匹配排第一", () => {
    expect(searchTerms(terms, "terran")[0].source).toBe("Terran");
  });

  it("译名、备注、领域都能搜到", () => {
    expect(searchTerms(terms, "主巢")).toHaveLength(1);
    expect(searchTerms(terms, "虫族")).toHaveLength(1);
    expect(searchTerms(terms, "单位")).toHaveLength(1);
    expect(searchTerms(terms, "不存在的东西")).toHaveLength(0);
  });

  it("空查询返回全部", () => {
    expect(searchTerms(terms, "")).toHaveLength(3);
  });
});

describe("词条 id", () => {
  it("同一原文 + 同一作用域得到同一个 id", () => {
    expect(makeLocalId("Hive", "global")).toBe(makeLocalId("  hive ", "global"));
  });

  it("不同作用域互不覆盖", () => {
    expect(makeLocalId("Hive", "global")).not.toBe(makeLocalId("Hive", "project"));
  });

  it("id 是 16 位十六进制", () => {
    expect(makeLocalId("Infested Terran", "project")).toMatch(/^[0-9a-f]{16}$/);
  });
});
