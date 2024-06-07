use crate::error::Error;

macro_rules! impl_http {
    (pub struct $name:ident($inner:ident) { $($variant:ident = $value:literal,)* }) => {
        #[derive(PartialEq)]
        pub struct $name($inner);

        #[derive(PartialEq)]
        #[allow(non_camel_case_types)]
        enum $inner{ $($variant,)* }

        impl $name {
            $(
                pub const $variant: Self = Self($inner::$variant);
            )*
        }
        impl ToString for $name {
            fn to_string(&self) -> String {
                match self.0 {
                    $($inner::$variant => $value.to_string(),)*
                }
            }
        }
        impl std::str::FromStr for $name {
            type Err = ();
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($value => Ok(Self::$variant),)*
                    _ => Err(())
                }
            }
        }
    };
}

impl_http! {
    pub struct Method(InnerMethod) {
        OPTIONS = "OPTIONS",
        GET     = "GET",
        POST    = "POST",
        PUT     = "PUT",
        DELETE  = "DELETE",
        HEAD    = "HEAD",
        TRACE   = "TRACE",
        CONNECT = "CONNECT",
        PATCH   = "PATCH",
    }
}

impl_http! {
    pub struct Version(InnerVersion) {
        HTTP_09 = "HTTP/0.9",
        HTTP_10 = "HTTP/1.0",
        HTTP_11 = "HTTP/1.1",
        HTTP_2  = "HTTP/2.0",
        HTTP_3  = "HTTP/3.0",
    }
}

pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub(crate) host: String,
    pub(crate) payload: Box<[u8]>,
}

impl Request {
    pub fn as_bytes(&self) -> &[u8] {
        &self.payload
    }

    pub fn host(&self) -> &str {
        &self.host
    }
}

pub struct Response {
    pub version: Version,
    pub status: u16,
    pub message: String
}

impl Response {
    pub fn new<T>(version: Version, status: u16, message: T) -> Self
    where
        T: ToString
    {
        Self { version, status, message: message.to_string() }
    }

    pub fn make<T>(status: u16, message: T) -> Self
    where
        T: ToString
    {
        Self { version: Version::HTTP_11, status, message: message.to_string() }
    }

    pub fn from_err(err: Error) -> Self {
        use async_std::io::ErrorKind::TimedOut;
        match err {
            Error::BadRequest(x) => 
                Self::make(400, x),
            Error::Io(e) if e.kind() == TimedOut =>
                Self::make(408, "Timeout"),
            Error::Io(_) =>
                Self::make(503, "Unavailable"),
            Error::Parse(_) =>
                Self::make(500, "Internal Server Error"),
            Error::Utf8(_) =>
                Self::make(500, "Internal Server Error"),
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::make(200, "OK")
    }
}

impl ToString for Response {
    fn to_string(&self) -> String {
        format!("{} {} {}\r\n\r\n", 
            self.version.to_string(), self.status, self.message)
    }
}
