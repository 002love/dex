# Uranus DEX

A Peer-2-Peer DEX built on Solana that enables leveraged predictions on any SPL Token since mint.

The project consists of:

- **Rust Solana Program**: Core on-chain logic for position management and operations
- **JavaScript Client API**: Easy-to-use interface for interacting with the on-chain program (perfect for any kind of implementation; DEX, Games, Bots..)

## Features
- **Leveraged Positions**: Support for up to 5x leverage (for now)
- **Long/Short Positions**: Predict in both directions
- **Position Management**: Initialize, modify, and close positions
- **PnL Processing**: Automatic profit and loss calculations
- **Market Liquidity**: Decentralized liquidity per ticker ensuring bad actors cannot bankrupt the DEX
- **Fee Structure**: Base fees (2%) + leverage fees (0.1% per leverage level) + account creation fee

### Program Instructions
- `INITIALIZE`: Create new leveraged positions
- `DEX_MODIFY`: Update position parameters (DEX authority only)
- `USER_MODIFY`: User-initiated position modifications
- `PROCESS_PNL`: Calculate and distribute profits/losses
- `FORCE_CLOSE`: Emergency position closure
- `MARKET_TRANSFER`: Transfer liquidity between markets

## Important Addresses

```javascript
const PROGRAM_ID = "URAa3qGD1qVKKqyQrF8iBVZRTwa4Q8RkMd6Gx7u2KL1";
const DEX_PUBKEY = "URAbknhQPhFiY92S5iM9nhzoZC5Vkch7S5VERa4PmuV";
const DEX_FEES_PUBKEY = "URAfeAaGMoavvTe8vqPwMX6cUvTjq8WMG5c9nFo7Q8j";
```

## Getting Started with the Client API

### Prerequisites
- Node.js 16+

```bash
mkdir my-project
cd my-project
npm install uranus-dex-onchain-api
```

#### Create a Position

```javascript
const { createUranusPositionTransaction } = require('uranus-dex-onchain-api');

const { transaction, positionNonce, positionPda, fees } = await createUranusPositionTransaction(
  connection,
  ownerKeypair,   // User's keypair
  mintAddress,    // Token mint address
  1.0,            // SOL amount
  3,              // 3x leverage
  "long"          // Direction: "long" or "short"
);

await sendAndConfirmTransaction(connection, transaction, [ownerKeypair]);
```

#### Close a Position

```javascript
const { closeUranusPosition } = require('./onchain-api');

const transaction = await closeUranusPosition(
  connection,
  positionNonce,  // Position ID
  ownerKeypair,   // Owner's keypair
  positionPda     // Position account address (optional)
);

await sendAndConfirmTransaction(connection, transaction, [ownerKeypair]);
```

#### Check the docs
For more, visit https://uranus.ag/docs



## API Reference

### Core Functions

| Function | Description |
|----------|-------------|
| `createUranusPositionTransaction()` | Create a new leveraged position |
| `closeUranusPosition()` | Close an existing position |
| `getOpenPositions()` | Retrieve open positions with filtering |
| `getMarketLiquidity()` | Get liquidity for a specific market |
| `getMarketVolume()` | Calculate trading volume for time period |
| `getAllMarkets()` | List all available trading markets |
| `getTickerPrice()` | Fetch current price for a ticker |

### Position Structure

```rust
pub struct PositionAccount {
    pub owner: Pubkey,              // Position owner
    pub market_mint: Pubkey,        // Market token mint
    pub market_symbol: [u8; 32],    // Market symbol (e.g., "SOL")
    pub entry_price: u64,           // Entry price in lamports
    pub liquidation_price: u64,     // Liquidation threshold
    pub paid_amount: u64,           // Amount paid (after fees)
    pub position_size: u64,         // Total position size
    pub leverage: u8,               // Leverage multiplier (1-5x)
    pub closed: u8,                 // Position status
    pub position_nonce: u64,        // Unique position ID
    pub pnl: i64,                   // Currently unused
    pub direction: i8,              // 1 = Long, -1 = Short
}
```

## Fee Structure

- **Base Fee**: 2% of position value
- **Leverage Fee**: 0.1% per leverage level
- **Minimum Position**: 0.01 SOL
- **Maximum Leverage**: 5x

### Example Fee Calculation

For a 1 SOL position with 3x leverage:
- Base Fee: 1 SOL × 2% = 0.02 SOL
- Leverage Fee: 1 SOL × 0.023% = 0.0023 SOL
- **Total Fees**: 0.023 SOL

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Links
- **Repository**: [https://github.com/URANUSDEX/dex](https://github.com/URANUSDEX/dex)
- **Issues**: [https://github.com/URANUSDEX/dex/issues](https://github.com/URANUSDEX/dex/issues)
- **Website**: [https://uranus.ag](https://uranus.ag)

## Support
For support and questions:
- Open an issue on GitHub
- Contact: sakidev (solo dev founder)