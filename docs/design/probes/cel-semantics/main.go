// Bounded comparison with a real CEL runtime, not Weftext's future adapter.
package main

import (
    "encoding/json"
    "fmt"
    "os"
    "cel.dev/cel-go/cel"
)

func main() {
    env, err := cel.NewEnv()
    if err != nil { panic(err) }
    cases := []struct { Expr, Want string; Budget uint64 }{
        {"false && (1 / 0 == 0)", "false", 1000},
        {"(1 / 0 == 0) && false", "false", 1000},
        {"true || (1 / 0 == 0)", "true", 1000},
        {"(1 / 0 == 0) || true", "true", 1000},
        {"true && (1 / 0 == 0)", "error", 1000},
        {"false || (1 / 0 == 0)", "error", 1000},
        {"[1, 0].exists(x, 1 / x > 0)", "true", 1000},
        {"[0, 1].exists(x, 1 / x > 0)", "true", 1000},
        {"[0, 1].all(x, 1 / x < 0)", "false", 1000},
        {"[1, 0].all(x, 1 / x < 0)", "false", 1000},
        {"[1, 0].filter(x, 1 / x > 0).size() == 1", "error", 1000},
        {"[1, 0].map(x, 1 / x).size() == 2", "error", 1000},
        {"false ? (1 / 0 == 0) : true", "true", 1000},
        {"has({}.label) ? {}.label == 'work' : false", "false", 1000},
        {"[1,2,3,4].map(x, x*x).size() == 4 || true", "error", 1},
    }
    rows := []map[string]any{}
    failed := false
    for _, c := range cases {
        ast, issues := env.Compile(c.Expr)
        if issues.Err() != nil { panic(issues.Err()) }
        program, err := env.Program(ast, cel.CostLimit(c.Budget))
        if err != nil { panic(err) }
        value, _, evalErr := program.Eval(map[string]any{})
        actual := "error"
        diagnostic := ""
        if evalErr == nil { actual = fmt.Sprint(value.Value()) } else { diagnostic = evalErr.Error() }
        ok := actual == c.Want
        failed = failed || !ok
        rows = append(rows, map[string]any{"expression":c.Expr,"budget":c.Budget,"expected":c.Want,"actual":actual,"pass":ok,"diagnostic":diagnostic})
    }
    out := map[string]any{"runtime":"cel.dev/cel-go v0.32.0","cases":rows,"passed":!failed,"scope":"CEL runtime only; Weftext Cell/Rust adapters and authorization not implemented"}
    data, _ := json.MarshalIndent(out,"","  ")
    fmt.Println(string(data))
    if failed { os.Exit(1) }
}

