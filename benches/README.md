# dot001 Criterion Benchmarks

Professional performance benchmarks for the dot001 parser and tracer using the Criterion library.

## Available Benchmarks

### Parser Benchmarks (`parser_bench.rs`)
- **parse_blend_file_***: Benchmark blend file parsing performance for different files
- **get_block_sequential**: Sequential block access patterns
- **get_block_random_access**: Random block access performance
- **blocks_len**: Block count operation performance
- **header_info**: Header information access
- **parse_options**: Performance with different ParseOptions configurations

### Tracer Benchmarks (`tracer_bench.rs`)
- **trace_dependencies_flat**: Flat dependency tracing
- **trace_dependencies_tree**: Tree dependency tracing
- **tracer_new**: Tracer initialization overhead
- **tracer_with_default_expanders**: Tracer with expanders initialization
- **trace_by_file**: Performance across different blend files
- **tracer_configurations**: Comparing different tracer configurations

## Running Benchmarks

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark Suite
```bash
cargo bench parser_bench    # Parser benchmarks only
cargo bench tracer_bench    # Tracer benchmarks only
```

### Run Specific Benchmark
```bash
cargo bench "parse_blend_file"
cargo bench "trace_dependencies"
```

### Save Baseline for Comparison
```bash
cargo bench -- --save-baseline main
```

### Compare with Baseline
```bash
cargo bench -- --baseline main
```

## Output and Reports

Criterion generates detailed reports in:
- `target/criterion/`: Detailed benchmark results
- `target/criterion/reports/index.html`: HTML reports with plots and statistics

### Key Metrics
- **Mean execution time**: Average performance
- **Standard deviation**: Performance consistency
- **Throughput**: Operations per second where applicable
- **Regression analysis**: Performance trends over time

## Configuration

### Benchmark Settings
Benchmarks are configured with:
- Multiple sample sizes for statistical accuracy
- Warm-up iterations to account for JIT compilation
- Confidence intervals for reliable measurements
- HTML report generation for detailed analysis

### Test Files
Benchmarks use test files from `tests/test-blendfiles/`:
- `library.blend`: Complex library file with dependencies
- `main.blend`: Main scene file
- `compressed.blend`: Compressed blend file

Missing test files are automatically skipped.

## Example Results Interpretation

```
parse_blend_file_library.blend
                        time:   [45.234 ms 45.891 ms 46.548 ms]
                        change: [-2.3% +0.1% +2.5%] (p = 0.89 > 0.05)
                        No change in performance detected.

trace_dependencies_flat time:   [12.345 ms 12.567 ms 12.789 ms]
                        thrpt:  [78.12 elem/s 79.55 elem/s 80.98 elem/s]
```

- **time**: [lower_bound mean upper_bound] confidence interval
- **change**: Performance change from baseline (if available)
- **thrpt**: Throughput measurements (elements/operations per second)

## Optimization Workflow

1. **Baseline**: `cargo bench -- --save-baseline before`
2. **Make changes**: Implement optimizations
3. **Compare**: `cargo bench -- --baseline before`
4. **Analyze**: Review HTML reports for detailed analysis
5. **Iterate**: Repeat with different approaches

## CI Integration

For continuous integration, use:
```bash
cargo bench -- --output-format terse
```

This provides machine-readable output suitable for automated analysis.

## Profiling Integration

For detailed profiling, combine with tools like:
```bash
cargo bench --bench parser_bench -- --profile-time=5
```

Or use with perf/flamegraph for detailed analysis.