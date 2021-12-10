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
    table_name: Vec<u8>,
    column_family: String,
}
impl LogsTable {
    pub fn new(client: Client, table_name: String, column_family: String) -> Self {
        Self {
            client,
            column_family,
            table_name: table_name.into(),
        }
    }
    pub fn put_logs(&mut self, logs: Logs) -> hbase_thrift::Result<()> {
        let mut batch_mutations = Vec::new();
        for log in logs {
            let mut mutations = Vec::new();
            let mut hasher = DefaultHasher::new();
            for (key, value) in log {
                let value = value.get();
                key.hash(&mut hasher);
                value.hash(&mut hasher);
                mutations.push(Mutation {
                    column: Some(format!("{}:{}", self.column_family.clone(), key).into()),
                    value: Some(value.into()),
                    is_delete: Some(false),
                    write_to_w_a_l: Some(false),
                });
            }
            batch_mutations.push(BatchMutation {
                row: Some(hasher.finish().to_be_bytes().to_vec()),
                mutations: Some(mutations),
            });
        }

        self.client
            .mutate_rows(self.table_name.clone(), batch_mutations, BTreeMap::new())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut channel = TTcpChannel::new();
    channel.open("localhost:9090")?;
    let (i_chan, o_chan) = channel.split()?;
    let i_prot = TBinaryInputProtocol::new(TBufferedReadTransport::new(i_chan), true);
    let o_prot = TBinaryOutputProtocol::new(TBufferedWriteTransport::new(o_chan), true);

    let client = HbaseSyncClient::new(i_prot, o_prot);

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
