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
