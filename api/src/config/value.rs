use std::fmt;

#[derive(Clone)]
pub struct Value<T: Sized> {
    value: T,
    source: &'static str,
    sensitive: bool,
}

impl <T: Sized> Value<T> {
    pub fn default(value: T) -> Self {
        Value { value, source: "default", sensitive: false }
    }
    pub fn default_sensitive(value: T) -> Self {
        Value { value, source: "default", sensitive: true }
    }
    pub fn env(value: T) -> Self {
        Value { value, source: "env", sensitive: false }
    }
    pub fn dotenv(value: T) -> Self {
        Value { value, source: "dotenv", sensitive: false }
    }
    pub fn arg(value: T) -> Self {
        Value { value, source: "arg", sensitive: false }
    }
    pub fn env_sensitive(value: T) -> Self {
        Value { value, source: "env", sensitive: true }
    }
    pub fn dotenv_sensitive(value: T) -> Self {
        Value { value, source: "dotenv", sensitive: true }
    }
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }
}

impl <T: fmt::Display> fmt::Display for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.sensitive {
            write!(f, "{}", sensitive(&format!("{}", self.value)))
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl <T: fmt::Display> fmt::Debug for Value<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.sensitive {
            write!(f, "{} ({})", sensitive(&format!("{}", self.value)), self.source)
        } else {
            write!(f, "{} ({})", self.value, self.source)
        }
    }
}

fn sensitive(val: &str) -> &'static str {
    if val.len() == 0 {
        "<EMPTY>"
    } else {
        "<HIDDEN>"
    }
}

use std::ops::Deref;

impl Deref for Value<String> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.value
    }
}

impl Deref for Value<u16> {
    type Target = u16;
    fn deref(&self) -> &u16 {
        &self.value
    }
}
