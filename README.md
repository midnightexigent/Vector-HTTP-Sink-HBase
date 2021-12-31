# Vector HTTP Sink HBase

This a server that listens for data sent by the 
[vector http sink](https://vector.dev/docs/reference/configuration/sinks/http/)
and writes it to [HBase](https://hbase.apache.org/)

It stores logs as structured data in an HBase column-family

## Usage

- Prepare HBase by creating it, opening its `thrift` port and creating the table/column-family where the structured 
logs will be stored
- In the `vector` configuration, add a sink with type `http` and set its `uri` to this process
- Start this process by setting the correct values (see `vector-http-sink-hbase --help`)


