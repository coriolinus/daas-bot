use actix_web::{
    HttpMessage,
    error::ParseError,
    http::header::{Header, HeaderName, HeaderValue, TryIntoHeaderValue, from_one_raw_str},
};

#[derive(derive_more::Deref)]
#[deref(forward)]
pub struct XSignatureEd25519(String);

impl XSignatureEd25519 {
    const NAME: HeaderName = HeaderName::from_static("x-signature-ed25519");
}

impl TryIntoHeaderValue for XSignatureEd25519 {
    type Error = <String as TryInto<HeaderValue>>::Error;

    fn try_into_value(self) -> Result<HeaderValue, Self::Error> {
        self.0.try_into()
    }
}

impl Header for XSignatureEd25519 {
    fn name() -> HeaderName {
        Self::NAME
    }

    fn parse<M: HttpMessage>(msg: &M) -> Result<Self, ParseError> {
        from_one_raw_str(msg.headers().get(Self::NAME)).map(Self)
    }
}

#[derive(derive_more::Deref)]
#[deref(forward)]
pub struct XSignatureTimestamp(String);

impl XSignatureTimestamp {
    const NAME: HeaderName = HeaderName::from_static("x-signature-timestamp");
}

impl TryIntoHeaderValue for XSignatureTimestamp {
    type Error = <String as TryInto<HeaderValue>>::Error;

    fn try_into_value(self) -> Result<HeaderValue, Self::Error> {
        self.0.try_into()
    }
}

impl Header for XSignatureTimestamp {
    fn name() -> HeaderName {
        Self::NAME
    }

    fn parse<M: HttpMessage>(msg: &M) -> Result<Self, ParseError> {
        from_one_raw_str(msg.headers().get(Self::NAME)).map(Self)
    }
}
