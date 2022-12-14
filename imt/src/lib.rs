pub mod error;
pub mod parse;
pub mod raster;
pub mod util;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        parse::test();
    }
}
