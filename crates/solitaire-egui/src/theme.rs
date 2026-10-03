use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2,
};
use solitaire_core::{Card, Color, Rank, Suit};

pub const FELT: Color32 = Color32::from_rgb(16, 65, 55);
pub const CHROME: Color32 = Color32::from_rgb(17, 42, 37);
pub const CREAM: Color32 = Color32::from_rgb(255, 250, 239);
pub const GOLD: Color32 = Color32::from_rgb(225, 196, 126);
pub const MUTED: Color32 = Color32::from_rgb(151, 183, 167);
pub const LEGAL: Color32 = Color32::from_rgb(134, 229, 174);

pub fn setup(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = CHROME;
    visuals.window_fill = CHROME;
    visuals.override_text_color = Some(CREAM);
    visuals.selection.bg_fill = Color32::from_rgb(54, 97, 76);
    visuals.selection.stroke = Stroke::new(1.0, GOLD);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 68, 56);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 91, 72);
    visuals.widgets.active.bg_fill = Color32::from_rgb(73, 108, 77);
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 9.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}

pub fn suit(painter: &Painter, center: Pos2, size: f32, suit: Suit, color: Color32) {
    suit_oriented(painter, center, size, suit, color, false);
}

fn suit_oriented(
    painter: &Painter,
    center: Pos2,
    size: f32,
    suit: Suit,
    color: Color32,
    inverted: bool,
) {
    let direction = if inverted { -1.0 } else { 1.0 };
    let p = |x: f32, y: f32| center + Vec2::new(x * size * direction, y * size * direction);
    let polygon =
        |points: Vec<Pos2>| painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    match suit {
        Suit::Diamonds => {
            polygon(vec![
                p(0.0, -0.52),
                p(0.36, 0.0),
                p(0.0, 0.52),
                p(-0.36, 0.0),
            ]);
        }
        Suit::Hearts => {
            painter.circle_filled(p(-0.21, -0.18), size * 0.27, color);
            painter.circle_filled(p(0.21, -0.18), size * 0.27, color);
            polygon(vec![p(-0.45, -0.08), p(0.45, -0.08), p(0.0, 0.52)]);
        }
        Suit::Clubs => {
            for (x, y) in [(0.0, -0.25), (-0.23, 0.06), (0.23, 0.06)] {
                painter.circle_filled(p(x, y), size * 0.25, color);
            }
            polygon(vec![p(-0.18, 0.48), p(0.18, 0.48), p(0.0, 0.05)]);
        }
        Suit::Spades => {
            polygon(vec![p(0.0, -0.52), p(0.44, 0.05), p(-0.44, 0.05)]);
            painter.circle_filled(p(-0.2, 0.06), size * 0.24, color);
            painter.circle_filled(p(0.2, 0.06), size * 0.24, color);
            polygon(vec![p(-0.18, 0.49), p(0.18, 0.49), p(0.0, 0.08)]);
        }
    }
}

pub fn card(painter: &Painter, rect: Rect, card: Option<Card>, selected: bool) {
    let scale = rect.width() / 104.0;
    painter.rect_filled(
        rect.translate(Vec2::new(1.5, 3.0)),
        8.0,
        Color32::from_black_alpha(55),
    );
    let border = if selected {
        Stroke::new(3.0, GOLD)
    } else {
        Stroke::new(1.0, Color32::from_rgb(221, 218, 201))
    };
    painter.rect(rect, 8.0, CREAM, border, StrokeKind::Inside);
    if let Some(card) = card {
        let color = match card.color() {
            Color::Red => Color32::from_rgb(183, 48, 59),
            Color::Black => Color32::from_rgb(30, 49, 46),
        };
        painter.text(
            rect.min + Vec2::new(10.0, 5.0) * scale,
            Align2::LEFT_TOP,
            card.rank.label(),
            FontId::proportional(23.0 * scale),
            color,
        );
        match card.rank {
            Rank::Ace => suit(painter, rect.center(), 40.0 * scale, card.suit, color),
            Rank::Jack | Rank::Queen | Rank::King => court(painter, rect, card, color),
            _ => {
                for &(x, y) in pip_positions(card.rank) {
                    let center = rect.min + Vec2::new(x * rect.width(), y * rect.height());
                    suit_oriented(painter, center, 20.0 * scale, card.suit, color, y > 0.5);
                }
            }
        }
        painter.text(
            rect.max - Vec2::new(10.0, 7.0) * scale,
            Align2::RIGHT_BOTTOM,
            card.rank.label(),
            FontId::proportional(19.0 * scale),
            color,
        );
    } else {
        let inner = rect.shrink(5.0 * scale);
        painter.rect_filled(inner, 5.0, Color32::from_rgb(34, 75, 88));
        painter.rect_stroke(
            inner.shrink(4.0 * scale),
            3.0,
            Stroke::new(1.0, Color32::from_rgb(118, 158, 162)),
            StrokeKind::Inside,
        );
        for row in 0..6 {
            for column in 0..4 {
                let center = inner.min
                    + Vec2::new(16.0 + column as f32 * 21.0, 16.0 + row as f32 * 20.0) * scale;
                suit(
                    painter,
                    center,
                    5.0 * scale,
                    Suit::Diamonds,
                    Color32::from_rgb(96, 133, 140),
                );
            }
        }
    }
}

fn pip_positions(rank: Rank) -> &'static [(f32, f32)] {
    match rank {
        Rank::Two => &[(0.5, 0.29), (0.5, 0.73)],
        Rank::Three => &[(0.5, 0.29), (0.5, 0.51), (0.5, 0.73)],
        Rank::Four => &[(0.27, 0.29), (0.73, 0.29), (0.27, 0.73), (0.73, 0.73)],
        Rank::Five => &[
            (0.27, 0.29),
            (0.73, 0.29),
            (0.5, 0.51),
            (0.27, 0.73),
            (0.73, 0.73),
        ],
        Rank::Six => &[
            (0.27, 0.29),
            (0.73, 0.29),
            (0.27, 0.51),
            (0.73, 0.51),
            (0.27, 0.73),
            (0.73, 0.73),
        ],
        Rank::Seven => &[
            (0.27, 0.29),
            (0.73, 0.29),
            (0.5, 0.4),
            (0.27, 0.51),
            (0.73, 0.51),
            (0.27, 0.73),
            (0.73, 0.73),
        ],
        Rank::Eight => &[
            (0.27, 0.29),
            (0.73, 0.29),
            (0.5, 0.4),
            (0.27, 0.51),
            (0.73, 0.51),
            (0.5, 0.62),
            (0.27, 0.73),
            (0.73, 0.73),
        ],
        Rank::Nine => &[
            (0.27, 0.27),
            (0.73, 0.27),
            (0.27, 0.43),
            (0.73, 0.43),
            (0.5, 0.51),
            (0.27, 0.59),
            (0.73, 0.59),
            (0.27, 0.75),
            (0.73, 0.75),
        ],
        Rank::Ten => &[
            (0.27, 0.27),
            (0.73, 0.27),
            (0.5, 0.35),
            (0.27, 0.43),
            (0.73, 0.43),
            (0.27, 0.59),
            (0.73, 0.59),
            (0.5, 0.67),
            (0.27, 0.75),
            (0.73, 0.75),
        ],
        _ => &[],
    }
}

/// Small, resolution-independent court illustrations, distinct for J / Q / K.
fn court(painter: &Painter, rect: Rect, card: Card, ink: Color32) {
    let p = |x: f32, y: f32| rect.min + Vec2::new(x * rect.width(), y * rect.height());
    let scale = rect.width() / 104.0;
    let frame = Rect::from_min_max(p(0.16, 0.24), p(0.84, 0.79));
    let paper = Color32::from_rgb(242, 231, 202);
    let gold = Color32::from_rgb(174, 133, 58);
    painter.rect(
        frame,
        3.0,
        paper,
        Stroke::new(1.0, gold),
        StrokeKind::Inside,
    );
    let polygon =
        |points: Vec<Pos2>, color| painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    // Shoulders, robe and gold collar.
    polygon(
        vec![
            p(0.24, 0.71),
            p(0.30, 0.58),
            p(0.43, 0.54),
            p(0.57, 0.54),
            p(0.70, 0.58),
            p(0.76, 0.71),
        ],
        ink,
    );
    polygon(vec![p(0.42, 0.55), p(0.58, 0.55), p(0.50, 0.67)], gold);
    painter.line_segment([p(0.5, 0.63), p(0.5, 0.71)], Stroke::new(1.4 * scale, gold));
    // Hair, face, eyes and nose.
    painter.circle_filled(p(0.5, 0.43), 12.0 * scale, ink);
    if card.rank == Rank::Queen {
        painter.rect_filled(Rect::from_min_max(p(0.38, 0.43), p(0.62, 0.58)), 3.0, ink);
    }
    painter.circle_filled(p(0.5, 0.44), 9.0 * scale, CREAM);
    for x in [0.466, 0.534] {
        painter.circle_filled(p(x, 0.435), 0.9 * scale, ink);
    }
    painter.line_segment(
        [p(0.50, 0.44), p(0.515, 0.466)],
        Stroke::new(0.8 * scale, gold),
    );
    painter.line_segment(
        [p(0.473, 0.483), p(0.527, 0.483)],
        Stroke::new(1.0 * scale, ink),
    );
    match card.rank {
        Rank::Jack => {
            // A beret and feather; a sword at the shoulder.
            polygon(
                vec![p(0.36, 0.37), p(0.41, 0.31), p(0.59, 0.32), p(0.64, 0.37)],
                ink,
            );
            painter.line_segment(
                [p(0.58, 0.33), p(0.68, 0.27)],
                Stroke::new(2.0 * scale, gold),
            );
            painter.line_segment(
                [p(0.29, 0.54), p(0.29, 0.70)],
                Stroke::new(2.0 * scale, gold),
            );
            painter.line_segment(
                [p(0.24, 0.64), p(0.34, 0.64)],
                Stroke::new(2.0 * scale, gold),
            );
        }
        Rank::Queen => {
            // A jeweled tiara and flower.
            painter.rect_filled(Rect::from_min_max(p(0.38, 0.34), p(0.62, 0.37)), 1.0, gold);
            for (x, y) in [(0.41, 0.325), (0.5, 0.305), (0.59, 0.325)] {
                suit(painter, p(x, y), 7.0 * scale, Suit::Diamonds, gold);
            }
            painter.line_segment(
                [p(0.71, 0.60), p(0.67, 0.70)],
                Stroke::new(1.5 * scale, gold),
            );
            for (x, y) in [(0.69, 0.58), (0.73, 0.58), (0.71, 0.605)] {
                painter.circle_filled(p(x, y), 3.0 * scale, gold);
            }
        }
        Rank::King => {
            // A three-point crown, beard, and royal sceptre.
            painter.rect_filled(Rect::from_min_max(p(0.36, 0.34), p(0.64, 0.37)), 1.0, gold);
            for x in [0.4, 0.5, 0.6] {
                polygon(
                    vec![p(x - 0.045, 0.34), p(x, 0.275), p(x + 0.045, 0.34)],
                    gold,
                );
            }
            polygon(vec![p(0.44, 0.49), p(0.56, 0.49), p(0.5, 0.535)], ink);
            painter.line_segment(
                [p(0.71, 0.56), p(0.71, 0.70)],
                Stroke::new(2.0 * scale, gold),
            );
            painter.circle_filled(p(0.71, 0.545), 4.0 * scale, gold);
        }
        _ => unreachable!("only court cards use the court renderer"),
    }
    // One large, high-contrast badge makes same-color suits unmistakable.
    // Corner ranks remain free of repeated suit marks.
    let emblem = p(0.5, 0.675);
    painter.circle_filled(emblem, 16.0 * scale, CREAM);
    painter.circle_stroke(emblem, 16.0 * scale, Stroke::new(1.3 * scale, gold));
    suit(painter, emblem, 25.0 * scale, card.suit, ink);
}

pub fn slot(painter: &Painter, rect: Rect, suit_id: Option<Suit>, highlighted: bool) {
    let color = if highlighted {
        LEGAL
    } else {
        Color32::from_rgb(70, 113, 94)
    };
    painter.rect(
        rect,
        8.0,
        Color32::from_rgb(20, 72, 60),
        Stroke::new(if highlighted { 2.5 } else { 1.2 }, color),
        StrokeKind::Inside,
    );
    if let Some(suit_id) = suit_id {
        suit(painter, rect.center(), 32.0, suit_id, color);
    }
}
