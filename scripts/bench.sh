#!/usr/bin/env bash

# dot001 Criterion Benchmark Runner
# Professional benchmark runner with various options and reporting

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_help() {
    cat << EOF
dot001 Criterion Benchmark Runner

USAGE:
    $0 [OPTIONS] [BENCHMARK]

OPTIONS:
    -h, --help              Show this help message
    -a, --all               Run all benchmarks (default)
    -p, --parser            Run parser benchmarks only
    -t, --tracer            Run tracer benchmarks only
    -b, --baseline NAME     Save results as baseline NAME
    -c, --compare NAME      Compare against baseline NAME
    -q, --quick             Run quick benchmarks (fewer samples)
    -v, --verbose           Enable verbose output
    --open                  Open HTML report after completion
    --save-baseline         Save as 'main' baseline
    --terse                 Output in terse format (CI-friendly)

EXAMPLES:
    $0                      # Run all benchmarks
    $0 -p                   # Run parser benchmarks only
    $0 --baseline before    # Save results as 'before' baseline
    $0 --compare before     # Compare against 'before' baseline
    $0 parse_blend_file     # Run specific benchmark matching pattern

BENCHMARK NAMES:
    parser_bench            All parser benchmarks
    tracer_bench            All tracer benchmarks
    parse_blend_file        File parsing benchmarks
    trace_dependencies      Dependency tracing benchmarks
EOF
}

# Default options
RUN_PARSER=true
RUN_TRACER=true
BASELINE=""
COMPARE=""
QUICK=false
VERBOSE=false
OPEN_REPORT=false
TERSE=false
BENCHMARK_PATTERN=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -a|--all)
            RUN_PARSER=true
            RUN_TRACER=true
            shift
            ;;
        -p|--parser)
            RUN_PARSER=true
            RUN_TRACER=false
            shift
            ;;
        -t|--tracer)
            RUN_PARSER=false
            RUN_TRACER=true
            shift
            ;;
        -b|--baseline)
            BASELINE="$2"
            shift 2
            ;;
        -c|--compare)
            COMPARE="$2"
            shift 2
            ;;
        -q|--quick)
            QUICK=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --open)
            OPEN_REPORT=true
            shift
            ;;
        --save-baseline)
            BASELINE="main"
            shift
            ;;
        --terse)
            TERSE=true
            shift
            ;;
        -*)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
        *)
            BENCHMARK_PATTERN="$1"
            shift
            ;;
    esac
done

echo -e "${GREEN}dot001 Criterion Benchmark Runner${NC}"
echo "=================================="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "benches" ]; then
    echo -e "${RED}Error: Please run this script from the project root directory${NC}"
    exit 1
fi

# Check for test files
if [ ! -f "tests/test-blendfiles/library.blend" ]; then
    echo -e "${YELLOW}Warning: Test file tests/test-blendfiles/library.blend not found${NC}"
    echo "Some benchmarks may be skipped"
fi

# Build the project first
echo -e "${YELLOW}Building project in release mode...${NC}"
cargo build --release

# Prepare cargo bench arguments
BENCH_ARGS=""

if [ "$TERSE" = true ]; then
    BENCH_ARGS="$BENCH_ARGS -- --output-format terse"
elif [ "$QUICK" = true ]; then
    BENCH_ARGS="$BENCH_ARGS -- --quick"
fi

if [ -n "$BASELINE" ]; then
    BENCH_ARGS="$BENCH_ARGS -- --save-baseline $BASELINE"
fi

if [ -n "$COMPARE" ]; then
    BENCH_ARGS="$BENCH_ARGS -- --baseline $COMPARE"
fi

# Run benchmarks
if [ -n "$BENCHMARK_PATTERN" ]; then
    echo -e "${BLUE}Running benchmarks matching: $BENCHMARK_PATTERN${NC}"
    if [ "$VERBOSE" = true ]; then
        cargo bench "$BENCHMARK_PATTERN" $BENCH_ARGS
    else
        cargo bench "$BENCHMARK_PATTERN" $BENCH_ARGS 2>/dev/null
    fi
else
    if [ "$RUN_PARSER" = true ] && [ "$RUN_TRACER" = true ]; then
        echo -e "${BLUE}Running all benchmarks...${NC}"
        if [ "$VERBOSE" = true ]; then
            cargo bench $BENCH_ARGS
        else
            cargo bench $BENCH_ARGS 2>/dev/null
        fi
    elif [ "$RUN_PARSER" = true ]; then
        echo -e "${BLUE}Running parser benchmarks...${NC}"
        if [ "$VERBOSE" = true ]; then
            cargo bench parser_bench $BENCH_ARGS
        else
            cargo bench parser_bench $BENCH_ARGS 2>/dev/null
        fi
    elif [ "$RUN_TRACER" = true ]; then
        echo -e "${BLUE}Running tracer benchmarks...${NC}"
        if [ "$VERBOSE" = true ]; then
            cargo bench tracer_bench $BENCH_ARGS
        else
            cargo bench tracer_bench $BENCH_ARGS 2>/dev/null
        fi
    fi
fi

echo ""
echo -e "${GREEN}Benchmarks completed!${NC}"

# Show results location
if [ "$TERSE" != true ]; then
    echo -e "${YELLOW}Results available at:${NC}"
    echo "  Text results: target/criterion/"
    echo "  HTML reports: target/criterion/reports/index.html"
fi

# Open HTML report if requested
if [ "$OPEN_REPORT" = true ]; then
    HTML_REPORT="target/criterion/reports/index.html"
    if [ -f "$HTML_REPORT" ]; then
        echo -e "${BLUE}Opening HTML report...${NC}"
        if command -v xdg-open > /dev/null; then
            xdg-open "$HTML_REPORT"
        elif command -v open > /dev/null; then
            open "$HTML_REPORT"
        else
            echo "Cannot open HTML report automatically. Open: $HTML_REPORT"
        fi
    fi
fi

echo -e "${GREEN}Use '$0 --help' for more options${NC}"