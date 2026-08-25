import { cp, mkdir, readFile, rm } from "node:fs/promises";

const root = new URL(".", import.meta.url);
const output = new URL("./dist/", root);
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
for (const file of ["index.html", "app.js", "api.js", "navigation.js", "style.css"]) {
  const source = new URL(file, root);
  const contents = await readFile(source, "utf8");
  if (!contents.trim()) throw new Error(`${file} is empty`);
  await cp(source, new URL(file, output));
}
const html = await readFile(new URL("index.html", output), "utf8");
if (!html.includes('src="/app.js"') || !html.includes('href="/style.css"')) {
  throw new Error("production HTML does not reference same-origin assets");
}
console.log("Built 5 same-origin WebUI assets");
