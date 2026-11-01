//! Text rendering and measurement, kept sharp across UI scales.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::theme::TextSize;

use super::metrics::ui_scale;
use super::style::KEY;

/// Physical raster size and the draw-time scale that undoes `ui_scale()`.
///
/// Text is rasterised at the screen's pixel density (font size × `ui_scale`)
/// and then drawn at `font_scale = 1 / ui_scale`, so it stays sharp on 1440p
/// and 4K instead of being drawn small and scaled up. Measuring must use the
/// same pair so drawn and measured widths agree.
pub(super) fn raster_params(px: f32, scale: f32) -> (u16, f32) {
    ((px * scale).round() as u16, 1.0 / scale)
}

/// The font cut for a text size: titles and headings are Bold, the rest Medium.
fn font_for(a: &Assets, size: TextSize) -> &Font {
    if size.bold() { &a.font_bold } else { &a.font }
}

/// Full line metrics (ascent + descent) for a text size, measured from a fixed
/// reference glyph pair so every segment of one line shares the same baseline.
fn line_metrics(a: &Assets, size: TextSize) -> TextDimensions {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    measure_text("Hg", Some(font_for(a, size)), font_size, font_scale)
}

/// Draws one line of text. `y` is the top of the line box; the baseline comes
/// from a fixed reference string rather than the string's own tallest glyph, so
/// segments without ascenders ("zoom", "save", "pause") sit on the same line
/// as their neighbours instead of floating higher.
pub(super) fn text(a: &Assets, t: &str, x: f32, y: f32, size: TextSize, col: Color) {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    let font = font_for(a, size);
    let baseline = line_metrics(a, size).offset_y;
    draw_text_ex(t, x.round(), (y + baseline).round(), TextParams {
        font: Some(font),
        font_size,
        font_scale,
        color: col,
        ..Default::default()
    });
}

pub(super) fn text_w(a: &Assets, t: &str, size: TextSize) -> f32 {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    measure_text(t, Some(font_for(a, size)), font_size, font_scale).width
}

/// Text centred in `r`, using the string's width and the fixed line height so
/// vertical centring doesn't shift for strings without ascenders or descenders.
pub(super) fn text_in(a: &Assets, t: &str, r: Rect, size: TextSize, col: Color) {
    let (font_size, font_scale) = raster_params(size.px(), ui_scale());
    let font = font_for(a, size);
    let w = measure_text(t, Some(font), font_size, font_scale).width;
    let h = line_metrics(a, size).height;
    text(a, t, r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, size, col);
}

pub(super) fn text_right(a: &Assets, t: &str, right: f32, y: f32, size: TextSize, col: Color) {
    text(a, t, right - text_w(a, t, size), y, size, col);
}

/// Renders a help line with the key/action before each " · " in the deep-gold
/// key colour and the description in the base colour, preserving the original
/// column spacing.
pub(super) fn text_keys(a: &Assets, t: &str, x: f32, y: f32, size: TextSize, base: Color) {
    let key_col = KEY;
    let mut cx = x;
    let mut rest = t;
    let span = |s: &str, cx: &mut f32, col: Color| {
        text(a, s, *cx, y, size, col);
        *cx += text_w(a, s, size);
    };
    while !rest.is_empty() {
        let (seg, tail) = match rest.find("  ") {
            Some(i) => rest.split_at(i),
            None => (rest, ""),
        };
        if let Some((key, after)) = seg.split_once(" · ") {
            span(key, &mut cx, key_col);
            span(" · ", &mut cx, base);
            span(after, &mut cx, base);
        } else {
            span(seg, &mut cx, base);
        }
        let n = tail.chars().take_while(|c| *c == ' ').count();
        if n > 0 {
            let spaces = &tail[..n];
            span(spaces, &mut cx, base);
            rest = &tail[n..];
        } else {
            rest = tail;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_params_render_at_physical_size() {
        assert_eq!(raster_params(17.0, 1.6), (27, 0.625));
        assert_eq!(raster_params(17.0, 1.0), (17, 1.0));
        assert_eq!(raster_params(13.0, 0.8).0, 10);
    }
}
