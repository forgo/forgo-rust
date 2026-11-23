// Load yaml-test-suite cases with `yaml` (eemeli) or js-yaml.
// Usage: node js_driver.mjs <yaml|js-yaml> <cases.txt> <start>
// Same output protocol as py_driver.py.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(process.env.NODE_DEPS + "/");
const [lib, casesFile, startArg] = process.argv.slice(2);

function makeLib() {
  if (lib === "yaml") {
    const YAML = require("yaml");
    return {
      load(text) {
        const docs = YAML.parseAllDocuments(text);
        if (!Array.isArray(docs)) throw new Error(String(docs.errors?.[0] ?? "empty"));
        for (const d of docs) if (d.errors.length) throw d.errors[0];
        return docs.map((d) => d.toJS({ maxAliasCount: -1 }));
      },
      dump: (docs) => docs.map((d) => YAML.stringify(d)).join("---\n"),
    };
  }
  const jsyaml = require("js-yaml");
  return {
    load: (text) => jsyaml.loadAll(text),
    dump: (docs) => docs.map((d) => jsyaml.dump(d)).join("---\n"),
  };
}

function plain(v) {
  if (v instanceof Map) return Object.fromEntries([...v].map(([k, x]) => [key(k), plain(x)]));
  if (v instanceof Set) return [...v].map(plain);
  if (Array.isArray(v)) return v.map(plain);
  if (v instanceof Date) return { $other: v.toISOString() };
  if (v instanceof Uint8Array) return { $bytes: Buffer.from(v).toString("hex") };
  if (typeof v === "bigint") return Number(v);
  if (typeof v === "number" && !Number.isFinite(v)) return { $float: String(v) };
  if (v && typeof v === "object") {
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, plain(x)]));
  }
  return v === undefined ? null : v;
}

function key(k) {
  if (k === null || k === undefined) return "";
  if (typeof k === "object") return JSON.stringify(plain(k));
  return String(k);
}

function same(a, b) {
  return JSON.stringify(plain(a)) === JSON.stringify(plain(b));
}

const { load, dump } = makeLib();
console.log(`version ${require(`${lib}/package.json`).version}`);
const paths = readFileSync(casesFile, "utf8").split("\n").filter(Boolean);
for (let i = Number(startArg); i < paths.length; i++) {
  console.log(`start ${i}`);
  const r = { ok: false, docs: [], rt: null, err: "" };
  try {
    const docs = load(readFileSync(paths[i], "utf8"));
    r.ok = true;
    r.docs = docs.map(plain);
    try {
      r.rt = same(docs, load(dump(docs)));
    } catch (e) {
      r.rt = false;
      r.err = `roundtrip: ${e}`;
    }
  } catch (e) {
    r.err = String(e).split("\n")[0];
  }
  console.log(`case ${i} ${JSON.stringify(r)}`);
}
