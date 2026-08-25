import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

async function render() {
  const workerUrl = new URL("../dist/server/index.js", import.meta.url);
  workerUrl.searchParams.set("test", `${process.pid}-${Date.now()}`);
  const { default: worker } = await import(workerUrl.href);

  return worker.fetch(
    new Request("http://localhost/", { headers: { accept: "text/html" } }),
    { ASSETS: { fetch: async () => new Response("Not found", { status: 404 }) } },
    { waitUntil() {}, passThroughOnException() {} },
  );
}

test("server-renders the Weftext interaction prototype", async () => {
  const response = await render();
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type") ?? "", /^text\/html\b/i);

  const html = await response.text();
  assert.match(html, /<html lang="zh-CN">/);
  assert.match(html, /<title>Weftext \/ 文缕 — 知识工作区原型<\/title>/);
  assert.match(html, /交互原型/);
  assert.match(html, /产品工作区/);
  assert.match(html, /写作/);
  assert.match(html, /源码/);
  assert.match(html, /阅读/);
  assert.match(html, /已保存/);
  assert.doesNotMatch(html, /codex-preview|Building your site|Your site is taking shape/);
});

test("keeps the live prototype behind the Rust Core preview and revision boundary", async () => {
  const page = await readFile(new URL("../app/page.tsx", import.meta.url), "utf8");
  const previewIndex = page.indexOf("/api/document/preview");
  const commitIndex = page.indexOf("/api/document/commit");

  assert.match(page, /new URLSearchParams\(window\.location\.hash/);
  assert.match(page, /\["127\.0\.0\.1", "localhost"\]/);
  assert.match(page, /Authorization: `Bearer \$\{token\}`/);
  assert.ok(previewIndex >= 0 && commitIndex > previewIndex);
  assert.match(page, /revision: liveDocument\.revision/);
  assert.match(page, /Core 已对当前 revision 生成确定性计划/);
  assert.match(page, /陈旧 revision 将进入冲突状态/);
  assert.match(page, /浏览器没有目录权限/);
  assert.match(page, /\/api\/workspace\/action\/preview/);
  assert.match(page, /\/api\/workspace\/action\/commit/);
  assert.match(page, /\/api\/document\/model/);
  assert.match(page, /renderModel\(activeModel, currentSource, citationComponent\)/);
  assert.match(page, /\/api\/citation\/analyze/);
  assert.match(page, /\/api\/citation\/macro-edit-preview/);
  assert.doesNotMatch(page, /\/api\/citation\/transaction\/commit/);
  assert.match(page, /历史参考文献记录仅作为只读转换证据/);
  assert.match(page, /value=\{currentSource\}/);
  assert.doesNotMatch(page, /function visibleDocument|function renderDemoAsciiDoc|stripBlockId/);
  assert.match(page, /coreBodyStart\(currentSource, activeModel\)/);
  assert.match(page, /draftModelState === "error"/);
  assert.match(page, /最长匹配/);
  assert.doesNotMatch(page, /showDirectoryPicker|FileSystemFileHandle|node:fs/);
});

test("keeps the Desktop production entry free of the isolated demo workspace", async () => {
  const desktop = await readFile(new URL("../../../apps/desktop/src/main.tsx", import.meta.url), "utf8");
  const page = await readFile(new URL("../app/page.tsx", import.meta.url), "utf8");
  const demo = await readFile(new URL("../app/demo-workspace.tsx", import.meta.url), "utf8");
  assert.match(desktop, /import \{ WeftextApp \} from/);
  assert.match(desktop, /<WeftextApp demo=\{null\} \/>/);
  assert.doesNotMatch(desktop, /import WeftextApp from/);
  assert.doesNotMatch(page, /交互原型|产品工作区|已在演示草稿|演示模式不执行结构写入/);
  assert.match(demo, /交互原型/);
  assert.match(demo, /产品工作区/);
});

test("publishes product metadata and branded preview assets", async () => {
  const layout = await readFile(new URL("../app/layout.tsx", import.meta.url), "utf8");
  assert.match(layout, /title: "Weftext \/ 文缕 — 知识工作区原型"/);
  assert.match(layout, /images: \[\{ url: "\/og\.png"/);
  assert.match(layout, /icons: \{ icon: "\/app-icon\.svg"/);
  await Promise.all([
    access(new URL("../public/og.png", import.meta.url)),
    access(new URL("../public/app-icon.svg", import.meta.url)),
    access(new URL("../public/logo-mark.svg", import.meta.url)),
  ]);
});
