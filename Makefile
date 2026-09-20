# =============================================================================
# quire code rs Makefile
# =============================================================================

CARGO ?= cargo

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make fmt              - Format with rustfmt"
	@echo "  make fmt-check        - Verify formatting (CI gate)"
	@echo "  make lint             - Clippy with -D warnings"
	@echo "  make test             - cargo test"
	@echo "  make build            - Release build"
	@echo "  make clean            - cargo clean"
	@echo "  make deny             - cargo deny check licenses"
	@echo "  make audit-unsafe     - Enforce // SAFETY: comments on unsafe blocks"
	@echo "  make coverage         - Test Matrix rows vs the suite (quire coverage)"
	@echo "  make check-single-grammar - quire-code-parse --features rust links one grammar (FR-013-AC-4)"
	@echo "  make ci               - All CI gates locally (fmt-check + lint + test + deny + audit-unsafe)"
	@echo "  make bench            - NFR-003 budget + NFR-004 corpus-scale recall (performance lane)"

# =============================================================================
# Format / Lint / Test
# =============================================================================

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all -- --check

.PHONY: lint
lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

.PHONY: test
test:
	$(CARGO) test --workspace

# FR-013-AC-4: a consumer that compiles quire-code-parse with only the `rust`
# feature links the Rust grammar and no other. Built as a real assertion
# rather than a comment: the crate must both compile under this feature set
# and the resolved dependency tree must not name the other grammars.
.PHONY: check-single-grammar
check-single-grammar:
	$(CARGO) build -p quire-code-parse --no-default-features --features rust
	$(CARGO) test -p quire-code-parse --no-default-features --features rust
	@tree_output="$$($(CARGO) tree -p quire-code-parse --no-default-features --features rust)"; \
	if echo "$$tree_output" | grep -q "tree-sitter-python"; then \
		echo "FAIL: tree-sitter-python is reachable with only the rust feature enabled" >&2; \
		exit 1; \
	fi; \
	if echo "$$tree_output" | grep -q "tree-sitter-typescript"; then \
		echo "FAIL: tree-sitter-typescript is reachable with only the rust feature enabled" >&2; \
		exit 1; \
	fi; \
	echo "check-single-grammar: only tree-sitter-rust is linked"

.PHONY: build
build:
	$(CARGO) build --release

.PHONY: clean
clean:
	$(CARGO) clean

# =============================================================================
# Supply chain & safety
# =============================================================================

.PHONY: deny
deny:
	$(CARGO) deny check licenses

.PHONY: cargo-audit
cargo-audit:
	$(CARGO) audit

.PHONY: audit-unsafe
audit-unsafe:
	bash scripts/check_unsafe_comments.sh

# Every matrix row is backed by a tagged test, or its own declared verification
# method says why no symbol can exist. Needs `quire` on PATH and a module path
# declaring the traceability model (#8).
.PHONY: coverage
coverage:
	bash scripts/check_coverage.sh

# =============================================================================
# Performance lane
# =============================================================================

# Gates on the NFR-003 thresholds and reports NFR-004 recall at corpus scale.
# Deliberately outside `make ci`: benchmark timings on a shared runner would
# make the ordinary suite flaky, which NFR-003's Verification section calls out.
# The lane is `#[ignore]`d test functions rather than a custom bench harness so
# that each measurement is a symbol its matrix row's trace tag can bind (#8).
.PHONY: bench
bench:
	$(CARGO) test --test perf_lane -- --ignored --nocapture

# =============================================================================
# Composite
# =============================================================================

.PHONY: ci
ci: fmt-check lint test deny audit-unsafe coverage check-single-grammar
