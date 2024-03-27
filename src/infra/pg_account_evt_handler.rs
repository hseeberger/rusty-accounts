use crate::domain::AccountEvt;
use eventsourced_projection::postgres::EvtHandler;
use sqlx::{Postgres, QueryBuilder, Transaction};
use std::iter::once;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PgAccountEvtHandler;

impl EvtHandler<AccountEvt> for PgAccountEvtHandler {
    type Error = sqlx::Error;

    #[instrument(skip(self, tx))]
    async fn handle_evt(
        &self,
        evt: AccountEvt,
        tx: &mut Transaction<'static, Postgres>,
    ) -> Result<(), Self::Error> {
        match evt {
            AccountEvt::Created { id } => {
                QueryBuilder::new("INSERT INTO account (id, balance) ")
                    .push_values(once(id), |mut q, id| {
                        q.push_bind(id).push_bind(0);
                    })
                    .build()
                    .execute(&mut **tx)
                    .await?;

                info!(%id, "inserted account");
                Ok(())
            }

            AccountEvt::Deposited {
                id,
                amount,
                balance,
            } => {
                update(id, balance, tx).await?;

                info!(amount, "account updated with deposited amount");
                Ok(())
            }

            AccountEvt::Withdrawn {
                id,
                balance,
                amount,
            } => {
                update(id, balance, tx).await?;

                info!(amount, "account updated with withdrawn amount");
                Ok(())
            }
        }
    }
}

#[instrument(skip(tx))]
async fn update(
    id: Uuid,
    balance: u64,
    tx: &mut Transaction<'static, Postgres>,
) -> Result<(), sqlx::Error> {
    QueryBuilder::new("UPDATE account SET balance = ")
        .push_bind(balance as i64)
        .push(" WHERE id = ")
        .push_bind(id)
        .build()
        .execute(&mut **tx)
        .await?;
    Ok(())
}
