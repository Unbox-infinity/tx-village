use diesel_derive_pg::PgCustomType;

struct SqlType;
#[allow(dead_code)]
struct SqlInnerType;

#[derive(Debug, PgCustomType)]
#[diesel_derive_pg(sql_type = SqlType)]
pub struct Newtype(#[diesel_derive_pg(sql_type = diesel::sql_types::Text)] pub String);

#[derive(Debug, PgCustomType)]
#[diesel_derive_pg(sql_type = SqlType)]
pub struct Struct {
    #[diesel_derive_pg(sql_type  = diesel::sql_types::Text)]
    field_one: String,

    #[diesel_derive_pg(sql_type  = diesel::sql_types::BigInt)]
    field_two: i64,
}
