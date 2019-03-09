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
      .send(Command(resp_array!["GET", key]))
      .map_err(Error::from)
      .and_then(|res| match res {
        Ok(val) => match val {
          RespValue::Error(err) => Err(mem_error("get_json error", err)),
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
) -> FutureResponse<()>
where
  T: serde::ser::Serialize,
{
  let value_str = match serde_json::to_string(value) {
    Ok(v) => v,
    Err(err) => return Box::new(future::err(mem_error("set_json error: serialization", err))),
  };
  let named_key = format!("{}#{}", prefix, key);
  Box::new(
    redis
      .send(Command(resp_array!["SET", named_key, value_str]))
      .map_err(Error::from)
      .and_then(|res| match res {
        Ok(val) => match val {
          RespValue::Error(err) => Err(mem_error("set_json error", err)),
          RespValue::SimpleString(s) => Ok(()),
          RespValue::BulkString(s) => Ok(()),
          other => Err(mem_error(
            "Redis set_json: Unknown response from GET",
            other,
          )),
        },
        Err(err) => Err(mem_error("set_json redis error", err)),
      }),
  )
}
