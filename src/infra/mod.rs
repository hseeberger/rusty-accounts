mod pg_account_evt_handler;
mod pg_account_repository;

pub use pg_account_evt_handler::*;
pub use pg_account_repository::*;

#[cfg(test)]
mod tests {
    use crate::{
        domain::{Account, AccountEvt, AccountRepository},
        infra::{PgAccountEvtHandler, PgAccountRepository},
    };
    use assert_matches::assert_matches;
    use error_ext::BoxError;
    use eventsourced_projection::postgres::EvtHandler;
    use futures::TryStreamExt;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use testcontainers::{clients::Cli, RunnableImage};
    use testcontainers_modules::postgres::Postgres as TCPostgres;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_account_repo_evt_handler() -> Result<(), BoxError> {
        let containers = Cli::default();

        let container =
            containers.run(RunnableImage::from(TCPostgres::default()).with_tag("16-alpine"));
        let port = container.get_host_port_ipv4(5432);

        let cnn_url = format!("postgresql://postgres:postgres@localhost:{port}");
        let cnn_options = cnn_url.parse::<PgConnectOptions>()?;
        let pool = PgPoolOptions::new().connect_with(cnn_options).await?;

        sqlx::migrate!().run(&pool).await?;

        let account_repository = PgAccountRepository::new(pool.clone());

        let accounts = account_repository
            .accounts()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        assert!(accounts.is_empty());

        let id_1 = Uuid::now_v7();
        let id_2 = Uuid::now_v7();
        let mut tx = pool.begin().await?;
        PgAccountEvtHandler
            .handle_evt(AccountEvt::Created { id: id_1 }, &mut tx)
            .await?;
        tx.commit().await?;
        let mut tx = pool.begin().await?;
        PgAccountEvtHandler
            .handle_evt(AccountEvt::Created { id: id_2 }, &mut tx)
            .await?;
        tx.commit().await?;

        let accounts = account_repository
            .accounts()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        assert_eq!(
            accounts,
            vec![
                Account {
                    id: id_1,
                    balance: 0
                },
                Account {
                    id: id_2,
                    balance: 0
                }
            ]
        );

        let account = account_repository.account_by_id(id_1).await?;
        assert_matches!(account, Some(Account { id, balance: 0 }) if id == id_1);

        let account = account_repository.account_by_id(Uuid::now_v7()).await?;
        assert!(account.is_none());

        let mut tx = pool.begin().await?;
        PgAccountEvtHandler
            .handle_evt(
                AccountEvt::Deposited {
                    id: id_1,
                    amount: 10,
                    balance: 10,
                },
                &mut tx,
            )
            .await?;
        tx.commit().await?;

        let account = account_repository.account_by_id(id_1).await?;
        assert_matches!(account, Some(Account { id, balance: 10 }) if id == id_1);

        Ok(())
    }
}
