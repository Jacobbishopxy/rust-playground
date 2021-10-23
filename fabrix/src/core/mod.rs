//! Fabrix core

pub mod dataframe;
pub mod row;
pub mod series;
pub mod util;
pub mod value;

pub use dataframe::*;
pub use row::*;
pub use series::*;
pub use value::*;

use crate::FabrixError;

/// a general naming for a default FDataFrame index
pub const IDX: &'static str = "index";

/// out of boundary error
pub(crate) fn oob_err(length: usize, len: usize) -> FabrixError {
    FabrixError::new_common_error(format!("length {:?} out of len {:?} boundary", length, len))
}

/// index not found error
pub(crate) fn inf_err<'a>(index: &Value) -> FabrixError {
    FabrixError::new_common_error(format!("index {:?} not found", index))
}

/// content empty error
pub(crate) fn cis_err(name: &str) -> FabrixError {
    FabrixError::new_common_error(format!("{:?} is empty", name))
}
