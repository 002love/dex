/* Define Schemas for Borsh Serialization */
class PositionAccountData {
  constructor(props) {
    this.owner = props.owner;
    this.market_mint = props.market_mint;
    this.market_symbol = props.market_symbol;
    this.entry_price = props.entry_price;
    this.liquidation_price = props.liquidation_price;
    this.paid_amount = props.paid_amount;
    this.position_size = props.position_size;
    this.leverage = props.leverage;
    this.closed = props.closed;
    this.position_nonce = props.position_nonce;
    this.pnl = props.pnl;
    this.direction = props.direction;
  }

  static schema = {
    struct: {
      owner: { array: { type: "u8", len: 32 } },
      market_mint: { array: { type: "u8", len: 32 } },
      market_symbol: { array: { type: "u8", len: 32 } },
      entry_price: "u64",
      liquidation_price: "u64",
      paid_amount: "u64",
      position_size: "u64",
      leverage: "u8",
      closed: "u8",
      position_nonce: "u64",
      pnl: "i64",
      direction: "i8",
    },
  };

  static size = 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1 + 8 + 8 + 1;
}

class InitializePositionData {
    constructor(props) {
        this.market_mint = props.market_mint;
        this.market_symbol = props.market_symbol;
        this.paid_amount = props.paid_amount;
        this.position_size = props.position_size;
        this.leverage = props.leverage;
        this.position_nonce = props.position_nonce;
        this.direction = props.direction;
    }

    static schema = {
        struct: {
            market_mint: { array: { type: 'u8', len: 32 } },
            market_symbol: { array: { type: 'u8', len: 32 } },
            paid_amount: 'u64',
            position_size: 'u64',
            leverage: 'u8',
            position_nonce: 'u64',
            direction: 'i8',
        }
    };
}

class ClosePositionData {
    constructor(props) {
        this.close_position = props.close_position;
        this.position_nonce = props.position_nonce;
    }

    static schema = {
        struct: {
            close_position: 'bool',
            position_nonce: 'u64',
        }
    };
}

module.exports = {
    PositionAccountData,
    InitializePositionData,
    ClosePositionData
};