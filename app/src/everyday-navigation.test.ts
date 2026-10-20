import { test } from "node:test";
import assert from "node:assert/strict";
import { EverydayNavigation, matchEveryday } from "./everyday-navigation";

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
