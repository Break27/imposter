macro_rules! impl_error {
    (pub enum $error:ident {
        $v1:ident($i1:literal), $($variant:ident($inner:path),)* }) =>
    {
        #[derive(Debug)]
        pub enum $error {
            $v1(String),
            $($variant($inner),)*
        }
        $(
            impl From<$inner> for $error {
                fn from(value: $inner) -> Self {
                    Self::$variant(value)
                }
            }
        )*
        impl std::fmt::Display for $error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $error::$v1(x) => write!(f, $i1, x),
                    $($error::$variant(e) => e.fmt(f),)*
                }
            }
        }
    };
}

impl_error! {
    pub enum Error {
        BadRequest("Missing part '{}'"),
        Io(std::io::Error),
        Parse(httparse::Error),
        Utf8(std::str::Utf8Error),
    }
}

pub type Result<T> = std::result::Result<T, Error>;
impl std::error::Error for Error {}

impl_error! {
    pub enum BuildError {
        Unsupported("Unsupported proxy protocol '{}'"),
        Io(std::io::Error),
        Client(ureq::Error),
        Tls(native_tls::Error),
        Decode(base64::DecodeError),
        Utf8(std::string::FromUtf8Error),
    }
}

pub type BuildResult<T> = std::result::Result<T, BuildError>;
impl std::error::Error for BuildError {}
