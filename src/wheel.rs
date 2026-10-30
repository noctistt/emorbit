pub const WIDTH: f32 = 440.0;
pub const HEIGHT: f32 = 520.0;
pub const CENTER: Point = Point { x: 220.0, y: 285.0 };
pub const SLOTS: usize = crate::PAGE_SIZE;
pub const ITEM_RADIUS: f32 = 23.0;
pub const SEARCH_CENTER: Point = Point { x: 220.0, y: 279.0 };
pub const SEARCH_RADIUS: f32 = 30.0;
pub const FAVORITES_CENTER: Point = Point { x: 190.0, y: 330.0 };
pub const ADD_CENTER: Point = Point { x: 220.0, y: 334.0 };
pub const PIN_CENTER: Point = Point { x: 250.0, y: 330.0 };
pub const ORBIT_RADIUS: f32 = 100.0;
pub const VARIANT_RADIUS: f32 = 22.0;

#[derive(Clone, Copy, Debug)]
pub struct ContextLayout {
    pub anchor: usize,
    pub count: usize,
    pub progress: f32,
}

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
    Favorites,
    AddFavorite,
    Close,
    Previous,
    Next,
    Item(usize),
    Variant(usize),
    Drag,
}

/// Resolve a press and release without turning a drag across toolbar buttons
/// into an accidental close, pin, or search action. Moving from the centre or
/// an emoji onto another emoji is a selection gesture; returning to centre
/// cancels it. A stationary centre click still opens search.
pub fn release_target(pressed: Hit, released: Hit) -> Option<Hit> {
    match (pressed, released) {
        (
            Hit::Search | Hit::Item(_) | Hit::Variant(_) | Hit::None,
            Hit::Item(_) | Hit::Variant(_),
        ) => Some(released),
        (Hit::Search, Hit::Search)
        | (Hit::Pin, Hit::Pin)
        | (Hit::Favorites, Hit::Favorites)
        | (Hit::AddFavorite, Hit::AddFavorite)
        | (Hit::Close, Hit::Close)
        | (Hit::Previous, Hit::Previous)
        | (Hit::Next, Hit::Next) => Some(released),
        _ => None,
    }
}

pub fn item_position(index: usize) -> Point {
    let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::TAU / SLOTS as f32;
    Point {
        x: CENTER.x + angle.cos() * ORBIT_RADIUS,
        y: CENTER.y + angle.sin() * ORBIT_RADIUS,
    }
}

pub fn variant_position(layout: ContextLayout, index: usize) -> Point {
    // A local outward fan follows the parent around the wheel. The full layout
    // fits within the existing popup, including bottom and side parents.
    let parent = item_position(layout.anchor);
    let direction = (parent.y - CENTER.y).atan2(parent.x - CENTER.x);
    let spread = 160.0_f32.to_radians();
    let offset = if layout.count <= 1 {
        0.0
    } else {
        (index as f32 / (layout.count - 1) as f32 - 0.5) * spread
    };
    let angle = direction + offset;
    let distance = 72.0 + 14.0 * layout.progress.clamp(0.0, 1.0);
    Point {
        x: parent.x + angle.cos() * distance,
        y: parent.y + angle.sin() * distance,
    }
}

fn inside(point: Point, centre: Point, radius: f32) -> bool {
    (point.x - centre.x).powi(2) + (point.y - centre.y).powi(2) <= radius.powi(2)
}

pub fn hit_test(point: Point, items: usize, context: Option<ContextLayout>) -> Hit {
    // Fixed hub controls stay reachable while any outward fan is expanded.
    if (226.0..246.0).contains(&point.y) {
        if (246.0..274.0).contains(&point.x) {
            return Hit::Close;
        }
        if (166.0..246.0).contains(&point.x) {
            return Hit::Drag;
        }
    }
    if inside(point, FAVORITES_CENTER, 14.0) {
        return Hit::Favorites;
    }
    if inside(point, ADD_CENTER, 12.0) {
        return Hit::AddFavorite;
    }
    if inside(point, PIN_CENTER, 14.0) {
        return Hit::Pin;
    }
    if inside(point, SEARCH_CENTER, SEARCH_RADIUS) {
        return Hit::Search;
    }
    if let Some(layout) = context {
        // Hit regions follow the same expanding positions as rendering.
        if layout.progress > 0.05 {
            let radius = VARIANT_RADIUS * (0.75 + 0.25 * layout.progress) + 2.0;
            for index in 0..layout.count {
                if inside(point, variant_position(layout, index), radius) {
                    return Hit::Variant(index);
                }
            }
        }
    }
    for index in 0..items.min(SLOTS) {
        if inside(point, item_position(index), ITEM_RADIUS + 2.0) {
            return Hit::Item(index);
        }
    }
    // Side arrows yield to suggestions that can occupy their space.
    if context.is_none() {
        if inside(
            point,
            Point {
                x: 20.0,
                y: CENTER.y,
            },
            18.0,
        ) {
            return Hit::Previous;
        }
        if inside(
            point,
            Point {
                x: WIDTH - 20.0,
                y: CENTER.y,
            },
            18.0,
        ) {
            return Hit::Next;
        }
    }
    Hit::None
}

/// One turn per outward excursion. Return to the central band to rearm.
pub struct EdgePager {
    armed: bool,
}
impl Default for EdgePager {
    fn default() -> Self {
        Self { armed: true }
    }
}
impl EdgePager {
    pub fn update(&mut self, point: Point) -> Option<i32> {
        if (100.0..340.0).contains(&point.x) {
            self.armed = true;
        }
        if !self.armed || (point.y - CENTER.y).abs() > 160.0 {
            return None;
        }
        let direction = if point.x <= 16.0 {
            -1
        } else if point.x >= WIDTH - 16.0 {
            1
        } else {
            return None;
        };
        self.armed = false;
        Some(direction)
    }
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
    fn mouse_drag_selects_the_release_target_including_variants() {
        assert_eq!(
            release_target(Hit::Search, Hit::Item(2)),
            Some(Hit::Item(2))
        );
        assert_eq!(
            release_target(Hit::Item(2), Hit::Variant(4)),
            Some(Hit::Variant(4))
        );
        assert_eq!(
            release_target(Hit::Item(0), Hit::Item(7)),
            Some(Hit::Item(7))
        );
        assert_eq!(
            release_target(Hit::Item(0), Hit::Item(0)),
            Some(Hit::Item(0))
        );
    }

    #[test]
    fn dragging_back_to_centre_or_outside_cancels_without_toolbar_actions() {
        for origin in [Hit::Item(0), Hit::Variant(1), Hit::Search] {
            assert_eq!(release_target(origin, Hit::None), None);
            assert_eq!(release_target(origin, Hit::Close), None);
            assert_eq!(release_target(origin, Hit::Pin), None);
        }
        assert_eq!(release_target(Hit::Item(0), Hit::Search), None);
        assert_eq!(release_target(Hit::Pin, Hit::Item(0)), None);
        assert_eq!(release_target(Hit::Search, Hit::Search), Some(Hit::Search));
        assert_eq!(release_target(Hit::Pin, Hit::Pin), Some(Hit::Pin));
    }

    #[test]
    fn every_visible_target_is_reachable() {
        for i in 0..SLOTS {
            assert_eq!(hit_test(item_position(i), SLOTS, None), Hit::Item(i));
        }
        for anchor in 0..SLOTS {
            let layout = ContextLayout {
                anchor,
                count: 6,
                progress: 1.0,
            };
            for i in 0..6 {
                assert_eq!(
                    hit_test(variant_position(layout, i), SLOTS, Some(layout)),
                    Hit::Variant(i)
                );
            }
        }
        assert_eq!(hit_test(CENTER, SLOTS, None), Hit::Search);
        assert_eq!(hit_test(item_position(7), 2, None), Hit::None);
    }

    #[test]
    fn fans_are_local_outward_and_inside_popup_for_every_parent() {
        for anchor in 0..SLOTS {
            let parent = item_position(anchor);
            for progress in [0.0, 0.25, 0.5, 1.0] {
                let layout = ContextLayout {
                    anchor,
                    count: 6,
                    progress,
                };
                let points: Vec<_> = (0..6).map(|i| variant_position(layout, i)).collect();
                for p in &points {
                    assert!((p.x - parent.x).hypot(p.y - parent.y) <= 86.01);
                    assert!(
                        p.x - VARIANT_RADIUS - 2.0 >= 0.0 && p.x + VARIANT_RADIUS + 2.0 <= WIDTH
                    );
                    assert!(
                        p.y - VARIANT_RADIUS - 2.0 >= 72.0 && p.y + VARIANT_RADIUS + 2.0 <= HEIGHT
                    );
                    assert!(
                        (p.x - parent.x) * (parent.x - CENTER.x)
                            + (p.y - parent.y) * (parent.y - CENTER.y)
                            > 0.0
                    );
                }
                let diameter = 2.0 * VARIANT_RADIUS * (0.75 + 0.25 * progress);
                for pair in points.windows(2) {
                    assert!((pair[0].x - pair[1].x).hypot(pair[0].y - pair[1].y) > diameter);
                }
            }
        }
    }

    #[test]
    fn animated_targets_match_their_visible_positions_and_hide_paging() {
        for progress in [0.1, 0.4, 0.8, 1.0] {
            let layout = ContextLayout {
                anchor: 4,
                count: 6,
                progress,
            };
            for index in 0..6 {
                assert_eq!(
                    hit_test(variant_position(layout, index), SLOTS, Some(layout)),
                    Hit::Variant(index)
                );
            }
            assert_ne!(
                hit_test(Point { x: 130.0, y: 460.0 }, SLOTS, Some(layout)),
                Hit::Previous
            );
        }
    }

    #[test]
    fn edge_paging_turns_once_then_requires_a_return_and_side_arrows_are_clickable() {
        let mut pager = EdgePager::default();
        assert_eq!(
            pager.update(Point {
                x: 450.0,
                y: CENTER.y
            }),
            Some(1)
        );
        assert_eq!(
            pager.update(Point {
                x: 500.0,
                y: CENTER.y
            }),
            None
        );
        assert_eq!(pager.update(CENTER), None);
        assert_eq!(
            pager.update(Point {
                x: -20.0,
                y: CENTER.y
            }),
            Some(-1)
        );
        assert_eq!(
            pager.update(Point {
                x: -50.0,
                y: CENTER.y
            }),
            None
        );
        assert_eq!(
            hit_test(
                Point {
                    x: 20.0,
                    y: CENTER.y
                },
                SLOTS,
                None
            ),
            Hit::Previous
        );
        assert_eq!(
            hit_test(
                Point {
                    x: WIDTH - 20.0,
                    y: CENTER.y
                },
                SLOTS,
                None
            ),
            Hit::Next
        );
        assert_eq!(release_target(Hit::Item(0), Hit::AddFavorite), None);
        assert_eq!(
            release_target(Hit::Favorites, Hit::Favorites),
            Some(Hit::Favorites)
        );
    }

    #[test]
    fn dense_ring_and_hub_controls_stay_reachable_with_every_fan() {
        for anchor in 0..SLOTS {
            let layout = ContextLayout {
                anchor,
                count: 6,
                progress: 1.0,
            };
            for (point, hit) in [
                (FAVORITES_CENTER, Hit::Favorites),
                (ADD_CENTER, Hit::AddFavorite),
                (PIN_CENTER, Hit::Pin),
                (SEARCH_CENTER, Hit::Search),
                (Point { x: 260.0, y: 236.0 }, Hit::Close),
                (Point { x: 210.0, y: 236.0 }, Hit::Drag),
            ] {
                assert_eq!(hit_test(point, SLOTS, Some(layout)), hit);
            }
            let a = item_position(anchor);
            let b = item_position((anchor + 1) % SLOTS);
            assert!((a.x - b.x).hypot(a.y - b.y) > 2.0 * (ITEM_RADIUS + 2.0));
        }
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
