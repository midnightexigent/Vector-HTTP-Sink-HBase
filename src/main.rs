use axum::{
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    AddExtensionLayer, Json, Router,
};
use clap::Parser;
use hbase_thrift::{BatchMutationBuilder, Client, MutationBuilder, THbaseSyncClientExt};
use serde_json::value::RawValue;
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::trace::TraceLayer;

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

type SharedState = Arc<RwLock<State>>;

struct State {
    column_family: String,
    table_name: String,
    client: Client,
}
impl State {
    pub fn new(client: Client, table_name: String, column_family: String) -> Self {
        Self {
            client,
            table_name,
            column_family,
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
        self.client.put(&self.table_name, row_batches, None, None)
    }
}

#[derive(Parser)]
#[clap(version, about, author)]
struct Cli {
    #[clap(long, default_value = "localhost:9090")]
    pub hbase_addr: String,
    #[clap(long, default_value = "logs")]
    pub table_name: String,
    #[clap(long, default_value = "data")]
    pub column_family: String,
    #[clap(long, default_value = "/")]
    pub listen_route: String,
    #[clap(long, default_value = "0.0.0.0:3000")]
    pub listen_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let client = hbase_thrift::client(cli.hbase_addr)?;
    let shared_state = SharedState::new(RwLock::new(State::new(
        client,
        cli.table_name,
        cli.column_family,
    )));

    let app = Router::new()
        .route("/", post(put_logs))
        .layer(AddExtensionLayer::new(shared_state))
        .layer(TraceLayer::new_for_http());

    tracing::debug!("listening on {}", cli.listen_addr);
    axum::Server::bind(&cli.listen_addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn put_logs<'a>(
    Json(logs): Json<Logs>,
    Extension(table): Extension<SharedState>,
) -> impl IntoResponse {
    table.write().unwrap().put_logs(logs)?;
    Ok::<_, Error>(StatusCode::CREATED)
}
