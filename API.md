# 📡 API Documentation

## 🚀 Run Server

```bash
cargo run -p minichain-server -- --data-dir ./data --port 3000
```

## 🔗 Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/api/status` | Get chain status |
| POST | `/api/init` | Initialize blockchain |
| POST | `/api/account/new` | Create new account |
| POST | `/api/account/balance` | Get account balance |
| POST | `/api/account/info` | Get account info |
| POST | `/api/account/list` | List all accounts |
| POST | `/api/account/mint` | Mint tokens |
| POST | `/api/tx/send` | Send transaction |
| POST | `/api/tx/list` | List mempool (pending txs) |
| POST | `/api/tx/clear` | Clear mempool |
| POST | `/api/tx/get` | Get transaction by hash (confirmed) |
| POST | `/api/tx/transactions` | List confirmed transactions |
| POST | `/api/block/list` | List blocks |
| POST | `/api/block/info` | Get block info |
| POST | `/api/block/produce` | Produce block |
| POST | `/api/contract/deploy` | Deploy contract |
| POST | `/api/contract/call` | Call contract |

## 🧾 Request Bodies

### `/api/init`
```json
{
  "data_dir": "optional",
  "authorities": 1,
  "block_time": 5
}
```

### `/api/account/new`
```json
{
  "data_dir": "optional",
  "name": "alice"
}
```

### `/api/account/balance`
```json
{
  "data_dir": "optional",
  "address": "0x..."
}
```

### `/api/account/info`
```json
{
  "data_dir": "optional",
  "address": "0x..."
}
```

### `/api/account/list`
```json
{
  "data_dir": "optional"
}
```

### `/api/account/mint`
```json
{
  "data_dir": "optional",
  "from": "authority_0",
  "to": "0x...",
  "amount": 1000
}
```

### `/api/tx/send`
```json
{
  "data_dir": "optional",
  "from": "alice",
  "to": "0x...",
  "amount": 100,
  "gas_price": 1
}
```

### `/api/tx/list`
```json
{
  "data_dir": "optional"
}
```

### `/api/tx/clear`
```json
{
  "data_dir": "optional"
}
```

### `/api/tx/get`
```json
{
  "data_dir": "optional",
  "tx_hash": "0x..."
}
```

### `/api/tx/transactions`
```json
{
  "data_dir": "optional",
  "count": 10
}
```

### `/api/block/list`
```json
{
  "data_dir": "optional",
  "count": 10
}
```

### `/api/block/info`
```json
{
  "data_dir": "optional",
  "block_id": "0"
}
```

### `/api/block/produce`
```json
{
  "data_dir": "optional",
  "authority": "authority_0"
}
```

### `/api/contract/deploy`
```json
{
  "data_dir": "optional",
  "from": "alice",
  "source": "./contracts/counter.asm",
  "gas_price": 1,
  "gas_limit": 30000
}
```

### `/api/contract/call`
```json
{
  "data_dir": "optional",
  "from": "alice",
  "to": "0x...",
  "data": "00",
  "amount": 0,
  "gas_price": 1
}
```

## ⚙️ Notes

- All requests accept an optional `data_dir` field  
- `data_dir` overrides the default data directory used by the node
- `/api/tx/list` returns pending transactions in mempool
- `/api/tx/transactions` returns confirmed transactions in blocks