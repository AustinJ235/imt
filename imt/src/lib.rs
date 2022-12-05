pub mod error;
pub mod parse;
pub mod raster;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        parse::test();
    }
}
