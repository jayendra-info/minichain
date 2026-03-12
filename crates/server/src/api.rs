use anyhow::{Context, Result};
use minichain_assembler::assemble;
use minichain_chain::{Blockchain, BlockchainConfig};
use minichain_consensus::{BlockProposer, PoAConfig};
use minichain_core::{Address, Block, Hash, Keypair, Transaction};
use minichain_storage::{ChainStore, StateManager, Storage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeypairInfo {
    pub address: String,
    pub public_key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
    pub is_contract: bool,
    pub code_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub parent_hash: String,
    pub state_root: String,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub nonce: u64,
    pub data: Option<String>,
}

pub fn init_blockchain(data_dir: &Path, authorities: usize, block_time: u64) -> Result<String> {
    fs::create_dir_all(data_dir)?;

    let storage = Storage::open(data_dir)?;
    let chain = ChainStore::new(&storage);

    if chain.is_initialized()? {
        return Ok("Blockchain already initialized".to_string());
    }

    let mut authority_keypairs = Vec::new();
    for _ in 0..authorities {
        let keypair = Keypair::generate();
        authority_keypairs.push(keypair);
    }

    let authority_addresses: Vec<Address> =
        authority_keypairs.iter().map(|k| k.address()).collect();

    let config = BlockchainConfig {
        consensus: PoAConfig::new(authority_addresses.clone(), block_time),
        max_block_size: 1000,
    };

    let mut blockchain = Blockchain::new(&storage, config);

    for keypair in &authority_keypairs {
        blockchain.register_authority(keypair.address(), keypair.public_key.clone());
    }

    let genesis_authority = &authority_keypairs[0];
    let genesis = Block::genesis(genesis_authority.address()).signed(genesis_authority);

    blockchain.init_genesis(&genesis)?;

    let keys_dir = data_dir.join("keys");
    fs::create_dir_all(&keys_dir)?;

    for (i, keypair) in authority_keypairs.iter().enumerate() {
        let key_file = keys_dir.join(format!("authority_{}.json", i));
        let key_json = serde_json::json!({
            "address": keypair.address().to_hex(),
            "public_key": hex::encode(keypair.public_key.as_bytes()),
            "private_key": hex::encode(keypair.private_key()),
        });
        fs::write(&key_file, serde_json::to_string_pretty(&key_json)?)?;
    }

    let config_file = data_dir.join("config.json");
    let config_json = serde_json::json!({
        "authorities": authority_addresses.iter().map(|a| a.to_hex()).collect::<Vec<_>>(),
        "block_time": block_time,
        "max_block_size": 1000,
    });
    fs::write(&config_file, serde_json::to_string_pretty(&config_json)?)?;

    Ok(format!(
        "Blockchain initialized with {} authorities. Genesis block: {}",
        authorities,
        genesis.hash().to_hex()
    ))
}

pub fn create_account(data_dir: &Path, name: Option<&str>) -> Result<KeypairInfo> {
    let keypair = Keypair::generate();
    let address = keypair.address();

    let keys_dir = data_dir.join("keys");
    fs::create_dir_all(&keys_dir)?;

    let filename = if let Some(n) = name {
        format!("{}.json", n)
    } else {
        format!("account_{}.json", &address.to_hex()[2..10])
    };

    let key_file = keys_dir.join(&filename);
    let key_json = serde_json::json!({
        "address": address.to_hex(),
        "public_key": hex::encode(keypair.public_key.as_bytes()),
        "private_key": hex::encode(keypair.private_key()),
    });

    fs::write(&key_file, serde_json::to_string_pretty(&key_json)?)?;

    Ok(KeypairInfo {
        address: address.to_hex(),
        public_key: hex::encode(keypair.public_key.as_bytes()),
        name: filename.trim_end_matches(".json").to_string(),
    })
}

pub fn get_balance(data_dir: &Path, address: &str) -> Result<String> {
    let address = Address::from_hex(address).context("Invalid address")?;
    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);
    let balance = state.get_balance(&address)?;
    Ok(balance.to_string())
}

pub fn get_account_info(data_dir: &Path, address: &str) -> Result<AccountInfo> {
    let address = Address::from_hex(address).context("Invalid address")?;
    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);
    let account = state.get_account(&address)?;

    Ok(AccountInfo {
        address: address.to_hex(),
        balance: account.balance.to_string(),
        nonce: account.nonce,
        is_contract: account.is_contract(),
        code_hash: account.code_hash.map(|h| h.to_hex()),
    })
}

pub fn list_accounts(data_dir: &Path) -> Result<Vec<KeypairInfo>> {
    let keys_dir = data_dir.join("keys");
    if !keys_dir.exists() {
        return Ok(vec![]);
    }

    let mut accounts = Vec::new();
    for entry in fs::read_dir(&keys_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let contents = fs::read_to_string(&path)?;
            let json: serde_json::Value = serde_json::from_str(&contents)?;

            if let (Some(address), Some(public_key)) = (
                json.get("address").and_then(|v| v.as_str()),
                json.get("public_key").and_then(|v| v.as_str()),
            ) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                accounts.push(KeypairInfo {
                    address: address.to_string(),
                    public_key: public_key.to_string(),
                    name,
                });
            }
        }
    }

    Ok(accounts)
}

pub fn mint_tokens(
    data_dir: &Path,
    from_name: &str,
    to_address: &str,
    amount: u64,
) -> Result<String> {
    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, from_name)?;
    let authority_addr = keypair.address();

    let config = load_config(data_dir)?;
    if !config.consensus.authorities.contains(&authority_addr) {
        anyhow::bail!("Address is not an authority");
    }

    let to_addr = Address::from_hex(to_address).context("Invalid to address")?;

    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);

    let current_balance = state.get_balance(&to_addr)?;
    let new_balance = current_balance
        .checked_add(amount)
        .context("Overflow: balance too large")?;

    state.set_balance(&to_addr, new_balance)?;

    Ok(format!(
        "Minted {} tokens to {}. New balance: {}",
        amount, to_address, new_balance
    ))
}

pub fn send_transaction(
    data_dir: &Path,
    from_name: &str,
    to_address: &str,
    amount: u64,
    gas_price: u64,
) -> Result<String> {
    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, from_name)?;
    let from = keypair.address();

    let to = Address::from_hex(to_address).context("Invalid to address")?;

    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);

    let nonce = state.get_nonce(&from)?;
    let balance = state.get_balance(&from)?;

    let total_cost = amount + (21_000 * gas_price);
    if balance < total_cost {
        anyhow::bail!(
            "Insufficient balance: have {}, need {}",
            balance,
            total_cost
        );
    }

    let tx = Transaction::transfer(from, to, amount, nonce, gas_price).signed(&keypair);
    let tx_hash = tx.hash();

    let config = load_config(data_dir)?;
    let mut blockchain = Blockchain::new(&storage, config);
    register_authorities(&mut blockchain, data_dir)?;

    blockchain.submit_transaction(tx)?;

    Ok(tx_hash.to_hex())
}

pub fn list_mempool(data_dir: &Path) -> Result<Vec<TransactionInfo>> {
    let storage = Storage::open(data_dir)?;
    let db = storage.inner();
    let prefix = b"mempool:tx:";

    let mut transactions = Vec::new();
    for (_key, value) in db.scan_prefix(prefix).flatten() {
        if let Ok(tx) = bincode::deserialize::<Transaction>(&value) {
            transactions.push(TransactionInfo {
                hash: tx.hash().to_hex(),
                from: tx.from.to_hex(),
                to: tx.to.map(|a| a.to_hex()),
                value: tx.value.to_string(),
                nonce: tx.nonce,
                data: if tx.data.is_empty() {
                    None
                } else {
                    Some(hex::encode(&tx.data))
                },
            });
        }
    }

    Ok(transactions)
}

pub fn clear_mempool(data_dir: &Path) -> Result<String> {
    let storage = Storage::open(data_dir)?;
    let db = storage.inner();
    let prefix = b"mempool:tx:";

    let mut count = 0;
    for (key, _) in db.scan_prefix(prefix).flatten() {
        let _ = db.remove(key);
        count += 1;
    }

    Ok(format!("Cleared {} pending transactions", count))
}

pub fn list_blocks(data_dir: &Path, count: usize) -> Result<Vec<BlockInfo>> {
    let storage = Storage::open(data_dir)?;
    let chain = ChainStore::new(&storage);
    let head_height = chain.get_height()?;

    let start_height = if head_height >= count as u64 {
        head_height - count as u64 + 1
    } else {
        0
    };

    let mut blocks = Vec::new();
    for height in start_height..=head_height {
        let block = chain
            .get_block_by_height(height)?
            .context("Block not found")?;
        blocks.push(BlockInfo {
            height,
            hash: block.hash().to_hex(),
            parent_hash: block.header.prev_hash.to_hex(),
            state_root: block.header.state_root.to_hex(),
            timestamp: block.header.timestamp,
            transactions: block
                .transactions
                .iter()
                .map(|t| t.hash().to_hex())
                .collect(),
        });
    }

    Ok(blocks)
}

pub fn get_block_info(data_dir: &Path, block_id: &str) -> Result<BlockInfo> {
    let storage = Storage::open(data_dir)?;
    let chain = ChainStore::new(&storage);

    let block = if let Ok(height) = block_id.parse::<u64>() {
        chain
            .get_block_by_height(height)?
            .context("Block not found")?
    } else {
        let hash = Hash::from_hex(block_id).context("Invalid block hash")?;
        chain.get_block_by_hash(&hash)?.context("Block not found")?
    };

    Ok(BlockInfo {
        height: block.header.height,
        hash: block.hash().to_hex(),
        parent_hash: block.header.prev_hash.to_hex(),
        state_root: block.header.state_root.to_hex(),
        timestamp: block.header.timestamp,
        transactions: block
            .transactions
            .iter()
            .map(|t| t.hash().to_hex())
            .collect(),
    })
}

pub fn produce_block(data_dir: &Path, authority_name: &str) -> Result<String> {
    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, authority_name)?;
    let authority_addr = keypair.address();

    let storage = Storage::open(data_dir)?;
    let config = load_config(data_dir)?;

    if !config.consensus.authorities.contains(&authority_addr) {
        anyhow::bail!("Address {} is not an authority", authority_addr.to_hex());
    }

    let consensus_config = config.consensus.clone();
    let mut blockchain = Blockchain::new(&storage, config);
    register_authorities(&mut blockchain, data_dir)?;

    let proposer = BlockProposer::new(keypair, consensus_config);

    let block = blockchain.propose_block(&proposer)?;
    blockchain.import_block(block.clone())?;

    Ok(format!(
        "Block produced: height={}, hash={}, txs={}",
        block.header.height,
        block.hash().to_hex(),
        block.transactions.len()
    ))
}

pub fn deploy_contract(
    data_dir: &Path,
    from_name: &str,
    source_path: &str,
    gas_price: u64,
    gas_limit: u64,
) -> Result<String> {
    let source_code = fs::read_to_string(source_path).context("Failed to read source file")?;
    let bytecode = assemble(&source_code).context("Failed to compile assembly")?;

    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, from_name)?;
    let from = keypair.address();

    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);
    let nonce = state.get_nonce(&from)?;

    let gas_required = 21_000 + (bytecode.len() as u64 * 200);
    if gas_required > gas_limit {
        anyhow::bail!(
            "Gas limit too low: required {}, got {}",
            gas_required,
            gas_limit
        );
    }

    let tx = Transaction::deploy(from, bytecode, nonce, gas_required, gas_price).signed(&keypair);
    let tx_hash = tx.hash();

    let contract_address = tx
        .contract_address()
        .expect("deploy tx must have contract address");

    let config = load_config(data_dir)?;
    let mut blockchain = Blockchain::new(&storage, config);
    register_authorities(&mut blockchain, data_dir)?;

    blockchain.submit_transaction(tx)?;

    Ok(format!(
        "Contract deployed at address: {}. Tx hash: {}",
        contract_address.to_hex(),
        tx_hash.to_hex()
    ))
}

pub fn call_contract(
    data_dir: &Path,
    from_name: &str,
    to_address: &str,
    data: Option<&str>,
    amount: u64,
    gas_price: u64,
) -> Result<String> {
    let keys_dir = data_dir.join("keys");
    let keypair = load_keypair(&keys_dir, from_name)?;
    let from = keypair.address();

    let to = Address::from_hex(to_address).context("Invalid contract address")?;

    let calldata = if let Some(d) = data {
        hex::decode(d).context("Invalid calldata hex")?
    } else {
        vec![]
    };

    let storage = Storage::open(data_dir)?;
    let state = StateManager::new(&storage);

    let nonce = state.get_nonce(&from)?;
    let target_account = state.get_account(&to)?;
    if !target_account.is_contract() {
        anyhow::bail!("Address {} is not a contract", to.to_hex());
    }

    let gas_limit = 21_000 + (calldata.len() as u64 * 68) + 2_100;
    let total_cost = amount + (gas_limit * gas_price);
    let balance = state.get_balance(&from)?;

    if balance < total_cost {
        anyhow::bail!(
            "Insufficient balance: have {}, need {}",
            balance,
            total_cost
        );
    }

    let tx =
        Transaction::call(from, to, calldata, amount, nonce, gas_limit, gas_price).signed(&keypair);
    let tx_hash = tx.hash();

    let config = load_config(data_dir)?;
    let mut blockchain = Blockchain::new(&storage, config);
    register_authorities(&mut blockchain, data_dir)?;

    blockchain.submit_transaction(tx)?;

    Ok(tx_hash.to_hex())
}

fn load_keypair(keys_dir: &Path, name: &str) -> Result<Keypair> {
    let key_file = keys_dir.join(format!("{}.json", name));
    let contents = fs::read_to_string(&key_file)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;

    let private_key_hex = json
        .get("private_key")
        .and_then(|v| v.as_str())
        .context("Missing private_key")?;

    let private_key_bytes = hex::decode(private_key_hex).context("Invalid private key hex")?;

    if private_key_bytes.len() != 32 {
        anyhow::bail!("Invalid private key length");
    }

    let mut private_key = [0u8; 32];
    private_key.copy_from_slice(&private_key_bytes);

    Keypair::from_private_key(&private_key).context("Failed to create keypair")
}

fn load_config(data_dir: &Path) -> Result<BlockchainConfig> {
    let config_file = data_dir.join("config.json");
    let contents = fs::read_to_string(config_file).context("Failed to read config")?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;

    let authorities: Vec<Address> = json
        .get("authorities")
        .and_then(|v| v.as_array())
        .context("Missing authorities")?
        .iter()
        .map(|v| {
            v.as_str()
                .and_then(|s| Address::from_hex(s).ok())
                .context("Invalid authority address")
        })
        .collect::<Result<Vec<_>>>()?;

    let block_time = json.get("block_time").and_then(|v| v.as_u64()).unwrap_or(5);
    let max_block_size = json
        .get("max_block_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000) as usize;

    Ok(BlockchainConfig {
        consensus: PoAConfig::new(authorities, block_time),
        max_block_size,
    })
}

fn register_authorities(blockchain: &mut Blockchain, data_dir: &Path) -> Result<()> {
    let keys_dir = data_dir.join("keys");

    for entry in fs::read_dir(&keys_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("authority_"))
        {
            let contents = fs::read_to_string(&path)?;
            let json: serde_json::Value = serde_json::from_str(&contents)?;

            let address_hex = json
                .get("address")
                .and_then(|v| v.as_str())
                .context("Missing address")?;
            let pubkey_hex = json
                .get("public_key")
                .and_then(|v| v.as_str())
                .context("Missing public_key")?;

            let address = Address::from_hex(address_hex)?;
            let pubkey_bytes = hex::decode(pubkey_hex)?;

            if pubkey_bytes.len() != 32 {
                continue;
            }

            let mut pubkey_arr = [0u8; 32];
            pubkey_arr.copy_from_slice(&pubkey_bytes);

            let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey_arr)
                .context("Invalid public key")?;
            let public_key = minichain_core::PublicKey(verifying_key);

            blockchain.register_authority(address, public_key);
        }
    }

    Ok(())
}
