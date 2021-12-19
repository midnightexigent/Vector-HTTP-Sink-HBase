use axum::{
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    AddExtensionLayer, Json, Router,
};
use hbase_thrift::{
    hbase::{BatchMutation, HbaseSyncClient, Mutation, THbaseSyncClient},
    thrift::{
        protocol::{TBinaryInputProtocol, TBinaryOutputProtocol},
        transport::{
            ReadHalf, TBufferedReadTransport, TBufferedWriteTransport, TIoChannel, TTcpChannel,
            WriteHalf,
        },
    },
    BatchMutationBuilder, MutationBuilder, THbaseSyncClientExt,
};
use serde_json::value::RawValue;
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};
use tower_http::trace::TraceLayer;

type Client = HbaseSyncClient<
    TBinaryInputProtocol<TBufferedReadTransport<ReadHalf<TTcpChannel>>>,
    TBinaryOutputProtocol<TBufferedWriteTransport<WriteHalf<TTcpChannel>>>,
>;

type Logs = Vec<BTreeMap<String, Box<RawValue>>>;

struct Error(hbase_thrift::Error);

impl From<hbase_thrift::Error> for Error {
    fn from(err: hbase_thrift::Error) -> Self {
        Self(err)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

struct LogsTable {
    client: Client,
    table_name: String,
    column_family: String,
}
impl LogsTable {
    pub fn new(client: Client, table_name: String, column_family: String) -> Self {
        Self {
            client,
            column_family,
            table_name,
        }
    }
    pub fn put_logs(&mut self, logs: Logs) -> hbase_thrift::Result<()> {
        let mut row_batches = Vec::new();
        for log in logs {
            let mut bmb = <BatchMutationBuilder>::default();
            for (k, v) in log {
                let mut mb = MutationBuilder::default();
                mb.value(v.get());
                mb.column(self.column_family.clone(), k);
                bmb.mutation(mb);
            }
            row_batches.push(bmb.build());
        }
        self.client
            .table(self.table_name.clone())
            .put(row_batches, None, None)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = hbase_thrift::client("localhost:9090")?;

    let app = Router::new()
        .route("/", post(put_logs))
        .layer(AddExtensionLayer::new(Arc::new(RwLock::new(
            LogsTable::new(client, "logs".to_string(), "data".to_string()),
        ))))
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:3000".parse()?;
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn put_logs(
    Json(logs): Json<Logs>,
    Extension(table): Extension<Arc<RwLock<LogsTable>>>,
) -> impl IntoResponse {
    table.write().unwrap().put_logs(logs)?;
    Ok::<_, Error>(StatusCode::CREATED)
}
