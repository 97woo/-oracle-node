# Oracle Node

Standalone BTC Oracle Node extracted from the BTCFi project. This node fetches real-time BTC prices from multiple exchanges and provides consensus-based price aggregation.

## Features

- 🔄 Multi-exchange price aggregation (Binance, Coinbase, Kraken)
- ✅ Consensus mechanism (2/3 agreement required)
- 📡 gRPC communication with aggregator service
- ⚙️ Configurable via environment variables
- 🔐 Cryptographic signatures for data integrity

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/oracle-node.git
cd oracle-node

# Build the project
cargo build --release
```

## Usage

Run the oracle node with a specific exchange:

```bash
# For Binance
cargo run --bin oracle-node -- --exchange binance

# For Coinbase
cargo run --bin oracle-node -- --exchange coinbase

# For Kraken
cargo run --bin oracle-node -- --exchange kraken
```

## Configuration

Set the following environment variables:

- `AGGREGATOR_URL`: URL of the aggregator service (default: `http://localhost:50051`)
- `EXCHANGE`: Exchange to fetch prices from (binance/coinbase/kraken)
- `RUST_LOG`: Logging level (debug/info/warn/error)

## Architecture

The oracle node is part of a larger oracle system:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Binance   │     │  Coinbase   │     │   Kraken    │
│   Oracle    │     │   Oracle    │     │   Oracle    │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                          │
                    ┌─────▼─────┐
                    │Aggregator │
                    └───────────┘
```

## Development

Run tests:

```bash
cargo test
```

Run with debug logging:

```bash
RUST_LOG=debug cargo run --bin oracle-node -- --exchange binance
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
