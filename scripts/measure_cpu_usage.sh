#!/bin/bash
# CPU and Memory Usage Measurement for Enteract
# Performance benchmarking for macOS audio loopback

set -e

APP_NAME="Enteract"
DURATION=60  # seconds
SAMPLE_INTERVAL=1  # seconds
OUTPUT_DIR="performance_metrics"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}ℹ${NC}  $1"
}

print_success() {
    echo -e "${GREEN}✅${NC} $1"
}

print_error() {
    echo -e "${RED}❌${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC}  $1"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--name)
            APP_NAME="$2"
            shift 2
            ;;
        -d|--duration)
            DURATION="$2"
            shift 2
            ;;
        -i|--interval)
            SAMPLE_INTERVAL="$2"
            shift 2
            ;;
        -h|--help)
            cat << EOF
Usage: $0 [options]

Options:
    -n, --name <name>       Application name (default: Enteract)
    -d, --duration <sec>    Measurement duration in seconds (default: 60)
    -i, --interval <sec>    Sample interval in seconds (default: 1)
    -h, --help              Show this help message

Examples:
    $0                              # Default: measure Enteract for 60s
    $0 -n "Enteract" -d 120         # Measure for 2 minutes
    $0 -n "Enteract" -i 2           # Sample every 2 seconds

Output:
    Creates CSV file in $OUTPUT_DIR/ directory with timestamp
    Generates statistics summary

EOF
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

# Find the process
find_process() {
    # Try multiple patterns
    PID=$(pgrep -f "$APP_NAME" | head -1)

    if [ -z "$PID" ]; then
        # Try without case sensitivity
        PID=$(pgrep -fi "$APP_NAME" | head -1)
    fi

    if [ -z "$PID" ]; then
        # Try process name only (not full path)
        PID=$(pgrep -x "$APP_NAME" | head -1)
    fi

    echo "$PID"
}

# Main measurement function
measure_performance() {
    print_info "Measuring CPU/Memory usage for: $APP_NAME"
    echo ""

    # Find the process
    PID=$(find_process)

    if [ -z "$PID" ]; then
        print_error "$APP_NAME is not running"
        echo ""
        echo "Please start the application first, then run this script."
        echo "Alternatively, specify the process name with -n flag:"
        echo "  $0 -n \"com.quaternionstudios.enteract\""
        exit 1
    fi

    print_success "Found $APP_NAME (PID: $PID)"

    # Get initial process info
    PROC_NAME=$(ps -p "$PID" -o comm= | xargs)
    print_info "Process name: $PROC_NAME"
    echo ""

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Generate output filename with timestamp
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    OUTPUT_FILE="$OUTPUT_DIR/cpu_usage_${TIMESTAMP}.csv"

    print_info "Measurement settings:"
    echo "  Duration: ${DURATION}s"
    echo "  Interval: ${SAMPLE_INTERVAL}s"
    echo "  Samples: $((DURATION / SAMPLE_INTERVAL))"
    echo "  Output: $OUTPUT_FILE"
    echo ""

    # Write CSV header
    echo "timestamp,elapsed_sec,cpu_percent,mem_mb,threads" > "$OUTPUT_FILE"

    print_info "Starting measurement..."
    echo ""

    START_TIME=$(date +%s)

    # Measurement loop
    for i in $(seq 1 $DURATION); do
        # Check if process still exists
        if ! ps -p "$PID" > /dev/null 2>&1; then
            print_error "Process $PID terminated during measurement"
            break
        fi

        # Get CPU usage (percentage)
        CPU=$(ps -p "$PID" -o %cpu= | xargs)

        # Get memory usage (RSS in KB, convert to MB)
        MEM_KB=$(ps -p "$PID" -o rss= | xargs)
        MEM_MB=$(echo "scale=2; $MEM_KB / 1024" | bc)

        # Get thread count
        THREADS=$(ps -M -p "$PID" | wc -l | xargs)
        THREADS=$((THREADS - 1))  # Subtract header line

        # Get current timestamp
        CURRENT_TIME=$(date +%s)
        ELAPSED=$((CURRENT_TIME - START_TIME))
        TIME_STR=$(date +"%H:%M:%S")

        # Write to CSV
        echo "$TIME_STR,$ELAPSED,$CPU,$MEM_MB,$THREADS" >> "$OUTPUT_FILE"

        # Print progress (overwrite line)
        printf "\r  [%3d/%3d] CPU: %6.2f%%  Memory: %8.2f MB  Threads: %3d" \
            "$i" "$DURATION" "$CPU" "$MEM_MB" "$THREADS"

        # Wait for next sample (unless last iteration)
        if [ "$i" -lt "$DURATION" ]; then
            sleep "$SAMPLE_INTERVAL"
        fi
    done

    echo ""  # New line after progress
    echo ""
    print_success "Measurement complete: $OUTPUT_FILE"
    echo ""

    # Calculate statistics
    calculate_statistics "$OUTPUT_FILE"
}

# Calculate and display statistics
calculate_statistics() {
    local file="$1"

    print_info "Performance Statistics:"
    echo ""

    # CPU statistics
    echo "CPU Usage:"
    CPU_AVG=$(awk -F',' 'NR>1 {sum+=$3; count++} END {printf "%.2f", sum/count}' "$file")
    CPU_MAX=$(awk -F',' 'NR>1 {if($3>max) max=$3} END {printf "%.2f", max}' "$file")
    CPU_MIN=$(awk -F',' 'NR>1 {if(NR==2) min=$3; if($3<min) min=$3} END {printf "%.2f", min}' "$file")

    echo "  Average: ${CPU_AVG}%"
    echo "  Peak:    ${CPU_MAX}%"
    echo "  Minimum: ${CPU_MIN}%"

    # Evaluate against targets
    if (( $(echo "$CPU_AVG < 5.0" | bc -l) )); then
        print_success "  Target: <5% (PASS)"
    elif (( $(echo "$CPU_AVG < 20.0" | bc -l) )); then
        print_warning "  Target: <5% (WARN - higher than target)"
    else
        print_error "  Target: <5% (FAIL - significantly higher)"
    fi

    echo ""

    # Memory statistics
    echo "Memory Usage:"
    MEM_AVG=$(awk -F',' 'NR>1 {sum+=$4; count++} END {printf "%.2f", sum/count}' "$file")
    MEM_MAX=$(awk -F',' 'NR>1 {if($4>max) max=$4} END {printf "%.2f", max}' "$file")
    MEM_MIN=$(awk -F',' 'NR>1 {if(NR==2) min=$4; if($4<min) min=$4} END {printf "%.2f", min}' "$file")

    echo "  Average: ${MEM_AVG} MB"
    echo "  Peak:    ${MEM_MAX} MB"
    echo "  Minimum: ${MEM_MIN} MB"

    # Check for memory growth (potential leak)
    MEM_START=$(awk -F',' 'NR==2 {print $4}' "$file")
    MEM_END=$(awk -F',' 'END {print $4}' "$file")
    MEM_GROWTH=$(echo "$MEM_END - $MEM_START" | bc)

    echo "  Growth:  ${MEM_GROWTH} MB"

    if (( $(echo "$MEM_GROWTH < 10" | bc -l) )); then
        print_success "  Stability: Good (minimal growth)"
    elif (( $(echo "$MEM_GROWTH < 50" | bc -l) )); then
        print_warning "  Stability: Fair (some growth detected)"
    else
        print_error "  Stability: Poor (significant growth - possible leak)"
    fi

    echo ""

    # Thread statistics
    echo "Thread Count:"
    THREADS_AVG=$(awk -F',' 'NR>1 {sum+=$5; count++} END {printf "%.1f", sum/count}' "$file")
    THREADS_MAX=$(awk -F',' 'NR>1 {if($5>max) max=$5} END {print max}' "$file")

    echo "  Average: ${THREADS_AVG}"
    echo "  Peak:    ${THREADS_MAX}"

    echo ""

    # Generate summary report
    REPORT_FILE="$OUTPUT_DIR/summary_${TIMESTAMP}.txt"
    cat > "$REPORT_FILE" << EOF
Performance Measurement Summary
================================

Application: $APP_NAME
Duration: ${DURATION}s
Sample Interval: ${SAMPLE_INTERVAL}s
Timestamp: $(date)

CPU Usage
---------
Average: ${CPU_AVG}%
Peak:    ${CPU_MAX}%
Minimum: ${CPU_MIN}%

Memory Usage
------------
Average: ${MEM_AVG} MB
Peak:    ${MEM_MAX} MB
Minimum: ${MEM_MIN} MB
Growth:  ${MEM_GROWTH} MB

Thread Count
------------
Average: ${THREADS_AVG}
Peak:    ${THREADS_MAX}

Raw Data: $OUTPUT_FILE
EOF

    print_info "Summary report: $REPORT_FILE"
    echo ""
}

# Run measurement
measure_performance

# Offer to visualize (if gnuplot available)
if command -v gnuplot &> /dev/null; then
    echo ""
    read -p "Generate performance graph? (requires gnuplot) [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        generate_graph "$OUTPUT_FILE"
    fi
fi

print_success "Performance measurement complete"
