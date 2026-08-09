pub mod curl_parser;
pub mod error;
pub mod model;
pub mod util;
pub mod variable;

pub use curl_parser::{parse_curl, CurlParsed};
pub use error::{not_found, openapi_error, validation, AppError};
pub use model::*;
pub use util::*;
pub use variable::{merge_variables, resolve_variables, resolve_variables_with, VariableMap};

pub type Result<T> = std::result::Result<T, AppError>;
