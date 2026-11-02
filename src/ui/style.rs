//! Shared layout constants and colour tokens, expressed once for the whole UI.

use macroquad::prelude::Color;

use crate::theme;

pub(super) const SLOT: f32 = 48.0;
pub(super) const GAP: f32 = 6.0;
pub(super) const QUICK_SLOT: f32 = 54.0;
pub(super) const MINIMAP: f32 = 190.0;
pub(super) const LINE: f32 = 21.0;
/// Top of a titled window's content: the header band (`HEADER_BAND` tall, 9 px
/// down) plus breathing room, so the first row never touches the band.
pub(super) const HEADER: f32 = 9.0 + HEADER_BAND + 16.0;
/// Height of the moss-green title band across a window's top.
pub(super) const HEADER_BAND: f32 = 42.0;

// Tokens, expressed once for the whole UI.
pub(super) const BG: Color = theme::color(theme::BG);
pub(super) const BG2: Color = theme::color(theme::BG2);
pub(super) const EDGE: Color = theme::color(theme::EDGE);
pub(super) const TEXT: Color = theme::color(theme::TEXT);
pub(super) const TEXT_DIM: Color = theme::color(theme::TEXT_DIM);
pub(super) const TITLE: Color = theme::color(theme::TITLE);
pub(super) const KEY: Color = theme::color(theme::KEY);
pub(super) const SLOT_WELL: Color = theme::color(theme::SLOT_WELL);
pub(super) const HONEY: Color = theme::color(theme::HONEY);
// Rust accent for warnings: darker than the light `PEACH` so it reads on paper.
pub(super) const PEACH: Color = theme::color(theme::TERRA_600);
pub(super) const TERRA: Color = theme::color(theme::TERRA);
pub(super) const TEAL: Color = theme::color(theme::TEAL);
// Darker verdigris for research unlock labels, readable on the cream card.
pub(super) const VERDIGRIS: Color = theme::color(theme::VERDIGRIS_700);
pub(super) const SHADE: Color = theme::color(theme::SHADE);
