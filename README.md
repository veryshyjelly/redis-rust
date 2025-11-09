# Redis Implementation in Rust

[![progress-banner](https://backend.codecrafters.io/progress/redis/cacd4cd1-d6af-4c20-9970-a9c0c4d87c0f)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

A fully-featured Redis server implementation written in Rust, built as part of the [CodeCrafters Redis Challenge](https://codecrafters.io/challenges/redis). This implementation supports a comprehensive set of Redis commands, data structures, and advanced features including replication, persistence, transactions, and pub/sub.

## Features

### Core Commands
- **Connection**: `PING`, `ECHO`, `INFO`
- **String Operations**: `GET`, `SET`, `INCR`
- **Key Management**: `TYPE`, `KEYS`

### Data Structures
- **Lists**: `LPUSH`, `RPUSH`, `LPOP`, `BLPOP`, `LRANGE`, `LLEN`
- **Sorted Sets (ZSet)**: `ZADD`, `ZCARD`, `ZCOUNT`, `ZRANK`, `ZRANGE`, `ZREM`, `ZSCORE`
- **Streams**: `XADD`, `XDEL`, `XLEN`, `XRANGE`, `XREAD`
- **Geospatial**: `GEOADD`, `GEOPOS`, `GEODIST`, `GEOSEARCH`

### Advanced Features
- **Transactions**: `MULTI`, `EXEC`, `DISCARD`
- **Pub/Sub**: `SUBSCRIBE`, `UNSUBSCRIBE`, `PUBLISH`
- **Replication**: Master-slave replication with `REPLCONF`, `PSYNC`, `WAIT`
- **Persistence**: RDB file format support with expiration tracking
- **Configuration**: `CONFIG GET`
- **ACL (Access Control Lists)**: `ACL WHOAMI`, `ACL GETUSER`, `ACL SETUSER`, `AUTH`

## Architecture

### Project Structure
```
src/
├── main.rs              # Entry point and server initialization
├── frame/               # RESP (REdis Serialization Protocol) implementation
│   ├── mod.rs
│   ├── decode.rs        # RESP protocol parsing
│   ├── encode.rs        # RESP protocol serialization
│   ├── frame.rs         # Frame type definitions
│   └── debug.rs         # Debug implementations
├── server/              # Command handlers
│   ├── server.rs        # Core server logic and command dispatch
│   ├── string.rs        # String commands
│   ├── list.rs          # List commands
│   ├── zset.rs          # Sorted set commands
│   ├── stream.rs        # Stream commands
│   ├── geospatial.rs    # Geospatial commands
│   ├── transaction.rs   # Transaction support
│   ├── pubsub.rs        # Pub/Sub implementation
│   ├── replication.rs   # Replication logic
│   ├── persistence.rs   # Configuration and persistence
│   ├── acl.rs           # Access control
│   ├── misc.rs          # Miscellaneous commands
│   └── errors.rs        # Error handling
├── store/               # Data storage layer
│   ├── mod.rs
│   ├── value.rs         # Value type implementations
│   ├── stream.rs        # Stream entry handling
│   └── info.rs          # Server info
├── rdb/                 # RDB persistence
│   ├── mod.rs
│   └── decode.rs        # RDB file parsing
├── parser.rs            # Frame parser
└── slave.rs             # Slave replication handler
```

### Key Components

#### Frame Protocol
Implements the RESP (REdis Serialization Protocol) supporting:
- Simple strings, bulk strings, errors
- Integers, doubles, booleans
- Arrays, maps, sets
- RDB file transfers

#### Storage Engine
- In-memory key-value store with multiple data types
- TTL/expiration support with priority queue
- Geospatial indexing using geohash encoding
- Stream entries with time-based IDs

#### Replication
- Master-slave architecture
- Full resynchronization with RDB snapshots
- Incremental replication with command propagation
- Offset tracking and acknowledgments

## Getting Started

### Prerequisites
- Rust 1.88 or higher
- Cargo

### Installation

```sh
# Clone the repository
git clone <repository-url>
cd redis-rust

# Build the project
cargo build --release
```

### Running the Server

#### Basic Usage
```sh
# Run on default port (6379)
./your_program.sh

# Run on custom port
./your_program.sh --port 6380
```

#### With Persistence
```sh
./your_program.sh --dir /path/to/data --dbfilename dump.rdb
```

#### As Replica
```sh
./your_program.sh --port 6380 --replicaof localhost 6379
```

### Testing

```sh
# Run tests
cargo test

# Run with debug logging
DEBUG=true cargo run
```

## Usage Examples

### Basic Operations
```bash
# Connect with redis-cli
redis-cli -p 6379

# String operations
SET mykey "Hello"
GET mykey
INCR counter

# List operations
LPUSH mylist "world"
LPUSH mylist "hello"
LRANGE mylist 0 -1

# Sorted sets
ZADD leaderboard 100 "player1"
ZADD leaderboard 200 "player2"
ZRANGE leaderboard 0 -1
```

### Transactions
```bash
MULTI
SET key1 "value1"
SET key2 "value2"
EXEC
```

### Pub/Sub
```bash
# Terminal 1 (Subscriber)
SUBSCRIBE mychannel

# Terminal 2 (Publisher)
PUBLISH mychannel "Hello subscribers!"
```

### Geospatial
```bash
GEOADD locations 13.361389 38.115556 "Palermo"
GEOADD locations 15.087269 37.502669 "Catania"
GEODIST locations "Palermo" "Catania" km
GEOSEARCH locations FROMLONLAT 15 37 BYRADIUS 200 km
```

### Streams
```bash
XADD mystream * sensor-id 1234 temperature 25.5
XRANGE mystream - +
XREAD BLOCK 1000 STREAMS mystream 0
```

## Technical Highlights

### Asynchronous I/O
- Built on Tokio for high-performance async networking
- Non-blocking command execution
- Concurrent client handling

### Memory Efficiency
- Zero-copy buffer management with `bytes` crate
- Efficient data structure implementations
- Smart expiration management

### Protocol Compliance
- Full RESP2/RESP3 protocol support
- Proper error handling and reporting
- Type checking and validation

### Geospatial Implementation
- Geohash encoding/decoding
- Haversine distance calculation
- Efficient radius queries

## Performance Characteristics

- **Single-threaded event loop**: Similar to Redis's architecture
- **Async I/O**: Non-blocking operations for high throughput
- **In-memory storage**: Fast read/write operations
- **Efficient serialization**: Minimal overhead in RESP encoding/decoding

## Limitations

This is an educational implementation with some intentional simplifications:
- Single-threaded execution model
- In-memory only (no AOF persistence)
- Simplified cluster support
- Limited to subset of Redis commands

## Contributing

This project was completed as part of the CodeCrafters challenge. While it's primarily for educational purposes, suggestions and improvements are welcome!

## License

This project is part of the CodeCrafters curriculum and follows their guidelines.

## Acknowledgments

- [CodeCrafters](https://codecrafters.io) for the excellent learning platform
- [Redis](https://redis.io) for the original design and protocol specification
- The Rust community for amazing async libraries (Tokio, bytes, etc.)

## Resources

- [Redis Protocol Specification](https://redis.io/docs/reference/protocol-spec/)
- [Redis Commands Documentation](https://redis.io/commands/)
- [Tokio Documentation](https://tokio.rs/)
- [CodeCrafters Redis Challenge](https://codecrafters.io/challenges/redis)
