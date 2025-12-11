#!/usr/bin/env bash
# Comparison testing script for Rust vs TypeScript versions
# Verifies CLI option compatibility, output consistency, and exit codes

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
PASSED=0
FAILED=0
SKIPPED=0

# Binary paths
RUST_BIN="${RUST_BIN:-./target/release/smart-scribe}"
TS_BIN="${TS_BIN:-bun run src/index.ts}"

print_header() {
    echo ""
    echo "=========================================="
    echo "$1"
    echo "=========================================="
}

print_test() {
    echo -n "  Testing: $1... "
}

pass() {
    echo -e "${GREEN}PASS${NC}"
    PASSED=$((PASSED + 1))
}

fail() {
    echo -e "${RED}FAIL${NC}"
    echo "    Expected: $1"
    echo "    Got: $2"
    FAILED=$((FAILED + 1))
}

skip() {
    echo -e "${YELLOW}SKIP${NC} ($1)"
    SKIPPED=$((SKIPPED + 1))
}

# Check if binaries exist
check_binaries() {
    print_header "Checking binaries"

    print_test "Rust binary exists"
    if [[ -x "$RUST_BIN" ]]; then
        pass
    else
        echo -e "${RED}FAIL${NC}"
        echo "    Rust binary not found at: $RUST_BIN"
        echo "    Run: cargo build --release"
        exit 1
    fi

    print_test "TypeScript can run"
    local has_bun=false
    local has_ts=false
    command -v bun > /dev/null 2>&1 && has_bun=true
    [[ -f "src/index.ts" ]] && has_ts=true

    if [[ "$has_bun" == "true" && "$has_ts" == "true" ]]; then
        pass
        TS_AVAILABLE=true
    else
        echo -e "${YELLOW}SKIP${NC} (TypeScript not available)"
        echo "    Note: Some comparison tests will be skipped"
        TS_AVAILABLE=false
    fi
}

# Test help output structure
test_help_output() {
    print_header "Testing --help output"

    print_test "--help exits with 0"
    if $RUST_BIN --help > /dev/null 2>&1; then
        pass
    else
        fail "exit 0" "exit $?"
    fi

    print_test "--help contains usage info"
    local help_output
    help_output=$($RUST_BIN --help 2>&1)
    if echo "$help_output" | grep -qi "usage"; then
        pass
    else
        fail "contains 'usage'" "missing"
    fi

    print_test "--help contains --duration"
    if echo "$help_output" | grep -qF -- "--duration"; then
        pass
    else
        fail "contains '--duration'" "missing"
    fi

    print_test "--help contains --domain"
    if echo "$help_output" | grep -qF -- "--domain"; then
        pass
    else
        fail "contains '--domain'" "missing"
    fi

    print_test "--help contains --daemon"
    if echo "$help_output" | grep -qF -- "--daemon"; then
        pass
    else
        fail "contains '--daemon'" "missing"
    fi

    print_test "--help contains --clipboard"
    if echo "$help_output" | grep -qF -- "--clipboard"; then
        pass
    else
        fail "contains '--clipboard'" "missing"
    fi

    print_test "--help contains --keystroke"
    if echo "$help_output" | grep -qF -- "--keystroke"; then
        pass
    else
        fail "contains '--keystroke'" "missing"
    fi

    print_test "--help contains --notify"
    if echo "$help_output" | grep -qF -- "--notify"; then
        pass
    else
        fail "contains '--notify'" "missing"
    fi

    print_test "--help contains config subcommand"
    if echo "$help_output" | grep -q "config"; then
        pass
    else
        fail "contains 'config'" "missing"
    fi

    print_test "--help lists all domains"
    if echo "$help_output" | grep -q "general" && \
       echo "$help_output" | grep -q "dev" && \
       echo "$help_output" | grep -q "medical" && \
       echo "$help_output" | grep -q "legal" && \
       echo "$help_output" | grep -q "finance"; then
        pass
    else
        fail "lists all domains" "some missing"
    fi
}

# Test version output
test_version_output() {
    print_header "Testing --version output"

    print_test "--version exits with 0"
    if $RUST_BIN --version > /dev/null 2>&1; then
        pass
    else
        fail "exit 0" "exit $?"
    fi

    print_test "--version contains version number"
    local version_output
    version_output=$($RUST_BIN --version 2>&1)
    if echo "$version_output" | grep -qE "[0-9]+\.[0-9]+\.[0-9]+"; then
        pass
    else
        fail "contains version number" "missing"
    fi

    print_test "--version contains program name"
    if echo "$version_output" | grep -qi "smart-scribe"; then
        pass
    else
        fail "contains 'smart-scribe'" "missing"
    fi
}

# Test config commands
test_config_commands() {
    print_header "Testing config commands"

    print_test "config path exits with 0"
    if $RUST_BIN config path > /dev/null 2>&1; then
        pass
    else
        fail "exit 0" "exit $?"
    fi

    print_test "config path contains config.toml"
    local path_output
    path_output=$($RUST_BIN config path 2>&1)
    if echo "$path_output" | grep -q "config.toml"; then
        pass
    else
        fail "contains 'config.toml'" "$path_output"
    fi

    print_test "config list exits with 0 (no config file)"
    if HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent $RUST_BIN config list > /dev/null 2>&1; then
        pass
    else
        fail "exit 0" "exit $?"
    fi

    print_test "config get unknown_key fails"
    if ! $RUST_BIN config get unknown_key > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi

    print_test "config set unknown_key fails"
    if ! $RUST_BIN config set unknown_key value > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi
}

# Test error handling
test_error_handling() {
    print_header "Testing error handling"

    print_test "invalid duration fails"
    if ! $RUST_BIN --duration invalid > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi

    print_test "invalid domain fails"
    if ! $RUST_BIN --domain invalid > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi

    print_test "daemon + duration conflict fails"
    if ! $RUST_BIN --daemon --duration 30s > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi

    print_test "missing API key fails"
    if ! HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent GEMINI_API_KEY= $RUST_BIN > /dev/null 2>&1; then
        pass
    else
        fail "exit non-zero" "exit 0"
    fi
}

# Test option formats
test_option_formats() {
    print_header "Testing option formats"

    # These tests verify the parser accepts the formats but will fail due to missing API key
    # We just check that we get past parsing

    print_test "duration format: 30s"
    local output
    output=$(HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent $RUST_BIN --duration 30s 2>&1 || true)
    if echo "$output" | grep -qi "api.*key\|api_key"; then
        pass  # Got past parsing to API key check
    else
        fail "accepts 30s format" "$output"
    fi

    print_test "duration format: 1m"
    output=$(HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent $RUST_BIN --duration 1m 2>&1 || true)
    if echo "$output" | grep -qi "api.*key\|api_key"; then
        pass
    else
        fail "accepts 1m format" "$output"
    fi

    print_test "duration format: 2m30s"
    output=$(HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent $RUST_BIN --duration 2m30s 2>&1 || true)
    if echo "$output" | grep -qi "api.*key\|api_key"; then
        pass
    else
        fail "accepts 2m30s format" "$output"
    fi

    print_test "short options: -d, -D, -c, -k, -n"
    output=$(HOME=/nonexistent XDG_CONFIG_HOME=/nonexistent $RUST_BIN -d 10s -D general -c -k -n 2>&1 || true)
    if echo "$output" | grep -qi "api.*key\|api_key"; then
        pass
    else
        fail "accepts short options" "$output"
    fi
}

# Compare with TypeScript version if available
test_ts_comparison() {
    if [[ "$TS_AVAILABLE" != "true" ]]; then
        print_header "TypeScript Comparison (skipped - not available)"
        return
    fi

    print_header "TypeScript Comparison"

    print_test "both --help have similar structure"
    local rust_help ts_help
    rust_help=$($RUST_BIN --help 2>&1)
    ts_help=$($TS_BIN --help 2>&1)

    # Check both mention key options
    rust_has_duration=$(echo "$rust_help" | grep -cF -- "--duration" || true)
    ts_has_duration=$(echo "$ts_help" | grep -cF -- "--duration" || true)

    if [[ "$rust_has_duration" -gt 0 && "$ts_has_duration" -gt 0 ]]; then
        pass
    else
        fail "both have --duration" "rust=$rust_has_duration, ts=$ts_has_duration"
    fi

    print_test "both config path return .toml paths"
    local rust_path ts_path
    rust_path=$($RUST_BIN config path 2>&1)
    ts_path=$($TS_BIN config path 2>&1)

    if echo "$rust_path" | grep -q ".toml" && echo "$ts_path" | grep -q ".toml"; then
        pass
    else
        fail "both return .toml" "rust=$rust_path, ts=$ts_path"
    fi
}

# Print summary
print_summary() {
    print_header "Summary"
    echo ""
    echo -e "  ${GREEN}Passed:${NC}  $PASSED"
    echo -e "  ${RED}Failed:${NC}  $FAILED"
    echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED"
    echo ""

    if [[ $FAILED -gt 0 ]]; then
        echo -e "${RED}Some tests failed!${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    fi
}

# Document known differences
print_differences() {
    print_header "Known Differences"
    echo ""
    echo "  The following differences are expected between versions:"
    echo ""
    echo "  1. Version number: Rust v2.0.0, TypeScript v1.0.0"
    echo "  2. Help text formatting may vary slightly"
    echo "  3. Error message wording may differ"
    echo "  4. Rust version uses SIGINT for early stop (vs SIGTERM)"
    echo "  5. Daemon signal handling: Rust uses SIGUSR1 for toggle"
    echo ""
}

# Main
main() {
    echo "SmartScribe Version Comparison Test"
    echo "Comparing: Rust ($RUST_BIN) vs TypeScript"

    check_binaries
    test_help_output
    test_version_output
    test_config_commands
    test_error_handling
    test_option_formats
    test_ts_comparison
    print_differences
    print_summary
}

main "$@"
