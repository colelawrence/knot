use actix::prelude::*;
use actix_redis::{Command, RedisActor, RespValue};
use futures::future::{self, Future};

use crate::prelude::*;

pub mod models;
pub mod sessions;

/// This is memory executor actor
#[derive(Clone)]
pub struct MemExecutor(Addr<RedisActor>);

pub trait MemModel {
    fn table_prefix() -> &'static str;
    fn table_key(&self) -> &str;
    fn named_key(&self) -> String {
        format!("{}#{}", Self::table_prefix(), self.table_key())
    }
}

impl MemExecutor {
    pub fn new(redis: Addr<RedisActor>) -> Self {
        MemExecutor(redis)
    }

    pub fn get_json<T>(&self, key: &str) -> impl Future<Item = Option<T>, Error = Error>
    where
        T: serde::de::DeserializeOwned + MemModel + 'static,
    {
        let redis = &self.0;
        let named_key = format!("{}#{}", T::table_prefix(), key);
        Box::new(
            redis
                .send(Command(resp_array!["GET", named_key]))
                .map_err(Error::from)
                .and_then(|res| match res {
                    Ok(val) => match val {
                        RespValue::Error(err) => Err(mem_error("get_json error", err)),
                        // I don't think SimpleString is possible based on docs...
                        RespValue::SimpleString(s) => Ok(Some(String::from(s))),
                        RespValue::BulkString(s) => String::from_utf8(s)
                            .map_err(|e| mem_error("Redis returned invalid utf8", e))
                            .map(Some),
                        RespValue::Nil => Ok(None),
                        other => Err(mem_error("get_json error: unknown response", other)),
                    },
                    Err(err) => Err(mem_error("get_json redis error", err)),
                })
                .and_then(|res_str_opt| match res_str_opt {
                    Some(res_str) => serde_json::from_str::<T>(&res_str)
                        .map_err(|err| mem_error("get_json error: deserialization", err))
                        .map(Some),
                    None => Ok(None),
                }),
        )
    }

    pub fn set_json<T>(&self, value: &T, expires_in: &std::time::Duration) -> AppFuture<()>
    where
        T: serde::ser::Serialize + MemModel,
    {
        let redis = &self.0;
        let named_key = value.named_key();
        let value_str = match serde_json::to_string(value) {
            Ok(v) => v,
            Err(err) => {
                return Box::new(future::err(mem_error("set_json error: serialization", err)));
            }
        };
        let expires_in_secs = format!("{}", expires_in.as_secs());
        Box::new(
            redis
                .send(Command(resp_array![
                    "SET",
                    named_key,
                    value_str,
                    "EX",
                    expires_in_secs
                ]))
                .map_err(Error::from)
                .and_then(|res| match res {
                    Ok(val) => match val {
                        RespValue::SimpleString(_) => Ok(()),
                        RespValue::Error(err) => Err(mem_error("set_json error", err)),
                        other => Err(mem_error(
                            "Redis set_json: Unknown response from SET",
                            other,
                        )),
                    },
                    Err(err) => Err(mem_error("set_json redis error", err)),
                }),
        )
    }

    pub fn set_json_if_not_exists<T>(
        &self,
        value: &T,
        expires_in: &std::time::Duration,
    ) -> AppFuture<bool>
    where
        T: serde::ser::Serialize + MemModel,
    {
        let redis = &self.0;
        let named_key = value.named_key();
        let value_str = match serde_json::to_string(value) {
            Ok(v) => v,
            Err(err) => {
                return Box::new(future::err(mem_error("set_json error: serialization", err)));
            }
        };
        let expires_in_secs = format!("{}", expires_in.as_secs());
        Box::new(
            redis
                .send(Command(resp_array![
                    "SET",
                    named_key,
                    value_str,
                    "EX",
                    expires_in_secs
                ]))
                .map_err(Error::from)
                .and_then(|res| match res {
                    Ok(val) => match val {
                        RespValue::SimpleString(_) => Ok(true),
                        RespValue::Nil => Ok(false),
                        RespValue::Error(err) => Err(mem_error("set_json error", err)),
                        other => Err(mem_error(
                            "Redis set_json: Unknown response from GET",
                            other,
                        )),
                    },
                    Err(err) => Err(mem_error("set_json redis error", err)),
                }),
        )
    }

    pub fn delete<T: MemModel>(&self, key: &str) -> impl Future<Item = (), Error = Error> {
        let redis = &self.0;
        let named_key = format!("{}#{}", T::table_prefix(), key);
        Box::new(
            redis
                .send(Command(resp_array!["DEL", named_key]))
                .map_err(Error::from)
                .and_then(|res| match res {
                    Ok(val) => match val {
                        RespValue::Integer(_) => Ok(()),
                        RespValue::Error(err) => Err(mem_error("delete error", err)),
                        other => Err(mem_error("Redis delete: Unknown response from DEL", other)),
                    },
                    Err(err) => Err(mem_error("delete redis error", err)),
                }),
        )
    }
}

fn mem_error<T: Into<String>, U: std::fmt::Debug>(message: T, err: U) -> Error {
    let mstr = message.into();
    error!("mem_error: {}; {:?}", mstr, err);
    Error::InternalServerError
}

use crate::app::AppState;
use actix::Addr;
use actix_web::{FromRequest, HttpRequest};

impl FromRequest<AppState> for MemExecutor {
    type Config = ();
    type Result = MemExecutor;

    #[inline]
    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        req.state().mem.clone()
    }
}
