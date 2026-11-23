"""Load yaml-test-suite cases with PyYAML or ruamel.yaml.

Usage: py_driver.py <pyyaml|ruamel> <cases.txt> <start>
cases.txt holds one in.yaml path per line. For each case from <start>,
prints `start <i>`, then `case <i> <json>` where json is
{"ok": bool, "docs": [...], "rt": bool|null, "err": str}.
"""
import io
import json
import math
import sys


def plain(v):
    """The loaded value as JSON; keys spelled as in.json spells them."""
    if hasattr(v, "value") and hasattr(v, "tag") and not isinstance(v, (dict, list)):
        v = v.value  # ruamel TaggedScalar
    if isinstance(v, dict):
        return {key(k): plain(x) for k, x in v.items()}
    if isinstance(v, (list, tuple, set)):
        return [plain(x) for x in v]
    if isinstance(v, bool) or v is None or isinstance(v, str):
        return v if not isinstance(v, str) else str(v)
    if isinstance(v, int):
        return int(v)
    if isinstance(v, float):
        f = float(v)
        return f if math.isfinite(f) else {"$float": repr(f)}
    if isinstance(v, bytes):
        return {"$bytes": v.hex()}
    return {"$other": str(v)}


def key(k):
    if k is None:
        return ""
    if isinstance(k, bool):
        return "true" if k else "false"
    if isinstance(k, (str, int, float)):
        return str(k)
    return json.dumps(plain(k), sort_keys=True)


def same(a, b):
    if isinstance(a, float) and isinstance(b, float) and math.isnan(a) and math.isnan(b):
        return True
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return type(a) == type(b) and a == b


def make(lib):
    if lib == "pyyaml":
        import yaml

        def load(text):
            return list(yaml.safe_load_all(text))

        def dump(docs):
            return yaml.safe_dump_all(docs, allow_unicode=True)

        return load, dump
    from ruamel.yaml import YAML

    def load(text):
        return list(YAML().load_all(text))

    def dump(docs):
        out = io.StringIO()
        YAML().dump_all(docs, out)
        return out.getvalue()

    return load, dump


def main():
    lib, cases, start = sys.argv[1], sys.argv[2], int(sys.argv[3])
    load, dump = make(lib)
    if lib == "pyyaml":
        import yaml as mod
    else:
        import ruamel.yaml as mod
    print(f"version {mod.__version__}", flush=True)
    paths = open(cases, encoding="utf-8").read().splitlines()
    for i in range(start, len(paths)):
        print(f"start {i}", flush=True)
        text = open(paths[i], encoding="utf-8").read()
        r = {"ok": False, "docs": [], "rt": None, "err": ""}
        try:
            docs = load(text)
            r["ok"] = True
            r["docs"] = [plain(d) for d in docs]
            try:
                again = load(dump(docs))
                r["rt"] = same([plain(d) for d in docs], [plain(d) for d in again])
            except Exception as e:  # noqa: BLE001 - any failure fails the round trip
                r["rt"] = False
                r["err"] = f"roundtrip: {type(e).__name__}: {e}"
        except Exception as e:  # noqa: BLE001
            r["err"] = f"{type(e).__name__}: {e}"
        print(f"case {i} {json.dumps(r, ensure_ascii=False)}", flush=True)


main()
