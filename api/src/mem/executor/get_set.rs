use actix::prelude::*;
use actix_redis::{Command, RedisActor, RespValue};
use actix_web::{error, Error, FutureResponse};

use futures::future::{self, Future};

use super::mem_error;

pub fn get_json<'a, T>(
    redis: &Addr<RedisActor>,
    prefix: &str,
    key: &str,
) -> FutureResponse<Option<T>>
where
    T: serde::de::DeserializeOwned + 'static,
{
    let named_key = format!("{}#{}", prefix, key);
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
                        .map_err(|_| error::ErrorInternalServerError("Redis returned invalid utf8"))
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

pub fn set_json<T>(
    redis: &Addr<RedisActor>,
    prefix: &str,
    key: &str,
    value: &T,
    expires_in: &std::time::Duration,
) -> FutureResponse<()>
where
    T: serde::ser::Serialize,
{
    let value_str = match serde_json::to_string(value) {
        Ok(v) => v,
        Err(err) => return Box::new(future::err(mem_error("set_json error: serialization", err))),
    };
    let named_key = format!("{}#{}", prefix, key);
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
                        "Redis set_json: Unknown response from GET",
                        other,
                    )),
                },
                Err(err) => Err(mem_error("set_json redis error", err)),
            }),
    )
}

pub fn set_json_if_not_exists<T>(
    redis: &Addr<RedisActor>,
    prefix: &str,
    key: &str,
    value: &T,
    expires_in: &std::time::Duration,
) -> FutureResponse<bool>
where
    T: serde::ser::Serialize,
{
    let value_str = match serde_json::to_string(value) {
        Ok(v) => v,
        Err(err) => return Box::new(future::err(mem_error("set_json error: serialization", err))),
    };
    let named_key = format!("{}#{}", prefix, key);
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

pub fn delete(redis: &Addr<RedisActor>, prefix: &str, key: &str) -> FutureResponse<()> {
    let named_key = format!("{}#{}", prefix, key);
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
