# Contributing to MEMZ

Thank you for your interest in contributing to MEMZ! 🧠

## Getting Started

1. **Fork** the repository
2. **Clone** your fork: `git clone https://github.com/your-username/memz.git`
3. **Build**: `cargo build`
4. **Test**: `cargo test`
5. **Lint**: `cargo clippy -- -D warnings`

## Development Setup

### Prerequisites

- Rust 1.75+ (2024 edition)
- (Optional) Ollama for LLM integration testing
- (Optional) Python 3.10+ for evaluation tooling

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p memz-core

# Run benchmarks
cargo bench --bench memory_system

# Run clippy
cargo clippy --all-targets -- -D warnings
```

## Coding Standards

### Rust

- **No `unsafe` blocks** in memz-core (safety-critical code)
- **No `.unwrap()`** in non-test code — use `Result<T, E>` with `?`
- All public APIs must have **doc comments with examples**
- Use `#[must_use]` on functions that return values that should be used
- Follow standard Rust naming conventions

### Performance

- All hot-path code must fit within the 2ms frame budget
- New benchmarks must be added for performance-critical changes
- Run `cargo bench` before and after changes to verify no regression

### Testing

- **Unit tests** for all public functions
- **Property-based tests** (proptest) for invariants
- **Criterion benchmarks** for performance-critical paths
- **Integration tests** for cross-crate interactions

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with clear, atomic commits
3. Ensure all CI checks pass:
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `cargo bench` (no regressions > 10%)
   - `cargo doc` (no warnings)
4. Write a clear PR description explaining the change
5. Request review

## Architecture

See `Project Memz.md` for the complete design specification.

### Crate Structure

| Crate | Purpose |
|-------|---------|
| `memz-core` | Game-agnostic memory library |
| `memz-llm` | LLM abstraction layer |
| `memz-veloren` | Veloren ECS integration |
| `memz-bench` | Benchmark suite |

## Code of Conduct

Be kind. Be constructive. Be inclusive. We're building something amazing together.

## License

By contributing to MEMZ, you agree that your contributions will be licensed under GPL-3.0.
