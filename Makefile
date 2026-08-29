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
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: test
test:
	$(CARGO) test

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
ci: fmt-check lint test deny audit-unsafe
