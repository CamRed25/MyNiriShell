---
description: 'Code block minimalism for Rust. Enforce write-only-what-is-needed rules: no speculative abstractions, no single-use helpers, no premature generics, no impossible error variants.'
applyTo: '**/*.rs'
---
# Code Block Minimalism (Rust)

Write only the code you need—no more, no less. Every extra line is extra work to read, test, and maintain. Use this checklist before adding any function, trait, struct, enum variant, generic, or wrapper type. Stop as soon as your code meets the current need.

---
## Clean Coding Checklist

### 1. Is it needed right now?
Only write code for real, current requirements. No "just in case" helpers, enum variants, or generics for possible future needs. Extra code is future technical debt.

### 2. Is it used in more than one place?
Extract a helper, trait, or abstraction only if it is used in two or more places. If it's used once, keep it inline.

| Situation | Action |
|-----------|--------|
| Used once | Inline it |
| Used twice in the same function | Consider a local closure |
| Used in 2+ separate functions/modules | Extract a helper |
| Used across 3+ crates | Consider a shared module |

### 3. Does the abstraction add real value?
Add traits, generics, or wrappers only if they solve a real problem:
- **Trait:** Only if you have two or more real implementations now, or a library/framework requires it.
- **Generic:** Only if multiple concrete types use it now. Don't generalize for the future.
- **Newtype:** Only if it prevents real type confusion (e.g., `UserId(u64)` vs `u64`).
- **Module split:** Only if a file is too big to navigate easily.

---
## Patterns to Avoid
| Pattern | Instead |
|---------|---------|
| Helper used once | Inline the code |
| Trait with one implementor | Use the concrete type |
| Generic with one type | Use the concrete type |
| Enum variant never used | Delete it |
| Error variant for impossible case | Delete it; use `unreachable!()` if needed |
| Unused `let` binding | Chain the call |
| Wrapper struct that just delegates | Use the inner type |
| `Default` for non-default types | Don't derive it |
| `Clone`/`Copy`/`Debug` not used | Only derive what you use |
| `pub` for private items | Use `pub(crate)` or remove |
| Builder for ≤3 fields | Use struct literal |
| `new()` that just sets fields | Use struct literal |

---
## Error Handling
- Only add error variants for real, possible errors.
- Don't add `NotFound`, `Timeout`, etc. unless a real code path produces them.
- Don't wrap every `std::io::Error` unless it adds real context.
- If an error enum has one variant, use `String` or `anyhow::Error` instead.

---
## Function Length
- Functions should do one thing. If a function is long, check if it mixes responsibilities.
- Don't split functions just to hit a line count.
- Only extract sub-functions if they clarify intent and are reused.
- A big `match` is fine if it keeps logic together.

---
## When Abstraction IS Needed
- **Protocol/format boundaries:** Types for wire messages, configs, or serialization.
- **Invariant enforcement:** Newtypes or constructors that enforce rules.
- **Framework requirements:** Traits needed by libraries.
- **Safety:** Wrappers for unsafe code.
- **Real polymorphism:** Traits with two or more real types, reducing duplication.

---
## Before Adding New Code Structure
- [ ] Is it needed for a real, current task?
- [ ] If a function: is it called from 2+ places?
- [ ] If a trait: are there 2+ real implementors?
- [ ] If a generic: do 2+ types use it?
- [ ] If an error variant: can it actually happen?
- [ ] If a derived trait: is it used?
- [ ] If `pub`: is it used outside the module?

If any box is unchecked, don't add it. Write the minimal code that solves the problem.