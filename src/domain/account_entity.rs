use crate::domain::account::Account;
use eventsourced::{Command, CommandEffect, EventSourced};
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
    type Event = AccountEvent;

    const TYPE_NAME: &'static str = "account";

    fn handle_event(self, event: Self::Event) -> Self {
        match self {
            AccountEntity::Nonexistent => match event {
                AccountEvent::Created { .. } => AccountEntity::Existing { balance: 0 },
                AccountEvent::Deposited { .. } => {
                    panic!("invalid event {event:?} in state Deleted")
                }
                AccountEvent::Withdrawn { .. } => {
                    panic!("invalid event {event:?} in state Deleted")
                }
            },

            AccountEntity::Existing { .. } => match event {
                AccountEvent::Created { .. } => panic!("invalid event {event:?} in state Deleted"),

                AccountEvent::Deposited { balance, .. } => AccountEntity::Existing { balance },

                AccountEvent::Withdrawn { balance, .. } => AccountEntity::Existing { balance },
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AccountEvent {
    Created { id: Uuid },
    Deposited { id: Uuid, amount: u64, balance: u64 },
    Withdrawn { id: Uuid, amount: u64, balance: u64 },
}

// Command: CreateAccount ==========================================================================

#[derive(Debug)]
pub struct CreateAccount;

impl Command<AccountEntity> for CreateAccount {
    type Reply = Account;
    type Error = CreateAccountError;

    fn handle_command(
        self,
        id: &Uuid,
        state: &AccountEntity,
    ) -> CommandEffect<AccountEntity, Self::Reply, Self::Error> {
        let id = *id;

        match state {
            AccountEntity::Nonexistent => {
                let event = AccountEvent::Created { id };
                CommandEffect::emit_and_reply(event, move |_| Account { id, balance: 0 })
            }

            AccountEntity::Existing { .. } => {
                CommandEffect::reject(CreateAccountError::AlreadyExisting(id))
            }
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

impl Command<AccountEntity> for Deposit {
    type Reply = Account;
    type Error = DepositError;

    fn handle_command(
        self,
        id: &Uuid,
        state: &AccountEntity,
    ) -> CommandEffect<AccountEntity, Self::Reply, Self::Error> {
        let id = *id;

        match state {
            AccountEntity::Nonexistent => CommandEffect::reject(DepositError::NotFound(id)),

            AccountEntity::Existing { balance } => {
                let event = AccountEvent::Deposited {
                    id,
                    amount: self.amount,
                    balance: balance + self.amount,
                };

                CommandEffect::emit_and_reply(event, move |state| match state {
                    AccountEntity::Nonexistent => {
                        panic!("invalid command Deposit in state Nonexistent")
                    }

                    AccountEntity::Existing { balance } => Account {
                        id,
                        balance: *balance,
                    },
                })
            }
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

impl Command<AccountEntity> for Withdraw {
    type Reply = Account;
    type Error = WithdrawError;

    fn handle_command(
        self,
        id: &Uuid,
        state: &AccountEntity,
    ) -> CommandEffect<AccountEntity, Self::Reply, Self::Error> {
        let id = *id;

        match state {
            AccountEntity::Nonexistent => CommandEffect::reject(WithdrawError::NotFound(id)),

            AccountEntity::Existing { balance } if self.amount > *balance => {
                CommandEffect::reject(WithdrawError::InsufficientBalance(id))
            }

            AccountEntity::Existing { balance } => {
                let event = AccountEvent::Withdrawn {
                    id,
                    amount: self.amount,
                    balance: balance - self.amount,
                };
                CommandEffect::emit_and_reply(event, move |state| match state {
                    AccountEntity::Nonexistent => {
                        panic!("invalid command Withdraw in state Nonexistent")
                    }

                    AccountEntity::Existing { balance } => Account {
                        id,
                        balance: *balance,
                    },
                })
            }
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
