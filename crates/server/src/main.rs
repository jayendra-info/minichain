use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;

use minichain_server::api::{
    self, AccountInfo, BlockInfo, KeypairInfo, TransactionInfo,
};

#[derive(Clone)]
struct AppState {
    data_dir: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ChainStatus {
    initialized: bool,
    height: u64,
    genesis_hash: Option<String>,
    authorities: Vec<String>,
}

#[derive(serde::Deserialize)]
struct InitRequest {
    data_dir: Option<String>,
    authorities: Option<usize>,
    block_time: Option<u64>,
}

#[derive(serde::Deserialize)]
struct NewAccountRequest {
    data_dir: Option<String>,
    name: Option<String>,
}

#[derive(serde::Deserialize)]
struct BalanceRequest {
    data_dir: Option<String>,
    address: String,
}

#[derive(serde::Deserialize)]
struct AccountInfoRequest {
    data_dir: Option<String>,
    address: String,
}

#[derive(serde::Deserialize)]
struct ListAccountsRequest {
    data_dir: Option<String>,
}

#[derive(serde::Deserialize)]
struct MintRequest {
    data_dir: Option<String>,
    from: String,
    to: String,
    amount: u64,
}

#[derive(serde::Deserialize)]
struct SendTxRequest {
    data_dir: Option<String>,
    from: String,
    to: String,
    amount: u64,
    gas_price: Option<u64>,
}

#[derive(serde::Deserialize)]
struct BlockListRequest {
    data_dir: Option<String>,
    count: Option<usize>,
}

#[derive(serde::Deserialize)]
struct BlockInfoRequest {
    data_dir: Option<String>,
    block_id: String,
}

#[derive(serde::Deserialize)]
struct ProduceBlockRequest {
    data_dir: Option<String>,
    authority: String,
}

#[derive(serde::Deserialize)]
struct DeployRequest {
    data_dir: Option<String>,
    from: String,
    source: String,
    gas_price: Option<u64>,
    gas_limit: u64,
}

#[derive(serde::Deserialize)]
struct CallRequest {
    data_dir: Option<String>,
    from: String,
    to: String,
    data: Option<String>,
    amount: Option<u64>,
    gas_price: Option<u64>,
}

fn get_data_dir(data_dir: &Option<String>) -> PathBuf {
    data_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./data"))
}

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "./data")]
    data_dir: String,
    
    #[arg(long, default_value = "3000")]
    port: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let data_dir = PathBuf::from(&args.data_dir);
    let port: u16 = args.port.parse().expect("Invalid port");

    let state = AppState { data_dir };

    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any::any())
        .allow_methods(tower_http::cors::Any::any())
        .allow_headers(tower_http::cors::Any::any());

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/init", post(init_blockchain))
        .route("/api/account/new", post(new_account))
        .route("/api/account/balance", post(get_balance))
        .route("/api/account/info", post(account_info))
        .route("/api/account/list", post(list_accounts))
        .route("/api/account/mint", post(mint_tokens))
        .route("/api/tx/send", post(send_transaction))
        .route("/api/tx/list", post(list_mempool))
        .route("/api/tx/clear", post(clear_mempool))
        .route("/api/block/list", post(list_blocks))
        .route("/api/block/info", post(block_info))
        .route("/api/block/produce", post(produce_block))
        .route("/api/contract/deploy", post(deploy_contract))
        .route("/api/contract/call", post(call_contract))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Starting minichain server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn status(State(state): State<AppState>) -> Json<ApiResponse<ChainStatus>> {
    let storage = match minichain_storage::Storage::open(&state.data_dir) {
        Ok(s) => s,
        Err(_) => {
            return Json(ApiResponse::ok(ChainStatus {
                initialized: false,
                height: 0,
                genesis_hash: None,
                authorities: vec![],
            }))
        }
    };

    let chain = minichain_storage::ChainStore::new(&storage);
    let initialized = chain.is_initialized().unwrap_or(false);
    let height = chain.get_height().unwrap_or(0);
    
    let genesis_hash = if let Ok(Some(genesis)) = chain.get_block_by_height(0) {
        Some(genesis.hash().to_hex())
    } else {
        None
    };

    let authorities = if state.data_dir.join("config.json").exists() {
        if let Ok(contents) = std::fs::read_to_string(state.data_dir.join("config.json")) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                json.get("authorities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Json(ApiResponse::ok(ChainStatus {
        initialized,
        height,
        genesis_hash,
        authorities,
    }))
}

async fn init_blockchain(
    State(state): State<AppState>,
    Json(req): Json<InitRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    let authorities = req.authorities.unwrap_or(1);
    let block_time = req.block_time.unwrap_or(5);

    match api::init_blockchain(&data_dir, authorities, block_time) {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn new_account(
    State(state): State<AppState>,
    Json(req): Json<NewAccountRequest>,
) -> Json<ApiResponse<KeypairInfo>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::create_account(&data_dir, req.name.as_deref()) {
        Ok(info) => Json(ApiResponse::ok(info)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn get_balance(
    State(state): State<AppState>,
    Json(req): Json<BalanceRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::get_balance(&data_dir, &req.address) {
        Ok(balance) => Json(ApiResponse::ok(balance)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn account_info(
    State(state): State<AppState>,
    Json(req): Json<AccountInfoRequest>,
) -> Json<ApiResponse<AccountInfo>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::get_account_info(&data_dir, &req.address) {
        Ok(info) => Json(ApiResponse::ok(info)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn list_accounts(
    State(state): State<AppState>,
    Json(req): Json<ListAccountsRequest>,
) -> Json<ApiResponse<Vec<KeypairInfo>>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::list_accounts(&data_dir) {
        Ok(accounts) => Json(ApiResponse::ok(accounts)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn mint_tokens(
    State(state): State<AppState>,
    Json(req): Json<MintRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::mint_tokens(&data_dir, &req.from, &req.to, req.amount) {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn send_transaction(
    State(state): State<AppState>,
    Json(req): Json<SendTxRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    let gas_price = req.gas_price.unwrap_or(1);
    match api::send_transaction(&data_dir, &req.from, &req.to, req.amount, gas_price) {
        Ok(hash) => Json(ApiResponse::ok(hash)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn list_mempool(
    State(state): State<AppState>,
    Json(req): Json<ListAccountsRequest>,
) -> Json<ApiResponse<Vec<TransactionInfo>>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::list_mempool(&data_dir) {
        Ok(txs) => Json(ApiResponse::ok(txs)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn clear_mempool(
    State(state): State<AppState>,
    Json(req): Json<ListAccountsRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::clear_mempool(&data_dir) {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn list_blocks(
    State(state): State<AppState>,
    Json(req): Json<BlockListRequest>,
) -> Json<ApiResponse<Vec<BlockInfo>>> {
    let data_dir = get_data_dir(&req.data_dir);
    let count = req.count.unwrap_or(10);
    match api::list_blocks(&data_dir, count) {
        Ok(blocks) => Json(ApiResponse::ok(blocks)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn block_info(
    State(state): State<AppState>,
    Json(req): Json<BlockInfoRequest>,
) -> Json<ApiResponse<BlockInfo>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::get_block_info(&data_dir, &req.block_id) {
        Ok(info) => Json(ApiResponse::ok(info)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn produce_block(
    State(state): State<AppState>,
    Json(req): Json<ProduceBlockRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    match api::produce_block(&data_dir, &req.authority) {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn deploy_contract(
    State(state): State<AppState>,
    Json(req): Json<DeployRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    let gas_price = req.gas_price.unwrap_or(1);
    match api::deploy_contract(&data_dir, &req.from, &req.source, gas_price, req.gas_limit) {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}

async fn call_contract(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Json<ApiResponse<String>> {
    let data_dir = get_data_dir(&req.data_dir);
    let gas_price = req.gas_price.unwrap_or(1);
    let amount = req.amount.unwrap_or(0);
    match api::call_contract(&data_dir, &req.from, &req.to, req.data.as_deref(), amount, gas_price) {
        Ok(hash) => Json(ApiResponse::ok(hash)),
        Err(e) => Json(ApiResponse::err(e.to_string())),
    }
}
