#!/bin/bash

# Oracle Multi-Node Runner Script
# This script starts multiple oracle nodes for different exchanges

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
AGGREGATOR_URL="${AGGREGATOR_URL:-http://localhost:50051}"
LOG_LEVEL="${RUST_LOG:-info}"

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if cargo is installed
check_dependencies() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust and Cargo first."
        exit 1
    fi
    print_info "Dependencies checked successfully"
}

# Function to build the project
build_project() {
    print_info "Building the oracle-node project..."
    cargo build --release
    if [ $? -eq 0 ]; then
        print_info "Build completed successfully"
    else
        print_error "Build failed"
        exit 1
    fi
}

# Function to start an oracle node
start_node() {
    local exchange=$1
    print_info "Starting $exchange oracle node..."
    
    RUST_LOG=$LOG_LEVEL AGGREGATOR_URL=$AGGREGATOR_URL \
        cargo run --release --bin oracle-node -- --exchange $exchange &
    
    local pid=$!
    echo $pid >> /tmp/oracle_nodes.pid
    print_info "$exchange oracle node started with PID: $pid"
}

# Function to stop all nodes
stop_all_nodes() {
    print_info "Stopping all oracle nodes..."
    
    if [ -f /tmp/oracle_nodes.pid ]; then
        while read pid; do
            if kill -0 $pid 2>/dev/null; then
                kill $pid
                print_info "Stopped node with PID: $pid"
            fi
        done < /tmp/oracle_nodes.pid
        rm /tmp/oracle_nodes.pid
    else
        print_warning "No PID file found. Nodes may not be running."
    fi
}

# Trap to handle script termination
trap 'stop_all_nodes; exit' INT TERM

# Main execution
main() {
    print_info "Oracle Multi-Node Runner"
    print_info "========================"
    print_info "Aggregator URL: $AGGREGATOR_URL"
    print_info "Log Level: $LOG_LEVEL"
    echo ""
    
    # Check dependencies
    check_dependencies
    
    # Build the project
    build_project
    
    # Clear previous PID file if exists
    [ -f /tmp/oracle_nodes.pid ] && rm /tmp/oracle_nodes.pid
    
    # Start oracle nodes for each exchange
    print_info "Starting oracle nodes for all exchanges..."
    echo ""
    
    start_node "binance"
    sleep 2
    
    start_node "coinbase"
    sleep 2
    
    start_node "kraken"
    
    echo ""
    print_info "All oracle nodes are running"
    print_info "Press Ctrl+C to stop all nodes"
    echo ""
    
    # Keep the script running
    while true; do
        sleep 1
    done
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --stop)
            stop_all_nodes
            exit 0
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --stop       Stop all running oracle nodes"
            echo "  --help, -h   Show this help message"
            echo ""
            echo "Environment Variables:"
            echo "  AGGREGATOR_URL   URL of the aggregator service (default: http://localhost:50051)"
            echo "  RUST_LOG         Logging level (default: info)"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
    shift
done

# Run main function
main