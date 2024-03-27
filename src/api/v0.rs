use crate::{
    api::AppState,
    domain::{Account, AccountEntity, AccountRepository, CreateAccount, CreateAccountError},
};
use axum::{extract::State, routing::get, Json, Router};
use error_ext::{axum::Error, StdErrorExt};
use eventsourced::{
    binarize::serde_json::SerdeJsonBinarize, evt_log::EvtLog,
    snapshot_store::noop::NoopSnapshotStore, EventSourcedExt,
};
use futures::TryStreamExt;
use serde::Serialize;
use std::num::NonZeroUsize;
use tracing::{error, instrument};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(list_accounts, create_accounts),
    components(schemas(ListAccountsResponse, Account))
)]
pub struct ApiDoc;

pub fn app<R, E>() -> Router<AppState<R, E>>
where
    R: AccountRepository,
    E: EvtLog<Id = Uuid> + Sync,
{
    Router::new().route("/accounts", get(list_accounts).post(create_accounts))
}

#[derive(Debug, Serialize, ToSchema)]
struct ListAccountsResponse {
    accounts: Vec<Account>,
}

/// List accounts.
#[utoipa::path(
    get,
    path = "/accounts",
    responses(
        (status = 200, description = "A list of accounts.", body = ListAccountsResponse),
    ),
    tag = "account",
)]
#[instrument(skip(app_state))]
async fn list_accounts<R, L>(
    State(app_state): State<AppState<R, L>>,
) -> Result<Json<ListAccountsResponse>, Error>
where
    R: AccountRepository,
    L: EvtLog,
{
    let accounts = app_state
        .account_repository
        .accounts()
        .await
        .map_err(|error| {
            error!(error = error.as_chain(), "cannot list accounts");
            Error::Internal
        })?;

    let accounts = accounts.try_collect::<Vec<_>>().await.map_err(|error| {
        error!(error = error.as_chain(), "cannot list accounts");
        Error::Internal
    })?;

    Ok(Json(ListAccountsResponse { accounts }))
}

/// Create an account.
#[utoipa::path(
    post,
    path = "/accounts",
    responses(
        (status = 200, description = "The created account", body = Account),
        (status = 409, description = "An account with the created ID already exists", body = Error),
    ),
    tag = "account",
)]
#[instrument(skip(app_state))]
async fn create_accounts<R, L>(
    State(app_state): State<AppState<R, L>>,
) -> Result<Json<Account>, Error>
where
    R: AccountRepository,
    L: EvtLog<Id = Uuid>,
{
    let id = Uuid::now_v7();

    let entity = AccountEntity::default()
        .entity()
        .spawn(
            id,
            None,
            NonZeroUsize::MIN,
            app_state.evt_log.clone(),
            NoopSnapshotStore::default(),
            SerdeJsonBinarize,
        )
        .await
        .map_err(|error| {
            error!(error = error.as_chain(), "cannot create AccountEntity");
            Error::Internal
        })?;

    let reply = entity.handle_cmd(CreateAccount).await.map_err(|error| {
        error!(
            error = error.as_chain(),
            "cannot handle CreateAccount command"
        );

        Error::Internal
    })?;

    reply
        .map_err(|error| match error {
            CreateAccountError::AlreadyExisting(_) => Error::conflict(error),
        })
        .map(Json)
}

// #[derive(Debug, Deserialize, ToSchema)]
// struct DepositRequest {
//     amount: u64,
// }

// async fn deposit<R>(
//     State(app_state): State<AppState<R>>,
//     id: Uuid,
//     Json(DepositRequest { amount }): Json<DepositRequest>,
// ) -> Result<Json<ListAccountsResponse>, Error>
// where
//     R: AccountRepository,
// {
//     todo!()
// }
