use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program::{invoke, invoke_signed},
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

solana_program::declare_id!("URAa3qGD1qVKKqyQrF8iBVZRTwa4Q8RkMd6Gx7u2KL1");

pub const DEX_PUBKEY: Pubkey = solana_program::pubkey!("URAbknhQPhFiY92S5iM9nhzoZC5Vkch7S5VERa4PmuV");

pub const INSTRUCTION_INITIALIZE: u8 = 0;
pub const INSTRUCTION_DEX_MODIFY: u8 = 1;
pub const INSTRUCTION_USER_MODIFY: u8 = 2;
pub const INSTRUCTION_PROCESS_PNL: u8 = 3;

pub const MIN_POSITION_SIZE_LAMPORTS: u64 = 10_000_000;

pub const BASE_FEE_BASIS_POINTS: u64 = 50;
pub const LEVERAGE_FEE_BASIS_POINTS: u64 = 5;

pub const MAXIMUM_LEVERAGE: u8 = 2;

pub const POSITION_LONG: i8 = 1;
pub const POSITION_SHORT: i8 = -1;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PositionAccount {
    pub owner: Pubkey,
    pub market_id: u64,
    pub market_symbol: String,
    pub entry_price: u64,
    pub liquidation_price: u64,
    pub paid_amount: u64,
    pub position_size: u64,
    pub leverage: u8,
    pub closed: u8,
    pub position_nonce: u64,
    pub pnl: i64,
    pub direction: i8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct InitializePositionData {
    pub market_id: u64,
    pub market_symbol: String,
    pub paid_amount: u64,
    pub position_size: u64,
    pub leverage: u8,
    pub position_nonce: u64,
    pub direction: i8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct DexModifyData {
    pub new_entry_price: u64,
    pub new_liquidation_price: u64,
    pub position_nonce: u64,
    pub new_close_state: u8,
    pub new_pnl: i64,
    pub new_market_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct UserModifyData {
    pub close_position: bool,
    pub position_nonce: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProcessPnlData {
    pub position_nonce: u64,
    pub final_pnl: i64,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction_type = instruction_data[0];

    match instruction_type {
        INSTRUCTION_INITIALIZE => {
            let initialize_data = InitializePositionData::try_from_slice(&instruction_data[1..])?;
            process_initialize(program_id, accounts, initialize_data)
        },
        INSTRUCTION_DEX_MODIFY => {
            let dex_data = DexModifyData::try_from_slice(&instruction_data[1..])?;
            process_dex_modify(program_id, accounts, dex_data)
        },
        INSTRUCTION_USER_MODIFY => {
            let user_data = UserModifyData::try_from_slice(&instruction_data[1..])?;
            process_user_modify(program_id, accounts, user_data)
        },
        INSTRUCTION_PROCESS_PNL => {
            let pnl_data = ProcessPnlData::try_from_slice(&instruction_data[1..])?;
            process_pnl(program_id, accounts, pnl_data)
        },
        _ => {
            msg!("Invalid instruction type: {}", instruction_type);
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

fn find_position_address(
    owner: &Pubkey,
    market_id: u64,
    position_nonce: u64,
    program_id: &Pubkey
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"uranus_position",
            owner.as_ref(),
            &position_nonce.to_le_bytes(),
        ],
        program_id,
    )
}

fn find_program_vault_address(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"uranus_program_vault",
        ],
        program_id,
    )
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initialize_data: InitializePositionData,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let payer_account = next_account_info(accounts_iter)?;
    let owner_account = next_account_info(accounts_iter)?;
    let position_account = next_account_info(accounts_iter)?;
    let program_vault_account = next_account_info(accounts_iter)?;
    let dex_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    
    if !payer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if initialize_data.position_size < MIN_POSITION_SIZE_LAMPORTS {
        msg!("Position size too small. Minimum required: {} lamports (0.01 SOL)", MIN_POSITION_SIZE_LAMPORTS);
        return Err(ProgramError::InvalidArgument);
    }
    
    let mut leverage = initialize_data.leverage;
    
    if leverage < 1 {
        leverage = 1;
        msg!("Leverage must be at least 1x. Setting to 1x.");
    } else if leverage > MAXIMUM_LEVERAGE {
        leverage = MAXIMUM_LEVERAGE;
        msg!("Requested leverage ({}x) exceeds maximum allowed ({}x). Capping at {}x.", 
            initialize_data.leverage, MAXIMUM_LEVERAGE, MAXIMUM_LEVERAGE);
    }
    
    if initialize_data.direction != POSITION_LONG && initialize_data.direction != POSITION_SHORT {
        msg!("Invalid position direction. Must be 1 (long) or -1 (short)");
        return Err(ProgramError::InvalidArgument);
    }
    
    let (program_vault_pda, _) = find_program_vault_address(program_id);
    if program_vault_account.key != &program_vault_pda {
        msg!("Invalid program vault account");
        return Err(ProgramError::InvalidArgument);
    }
    
    if dex_account.key != &DEX_PUBKEY {
        msg!("Invalid DEX account");
        return Err(ProgramError::InvalidArgument);
    }
    
    let (position_pda, bump_seed) = find_position_address(
        owner_account.key,
        initialize_data.market_id,
        initialize_data.position_nonce,
        program_id
    );
    
    if position_pda != *position_account.key {
        msg!("Position account address does not match the PDA");
        return Err(ProgramError::InvalidArgument);
    }
    
    let position = PositionAccount {
        owner: *owner_account.key,
        market_id: initialize_data.market_id,
        market_symbol: initialize_data.market_symbol.clone(),
        entry_price: 0,
        liquidation_price: 0,
        paid_amount: initialize_data.paid_amount,
        position_size: initialize_data.position_size,
        leverage: leverage,
        closed: 0,
        position_nonce: initialize_data.position_nonce,
        pnl: 0,
        direction: initialize_data.direction,
    };
    
    let data_len = position.try_to_vec()?.len();
    
    let rent = Rent::get()?;
    let account_rent = rent.minimum_balance(data_len);
    
    let base_fee = initialize_data.paid_amount.checked_mul(BASE_FEE_BASIS_POINTS)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(ProgramError::ArithmeticOverflow)?;
        
    let leverage_fee = initialize_data.paid_amount.checked_mul(LEVERAGE_FEE_BASIS_POINTS)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_mul(leverage as u64)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let fee_amount = base_fee.checked_add(leverage_fee)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let seeds = &[
        b"uranus_position",
        owner_account.key.as_ref(),
        &initialize_data.position_nonce.to_le_bytes(),
        &[bump_seed],
    ];
    
    invoke(
        &system_instruction::transfer(
            payer_account.key,
            dex_account.key,
            fee_amount,
        ),
        &[
            payer_account.clone(),
            dex_account.clone(),
            system_program.clone(),
        ],
    )?;

    let position_amount = initialize_data.paid_amount.checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    invoke_signed(
        &system_instruction::create_account(
            payer_account.key,
            position_account.key,
            position_amount,
            data_len as u64,
            program_id,
        ),
        &[
            payer_account.clone(),
            position_account.clone(),
            system_program.clone(),
        ],
        &[seeds],
    )?;

    position.serialize(&mut *position_account.data.borrow_mut())?;

    let fee_percentage = (BASE_FEE_BASIS_POINTS as f64 / 10000.0) + 
                         (LEVERAGE_FEE_BASIS_POINTS as f64 / 10000.0 * leverage as f64);
    let fee_percentage_str = format!("{:.3}%", fee_percentage * 100.0);

    msg!("Uranus position initialized successfully with position nonce: {}", initialize_data.position_nonce);
    msg!("Fee collected: {} lamports sent to DEX wallet ({})", fee_amount, fee_percentage_str);
    msg!("Base fee: 0.5% (50 basis points) + 0.05% per leverage unit");
    msg!("Position amount locked: {} lamports", position_amount);
    msg!("Position leverage set to: {}x (maximum allowed is {}x)", leverage, MAXIMUM_LEVERAGE);
    msg!("Position direction: {}", if initialize_data.direction == POSITION_LONG { "LONG" } else { "SHORT" });
    Ok(())
}

fn process_dex_modify(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    dex_data: DexModifyData,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let position_account = next_account_info(accounts_iter)?;
    let dex_account = next_account_info(accounts_iter)?;
    
    if !dex_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if dex_account.key != &DEX_PUBKEY {
        msg!("Only DEX wallet can perform this action");
        return Err(ProgramError::InvalidAccountData);
    }
    
    if position_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let mut position = PositionAccount::try_from_slice(&position_account.data.borrow())?;
    
    if position.position_nonce != dex_data.position_nonce {
        msg!("Position nonce mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    
    position.entry_price = dex_data.new_entry_price;
    position.liquidation_price = dex_data.new_liquidation_price;
    position.closed = dex_data.new_close_state;
    position.pnl = dex_data.new_pnl;
    position.market_id = dex_data.new_market_id;
    
    msg!("Position nonce: {} - Entry price updated to {}", position.position_nonce, dex_data.new_entry_price);
    msg!("Position nonce: {} - Liquidation price updated to {}", position.position_nonce, dex_data.new_liquidation_price);
    msg!("Position nonce: {} - Market ID updated to {}", position.position_nonce, dex_data.new_market_id);
    
    if dex_data.new_close_state == 0 {
        msg!("Position nonce: {} - Position marked as open", position.position_nonce);
    } else {
        msg!("Position nonce: {} - Position marked as closed", position.position_nonce);
    }
    
    msg!("Position nonce: {} - PnL updated to {}", position.position_nonce, dex_data.new_pnl);
    msg!("Position leverage: {}x", position.leverage);
    msg!("Position direction: {}", if position.direction == POSITION_LONG { "LONG" } else { "SHORT" });
    
    position.serialize(&mut *position_account.data.borrow_mut())?;
    
    Ok(())
}

fn process_user_modify(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user_data: UserModifyData,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let position_account = next_account_info(accounts_iter)?;
    let user_account = next_account_info(accounts_iter)?;
    
    if !user_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if position_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let mut position = PositionAccount::try_from_slice(&position_account.data.borrow())?;
    
    if position.position_nonce != user_data.position_nonce {
        msg!("Position nonce mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    
    if position.owner != *user_account.key {
        if user_account.key != &DEX_PUBKEY {
            msg!("Only position owner or DEX wallet can perform this action");
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    if position.closed != 0 {
        msg!("Position is already closed");
        return Err(ProgramError::InvalidAccountData);
    }
    
    if user_data.close_position {
        position.closed = 1;
        msg!("Position nonce: {} closed successfully", position.position_nonce);
    }
    
    position.serialize(&mut *position_account.data.borrow_mut())?;
    
    Ok(())
}

fn process_pnl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    pnl_data: ProcessPnlData,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let position_account = next_account_info(accounts_iter)?;
    let dex_account = next_account_info(accounts_iter)?;
    let owner_account = next_account_info(accounts_iter)?;
    let program_vault_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    
    if !dex_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if dex_account.key != &DEX_PUBKEY {
        msg!("Only DEX wallet can perform this action");
        return Err(ProgramError::InvalidAccountData);
    }
    
    if position_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let (program_vault_pda, vault_bump) = find_program_vault_address(program_id);
    if program_vault_account.key != &program_vault_pda {
        msg!("Invalid program vault account");
        return Err(ProgramError::InvalidArgument);
    }
    
    let position = PositionAccount::try_from_slice(&position_account.data.borrow())?;
    
    if position.position_nonce != pnl_data.position_nonce {
        msg!("Position nonce mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    
    if position.closed != 1 {
        msg!("Cannot process PnL for an open position");
        return Err(ProgramError::InvalidAccountData);
    }
    
    if &position.owner != owner_account.key {
        msg!("Owner account mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    
    let position_lamports = position_account.lamports();
    
    if pnl_data.final_pnl > 0 {
        let vault_signer_seeds: &[&[u8]] = &[
            b"uranus_program_vault" ,
            &[vault_bump],
        ];
        
        invoke_signed(
            &system_instruction::transfer(
                program_vault_account.key,
                owner_account.key,
                pnl_data.final_pnl as u64,
            ),
            &[
                program_vault_account.clone(),
                owner_account.clone(),
                system_program.clone(),
            ],
            &[vault_signer_seeds],
        )?;
        
        **owner_account.lamports.borrow_mut() = owner_account
            .lamports()
            .checked_add(position_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        **position_account.lamports.borrow_mut() = 0;
        
        msg!("Positive PnL: {} lamports paid to owner", pnl_data.final_pnl);
        msg!("Locked funds: {} lamports returned to owner", position_lamports);
    } else if pnl_data.final_pnl < 0 {
        let pnl_abs = (-pnl_data.final_pnl) as u64;
        
        if position_lamports <= pnl_abs {
            **program_vault_account.lamports.borrow_mut() = program_vault_account
                .lamports()
                .checked_add(position_lamports)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            **position_account.lamports.borrow_mut() = 0;
            
            msg!("Negative PnL exceeds locked funds: all {} lamports transferred to vault", 
                position_lamports);
        } else {
            let remaining_funds = position_lamports.checked_sub(pnl_abs)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            **program_vault_account.lamports.borrow_mut() = program_vault_account
                .lamports()
                .checked_add(pnl_abs)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            **owner_account.lamports.borrow_mut() = owner_account
                .lamports()
                .checked_add(remaining_funds)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            **position_account.lamports.borrow_mut() = 0;
            
            msg!("Negative PnL: {} lamports subtracted from locked funds", pnl_abs);
            msg!("Remaining: {} lamports returned to owner", remaining_funds);
        }
    } else {
        **owner_account.lamports.borrow_mut() = owner_account
            .lamports()
            .checked_add(position_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        **position_account.lamports.borrow_mut() = 0;
        
        msg!("Zero PnL: All locked funds ({} lamports) returned to owner", position_lamports);
    }
    
    {
        let mut data = position_account.try_borrow_mut_data()?;
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }
    
    msg!("Position nonce: {} closed with PnL: {}", position.position_nonce, pnl_data.final_pnl);
    msg!("Position had leverage: {}x", position.leverage);
    msg!("Position direction was: {}", if position.direction == POSITION_LONG { "LONG" } else { "SHORT" });
    msg!("Position account closed");
    
    Ok(())
}

pub fn create_program_vault_if_needed(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    let payer_account = next_account_info(accounts_iter)?;
    let program_vault_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    
    if !payer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if program_vault_account.data_is_empty() {
        let (vault_pda, vault_bump) = find_program_vault_address(program_id);
        
        if vault_pda != *program_vault_account.key {
            msg!("Vault account address does not match the PDA");
            return Err(ProgramError::InvalidArgument);
        }
        
        let rent = Rent::get()?;
        let minimum_balance = rent.minimum_balance(0);
        
        let vault_seeds = &[
            b"uranus_program_vault" as &[u8],
            &[vault_bump],
        ];
        
        invoke_signed(
            &system_instruction::create_account(
                payer_account.key,
                program_vault_account.key,
                minimum_balance,
                0,
                program_id,
            ),
            &[
                payer_account.clone(),
                program_vault_account.clone(),
                system_program.clone(),
            ],
            &[vault_seeds],
        )?;
        
        msg!("Program vault created successfully");
    }
    
    Ok(())
}
