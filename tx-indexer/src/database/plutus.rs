#[cfg(feature = "diesel")]
use diesel::sql_types::{Array, Bytea, Nullable};
use num_bigint::BigInt;
use pla::csl::pla_to_csl::TryToCSL;
use plutus_ledger_api as pla;
use thiserror::Error;
use plutus_ledger_api::csl::{
    lib as csl,
    csl_to_pla::{TryFromCSLError, TryToPLA},
    pla_to_csl::TryFromPLAError,
};

#[derive(Error, Debug)]
pub enum DBTypeConversionError {
    #[error("Couldn't parse DB type, because some invariants weren't valid: {0}")]
    InvariantBroken(String),

    #[error("Cannot represent BigInt as PostgreSQL BIGINT type: {0}")]
    BigIntConversion(num_bigint::TryFromBigIntError<BigInt>),

    #[error(transparent)]
    PlutusDataEncodingError(#[from] PlutusDataEncodingError),
}

#[cfg(feature = "diesel")]
pub mod sql_types {

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "address", schema = "plutus"))]
    pub struct Address;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "asset_quantity", schema = "plutus"))]
    pub struct AssetQuantity;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "chain_pointer", schema = "plutus"))]
    pub struct ChainPointer;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "credential", schema = "plutus"))]
    pub struct Credential;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "currency_symbol", schema = "plutus"))]
    pub struct CurrencySymbol;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "datum_hash", schema = "plutus"))]
    pub struct DatumHash;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "ed25519_pub_key_hash", schema = "plutus"))]
    pub struct Ed25519PubKeyHash;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "hash28", schema = "plutus"))]
    pub struct Hash28;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "hash32", schema = "plutus"))]
    pub struct Hash32;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "output_datum", schema = "plutus"))]
    pub struct OutputDatum;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "plutus_data", schema = "plutus"))]
    pub struct PlutusData;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "script_hash", schema = "plutus"))]
    pub struct ScriptHash;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "slot", schema = "plutus"))]
    pub struct Slot;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "staking_credential", schema = "plutus"))]
    pub struct StakingCredential;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "token_name", schema = "plutus"))]
    pub struct TokenName;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "transaction_hash", schema = "plutus"))]
    pub struct TransactionHash;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "transaction_input", schema = "plutus"))]
    pub struct TransactionInput;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "transaction_output", schema = "plutus"))]
    pub struct TransactionOutput;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "tx_in_info", schema = "plutus"))]
    pub struct TxInInfo;

    #[derive(diesel::QueryId, diesel::SqlType)]
    #[diesel(postgres_type(name = "value", schema = "plutus"))]
    pub struct Value;
}

//////////////////////
/// Hash28
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Hash28), 
    diesel_derive_pg(sql_type = sql_types::Hash28)
)]
pub struct Hash28(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Bytea))] pub Vec<u8>);

impl From<pla::v3::crypto::LedgerBytes> for Hash28 {
    fn from(item: pla::v3::crypto::LedgerBytes) -> Self {
        Hash28(item.0)
    }
}

impl From<Hash28> for pla::v3::crypto::LedgerBytes {
    fn from(item: Hash28) -> Self {
        pla::v3::crypto::LedgerBytes(item.0)
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for Hash28 {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.hash28")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Vec<u8> as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// Hash32
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Hash32),
    diesel_derive_pg(sql_type = sql_types::Hash32)
)]
pub struct Hash32(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Bytea))] pub Vec<u8>);

impl From<pla::v3::crypto::LedgerBytes> for Hash32 {
    fn from(item: pla::v3::crypto::LedgerBytes) -> Self {
        Hash32(item.0)
    }
}

impl From<Hash32> for pla::v3::crypto::LedgerBytes {
    fn from(item: Hash32) -> Self {
        pla::v3::crypto::LedgerBytes(item.0)
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for Hash32 {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.hash32")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Vec<u8> as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// CurrencySymbol
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::CurrencySymbol),
    diesel_derive_pg(sql_type = sql_types::CurrencySymbol)
)]
pub struct CurrencySymbol(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Bytea))] pub Vec<u8>);

impl From<pla::v3::value::CurrencySymbol> for CurrencySymbol {
    fn from(item: pla::v3::value::CurrencySymbol) -> Self {
        match item {
            pla::v3::value::CurrencySymbol::Ada => CurrencySymbol(Vec::with_capacity(0)),
            pla::v3::value::CurrencySymbol::NativeToken(pla::v3::script::MintingPolicyHash(
                pla::v3::script::ScriptHash(pla::v3::crypto::LedgerBytes(bytes)),
            )) => CurrencySymbol(bytes),
        }
    }
}

impl From<CurrencySymbol> for pla::v3::value::CurrencySymbol {
    fn from(item: CurrencySymbol) -> Self {
        let CurrencySymbol(bytes) = item;
        if bytes.is_empty() {
            pla::v3::value::CurrencySymbol::Ada
        } else {
            pla::v3::value::CurrencySymbol::NativeToken(pla::v3::script::MintingPolicyHash(
                pla::v3::script::ScriptHash(pla::v3::crypto::LedgerBytes(bytes)),
            ))
        }
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for CurrencySymbol {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.currency_symbol")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Vec<u8> as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// TokenName
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::TokenName),
    diesel_derive_pg(sql_type = sql_types::TokenName)
)]
pub struct TokenName(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Bytea))] pub Vec<u8>);

impl From<pla::v3::value::TokenName> for TokenName {
    fn from(item: pla::v3::value::TokenName) -> Self {
        TokenName(item.0 .0)
    }
}

impl From<TokenName> for pla::v3::value::TokenName {
    fn from(item: TokenName) -> Self {
        pla::v3::value::TokenName(pla::v3::crypto::LedgerBytes(item.0))
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for TokenName {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.token_name")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Vec<u8> as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// TransactionHash
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::TransactionHash),
    diesel_derive_pg(sql_type = sql_types::TransactionHash)
)]
pub struct TransactionHash(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Hash32))] pub Hash32);

impl From<pla::v3::transaction::TransactionHash> for TransactionHash {
    fn from(item: pla::v3::transaction::TransactionHash) -> Self {
        TransactionHash(item.0.into())
    }
}

impl From<TransactionHash> for pla::v3::transaction::TransactionHash {
    fn from(item: TransactionHash) -> Self {
        pla::v3::transaction::TransactionHash(item.0.into())
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for TransactionHash {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.transaction_hash")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Hash32 as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// Ed25519PubKeyHash
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Ed25519PubKeyHash),
    diesel_derive_pg(sql_type = sql_types::Ed25519PubKeyHash)
)]
pub struct Ed25519PubKeyHash(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Hash28))] pub Hash28);

impl From<pla::v3::crypto::Ed25519PubKeyHash> for Ed25519PubKeyHash {
    fn from(item: pla::v3::crypto::Ed25519PubKeyHash) -> Self {
        Ed25519PubKeyHash(item.0.into())
    }
}

impl From<Ed25519PubKeyHash> for pla::v3::crypto::Ed25519PubKeyHash {
    fn from(item: Ed25519PubKeyHash) -> Self {
        pla::v3::crypto::Ed25519PubKeyHash(item.0.into())
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for Ed25519PubKeyHash {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.ed25519_pub_key_hash")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Hash28 as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// ScriptHash
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::ScriptHash),
    diesel_derive_pg(sql_type = sql_types::ScriptHash)
)]
pub struct ScriptHash(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Hash28))] pub Hash28);

impl From<pla::v3::script::ScriptHash> for ScriptHash {
    fn from(item: pla::v3::script::ScriptHash) -> Self {
        ScriptHash(item.0.into())
    }
}

impl From<ScriptHash> for pla::v3::script::ScriptHash {
    fn from(item: ScriptHash) -> Self {
        pla::v3::script::ScriptHash(item.0.into())
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for ScriptHash {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.script_hash")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Hash28 as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// DatumHash
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::DatumHash),
    diesel_derive_pg(sql_type = sql_types::DatumHash)
    )]
pub struct DatumHash(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Hash32))] pub Hash32);

impl From<pla::v3::datum::DatumHash> for DatumHash {
    fn from(item: pla::v3::datum::DatumHash) -> Self {
        DatumHash(item.0.into())
    }
}

impl From<DatumHash> for pla::v3::datum::DatumHash {
    fn from(item: DatumHash) -> Self {
        pla::v3::datum::DatumHash(item.0.into())
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for DatumHash {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.datum_hash")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <Hash32 as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// Slot
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Slot),
    diesel_derive_pg(sql_type = sql_types::Slot)
)]
pub struct Slot(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))] pub i64);

impl From<u64> for Slot {
    fn from(item: u64) -> Self {
        Slot(item as i64)
    }
}

impl From<&Slot> for u64 {
    fn from(item: &Slot) -> Self {
        item.0 as u64
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for Slot {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.slot")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty || <i64 as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}


//////////////////////
/// PlutusData
//////////////////////

#[derive(Error, Debug)]
pub enum PlutusDataEncodingError {
    #[error(transparent)]
    CSLConversionError(#[from] csl::JsError),

    #[error(transparent)]
    TryFromPLAError(#[from] TryFromPLAError),

    #[error(transparent)]
    TryFromCSLError(#[from] TryFromCSLError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Encode, sqlx::Decode))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::PlutusData),
    diesel_derive_pg(sql_type = sql_types::PlutusData)
)]
pub struct PlutusData(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::Jsonb))] pub serde_json::Value);

impl TryFrom<pla::plutus_data::PlutusData> for PlutusData {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::plutus_data::PlutusData) -> Result<Self, Self::Error> {
        Ok(PlutusData(
            csl::decode_plutus_datum_to_json_value(
                &item
                    .try_to_csl()
                    .map_err(PlutusDataEncodingError::TryFromPLAError)?,
                csl::PlutusDatumSchema::DetailedSchema,
            )
            .map_err(PlutusDataEncodingError::CSLConversionError)?,
        ))
    }
}

impl TryFrom<PlutusData> for pla::plutus_data::PlutusData {
    type Error = DBTypeConversionError;

    fn try_from(item: PlutusData) -> Result<Self, Self::Error> {
        Ok(
            csl::encode_json_value_to_plutus_datum(item.0, csl::PlutusDatumSchema::DetailedSchema)
                .map_err(PlutusDataEncodingError::CSLConversionError)?
                .try_to_pla()
                .map_err(PlutusDataEncodingError::TryFromCSLError)?,
        )
    }
}

#[cfg(feature = "sqlx")]
impl ::sqlx::Type<::sqlx::postgres::Postgres> for PlutusData {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("plutus.plutus_data")
    }
    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> ::std::primitive::bool {
        Self::type_info() == *ty
            || <serde_json::Value as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
    }
}

//////////////////////
/// Credential
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "plutus.credential"))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Credential),
    diesel_derive_pg(sql_type = sql_types::Credential)
)]
pub struct Credential {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::Ed25519PubKeyHash>))]
    pub_key_hash: Option<Ed25519PubKeyHash>,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::ScriptHash>))]
    script_hash: Option<ScriptHash>,
}

impl From<pla::v3::address::Credential> for Credential {
    fn from(item: pla::v3::address::Credential) -> Self {
        match item {
            pla::v3::address::Credential::PubKey(pkh) => Credential {
                pub_key_hash: Some(pkh.into()),
                script_hash: None,
            },
            pla::v3::address::Credential::Script(pla::v3::script::ValidatorHash(sh)) => {
                Credential {
                    pub_key_hash: None,
                    script_hash: Some(sh.into()),
                }
            }
        }
    }
}

impl TryFrom<Credential> for pla::v3::address::Credential {
    type Error = DBTypeConversionError;

    fn try_from(item: Credential) -> Result<Self, Self::Error> {
        Ok(match item {
            Credential {
                pub_key_hash: Some(pkh_db),
                script_hash: None,
            } => pla::v3::address::Credential::PubKey(pkh_db.into()),
            Credential {
                pub_key_hash: None,
                script_hash: Some(sh_db),
            } => pla::v3::address::Credential::Script(pla::v3::script::ValidatorHash(sh_db.into())),
            _ => Err(DBTypeConversionError::InvariantBroken(
                "DB Credential must have either 'pub_key_hash' or 'script_hash'".to_string(),
            ))?,
        })
    }
}

//////////////////////
/// ChainPointer
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(type_name = "plutus.chain_pointer"))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::ChainPointer),
    diesel_derive_pg(sql_type = sql_types::ChainPointer)
)]
pub struct ChainPointer {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))]
    slot_num: i64,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))]
    tx_idx: i64,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))]
    cert_idx: i64,
}

impl TryFrom<pla::v3::address::ChainPointer> for ChainPointer {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::address::ChainPointer) -> Result<Self, Self::Error> {
        Ok(ChainPointer {
            slot_num: item
                .slot_number
                .0
                .try_into()
                .map_err(DBTypeConversionError::BigIntConversion)?,
            tx_idx: item
                .transaction_index
                .0
                .try_into()
                .map_err(DBTypeConversionError::BigIntConversion)?,
            cert_idx: item
                .certificate_index
                .0
                .try_into()
                .map_err(DBTypeConversionError::BigIntConversion)?,
        })
    }
}

impl From<ChainPointer> for pla::v3::address::ChainPointer {
    fn from(item: ChainPointer) -> Self {
        pla::v3::address::ChainPointer {
            slot_number: pla::v3::address::Slot(BigInt::from(item.slot_num)),
            transaction_index: pla::v3::address::TransactionIndex(BigInt::from(item.tx_idx)),
            certificate_index: pla::v3::address::CertificateIndex(BigInt::from(item.cert_idx)),
        }
    }
}

//////////////////////
/// StakingCredential
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(type_name = "plutus.staking_credential"))]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::StakingCredential),
    diesel_derive_pg(sql_type = sql_types::StakingCredential)
)]
pub struct StakingCredential {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::Credential>))]
    staking_hash: Option<Credential>,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::ChainPointer>))]
    staking_ptr: Option<ChainPointer>,
}

impl TryFrom<pla::v3::address::StakingCredential> for StakingCredential {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::address::StakingCredential) -> Result<Self, Self::Error> {
        Ok(match item {
            pla::v3::address::StakingCredential::Hash(cred) => StakingCredential {
                staking_hash: Some(cred.into()),
                staking_ptr: None,
            },
            pla::v3::address::StakingCredential::Pointer(ptr) => StakingCredential {
                staking_hash: None,
                staking_ptr: Some(ptr.try_into()?),
            },
        })
    }
}

impl TryFrom<StakingCredential> for pla::v3::address::StakingCredential {
    type Error = DBTypeConversionError;

    fn try_from(item: StakingCredential) -> Result<Self, Self::Error> {
        Ok(match item {
            StakingCredential {
                staking_hash: Some(cred),
                staking_ptr: None,
            } => pla::v3::address::StakingCredential::Hash(cred.try_into()?),
            StakingCredential {
                staking_hash: None,
                staking_ptr: Some(ptr),
            } => pla::v3::address::StakingCredential::Pointer(ptr.into()),

            _ => Err(DBTypeConversionError::InvariantBroken(
                "DB StakingCredential must have either 'staking_hash' or 'staking_ptr'".to_string(),
            ))?,
        })
    }
}

//////////////////////
/// Address
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx", 
    derive(sqlx::Type),
    sqlx(type_name = "plutus.address")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Address),
    diesel_derive_pg(sql_type = sql_types::Address)
)]
pub struct Address {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Credential))]
    credential: Credential,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::StakingCredential>))]
    staking_credential: Option<StakingCredential>,
}

impl TryFrom<pla::v3::address::Address> for Address {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::address::Address) -> Result<Self, Self::Error> {
        Ok(Address {
            credential: item.credential.into(),
            staking_credential: item
                .staking_credential
                .map(StakingCredential::try_from)
                .transpose()?,
        })
    }
}

impl TryFrom<Address> for pla::v3::address::Address {
    type Error = DBTypeConversionError;

    fn try_from(item: Address) -> Result<Self, Self::Error> {
        Ok(pla::v3::address::Address {
            credential: item.credential.try_into()?,
            staking_credential: item
                .staking_credential
                .map(pla::v3::address::StakingCredential::try_from)
                .transpose()?,
        })
    }
}

//////////////////////
/// AssetQuantity
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx", derive(sqlx::Type),
    sqlx(type_name = "plutus.asset_quantity")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::AssetQuantity),
    diesel_derive_pg(sql_type = sql_types::AssetQuantity)
)]
pub struct AssetQuantity {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::CurrencySymbol))]
    currency_symbol: CurrencySymbol,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::TokenName))]
    token_name: TokenName,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))]
    amount: i64,
}

impl
    TryFrom<(
        pla::v3::value::CurrencySymbol,
        pla::v3::value::TokenName,
        BigInt,
    )> for AssetQuantity
{
    type Error = DBTypeConversionError;

    fn try_from(
        item: (
            pla::v3::value::CurrencySymbol,
            pla::v3::value::TokenName,
            BigInt,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(AssetQuantity {
            currency_symbol: item.0.into(),
            token_name: item.1.into(),
            amount: item
                .2
                .try_into()
                .map_err(DBTypeConversionError::BigIntConversion)?,
        })
    }
}

impl From<AssetQuantity>
    for (
        pla::v3::value::CurrencySymbol,
        pla::v3::value::TokenName,
        BigInt,
    )
{
    fn from(item: AssetQuantity) -> Self {
        (
            item.currency_symbol.into(),
            item.token_name.into(),
            item.amount.into(),
        )
    }
}

//////////////////////
/// Value
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "plutus.asset_quantity[]")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::Value),
    diesel_derive_pg(sql_type = sql_types::Value)
)]
pub struct Value(#[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Array<sql_types::AssetQuantity>))] pub Vec<AssetQuantity>);

impl TryFrom<pla::v3::value::Value> for Value {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::value::Value) -> Result<Self, Self::Error> {
        let assets = item
            .0
            .iter()
            .flat_map(|(cs, assets)| {
                assets
                    .iter()
                    .map(|(tn, amount)| {
                        AssetQuantity::try_from((cs.to_owned(), tn.to_owned(), amount.to_owned()))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Result<Vec<AssetQuantity>, DBTypeConversionError>>()?;

        Ok(Value(assets))
    }
}

impl From<Value> for pla::v3::value::Value {
    fn from(item: Value) -> Self {
        item.0.into_iter().fold(
            pla::v3::value::Value::new(),
            |value,
             AssetQuantity {
                 currency_symbol,
                 token_name,
                 amount,
             }| {
                value.insert_token(&currency_symbol.into(), &token_name.into(), &amount.into())
            },
        )
    }
}

//////////////////////
/// TransactionInput
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "plutus.transaction_input")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::TransactionInput),
    diesel_derive_pg(sql_type = sql_types::TransactionInput)
)]
pub struct TransactionInput {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::TransactionHash))]
    tx_id: TransactionHash,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = diesel::sql_types::BigInt))]
    tx_idx: i64,
}

impl TryFrom<pla::v3::transaction::TransactionInput> for TransactionInput {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::transaction::TransactionInput) -> Result<Self, Self::Error> {
        Ok(TransactionInput {
            tx_id: item.transaction_id.into(),
            tx_idx: item
                .index
                .try_into()
                .map_err(DBTypeConversionError::BigIntConversion)?,
        })
    }
}

impl From<TransactionInput> for pla::v3::transaction::TransactionInput {
    fn from(item: TransactionInput) -> Self {
        pla::v3::transaction::TransactionInput {
            transaction_id: item.tx_id.into(),
            index: item.tx_idx.into(),
        }
    }
}

//////////////////////
/// OutputDatum
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "plutus.output_datum")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::OutputDatum),
    diesel_derive_pg(sql_type = sql_types::OutputDatum)
)]
pub struct OutputDatum {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::DatumHash>))]
    datum_hash: Option<DatumHash>,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::PlutusData>))]
    inline_datum: Option<PlutusData>,
}

impl TryFrom<pla::v3::datum::OutputDatum> for OutputDatum {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::datum::OutputDatum) -> Result<Self, Self::Error> {
        Ok(match item {
            pla::v3::datum::OutputDatum::DatumHash(dh) => OutputDatum {
                datum_hash: Some(dh.into()),
                inline_datum: None,
            },
            pla::v3::datum::OutputDatum::InlineDatum(pla::v3::datum::Datum(datum)) => OutputDatum {
                datum_hash: None,
                inline_datum: Some(datum.try_into()?),
            },
            pla::v3::datum::OutputDatum::None => OutputDatum {
                datum_hash: None,
                inline_datum: None,
            },
        })
    }
}

impl TryFrom<OutputDatum> for pla::v3::datum::OutputDatum {
    type Error = DBTypeConversionError;

    fn try_from(item: OutputDatum) -> Result<Self, Self::Error> {
        Ok(match item {
            OutputDatum {
                datum_hash: Some(dh_db),
                ..
            } => pla::v3::datum::OutputDatum::DatumHash(dh_db.into()),
            OutputDatum {
                inline_datum: Some(datum_db),
                ..
            } => pla::v3::datum::OutputDatum::InlineDatum(pla::v3::datum::Datum(
                datum_db.try_into()?,
            )),
            _ => pla::v3::datum::OutputDatum::None,
        })
    }
}

//////////////////////
/// TransactionOutput
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "plutus.transaction_output")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::TransactionOutput),
    diesel_derive_pg(sql_type = sql_types::TransactionOutput)
)]
pub struct TransactionOutput {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Address))]
    address: Address,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::Value))]
    assets: Value,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::OutputDatum))]
    datum: OutputDatum,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = Nullable<sql_types::ScriptHash>))]
    reference_script: Option<ScriptHash>,
}

impl TryFrom<pla::v3::transaction::TransactionOutput> for TransactionOutput {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::transaction::TransactionOutput) -> Result<Self, Self::Error> {
        Ok(TransactionOutput {
            address: item.address.try_into()?,
            assets: item.value.try_into()?,
            datum: item.datum.try_into()?,
            reference_script: item.reference_script.map(ScriptHash::from),
        })
    }
}

impl TryFrom<TransactionOutput> for pla::v3::transaction::TransactionOutput {
    type Error = DBTypeConversionError;

    fn try_from(item: TransactionOutput) -> Result<Self, Self::Error> {
        Ok(pla::v3::transaction::TransactionOutput {
            address: item.address.try_into()?,
            value: item.assets.into(),
            datum: item.datum.try_into()?,
            reference_script: item.reference_script.map(pla::v3::script::ScriptHash::from),
        })
    }
}

//////////////////////
/// TxInInfo
//////////////////////

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "plutus.tx_in_info")
)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow, diesel_derive_pg::PgCustomType),
    diesel(sql_type = sql_types::TxInInfo),
    diesel_derive_pg(sql_type = sql_types::TxInInfo)
)]
pub struct TxInInfo {
    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::TransactionInput))]
    reference: TransactionInput,

    #[cfg_attr(feature = "diesel", diesel_derive_pg(sql_type = sql_types::TransactionOutput))]
    output: TransactionOutput,
}

impl TryFrom<pla::v3::transaction::TxInInfo> for TxInInfo {
    type Error = DBTypeConversionError;

    fn try_from(item: pla::v3::transaction::TxInInfo) -> Result<Self, Self::Error> {
        Ok(TxInInfo {
            reference: item.reference.try_into()?,
            output: item.output.try_into()?,
        })
    }
}

impl TryFrom<TxInInfo> for pla::v3::transaction::TxInInfo {
    type Error = DBTypeConversionError;

    fn try_from(item: TxInInfo) -> Result<Self, Self::Error> {
        Ok(pla::v3::transaction::TxInInfo {
            reference: item.reference.into(),
            output: item.output.try_into()?,
        })
    }
}

