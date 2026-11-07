//! The research panel: a scrollable grid of tech cards.

use macroquad::prelude::*;

use crate::items::*;
use crate::render::*;
use crate::theme::{self, TextSize, line, radius, space};
use crate::{Game, Panel as OpenPanel};

use super::super::metrics::{clip, ui_height, ui_width, unclip};
use super::super::primitives::{Elevation, clicked, hovered, rrect, rrect_lines, surface};
use super::super::style::{EDGE, GAP, HEADER, HONEY, PEACH, TEXT, TEXT_DIM, TITLE, VERDIGRIS};
use super::super::text::{text, text_right};
use super::super::widgets::{Panel, bar};
use super::{centered, slide_px};

#[derive(Clone, Copy, Debug)]
struct ResearchLayout {
    width: f32,
    height: f32,
    cols: usize,
    card_w: f32,
    card_h: f32,
    grid_h: f32,
    view_h: f32,
    max_scroll: f32,
}

/// Research panel geometry for a logical screen size. Columns come from the
/// available width, and any overflow becomes a scroll range so no card is lost.
fn research_layout(sw: f32, sh: f32) -> ResearchLayout {
    let card_h = 106.0;
    let min_card_w = 300.0;
    let width = (sw - 2.0 * space::L).min(1100.0);
    let inner_w = width - 2.0 * space::XL;
    let cols = (((inner_w + GAP) / (min_card_w + GAP)).floor() as usize).max(1);
    let rows = TECHS.len().div_ceil(cols);
    let grid_h = rows as f32 * (card_h + GAP) - GAP;
    let want_h = HEADER + grid_h + 2.0 * space::L;
    let height = want_h.min(sh - 2.0 * space::L);
    // The view is exactly the panel's content area, so when the panel is tall
    // enough for the whole grid, `max_scroll` is 0 and there is no dead strip.
    let view_h = (height - HEADER - space::L).max(1.0);
    let card_w = (inner_w - (cols as f32 - 1.0) * GAP) / cols as f32;
    ResearchLayout {
        width,
        height,
        cols,
        card_w,
        card_h,
        grid_h,
        view_h,
        max_scroll: (grid_h - view_h).max(0.0),
    }
}

pub(crate) fn research_panel(g: &mut Game) {
    let a = g.assets.clone();
    let layout = research_layout(ui_width(), ui_height());
    let (cols, card_w, card_h, grid_h, max_scroll) =
        (layout.cols, layout.card_w, layout.card_h, layout.grid_h, layout.max_scroll);
    let r = centered(layout.width, layout.height);
    let r = Rect::new(r.x, r.y - slide_px(g), r.w, r.h);
    let (content, closed) = Panel::new(r)
        .title("Research")
        .subtitle("Labs slowly use up the data vials shown · pick something to study")
        .closable()
        .show(g);
    if closed {
        g.panel = OpenPanel::None;
        return;
    }

    let view = Rect::new(content.x, content.y, content.w, layout.view_h);
    if hovered(view) && max_scroll > 0.0 {
        g.research_scroll = (g.research_scroll - mouse_wheel().1 * 44.0).clamp(0.0, max_scroll);
    } else {
        g.research_scroll = g.research_scroll.clamp(0.0, max_scroll);
    }
    let scroll = g.research_scroll;

    clip(view);
    for (k, tech) in TECHS.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let cr = Rect::new(
            content.x + col as f32 * (card_w + GAP),
            content.y + row as f32 * (card_h + GAP) - scroll,
            card_w,
            card_h,
        );
        if cr.bottom() < view.y - 100.0 || cr.y > view.bottom() + 100.0 {
            continue;
        }
        g.clickables.push(cr);
        let done = g.world.research.done[k];
        let current = g.world.research.current == Some(k);
        let available = g.world.tech_available(k);
        let hot = available && hovered(cr);
        let tint = if done {
            Some(theme::alpha(theme::SAGE_700, 0.22))
        } else if current {
            Some(theme::alpha(theme::LACQUER_500, 0.16))
        } else if hot {
            Some(theme::alpha(theme::WHITE, 0.5))
        } else {
            None
        };
        surface(cr, Elevation::Card);
        if let Some(t) = tint {
            rrect(cr, radius::MD, t);
        }
        rrect_lines(
            cr,
            radius::MD,
            if current { line::STRONG } else { line::BASE },
            EDGE,
        );
        let name_col = if done {
            theme::color(theme::SAGE_700)
        } else if current {
            TITLE
        } else if available {
            TEXT
        } else {
            TEXT_DIM
        };
        text(&a, tech.name, cr.x + space::L, cr.y + space::M, TextSize::L, name_col);
        if done {
            text_right(&a, "Done", cr.right() - space::L, cr.y + space::M + 2.0, TextSize::S, theme::color(theme::SAGE_700));
        }
        let mut x = cr.x + space::L;
        for &s in tech.science {
            draw_item(&a, vec2(x + 10.0, cr.y + 44.0), 22.0, s);
            x += 22.0;
        }
        text(&a, &format!("x{} · {}s each", tech.units, tech.time), x + space::S, cr.y + 40.0, TextSize::S, TEXT_DIM);
        let unlocks = match tech.bonus {
            Some(Bonus::ArmSpeed) => "Arms move 50% faster".to_string(),
            Some(Bonus::DrillSpeed) => "Drills mine 50% faster".to_string(),
            None => tech.recipes.join(", "),
        };
        text(&a, &unlocks, cr.x + space::L, cr.y + 70.0, TextSize::S, VERDIGRIS);
        if !done && !available {
            let req: Vec<&str> = tech.prereqs.iter().map(|&p| TECHS[p].name).collect();
            text(&a, &format!("Needs {}", req.join(", ")), cr.x + space::L, cr.y + 90.0, TextSize::Xs, PEACH);
        }
        if current {
            bar(
                Rect::new(cr.x + space::L, cr.y + 88.0, cr.w - 2.0 * space::L, 8.0),
                g.world.research.progress as f32 / tech.units as f32,
                HONEY,
            );
        }
        if available && clicked(cr) {
            g.world.set_research(k);
            g.toast(format!("Researching {}", tech.name));
        }
    }
    unclip();

    if max_scroll > 0.0 {
        let track = Rect::new(view.right() + 3.0, view.y, 3.0, view.h);
        rrect(track, 1.5, theme::alpha(theme::EDGE, 0.5));
        let frac = view.h / grid_h;
        let thumb_h = (view.h * frac).max(24.0);
        let thumb_y = view.y + (view.h - thumb_h) * (scroll / max_scroll);
        rrect(Rect::new(track.x, thumb_y, track.w, thumb_h), 1.5, theme::alpha(theme::HONEY, 0.8));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_grid_never_loses_a_card() {
        for (w, h) in [
            (1440.0, 900.0),
            (1600.0, 900.0),
            (1920.0, 1080.0),
            (1280.0, 750.0),
            (1000.0, 750.0),
            (700.0, 750.0),
        ] {
            let l = research_layout(w, h);
            assert!(l.width <= w - 2.0 * space::L + 0.01);
            assert!(l.height <= h - 2.0 * space::L + 0.01);
            assert!(l.cols >= 1);
            assert!(l.cols * TECHS.len().div_ceil(l.cols) >= TECHS.len());
            assert!(
                l.grid_h - l.max_scroll <= l.view_h + 0.01,
                "{w}x{h}: grid {} scroll {} view {}",
                l.grid_h,
                l.max_scroll,
                l.view_h
            );
        }
    }

    #[test]
    fn research_grid_fits_without_scrolling_when_there_is_room() {
        // A tall screen holds the whole grid: no scroll, and the view is big
        // enough that the last row is fully visible (no dead strip).
        let l = research_layout(1600.0, 1200.0);
        assert_eq!(l.max_scroll, 0.0, "tall screen should not scroll, got {}", l.max_scroll);
        assert!(l.grid_h <= l.view_h + 0.01, "grid clipped in a tall screen");
    }

    #[test]
    fn research_grid_scrolls_to_every_card() {
        // A short screen clips the grid; scrolling to the maximum must bring the
        // last row's bottom to the bottom of the view.
        let l = research_layout(1000.0, 500.0);
        assert!(l.max_scroll > 0.0, "clipped layout must scroll");
        assert!(
            l.grid_h - l.max_scroll <= l.view_h + 0.01,
            "max scroll {} leaves last row off-screen (grid {} view {})",
            l.max_scroll,
            l.grid_h,
            l.view_h
        );
    }
}
