use crate::parse::Outline;
use crate::raster::ScaledGlyph;

pub mod uniform;

/// Defines the behavior when text overflows horizonally
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImtHoriBehav {
    #[default]
    None,
    Shift,
    Regular,
}

/// Defines the behavior when text overflows vertically
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImtVertBehav {
    #[default]
    None,
    Shift,
}

/// Defines how text is aligned horizontally
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImtHoriAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Defines how text is aligned vertically
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImtVertAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

/// Defines the body which text is placed into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImtBody {
    pub x: i32,
    pub y: i32,
    /// When *zero* the body has *infinite* width.
    pub width: u32,
    /// When *zero* the body has *infinite* height.
    pub height: u32,
}

/// Defines an area within the body that text can not be placed into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImtBlock {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Defines the amount of overflow after a layout operation.
///
/// # Notes
/// - If `ImtBody.width` is *zero* `left` & `right` will be *zero`.
/// - If `ImtBody.height` is *zero* `top` & `bottom` will be *zero*.
/// - Negative overflow occurs when an axis's dimension was not fully utilized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImtOverflow {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

/// A glyph that has been positioned within an `ImtBody`.
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    /// The glyph x position
    pub x: i32,
    /// The glyphs y position
    pub y: i32,
    /// Width the image should be
    pub width: u32,
    /// Height the image should be
    pub height: u32,
    /// Outline point values will be between `0..=1` with `Y` down.
    pub outline: Option<Outline>,
    /// An unique ID derived from glyph_id, size, and axis coordinates.
    pub unique_id: u64,
}

impl PositionedGlyph {
    pub fn from_scaled(x: i32, y: i32, scaled: ScaledGlyph) -> Self {
        Self {
            x,
            y,
            width: scaled.width,
            height: scaled.height,
            outline: scaled.outline,
            unique_id: scaled.unique_id,
        }
    }
}
