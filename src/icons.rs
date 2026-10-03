use crate::ui::{CYAN, MUTED, PANEL};
use geode_egui::egui::{self, Color32, Pos2, Rect, Stroke};

#[derive(Clone, Copy)]
pub enum Icon {
    Trash,
    Pencil,
    Star,
    Copy,
    Up,
    Down,
    Check,
    Close,
}

/// Original vector icons: crisp at every interface scale, with accessible labels.
pub fn button(ui: &mut egui::Ui, icon: Icon, label: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let color = if active || response.hovered() {
        CYAN
    } else {
        MUTED
    };
    ui.painter().rect_filled(
        rect,
        6,
        if response.hovered() {
            Color32::from_rgb(36, 52, 65)
        } else {
            PANEL
        },
    );
    let r = rect.shrink(7.0);
    let p = ui.painter();
    let s = Stroke::new(1.6, color);
    let line = |a: Pos2, b: Pos2| {
        p.line_segment([a, b], s);
    };
    match icon {
        Icon::Trash => {
            line(
                r.left_top() + egui::vec2(0.0, 3.0),
                r.right_top() + egui::vec2(0.0, 3.0),
            );
            p.rect_stroke(
                Rect::from_min_max(r.min + egui::vec2(3.0, 4.0), r.max - egui::vec2(3.0, 0.0)),
                2,
                s,
                egui::StrokeKind::Inside,
            );
            line(r.min + egui::vec2(6.0, 0.0), r.min + egui::vec2(10.0, 0.0));
            for x in [6.0, 10.0] {
                line(r.min + egui::vec2(x, 7.0), r.min + egui::vec2(x, 13.0));
            }
        }
        Icon::Pencil => {
            let a = r.left_bottom() - egui::vec2(0.0, 3.0);
            let b = r.right_top() - egui::vec2(3.0, 0.0);
            p.add(egui::Shape::closed_line(
                vec![a, b, b + egui::vec2(3.0, 3.0), a + egui::vec2(3.0, 3.0)],
                s,
            ));
            line(a, r.left_bottom());
            line(r.left_bottom(), a + egui::vec2(3.0, 3.0));
        }
        Icon::Star => {
            let points = (0..10)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 10.0 - std::f32::consts::FRAC_PI_2;
                    let rad = if i % 2 == 0 { 8.0 } else { 3.5 };
                    r.center() + egui::vec2(a.cos(), a.sin()) * rad
                })
                .collect();
            p.add(egui::Shape::closed_line(points, s));
            if active {
                p.circle_filled(r.center(), 2.0, color);
            }
        }
        Icon::Copy => {
            p.rect_stroke(
                Rect::from_min_max(r.min, r.max - egui::vec2(4.0, 4.0)),
                2,
                s,
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                Rect::from_min_max(r.min + egui::vec2(4.0, 4.0), r.max),
                2,
                PANEL,
            );
            p.rect_stroke(
                Rect::from_min_max(r.min + egui::vec2(4.0, 4.0), r.max),
                2,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Up | Icon::Down => {
            let dy = if matches!(icon, Icon::Up) { -1.0 } else { 1.0 };
            let c = r.center();
            line(c - egui::vec2(0.0, 7.0 * dy), c + egui::vec2(0.0, 7.0 * dy));
            line(
                c + egui::vec2(0.0, 7.0 * dy),
                c + egui::vec2(-5.0, 2.0 * dy),
            );
            line(c + egui::vec2(0.0, 7.0 * dy), c + egui::vec2(5.0, 2.0 * dy));
        }
        Icon::Check => {
            line(r.min + egui::vec2(1.0, 8.0), r.min + egui::vec2(6.0, 13.0));
            line(r.min + egui::vec2(6.0, 13.0), r.min + egui::vec2(15.0, 2.0));
        }
        Icon::Close => {
            line(r.left_top(), r.right_bottom());
            line(r.right_top(), r.left_bottom());
        }
    }
    response.on_hover_text(label)
}
