pub const WIDTH: f32 = 440.0;
pub const HEIGHT: f32 = 520.0;
pub const CENTER: Point = Point { x: 220.0, y: 285.0 };
pub const ITEM_RADIUS: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    None,
    Search,
    Pin,
    Close,
    Previous,
    Next,
    Item(usize),
    Variant(usize),
    Drag,
}

pub fn item_position(index: usize) -> Point {
    let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::FRAC_PI_4;
    Point {
        x: CENTER.x + angle.cos() * 112.0,
        y: CENTER.y + angle.sin() * 112.0,
    }
}

pub fn variant_position(index: usize, count: usize) -> Point {
    // Stable outer arc above the main wheel, kept inside the popup work area.
    let angle =
        std::f32::consts::PI + (index as f32 + 0.5) * std::f32::consts::PI / count.max(1) as f32;
    Point {
        x: CENTER.x + angle.cos() * 180.0,
        y: CENTER.y + angle.sin() * 180.0,
    }
}

fn inside(point: Point, centre: Point, radius: f32) -> bool {
    (point.x - centre.x).powi(2) + (point.y - centre.y).powi(2) <= radius.powi(2)
}

pub fn hit_test(point: Point, items: usize, variants: usize) -> Hit {
    if (12.0..48.0).contains(&point.y) {
        if (340.0..380.0).contains(&point.x) {
            return Hit::Pin;
        }
        if (388.0..428.0).contains(&point.x) {
            return Hit::Close;
        }
        if (16.0..300.0).contains(&point.x) {
            return Hit::Drag;
        }
    }
    for index in 0..variants {
        if inside(point, variant_position(index, variants), 29.0) {
            return Hit::Variant(index);
        }
    }
    for index in 0..items.min(8) {
        if inside(point, item_position(index), ITEM_RADIUS) {
            return Hit::Item(index);
        }
    }
    if inside(point, CENTER, 47.0) {
        return Hit::Search;
    }
    if (440.0..478.0).contains(&point.y) {
        if (100.0..160.0).contains(&point.x) {
            return Hit::Previous;
        }
        if (280.0..340.0).contains(&point.x) {
            return Hit::Next;
        }
    }
    Hit::None
}

pub fn clamp_origin(x: i32, y: i32, width: i32, height: i32, work: [i32; 4]) -> (i32, i32) {
    let [left, top, right, bottom] = work;
    (
        x.clamp(left, (right - width).max(left)),
        y.clamp(top, (bottom - height).max(top)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_visible_target_is_reachable() {
        for i in 0..8 {
            assert_eq!(hit_test(item_position(i), 8, 0), Hit::Item(i));
        }
        for i in 0..6 {
            assert_eq!(hit_test(variant_position(i, 6), 8, 6), Hit::Variant(i));
        }
        assert_eq!(hit_test(CENTER, 8, 0), Hit::Search);
        assert_eq!(hit_test(item_position(7), 2, 0), Hit::None);
    }

    #[test]
    fn placement_handles_negative_monitors_and_small_work_areas() {
        assert_eq!(
            clamp_origin(-100, 1000, 440, 520, [-1920, 0, 0, 1080]),
            (-440, 560)
        );
        assert_eq!(clamp_origin(500, 500, 440, 520, [0, 0, 320, 240]), (0, 0));
    }
}
