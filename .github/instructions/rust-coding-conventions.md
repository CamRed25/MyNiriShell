---
description: 'Rust programming language coding conventions and best practices'
applyTo: '**/*.rs'
---
# Rust Coding Conventions and Best Practices

Write Rust code that is clear, safe, and easy to maintain. Follow these rules, based on The Rust Book, Rust API Guidelines, and community standards.

## General Principles
- Make code easy to read and maintain.
- Use Rust's type system and ownership for safety.
- Split up big functions.
- Comment on why, not just what.
- Use `Result<T, E>` for errors and give clear messages.
- Document why you use each external crate.
- Use standard naming (see RFC 430).
- Code should be idiomatic, safe, and efficient.
- Code must compile without warnings.

## Good Patterns
- Use modules (`mod`) and `pub` to organize code.
- Handle errors with `?`, `match`, or `if let`.
- Use `serde` for serialization, `thiserror` or `anyhow` for errors.
- Use traits to abstract over services or dependencies.
- Use `async/await` and `tokio` or `async-std` for async code.
- Prefer enums for state, not flags.
- Use builders for complex objects.
- Split binary and library code for easier testing.
- Use iterators, not index-based loops.
- Use `&str` for parameters unless you need ownership.
- Prefer borrowing and zero-copy.

### Ownership, Borrowing, and Lifetimes
- Borrow (`&T`) instead of cloning when possible.
- Use `&mut T` to change borrowed data.
- Add lifetimes only when needed.
- Use `Rc<T>`/`Arc<T>` for shared ownership.
- Use `RefCell<T>`, `Mutex<T>`, or `RwLock<T>` for mutability.

## What to Avoid
- Don't use `unwrap()` or `expect()` unless you must.
- Don't panic in libraries—return `Result`.
- Don't use global mutable state.
- Avoid deep nesting—refactor as needed.
- Treat warnings as errors in CI.
- Avoid `unsafe` unless needed and document it.
- Don't overuse `clone()`—borrow instead.
- Don't collect iterators early—keep them lazy.
- Avoid unnecessary allocations.

## Code Style
- Use `rustfmt` for formatting.
- Keep lines under 100 characters.
- Put `///` docs before items.
- Use `cargo clippy` to catch mistakes.

## Error Handling
- Use `Result<T, E>` for recoverable errors, `panic!` only for unrecoverable ones.
- Use `?` to propagate errors.
- Make custom error types with `thiserror` or `std::error::Error`.
- Use `Option<T>` for optional values.
- Give clear error messages.
- Validate function arguments.

## API Design
- Implement common traits: `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`.
- Use `From`, `AsRef`, `AsMut` for conversions.
- Collections should implement `FromIterator` and `Extend`.
- Use newtypes for type safety.
- Use specific types, not just `bool`.
- Use `Option<T>` for optional values.
- Only smart pointers should implement `Deref`/`DerefMut`.
- Use sealed traits and private fields to future-proof APIs.

## Testing and Docs
- Write unit tests with `#[cfg(test)]` and `#[test]`.
- Put test modules next to the code they test.
- Write integration tests in `tests/`.
- Document all public APIs with `///` comments.
- Use `#[doc(hidden)]` for private details.
- Document error conditions and safety.
- Use `?` in examples, not `unwrap()`.

## Project Organization
- Use semantic versioning in `Cargo.toml`.
- Add metadata: `description`, `license`, `repository`, `keywords`, `categories`.
- Use feature flags for optional features.
- Organize code into modules.
- Keep `main.rs`/`lib.rs` minimal—move logic to modules.

## Quality Checklist
- [ ] Naming follows RFC 430
- [ ] Implements `Debug`, `Clone`, `PartialEq` where needed
- [ ] Uses `Result<T, E>` and meaningful error types
- [ ] All public items have documentation
- [ ] Comprehensive test coverage
- [ ] No unnecessary `unsafe` code
- [ ] Efficient use of iterators and allocations
- [ ] Predictable, type-safe APIs
- [ ] Private fields and sealed traits where needed
- [ ] Passes `cargo fmt`, `cargo clippy`, and `cargo test`
