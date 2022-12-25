use crate::layout::*;
use crate::parse::Font;
use crate::raster::ScaledGlyph;

/// Output of the method `uniform_layout`.
pub struct UniformLayout {
    pub glyphs: Vec<PositionedGlyph>,
    pub overflow: ImtOverflow,
}

/// Parameters used for the method `uniform_layout`.
pub struct UniformLayoutParams<'a> {
    pub font: &'a Font,
    pub size: f32,
    pub body: ImtBody,
    pub hori_behav: ImtHoriBehav,
    pub hori_align: ImtHoriAlign,
    pub vert_behav: ImtVertBehav,
    pub vert_align: ImtVertAlign,
    pub glyphs: &'a [ScaledGlyph],
    // TODO: blocks: &'a [ImtBlock],
}

/// Layout `ScaledGlyph`'s that are from the same `Font` and share size.
pub fn uniform_layout(_params: UniformLayoutParams) -> Vec<PositionedGlyph> {
    todo!()
}
