//! SQLx column encoding of [`SecretString`]: values are encrypted on write and
//! decrypted on read with the configured field-encryption key.

use super::{SecretString, decrypt_configured_secret, encrypt_configured_secret};

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl<'r> sqlx::Decode<'r, sqlx::Any> for SecretString {
    fn decode(
        value: sqlx::any::AnyValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<sqlx::Any>>::decode(value)?;
        let decrypted = decrypt_configured_secret(&text)?;
        Ok(SecretString(decrypted))
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl<'q> sqlx::Encode<'q, sqlx::Any> for SecretString {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Any as sqlx::database::Database>::ArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let encrypted = encrypt_configured_secret(&self.0)?;
        <String as sqlx::Encode<sqlx::Any>>::encode(encrypted, buf)
    }
}

#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
impl sqlx::Type<sqlx::Any> for SecretString {
    fn type_info() -> sqlx::any::AnyTypeInfo {
        <String as sqlx::Type<sqlx::Any>>::type_info()
    }
}

// Support for strictly typed databases in Rullst
#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl<'r> sqlx::Decode<'r, crate::database::RullstDatabase> for SecretString {
    fn decode(
        value: <crate::database::RullstDatabase as sqlx::database::Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<crate::database::RullstDatabase>>::decode(value)?;
        let decrypted = decrypt_configured_secret(&text)?;
        Ok(SecretString(decrypted))
    }
}

#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl<'q> sqlx::Encode<'q, crate::database::RullstDatabase> for SecretString {
    fn encode_by_ref(
        &self,
        buf: &mut <crate::database::RullstDatabase as sqlx::database::Database>::ArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let encrypted = encrypt_configured_secret(&self.0)?;
        <String as sqlx::Encode<crate::database::RullstDatabase>>::encode(encrypted, buf)
    }
}

#[cfg_attr(test, mutants::skip)]
#[cfg(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
))]
impl sqlx::Type<crate::database::RullstDatabase> for SecretString {
    fn type_info() -> <crate::database::RullstDatabase as sqlx::database::Database>::TypeInfo {
        <String as sqlx::Type<crate::database::RullstDatabase>>::type_info()
    }
}
