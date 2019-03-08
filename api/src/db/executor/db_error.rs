use actix_web::Error;

pub fn db_error<T: Into<String>, U: std::fmt::Debug>(message: T, err: U) -> Error {
    let mstr = message.into();
    error!("db_error: {}; {:?}", mstr, err);
    std::io::Error::new(std::io::ErrorKind::Other, mstr).into()
}
