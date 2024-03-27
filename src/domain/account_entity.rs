use crate::domain::account::Account;
use eventsourced::{Cmd, EventSourced};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountEntity {
    #[default]
    Nonexistent,

    Existing {
        balance: u64,
    },
}

impl EventSourced for AccountEntity {
    type Id = Uuid;
    type Evt = AccountEvt;

    const TYPE_NAME: &'static str = "account";

    fn handle_evt(self, evt: &Self::Evt) -> Self {
        match self {
            AccountEntity::Nonexistent => match evt {
                AccountEvt::Created { .. } => AccountEntity::Existing { balance: 0 },
                AccountEvt::Deposited { .. } => panic!("invalid event {evt:?} in state Deleted"),
                AccountEvt::Withdrawn { .. } => panic!("invalid event {evt:?} in state Deleted"),
            },

            AccountEntity::Existing { .. } => match evt {
                AccountEvt::Created { .. } => panic!("invalid event {evt:?} in state Deleted"),

                AccountEvt::Deposited { balance, .. } => {
                    AccountEntity::Existing { balance: *balance }
                }

                AccountEvt::Withdrawn { balance, .. } => {
                    AccountEntity::Existing { balance: *balance }
                }
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AccountEvt {
    Created { id: Uuid },
    Deposited { id: Uuid, amount: u64, balance: u64 },
    Withdrawn { id: Uuid, amount: u64, balance: u64 },
}

// Command: CreateAccount ==========================================================================

#[derive(Debug)]
pub struct CreateAccount;

impl Cmd<AccountEntity> for CreateAccount {
    type Reply = Account;
    type Error = CreateAccountError;

    fn handle_cmd(&self, id: &Uuid, state: &AccountEntity) -> Result<AccountEvt, Self::Error> {
        match state {
            AccountEntity::Nonexistent => Ok(AccountEvt::Created { id: *id }),
            AccountEntity::Existing { .. } => Err(CreateAccountError::AlreadyExisting(*id)),
        }
    }

    fn make_reply(&self, id: &Uuid, _state: &AccountEntity, _evt: AccountEvt) -> Self::Reply {
        Account {
            id: *id,
            balance: 0,
        }
    }
}

#[derive(Debug, Error)]
pub enum CreateAccountError {
    #[error("account with ID {0} already exists")]
    AlreadyExisting(Uuid),
}

// Command: Deposit ================================================================================

#[derive(Debug)]
pub struct Deposit {
    amount: u64,
}

impl From<u64> for Deposit {
    fn from(amount: u64) -> Self {
        Self { amount }
    }
}

impl Cmd<AccountEntity> for Deposit {
    type Reply = Account;
    type Error = DepositError;

    fn handle_cmd(&self, id: &Uuid, state: &AccountEntity) -> Result<AccountEvt, Self::Error> {
        match state {
            AccountEntity::Nonexistent => Err(DepositError::NotFound(*id)),

            AccountEntity::Existing { balance } => Ok(AccountEvt::Deposited {
                id: *id,
                amount: self.amount,
                balance: balance + self.amount,
            }),
        }
    }

    fn make_reply(&self, id: &Uuid, state: &AccountEntity, _evt: AccountEvt) -> Self::Reply {
        match state {
            AccountEntity::Nonexistent => panic!("invalid command Deposit in state Nonexistent"),

            AccountEntity::Existing { balance } => Account {
                id: *id,
                balance: *balance,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum DepositError {
    #[error("account with ID {0} not found")]
    NotFound(Uuid),
}

// Command: Withdraw ===============================================================================

#[derive(Debug)]
pub struct Withdraw {
    amount: u64,
}

impl From<u64> for Withdraw {
    fn from(amount: u64) -> Self {
        Self { amount }
    }
}

impl Cmd<AccountEntity> for Withdraw {
    type Reply = Account;
    type Error = WithdrawError;

    fn handle_cmd(&self, id: &Uuid, state: &AccountEntity) -> Result<AccountEvt, Self::Error> {
        match state {
            AccountEntity::Nonexistent => Err(WithdrawError::NotFound(*id)),

            AccountEntity::Existing { balance } if self.amount > *balance => {
                Err(WithdrawError::InsufficientBalance(*id))
            }

            AccountEntity::Existing { balance } => Ok(AccountEvt::Withdrawn {
                id: *id,
                amount: self.amount,
                balance: balance - self.amount,
            }),
        }
    }

    fn make_reply(&self, id: &Uuid, state: &AccountEntity, _evt: AccountEvt) -> Self::Reply {
        match state {
            AccountEntity::Nonexistent => panic!("invalid command Withdraw in state Nonexistent"),

            AccountEntity::Existing { balance } => Account {
                id: *id,
                balance: *balance,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum WithdrawError {
    #[error("account with ID {0} not found")]
    NotFound(Uuid),

    #[error("account with ID {0} has insufficient balance for withdrawal")]
    InsufficientBalance(Uuid),
}
