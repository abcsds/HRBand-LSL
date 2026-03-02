# HRBand-LSL Rust Project Tasks

# Default target
default: check

# Run the application
run:
    cargo run

# Run with logging hook example
run-example:
    cargo run --example with_logging_hook

# Run all tests
test:
    cargo test

# Run tests with no-default-features
test-no-lsl:
    cargo test --no-default-features

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt --check

# Run clippy linter
clippy:
    cargo clippy -- -D warnings

# Run clippy on all features
clippy-all:
    cargo clippy --all-features -- -D warnings
    cargo clippy --no-default-features -- -D warnings

# Check code quality (fmt + clippy + test)
check: fmt-check clippy test

# Build debug binary
build:
    cargo build

# Build release binary
build-release:
    cargo build --release

# Build example
build-example:
    cargo build --example with_logging_hook

# Generate documentation
doc:
    cargo doc --no-deps --open

# Clean build artifacts
clean:
    cargo clean

# Run all quality checks before committing
pre-commit: fmt clippy test clippy-all
    @echo "All checks passed!"

# Quick validation (no build)
quick-check: fmt-check clippy-all

# Full CI-like pipeline
ci: fmt clippy test build-release test-no-lsl build-example doc
    @echo "CI pipeline completed successfully!"

# Watch for changes and run tests
watch:
    @command -v cargo-watch >/dev/null 2>&1 && cargo watch -x test || echo "cargo-watch not installed. Install with: cargo install cargo-watch"

# Install development tools
install-tools:
    rustup component add rustfmt clippy
    cargo install cargo-watch cargo-tarpaulin

# Run code coverage
coverage:
    @command -v cargo-tarpaulin >/dev/null 2>&1 && cargo tarpaulin --out Html || echo "cargo-tarpaulin not installed. Run: just install-tools"

# Benchmark (if criterion is added)
bench:
    @echo "Benchmarking is not yet configured. Add criterion dependency to Cargo.toml"

# Help message
help:
    @echo "HRBand-LSL Rust Project Commands:"
    @echo ""
    @echo "Building:"
    @echo "  just build              - Build debug binary"
    @echo "  just build-release      - Build optimized release binary"
    @echo "  just build-example      - Build example with logging hook"
    @echo ""
    @echo "Testing:"
    @echo "  just test               - Run all tests"
    @echo "  just test-no-lsl        - Run tests without LSL feature"
    @echo "  just coverage           - Generate code coverage report"
    @echo ""
    @echo "Code Quality:"
    @echo "  just fmt                - Auto-format code"
    @echo "  just fmt-check          - Check formatting without modifying"
    @echo "  just clippy             - Run linter (clippy)"
    @echo "  just clippy-all         - Run clippy on all feature combinations"
    @echo "  just check              - Run fmt-check, clippy, and test"
    @echo "  just quick-check        - Quick check without building"
    @echo ""
    @echo "Running:"
    @echo "  just run                - Run the application"
    @echo "  just run-example        - Run with logging hook example"
    @echo "  just watch              - Watch for changes and run tests"
    @echo ""
    @echo "Maintenance:"
    @echo "  just clean              - Clean build artifacts"
    @echo "  just doc                - Generate and open documentation"
    @echo "  just install-tools      - Install development tools"
    @echo "  just pre-commit         - Run all checks before committing"
    @echo "  just ci                 - Run full CI pipeline"
    @echo ""
