use crate::utxo_db::error::UtxoIndexerError;
use diesel::prelude::*;
use plutus_ledger_api::v3::{
    address::Address,
    transaction::{TransactionInput, TxInInfo},
};
use strum_macros::Display;
use tracing::{error, info_span};
use tx_indexer::database::plutus as db;

#[derive(
    Clone, Debug, Eq, PartialEq, diesel::Queryable, diesel::Selectable, diesel::Insertable,
)]
#[diesel(table_name = crate::schema::utxos)]
pub struct UtxosTable {
    pub utxo_ref: db::TransactionInput,
    pub value: db::Value,
    pub address: db::Address,
    pub datum: db::OutputDatum,
    pub created_at: db::Slot,
    pub deleted_at: Option<db::Slot>,
}

#[derive(Debug)]
pub struct RollbackResult<T>
where
    T: std::fmt::Debug,
{
    pub recovered: Vec<T>,
    pub deleted: Vec<T>,
}

impl UtxosTable {
    pub fn new(utxo: TxInInfo, created_at: u64) -> Result<Self, db::DBTypeConversionError> {
        Ok(Self {
            utxo_ref: utxo.reference.try_into()?,
            value: utxo.output.value.try_into()?,
            address: utxo.output.address.try_into()?,
            datum: utxo.output.datum.try_into()?,
            created_at: created_at.into(),
            deleted_at: None,
        })
    }

    pub fn list_by_address(
        addr: Address,
        conn: &mut diesel::PgConnection,
    ) -> Result<Vec<Self>, UtxoIndexerError> {
        use crate::schema::utxos::dsl::*;

        utxos
            .filter(address.eq(db::Address::try_from(addr)?))
            .select(UtxosTable::as_select())
            .load(conn)
            .map_err(|err| {
                error!(%err);
                UtxoIndexerError::DbError(err)
            })
    }

    pub fn delete_by_id(
        utxo_ref: TransactionInput,
        slot: u64,
        conn: &mut diesel::PgConnection,
    ) -> Result<usize, UtxoIndexerError> {
        use crate::schema::utxos::dsl::{deleted_at, utxos};

        let utxo_ref: db::TransactionInput = utxo_ref.try_into()?;
        let slot: db::Slot = slot.into();

        let res = diesel::update(utxos.find(utxo_ref))
            .set(deleted_at.eq(Some(slot)))
            .execute(conn)
            .map_err(|err| {
                error!(%err);
                UtxoIndexerError::DbError(err)
            })?;

        Ok(res)
    }

    pub fn store(self, conn: &mut diesel::PgConnection) -> Result<(), UtxoIndexerError> {
        use crate::schema::utxos;

        diesel::insert_into(utxos::table)
            .values(self)
            .execute(conn)
            .map_err(|err| {
                error!(%err);
                UtxoIndexerError::DbError(err)
            })?;

        Ok(())
    }

    pub fn rollback_after_block(
        conn: &mut diesel::PgConnection,
        transaction_block: u64,
    ) -> Result<RollbackResult<UtxosTable>, UtxoIndexerError> {
        use crate::schema::utxos::dsl::*;

        let span = info_span!("Rollback", %transaction_block);
        let _entered = span.enter();

        let slot = db::Slot::from(transaction_block);
        let deleted = utxos
            .filter(created_at.gt(&slot))
            .select(UtxosTable::as_select())
            .load(conn)
            .map_err(|err| {
                error!(label=%Event::SqlxError, ?err);
                UtxoIndexerError::DbError(err)
            })?;

        let recovered = utxos
            .filter(deleted_at.gt(&slot))
            .select(UtxosTable::as_select())
            .load(conn)
            .map_err(|err| {
                error!(label=%Event::SqlxError, ?err);
                UtxoIndexerError::DbError(err)
            })?;

        let deleted_refs = deleted
            .iter()
            .map(|utxo| utxo.utxo_ref.clone())
            .collect::<Vec<_>>();

        diesel::delete(utxos.filter(utxo_ref.eq_any(deleted_refs))).execute(conn)?;

        let recovered_refs = recovered
            .iter()
            .map(|utxo| utxo.utxo_ref.clone())
            .collect::<Vec<_>>();

        diesel::update(utxos.filter(utxo_ref.eq_any(recovered_refs)))
            .set(deleted_at.eq(None::<db::Slot>))
            .execute(conn)?;

        Ok(RollbackResult { deleted, recovered })
    }
}

#[derive(Display)]
enum Event {
    SqlxError,
}
