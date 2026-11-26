import { test } from "node:test";
import assert from "node:assert/strict";
import { EverydayNavigation, matchEveryday, rankQuickOpen } from "./everyday-navigation";

test("successful back/forward, failed target and new branch preserve actual visits",()=>{
  const nav=new EverydayNavigation();nav.visit("a");nav.visit("b");nav.visit("c");
  assert.equal(nav.target(-1),"b");assert.equal(nav.current,"c");
  nav.arrive(-1,"b");assert.equal(nav.target(1),"c");
  assert.throws(()=>nav.arrive(1,"unavailable"));assert.equal(nav.current,"b");
  nav.visit("d");assert.equal(nav.target(1),undefined);assert.deepEqual(nav.recent,["d","b","c","a"]);
  nav.clear();assert.equal(nav.current,undefined);assert.deepEqual(nav.recent,[]);
});
test("same titles retain separate identities and path search disambiguates",()=>{
  const items=[{title:"同名",detail:"甲/笔记.typ"},{title:"同名",detail:"乙/笔记.typ"}];
  assert.deepEqual(matchEveryday(items,"同名 乙"),[items[1]]);assert.equal(matchEveryday(items,"同名").length,2);
});

test("Quick Open ranks exact Chinese title and filename above incidental folder matches", () => {
  const folder = { key: "first", title: "例题.typ", fileName: "例题.typ", detail: "总复习/例题.typ" };
  const appendix = { key: "second", title: "总复习附录", fileName: "附录.typ", detail: "甲/附录.typ" };
  const exact = { key: "third", title: "总复习", fileName: "第一册.typ", detail: "甲/第一册.typ" };
  const exactFile = { key: "fourth", title: "复习资料", fileName: "总复习.typ", detail: "乙/总复习.typ" };
  const items = [folder, appendix, exact, exactFile];
  assert.deepEqual(rankQuickOpen(items, " 总复习 ", [folder.key]), [exact, exactFile, appendix, folder]);
  assert.deepEqual(rankQuickOpen(items, "总复习.TYP", []), [exactFile]);
  assert.deepEqual(matchEveryday(items, "总复习"), items);
});

test("Quick Open prioritizes known recent visits when empty, but keeps duplicate titles and path narrowing", () => {
  const a = { key: "a", title: "同名", fileName: "笔记.typ", detail: "甲/笔记.typ" };
  const b = { key: "b", title: "同名", fileName: "笔记.typ", detail: "乙/笔记.typ" };
  const c = { key: "c", title: "余下项", fileName: "其他.typ", detail: "丙/其他.typ" };
  const items = [a, b, c];
  assert.deepEqual(rankQuickOpen(items, "  ", ["closed", c.key, b.key]), [c, b, a]);
  assert.deepEqual(rankQuickOpen(items, "同名", [b.key]), [a, b]);
  assert.deepEqual(rankQuickOpen(items, "同名 乙", []), [b]);
  assert.deepEqual(rankQuickOpen(items, "不存在", []), []);
  assert.deepEqual(items, [a, b, c]);
});
