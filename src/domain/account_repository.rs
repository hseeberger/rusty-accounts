use crate::domain::Account;
use futures::Stream;
use std::error::Error as StdError;
use uuid::Uuid;

#[trait_variant::make(Send)]
pub trait AccountRepository
where
    Self: Clone + Send + Sync + 'static,
{
    // type Error: StdError + Send + Sync + 'static;
    type Error: StdError;

    async fn accounts(
        &self,
    ) -> Result<impl Stream<Item = Result<Account, Self::Error>> + Send, Self::Error>;

    async fn account_by_id(&self, id: Uuid) -> Result<Option<Account>, Self::Error>;
}
