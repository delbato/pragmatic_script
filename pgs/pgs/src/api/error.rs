pub enum APIError {
    Unknown,
    NoFnSignature,
    ArgDeserializeError,
    ArgSerializeError
}

pub type APIResult<T> = Result<T, APIError>;