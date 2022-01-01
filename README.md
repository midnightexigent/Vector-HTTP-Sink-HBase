# Vector HTTP Sink HBase

This a server that listens for data sent by the
[vector http sink](https://vector.dev/docs/reference/configuration/sinks/http/)
and writes it to [HBase](https://hbase.apache.org/)

It stores logs as structured data in an HBase column-family

This project interacts with `HBase`'s `thrift` API.
It uses [hbase-thrift](https://github.com/midnightexigent/hbase-thrift-rs)
and [thrift-pool](https://github.com/midnightexigent/thrift-pool-rs)

## Installation & Running

Either clone the repo and build it

```bash
git clone https://github.com/midnightexigent/vector-http-sink-hbase-rs.git
cd vector-http-sink-hbase-rs
cargo build --release

./target/release/vector-http-sink-hbase --help

```

Or install directly

```bash
cargo install --git https://github.com/midnightexigent/vector-http-sink-hbase-rs.git

vector-http-sink-hbase --help

```

Note: those 2 installation methods require [cargo](https://rustup.rs/)

This can also built with docker 

```bash
git clone https://github.com/midnightexigent/vector-http-sink-hbase-rs.git
cd vector-http-sink-hbase-rs

docker build -t vector-http-sink-hbase .

docker run vector-http-sink-hbase --help

```


## Usage

- Prepare HBase by creating it, opening its `thrift` port and creating the table/column-family where the structured
  logs will be stored
- In the `vector` configuration, add a sink with type `http` and set its `uri` to this process
- Start this process by setting the correct values (see `vector-http-sink-hbase --help`)
