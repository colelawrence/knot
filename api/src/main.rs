#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate redis_async;
#[macro_use]
extern crate serde_derive;

mod access;
mod db;
mod mem;

fn main() {
    println!("Hello, world!");
}
