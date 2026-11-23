// Load yaml-test-suite cases with go-yaml (go.yaml.in/yaml/v3, the YAML
// organization's continuation of gopkg.in/yaml.v3).
// Usage: driver <cases.txt> <start>; same output protocol as py_driver.py.
package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"os"
	"reflect"
	"runtime/debug"
	"strconv"
	"strings"

	"go.yaml.in/yaml/v3"
)

func load(text []byte) ([]any, error) {
	dec := yaml.NewDecoder(bytes.NewReader(text))
	var docs []any
	for {
		var v any
		err := dec.Decode(&v)
		if errors.Is(err, io.EOF) {
			return docs, nil
		}
		if err != nil {
			return nil, err
		}
		docs = append(docs, v)
	}
}

func dump(docs []any) ([]byte, error) {
	var b bytes.Buffer
	enc := yaml.NewEncoder(&b)
	for _, d := range docs {
		if err := enc.Encode(d); err != nil {
			return nil, err
		}
	}
	enc.Close()
	return b.Bytes(), nil
}

func key(k any) string {
	switch k := k.(type) {
	case nil:
		return ""
	case string:
		return k
	case bool, int, int64, uint64, float64:
		return fmt.Sprint(k)
	default:
		j, _ := json.Marshal(plain(k))
		return string(j)
	}
}

func plain(v any) any {
	switch v := v.(type) {
	case map[string]any:
		out := map[string]any{}
		for k, x := range v {
			out[k] = plain(x)
		}
		return out
	case map[any]any:
		out := map[string]any{}
		for k, x := range v {
			out[key(k)] = plain(x)
		}
		return out
	case []any:
		out := make([]any, len(v))
		for i, x := range v {
			out[i] = plain(x)
		}
		return out
	case float64:
		if math.IsNaN(v) || math.IsInf(v, 0) {
			return map[string]any{"$float": strconv.FormatFloat(v, 'g', -1, 64)}
		}
		return v
	case nil, bool, string, int, int64, uint64:
		return v
	default:
		return map[string]any{"$other": fmt.Sprint(v)}
	}
}

func main() {
	if info, ok := debug.ReadBuildInfo(); ok {
		for _, d := range info.Deps {
			if d.Path == "go.yaml.in/yaml/v3" {
				fmt.Printf("version %s\n", strings.TrimPrefix(d.Version, "v"))
			}
		}
	}
	cases, _ := os.ReadFile(os.Args[1])
	start, _ := strconv.Atoi(os.Args[2])
	paths := strings.Split(strings.TrimSpace(string(cases)), "\n")
	for i := start; i < len(paths); i++ {
		fmt.Printf("start %d\n", i)
		r := map[string]any{"ok": false, "docs": []any{}, "rt": nil, "err": ""}
		text, _ := os.ReadFile(paths[i])
		func() {
			defer func() {
				if p := recover(); p != nil {
					r["ok"], r["err"] = false, fmt.Sprint("panic: ", p)
				}
			}()
			docs, err := load(text)
			if err != nil {
				r["err"] = err.Error()
				return
			}
			r["ok"] = true
			out := make([]any, len(docs))
			for j, d := range docs {
				out[j] = plain(d)
			}
			r["docs"] = out
			text2, err := dump(docs)
			if err != nil {
				r["rt"], r["err"] = false, "roundtrip: "+err.Error()
				return
			}
			again, err := load(text2)
			if err != nil {
				r["rt"], r["err"] = false, "roundtrip: "+err.Error()
				return
			}
			back := make([]any, len(again))
			for j, d := range again {
				back[j] = plain(d)
			}
			r["rt"] = reflect.DeepEqual(out, back)
		}()
		j, _ := json.Marshal(r)
		fmt.Printf("case %d %s\n", i, j)
	}
}
