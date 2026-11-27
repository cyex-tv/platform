//! Postgres mappings for the enums stored as `text` with a CHECK (see the migrations). They read
//! and write the same strings as the JSON form.

use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, Postgres, Type};

use crate::model::{ApprovalState, EmoteState, Platform, SetKind, SetVisibility};

macro_rules! text_enum {
    ($($ty:ty),+) => {$(
        impl Type<Postgres> for $ty {
            fn type_info() -> PgTypeInfo {
                <&str as Type<Postgres>>::type_info()
            }

            fn compatible(ty: &PgTypeInfo) -> bool {
                <&str as Type<Postgres>>::compatible(ty)
            }
        }

        impl Encode<'_, Postgres> for $ty {
            fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
                <&str as Encode<Postgres>>::encode(self.as_str(), buf)
            }
        }

        impl<'r> Decode<'r, Postgres> for $ty {
            fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
                Ok(<&str as Decode<Postgres>>::decode(value)?.parse()?)
            }
        }
    )+};
}

text_enum!(Platform, EmoteState, ApprovalState, SetKind, SetVisibility);
