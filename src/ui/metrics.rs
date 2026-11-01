//! Logical UI-space metrics: scale, size, mouse, camera and scissor clip.

use macroquad::prelude::*;

/// Pure scale function so the layout maths can be tested without a window.
fn scale_for_height(h: f32) -> f32 {
    (h / 900.0).clamp(0.8, 1.6)
}

/// Uniform UI scale: ~1.0 at 900p, clamped so tiny windows stay usable and
/// 4K screens get a proportionally larger interface instead of a speck.
pub(super) fn ui_scale() -> f32 {
    scale_for_height(screen_height())
}

/// Logical UI space, in which every layout below is written.
pub(super) fn ui_size() -> Vec2 {
    vec2(screen_width(), screen_height()) / ui_scale()
}

pub(super) fn ui_width() -> f32 {
    ui_size().x
}

pub(super) fn ui_height() -> f32 {
    ui_size().y
}

/// Mouse position in logical UI space.
pub fn ui_mouse() -> Vec2 {
    let m: Vec2 = mouse_position().into();
    m / ui_scale()
}

/// The UI camera, laid out in logical UI space over the given size.
///
/// macroquad 0.4 inverts Y again for cameras without a render target, so
/// `from_display_rect` alone draws the UI upside down. Flip it back to y-down.
pub(super) fn ui_camera(w: f32, h: f32) -> Camera2D {
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
    cam.zoom.y = -cam.zoom.y;
    cam
}

/// Restrict drawing to `r` while the research grid scrolls.
pub(super) fn clip(r: Rect) {
    let s = ui_scale();
    unsafe {
        get_internal_gl().quad_gl.scissor(Some((
            (r.x * s) as i32,
            (r.y * s) as i32,
            (r.w * s) as i32,
            (r.h * s) as i32,
        )));
    }
}

pub(super) fn unclip() {
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_scale_stays_readable() {
        assert!((scale_for_height(900.0) - 1.0).abs() < 1e-6);
        assert!((scale_for_height(768.0) - 768.0 / 900.0).abs() < 1e-6);
        assert_eq!(scale_for_height(600.0), 0.8);
        assert_eq!(scale_for_height(2160.0), 1.6);
    }

    #[test]
    fn ui_camera_is_y_down() {
        let cam = ui_camera(1000.0, 500.0);
        let m = cam.matrix();
        let origin = m.project_point3(vec3(0.0, 0.0, 0.0));
        let corner = m.project_point3(vec3(1000.0, 500.0, 0.0));
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(origin.x, -1.0) && close(origin.y, 1.0), "origin projected to {origin:?}");
        assert!(close(corner.x, 1.0) && close(corner.y, -1.0), "corner projected to {corner:?}");
    }
}
