pub mod variation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImtUtilError {
    NoData,
    InvalidCoords,
    MissingTable,
    MalformedFont,
}
