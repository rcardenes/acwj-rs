# Proposed Tests — Coverage Gaps

## 1. `pars.rs` — Parser edge cases (medium priority)

| Test | Input | Expected |
|------|-------|----------|
| Nested if/else | `{ if (1) { if (2) { } else { } } else { } }` | Nested If nodes |
| While inside if | `{ if (1) { while (1) { } } }` | If containing While |

## 2. `sym.rs` — Symbol table (lower priority)

| Test | Input | Expected |
|------|-------|----------|
| `is_empty` returns true | Empty table | `true` |
| `is_empty` returns false | Table with entry | `false` |
| `has_global` returns false | Non-existent name | `false` |
| `Default` impl | `SymbolTable::default()` | Same as `new()`, `is_empty() == true` |
| Overwrite check | Add same name twice | `len() == 2`, `find_glob` returns first |
| `DataType::from(Token::Void)` | `Token::Void` | `DataType::Void` |
| `DataType::from` panics on non-type | `Token::Plus` | Should panic |

## 3. Integration: compilation tests

The `resources/test/comp/` directory has source files not referenced from any test.
Add integration tests that:

1. Run the compiler on each `.sc` file: `cargo run -- <file> -o <tmp>.s`
2. Assemble/link with `gcc`: `gcc -no-pie -o <tmp> <tmp>.s` lib/printint.c
3. Execute and verify expected stdout output

Files and expected outputs:

| File | Expected output |
|------|----------------|
| `print-statement.sc` | `42` |
| `precedence.sc` | `25` |
| `comparison.sc` | `1` |
| `if-statement.sc` | `10` |
| `while-statement.sc` | `5 4 3 2 1` |
| `for-statement.sc` | `0 1 2 3 4` |
| `global-variables.sc` | `5 100` |
| `global-variables-2.sc` | `10` |
| `types-1.sc` | `10 20` |
| `functions-1.sc` | `12` |
| `functions-2.sc` | `10` |
