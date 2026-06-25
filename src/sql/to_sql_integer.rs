use serenity::all::{ChannelId, GuildId, MessageId, UserId};

pub(crate) trait ToSqlInteger: Sized {
    /// Transform `self` into a `u64`
    fn to_u64(self) -> u64;

    /// Transform `self` into a `i64` which sqlite can handle
    ///
    /// This is a pure binary transform; we don't care about the value produced, only
    /// that it fits into an `i64` which is a Sqlite `INTEGER` type.
    fn to_sql(self) -> i64 {
        i64::from_ne_bytes(self.to_u64().to_ne_bytes())
    }
}

macro_rules! impl_to_sql_integer_via_get {
    ($( $t:ty ),*) => {
        $(
            impl ToSqlInteger for $t {
                fn to_u64(self) -> u64 { self.get() }
            }
        )*
    };
}

impl_to_sql_integer_via_get!(ChannelId, GuildId, UserId, MessageId);
