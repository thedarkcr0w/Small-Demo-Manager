//! Look and feel: the warm "keycap" light theme shared with XMouse — bundled fonts, one
//! palette, one spacing/radius/motion scale, and the custom widgets used everywhere.

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Galley,
    Layout, Margin, Mesh, Painter, Pos2, Rect, Response, RichText, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};
use std::sync::Arc;

pub const PAGE: Color32 = Color32::from_rgb(0xF7, 0xF4, 0xEE);
pub const GROUND: Color32 = Color32::from_rgb(0xEC, 0xE8, 0xE1);
pub const KEY: Color32 = Color32::from_rgb(0xFB, 0xF9, 0xF5);
pub const EDGE: Color32 = Color32::from_rgb(0xCE, 0xC7, 0xBB);
pub const IDLE: Color32 = Color32::from_rgb(0xE3, 0xDE, 0xD5);
pub const HOVER: Color32 = Color32::from_rgb(0xF3, 0xEF, 0xE7);
pub const INK: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1F);
pub const MUTED: Color32 = Color32::from_rgb(0x5F, 0x5B, 0x55);
pub const LABEL: Color32 = Color32::from_rgb(0x49, 0x48, 0x4A);
pub const SIGNAL: Color32 = Color32::from_rgb(0xF0, 0x45, 0x2B);
pub const SIGNAL_EDGE: Color32 = Color32::from_rgb(0xA8, 0x2B, 0x17);
pub const SIGNAL_TEXT: Color32 = Color32::from_rgb(0xC8, 0x36, 0x1F);
pub const OK: Color32 = Color32::from_rgb(0x2E, 0x7D, 0x4F);
/// The second team's colour (CT blue), readable on the light page.
pub const TEAM_B: Color32 = Color32::from_rgb(0x2F, 0x67, 0xB0);
/// The dark "screen" used behind map artwork.
pub const SCREEN: Color32 = Color32::from_rgb(0x24, 0x23, 0x27);

/// The one spacing scale (points). Gaps between things come from here.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// The corner radii: small keys and chips, controls, buttons and floating cards, cards.
pub mod radius {
    pub const TINY: u8 = 5;
    pub const KEY: u8 = 8;
    pub const CONTROL: u8 = 10;
    pub const BUTTON: u8 = 14;
    pub const CARD: u8 = 16;
}

/// Motion: every animation takes its duration and easing from here, so hover, press,
/// selection and page changes all move the same calm way.
pub mod motion {
    use eframe::egui;
    use std::time::Duration;

    /// Hover and press feedback.
    pub const FAST: f32 = 0.12;
    /// Switches, selection, focus rings, enabled ↔ disabled.
    pub const BASE: f32 = 0.18;
    /// Page changes, the sliding pill of a segmented control.
    pub const SLOW: f32 = 0.24;
    /// How often an animation asks for a new frame (~144 per second, never a busy loop).
    pub const FRAME: Duration = Duration::from_millis(6);
    /// How often a spinner asks for a new frame.
    pub const SPINNER_FRAME: Duration = Duration::from_millis(16);

    /// One step of the spring-like approach all the animated values use: move `value`
    /// towards `target` so that it has (as good as) arrived after `secs`. It never
    /// overshoots, and turning around mid-way is smooth because only the current value matters.
    pub fn approach(value: f32, target: f32, dt: f32, secs: f32) -> f32 {
        if secs <= 0.0 {
            return target;
        }
        // 99 % of the way after `secs`.
        let k = 1.0 - (-dt.max(0.0) * 4.6 / secs).exp();
        value + (target - value) * k
    }

    #[derive(Clone, Copy)]
    struct Anim {
        value: f32,
        time: f64,
        frame: u64,
        resting: bool,
    }

    /// Animate towards `target` (see [`approach`]). The first time, or when the widget was
    /// not on screen in the previous frame, the value is simply `target`: nothing animates
    /// just because a view opened. Asks for new frames only while it is still moving.
    pub fn value_eps(ctx: &egui::Context, id: egui::Id, target: f32, secs: f32, eps: f32) -> f32 {
        let now = ctx.input(|i| i.time);
        let frame = ctx.cumulative_frame_nr();
        let value = ctx.data_mut(|d| {
            let a = d.get_temp_mut_or(
                id,
                Anim {
                    value: target,
                    time: now,
                    frame,
                    resting: true,
                },
            );
            if frame > a.frame + 1 || (a.value - target).abs() <= eps {
                a.value = target;
            } else {
                // A value that was at rest starts moving now, not when it was last drawn.
                let dt = if a.resting {
                    1.0 / 120.0
                } else {
                    ((now - a.time) as f32).clamp(0.0, 1.0 / 30.0)
                };
                a.value = approach(a.value, target, dt, secs);
            }
            a.time = now;
            a.frame = frame;
            a.resting = a.value == target;
            a.value
        });
        if value != target {
            ctx.request_repaint_after(FRAME);
        }
        value
    }

    /// [`value_eps`] for values in 0..=1.
    pub fn value(ctx: &egui::Context, id: egui::Id, target: f32, secs: f32) -> f32 {
        value_eps(ctx, id, target, secs, 0.004)
    }

    /// Animate a state: 0 when `on` is false, 1 when true.
    pub fn flag(ctx: &egui::Context, id: egui::Id, on: bool, secs: f32) -> f32 {
        value(ctx, id, if on { 1.0 } else { 0.0 }, secs)
    }

    /// Where an animated value is right now, without moving it.
    #[cfg(test)]
    pub fn peek(ctx: &egui::Context, id: egui::Id) -> f32 {
        ctx.data(|d| d.get_temp::<Anim>(id))
            .map_or(0.0, |a| a.value)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn approach_arrives_in_time_and_never_overshoots() {
            for (from, to) in [(0.0f32, 1.0f32), (1.0, 0.0), (20.0, 340.0)] {
                let mut v = from;
                let mut last = (to - v).abs();
                for _ in 0..24 {
                    v = approach(v, to, 0.01, SLOW);
                    let left = (to - v).abs();
                    assert!(left <= last, "moves towards the target only");
                    last = left;
                }
                assert!(
                    last <= (to - from).abs() * 0.011,
                    "there after SLOW seconds"
                );
            }
        }

        #[test]
        fn value_snaps_first_then_eases_and_settles() {
            let ctx = egui::Context::default();
            let id = egui::Id::unique("motion-test");
            let mut seen = Vec::new();
            for i in 0..80 {
                let input = egui::RawInput {
                    time: Some(i as f64 * 0.01),
                    ..Default::default()
                };
                // Headless: no GPU takes the font atlas, so drop its upload.
                ctx.run_ui(input, |ui| {
                    seen.push(flag(ui.ctx(), id, i >= 2, FAST));
                })
                .textures_delta
                .clear();
            }
            assert_eq!(seen[0], 0.0, "first sight: no animation");
            assert!(
                seen[2] > 0.0 && seen[2] < 0.5,
                "starts gently, got {}",
                seen[2]
            );
            assert!(seen.windows(2).all(|w| w[1] >= w[0]), "monotonic");
            assert_eq!(
                *seen.last().expect("frames ran"),
                1.0,
                "settles exactly on the target"
            );
            assert_eq!(peek(&ctx, id), 1.0);
        }
    }
}

/// Blend two colours (`t` = 0 gives `a`, 1 gives `b`).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

fn brighten(c: Color32) -> Color32 {
    let f = |v: u8| (v as f32 + (255.0 - v as f32) * 0.08) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, family("bold"))
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, family("medium"))
}
pub fn display(size: f32) -> FontId {
    FontId::new(size, family("display"))
}
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Install fonts, visuals and image loaders.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (
            "onest",
            &include_bytes!("../../assets/fonts/Onest-Regular.ttf")[..],
        ),
        (
            "onest-medium",
            include_bytes!("../../assets/fonts/Onest-Medium.ttf"),
        ),
        (
            "onest-bold",
            include_bytes!("../../assets/fonts/Onest-Bold.ttf"),
        ),
        (
            "unbounded",
            include_bytes!("../../assets/fonts/Unbounded-ExtraBold.ttf"),
        ),
        (
            "jetbrains",
            include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf"),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    // Player names and server titles can hold scripts and symbols the bundled fonts lack;
    // Windows' own UI fonts stand behind them (read at runtime, never redistributed).
    let mut system = Vec::new();
    #[cfg(windows)]
    for (name, file) in [
        ("Segoe UI", r"C:\Windows\Fonts\segoeui.ttf"),
        ("Segoe UI Symbol", r"C:\Windows\Fonts\seguisym.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(file) {
            fonts
                .font_data
                .insert(name.into(), Arc::new(FontData::from_owned(bytes)));
            system.push(name.to_string());
        }
    }
    let fallback: Vec<String> = system
        .into_iter()
        .chain(
            fonts
                .families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        )
        .collect();
    let with_fallback = |first: &str| {
        let mut v = vec![first.to_string()];
        v.extend(fallback.iter().cloned());
        v
    };
    fonts
        .families
        .insert(FontFamily::Proportional, with_fallback("onest"));
    fonts
        .families
        .insert(FontFamily::Monospace, with_fallback("jetbrains"));
    fonts
        .families
        .insert(family("bold"), with_fallback("onest-bold"));
    fonts
        .families
        .insert(family("medium"), with_fallback("onest-medium"));
    fonts
        .families
        .insert(family("display"), with_fallback("unbounded"));
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);

    let mut v = egui::Visuals::light();
    v.panel_fill = PAGE;
    v.window_fill = KEY;
    v.window_stroke = Stroke::new(1.0, EDGE);
    v.window_corner_radius = CornerRadius::same(radius::CARD);
    v.menu_corner_radius = CornerRadius::same(12);
    v.extreme_bg_color = KEY;
    v.faint_bg_color = HOVER;
    v.code_bg_color = GROUND;
    v.hyperlink_color = INK;
    v.selection.bg_fill = Color32::from_rgb(0xF6, 0xC9, 0xBF);
    v.selection.stroke = Stroke::new(1.5, INK);
    v.text_cursor.stroke = Stroke::new(2.0, INK);
    v.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(30),
    };
    v.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(38),
    };
    let w = &mut v.widgets;
    w.noninteractive.fg_stroke = Stroke::new(1.0, INK);
    w.noninteractive.bg_stroke = Stroke::new(1.0, EDGE);
    w.noninteractive.bg_fill = KEY;
    for (s, fill, stroke) in [
        (&mut w.inactive, GROUND, MUTED),
        (&mut w.hovered, IDLE, INK),
        (&mut w.active, IDLE, INK),
        (&mut w.open, IDLE, INK),
    ] {
        s.weak_bg_fill = fill;
        s.bg_fill = fill;
        s.bg_stroke = Stroke::new(1.5, stroke);
        s.fg_stroke = Stroke::new(1.5, INK);
        s.corner_radius = CornerRadius::same(radius::CONTROL);
        s.expansion = 0.0;
    }

    let mut style = egui::Style {
        visuals: v,
        ..Default::default()
    };
    // egui's own animations (menus opening, scrolling to a spot) use our timing too.
    style.animation_time = motion::BASE;
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(12.0, 6.0);
    style.spacing.interact_size = vec2(40.0, 30.0);
    style.spacing.window_margin = Margin::same(22);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.combo_width = 220.0;
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.spacing.scroll.dormant_handle_opacity = 0.0;
    style.spacing.scroll.dormant_background_opacity = 0.0;
    style.text_styles = [
        (egui::TextStyle::Body, body(14.5)),
        (egui::TextStyle::Button, bold(14.0)),
        (egui::TextStyle::Small, body(12.5)),
        (egui::TextStyle::Monospace, mono(12.5)),
        (egui::TextStyle::Heading, bold(19.0)),
    ]
    .into();
    ctx.set_theme(egui::Theme::Light);
    ctx.set_style_of(egui::Theme::Light, style);
}

// ---------------------------------------------------------------------------- text

/// One line of text cut off with "…" at `max_w`.
pub fn one_line(
    ui: &Ui,
    text: impl Into<String>,
    font: FontId,
    color: Color32,
    max_w: f32,
) -> Arc<Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_w.max(1.0));
    ui.painter().layout_job(job)
}

/// Monospace uppercase section label.
pub fn section(ui: &mut Ui, label: &str) {
    ui.label(
        RichText::new(label.to_uppercase())
            .font(mono(11.5))
            .color(LABEL),
    );
}

/// The one style for explanations and secondary lines.
pub fn hint(ui: &mut Ui, t: impl Into<String>) {
    ui.add(egui::Label::new(RichText::new(t.into()).font(body(13.0)).color(MUTED)).wrap());
}

/// The one style for "that went wrong" lines.
pub fn error_text(ui: &mut Ui, t: impl Into<String>) {
    ui.add(
        egui::Label::new(
            RichText::new(t.into())
                .font(medium(13.5))
                .color(SIGNAL_TEXT),
        )
        .wrap(),
    );
}

// ---------------------------------------------------------------------------- surfaces

/// A keycap card: light fill, thin edge and a thicker bottom edge.
pub fn group<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    group_with(ui, Margin::same(16), add)
}

/// [`group`] with its own padding (`Margin::ZERO` for edge-to-edge rows).
pub fn group_with<R>(ui: &mut Ui, margin: Margin, add: impl FnOnce(&mut Ui) -> R) -> R {
    let edge_idx = ui.painter().add(Shape::Noop);
    let r = egui::Frame::new()
        .fill(KEY)
        .stroke(Stroke::new(1.0, EDGE))
        .corner_radius(CornerRadius::same(radius::CARD))
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    ui.painter().set(
        edge_idx,
        Shape::rect_filled(
            r.response.rect.translate(vec2(0.0, 3.0)),
            CornerRadius::same(radius::CARD),
            EDGE,
        ),
    );
    ui.add_space(space::M);
    r.inner
}

/// Thin line across the available width.
pub fn divider(ui: &mut Ui) {
    let r = ui.available_rect_before_wrap();
    ui.painter()
        .hline(r.x_range(), r.top(), Stroke::new(1.0, EDGE));
    ui.add_space(1.0);
}

/// A settings row inside a [`group_with`] `Margin::ZERO` card: bold title with the control
/// on the same line, explanation underneath.
pub fn control_row(
    ui: &mut Ui,
    first: bool,
    title: &str,
    sub: &str,
    right: impl FnOnce(&mut Ui),
) -> Response {
    if !first {
        divider(ui);
    }
    egui::Frame::new()
        .inner_margin(Margin::symmetric(16, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 0.0),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.set_min_height(32.0);
                    right(ui);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(title).font(bold(15.0)).color(INK))
                                .wrap(),
                        );
                    });
                },
            );
            if !sub.is_empty() {
                ui.add_space(2.0);
                hint(ui, sub);
            }
        })
        .response
}

// ---------------------------------------------------------------------------- glyphs

/// Small painted icons, so nothing depends on a font having the symbol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glyph {
    Play,
    Pause,
    Stop,
    Star,
    StarFilled,
    Folder,
    Plus,
    Cross,
    ChevronDown,
    Rows,
    Grid,
    Split,
    Sidebar,
    Minimize,
    Maximize,
    Check,
    Refresh,
    Search,
    Gear,
    Info,
    Clock,
    Wave,
}

/// Paint `g` centred on `c`, roughly `s` points across.
pub fn paint_glyph(p: &Painter, g: Glyph, c: Pos2, s: f32, color: Color32) {
    let u = s / 2.0;
    let w = (s / 9.0).clamp(1.3, 2.2);
    let line = Stroke::new(w, color);
    let seg = |a: Vec2, b: Vec2| {
        p.line_segment([c + a * u, c + b * u], line);
    };
    match g {
        Glyph::Play => {
            p.add(Shape::convex_polygon(
                vec![
                    c + vec2(-0.55, -0.8) * u,
                    c + vec2(0.85, 0.0) * u,
                    c + vec2(-0.55, 0.8) * u,
                ],
                color,
                Stroke::NONE,
            ));
        }
        Glyph::Pause => {
            for x in [-0.55, 0.15] {
                p.rect_filled(
                    Rect::from_min_size(c + vec2(x, -0.75) * u, vec2(0.42, 1.5) * u),
                    CornerRadius::same(1),
                    color,
                );
            }
        }
        Glyph::Stop => {
            p.rect_filled(
                Rect::from_center_size(c, Vec2::splat(1.35 * u)),
                CornerRadius::same(2),
                color,
            );
        }
        Glyph::Star | Glyph::StarFilled => {
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let r = if i % 2 == 0 { u } else { u * 0.45 };
                    c + vec2(a.cos(), a.sin()) * r + vec2(0.0, 0.08 * u)
                })
                .collect();
            if g == Glyph::StarFilled {
                let mut mesh = Mesh::default();
                mesh.colored_vertex(c + vec2(0.0, 0.08 * u), color);
                for pt in &pts {
                    mesh.colored_vertex(*pt, color);
                }
                for i in 0..10u32 {
                    mesh.add_triangle(0, 1 + i, 1 + (i + 1) % 10);
                }
                p.add(Shape::mesh(mesh));
            } else {
                p.add(Shape::closed_line(pts, line));
            }
        }
        Glyph::Folder => {
            p.add(Shape::closed_line(
                [
                    vec2(-0.9, -0.65),
                    vec2(-0.2, -0.65),
                    vec2(0.05, -0.4),
                    vec2(0.9, -0.4),
                    vec2(0.9, 0.7),
                    vec2(-0.9, 0.7),
                ]
                .into_iter()
                .map(|v| c + v * u)
                .collect(),
                line,
            ));
        }
        Glyph::Plus => {
            seg(vec2(-0.75, 0.0), vec2(0.75, 0.0));
            seg(vec2(0.0, -0.75), vec2(0.0, 0.75));
        }
        Glyph::Cross => {
            seg(vec2(-0.6, -0.6), vec2(0.6, 0.6));
            seg(vec2(-0.6, 0.6), vec2(0.6, -0.6));
        }
        Glyph::ChevronDown => {
            seg(vec2(-0.6, -0.25), vec2(0.0, 0.35));
            seg(vec2(0.0, 0.35), vec2(0.6, -0.25));
        }
        Glyph::Rows => {
            for y in [-0.6, 0.0, 0.6] {
                seg(vec2(-0.85, y), vec2(0.85, y));
            }
        }
        Glyph::Grid => {
            for (x, y) in [(-0.85, -0.85), (0.1, -0.85), (-0.85, 0.1), (0.1, 0.1)] {
                p.rect_stroke(
                    Rect::from_min_size(c + vec2(x, y) * u, vec2(0.75, 0.75) * u),
                    CornerRadius::same(1),
                    line,
                    StrokeKind::Inside,
                );
            }
        }
        Glyph::Split | Glyph::Sidebar => {
            let r = Rect::from_center_size(c, vec2(1.8, 1.5) * u);
            p.rect_stroke(r, CornerRadius::same(2), line, StrokeKind::Inside);
            let x = r.left() + r.width() * 0.38;
            if g == Glyph::Sidebar {
                p.rect_filled(
                    Rect::from_min_max(r.min, pos2(x, r.max.y)),
                    CornerRadius::same(2),
                    color,
                );
            } else {
                p.vline(x, r.y_range(), line);
            }
        }
        Glyph::Minimize => seg(vec2(-0.7, 0.0), vec2(0.7, 0.0)),
        Glyph::Maximize => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(1.3 * u)),
                CornerRadius::same(1),
                line,
                StrokeKind::Inside,
            );
        }
        Glyph::Check => {
            p.add(Shape::line(
                vec![
                    c + vec2(-0.7, 0.0) * u,
                    c + vec2(-0.2, 0.5) * u,
                    c + vec2(0.7, -0.5) * u,
                ],
                line,
            ));
        }
        Glyph::Refresh => {
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = -0.6 + i as f32 / 16.0 * 4.9;
                    c + vec2(a.cos(), a.sin()) * 0.75 * u
                })
                .collect();
            p.add(Shape::line(pts, line));
            let a = -0.6f32;
            let tip = c + vec2(a.cos(), a.sin()) * 0.75 * u;
            p.add(Shape::convex_polygon(
                vec![
                    tip + vec2(-0.45, -0.35) * u,
                    tip + vec2(0.35, -0.5) * u,
                    tip + vec2(0.15, 0.3) * u,
                ],
                color,
                Stroke::NONE,
            ));
        }
        Glyph::Clock => {
            p.circle_stroke(c, 0.85 * u, line);
            seg(vec2(0.0, 0.0), vec2(0.0, -0.5));
            seg(vec2(0.0, 0.0), vec2(0.38, 0.2));
        }
        Glyph::Wave => {
            for (i, h) in [0.45f32, 0.95, 1.4, 0.8, 0.5].into_iter().enumerate() {
                let x = (-0.8 + i as f32 * 0.4) * u;
                p.line_segment(
                    [c + vec2(x, -h * 0.5 * u), c + vec2(x, h * 0.5 * u)],
                    Stroke::new(w * 1.1, color),
                );
            }
        }
        Glyph::Search => {
            p.circle_stroke(c + vec2(-0.15, -0.15) * u, 0.55 * u, line);
            seg(vec2(0.27, 0.27), vec2(0.8, 0.8));
        }
        Glyph::Gear => {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.cos(), a.sin());
                p.line_segment(
                    [c + d * 0.55 * u, c + d * 0.9 * u],
                    Stroke::new(w * 1.4, color),
                );
            }
            p.circle_stroke(c, 0.55 * u, line);
            p.circle_filled(c, 0.18 * u, color);
        }
        Glyph::Info => {
            p.circle_stroke(c, 0.85 * u, line);
            p.circle_filled(c + vec2(0.0, -0.38) * u, w * 0.7, color);
            seg(vec2(0.0, -0.08), vec2(0.0, 0.45));
        }
    }
}

// ---------------------------------------------------------------------------- the logo

/// Our mark: a red key with a little voice waveform on it.
pub fn logo(ui: &mut Ui) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap("Small Demo Manager".into(), bold(17.0), INK);
    let (rect, resp) = ui.allocate_exact_size(vec2(36.0 + galley.size().x, 30.0), Sense::hover());
    let p = ui.painter();
    let icon = Rect::from_min_size(rect.left_top() + vec2(0.0, 0.0), vec2(27.0, 27.0));
    p.rect_filled(
        icon.translate(vec2(0.0, 2.5)),
        CornerRadius::same(7),
        SIGNAL_EDGE,
    );
    p.rect_filled(icon, CornerRadius::same(7), SIGNAL);
    for (i, h) in [6.0f32, 12.0, 15.0, 9.0].into_iter().enumerate() {
        let x = icon.center().x - 6.75 + i as f32 * 4.5;
        p.rect_filled(
            Rect::from_center_size(pos2(x, icon.center().y), vec2(2.6, h)),
            CornerRadius::same(1),
            INK,
        );
    }
    p.galley(
        pos2(icon.right() + 9.0, rect.center().y - galley.size().y / 2.0),
        galley,
        INK,
    );
    resp
}

// ---------------------------------------------------------------------------- buttons

/// How far (0..=1) a clickable thing is hovered and pressed, eased with the shared timing.
pub fn hover_press(ui: &Ui, resp: &Response, enabled: bool) -> (f32, f32) {
    let ctx = ui.ctx();
    let down = enabled && resp.is_pointer_button_down_on();
    let hover = motion::flag(
        ctx,
        resp.id.with("hover"),
        enabled && (resp.hovered() || down),
        motion::FAST,
    );
    let press = motion::flag(ctx, resp.id.with("press"), down, motion::FAST);
    (hover, press)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The red key: the one main action of a view.
    Primary,
    /// A cream key.
    Secondary,
    /// A cream key with red text: removes or moves something away.
    Danger,
}

impl Tone {
    fn colors(self) -> (Color32, Color32, Color32) {
        match self {
            Tone::Primary => (SIGNAL, SIGNAL_EDGE, INK),
            Tone::Secondary => (KEY, EDGE, INK),
            Tone::Danger => (KEY, EDGE, SIGNAL_TEXT),
        }
    }
}

struct KeySpec {
    height: f32,
    depth: f32,
    font: FontId,
    pad: f32,
    radius: u8,
}

fn key_button(
    ui: &mut Ui,
    glyph: Option<Glyph>,
    label: &str,
    tone: Tone,
    min_width: f32,
    spec: KeySpec,
    enabled: bool,
) -> Response {
    let enabled = enabled && ui.is_enabled();
    let (fill, edge, ink) = tone.colors();
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), spec.font.clone(), Color32::PLACEHOLDER);
    let icon_w = if glyph.is_some() {
        spec.font.size + if label.is_empty() { 0.0 } else { 7.0 }
    } else {
        0.0
    };
    let content = galley.size().x + icon_w;
    let w = min_width.max(content + 2.0 * spec.pad);
    let h = spec.height;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(w, h + spec.depth),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    // Hover lifts the key a little and brightens it; a press sinks it onto its edge;
    // turning on or off is a fade.
    let (hover, press) = hover_press(ui, &resp, enabled);
    let on = motion::flag(ui.ctx(), resp.id.with("enabled"), enabled, motion::BASE);
    let shift = spec.depth * 0.75 * press - 1.0 * hover * (1.0 - press);
    let top = Rect::from_min_size(rect.min + vec2(0.0, shift), vec2(w, h));
    let alpha = (105.0 + 150.0 * on).round() as u8;
    let a = |c: Color32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
    let p = ui.painter();
    let r = CornerRadius::same(spec.radius);
    p.rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, spec.depth), vec2(w, h)),
        r,
        a(edge),
    );
    p.rect(
        top,
        r,
        a(mix(fill, brighten(fill), hover)),
        if edge == EDGE {
            Stroke::new(1.0, a(EDGE))
        } else {
            Stroke::NONE
        },
        StrokeKind::Inside,
    );
    let x0 = top.center().x - content / 2.0;
    if let Some(g) = glyph {
        paint_glyph(
            p,
            g,
            pos2(x0 + spec.font.size / 2.0, top.center().y),
            spec.font.size * 0.85,
            a(ink),
        );
    }
    p.galley(
        pos2(x0 + icon_w, top.center().y - galley.size().y / 2.0),
        galley,
        a(ink),
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn compact_spec() -> KeySpec {
    KeySpec {
        height: 34.0,
        depth: 3.0,
        font: bold(14.0),
        pad: 14.0,
        radius: radius::CONTROL,
    }
}

/// The standard (compact) 3D key button.
pub fn button(ui: &mut Ui, label: &str, tone: Tone, enabled: bool) -> Response {
    key_button(ui, None, label, tone, 0.0, compact_spec(), enabled)
}

/// [`button`] with an icon before the label.
pub fn icon_button(ui: &mut Ui, glyph: Glyph, label: &str, tone: Tone, enabled: bool) -> Response {
    key_button(ui, Some(glyph), label, tone, 0.0, compact_spec(), enabled)
}

/// [`icon_button`] at least `width` wide.
pub fn icon_button_wide(
    ui: &mut Ui,
    glyph: Glyph,
    label: &str,
    tone: Tone,
    width: f32,
    enabled: bool,
) -> Response {
    key_button(ui, Some(glyph), label, tone, width, compact_spec(), enabled)
}

/// [`button`] at least `width` wide (for full-width buttons in a column).
pub fn wide_button(ui: &mut Ui, label: &str, tone: Tone, width: f32, enabled: bool) -> Response {
    key_button(ui, None, label, tone, width, compact_spec(), enabled)
}

/// The big 3D key, for the main action of a view.
pub fn big_button(
    ui: &mut Ui,
    glyph: Option<Glyph>,
    label: &str,
    tone: Tone,
    width: f32,
    enabled: bool,
) -> Response {
    key_button(
        ui,
        glyph,
        label,
        tone,
        width,
        KeySpec {
            height: 46.0,
            depth: 5.0,
            font: bold(16.0),
            pad: 24.0,
            radius: radius::BUTTON,
        },
        enabled,
    )
}

/// A square keycap holding just an icon.
pub fn icon_key(ui: &mut Ui, glyph: Glyph, enabled: bool) -> Response {
    let enabled = enabled && ui.is_enabled();
    let size = 30.0;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(size, size + 3.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let (hover, press) = hover_press(ui, &resp, enabled);
    let top = Rect::from_min_size(rect.min + vec2(0.0, 2.0 * press), Vec2::splat(size));
    let p = ui.painter();
    let fade = if enabled { 1.0 } else { 0.45 };
    p.rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, 3.0), Vec2::splat(size)),
        CornerRadius::same(radius::KEY),
        EDGE.gamma_multiply(fade),
    );
    p.rect(
        top,
        CornerRadius::same(radius::KEY),
        mix(KEY, HOVER, hover).gamma_multiply(fade),
        Stroke::new(1.0, EDGE.gamma_multiply(fade)),
        StrokeKind::Inside,
    );
    paint_glyph(p, glyph, top.center(), 13.0, INK.gamma_multiply(fade));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A flat, quiet icon button (window controls, favourites in lists).
pub fn ghost_icon(ui: &mut Ui, glyph: Glyph, size: Vec2, color: Color32, danger: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let (hover, _) = hover_press(ui, &resp, true);
    let p = ui.painter();
    if hover > 0.0 {
        let fill = if danger { SIGNAL } else { IDLE };
        p.rect_filled(
            rect,
            CornerRadius::same(radius::KEY),
            fill.gamma_multiply(hover),
        );
    }
    let ink = if danger {
        mix(color, KEY, hover)
    } else {
        color
    };
    paint_glyph(p, glyph, rect.center(), 12.0, ink);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// An underlined text link. The underline is quiet until you point at it.
pub fn small_link(ui: &mut Ui, label: &str) -> Response {
    link_colored(ui, label, INK)
}

/// [`small_link`] in another ink (e.g. red for "remove").
pub fn link_colored(ui: &mut Ui, label: &str, color: Color32) -> Response {
    let enabled = ui.is_enabled();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), bold(13.5), Color32::PLACEHOLDER);
    let size = galley.size();
    let (rect, resp) = ui.allocate_exact_size(
        size + vec2(0.0, 2.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let (hover, press) = hover_press(ui, &resp, enabled);
    let p = ui.painter();
    let ink = mix(color, SIGNAL_TEXT, press);
    p.galley(rect.left_top(), galley, ink);
    p.hline(
        rect.x_range(),
        rect.top() + size.y,
        Stroke::new(
            1.0 + 0.5 * hover,
            mix(mix(MUTED, color, hover), SIGNAL_TEXT, press),
        ),
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Top-band pill that goes back. The arrow leans the way you are about to go.
pub fn back_pill(ui: &mut Ui, label: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), bold(14.0), INK);
    let size = vec2(galley.size().x + 46.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let (hover, press) = hover_press(ui, &resp, true);
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(17),
        mix(mix(KEY, HOVER, hover), IDLE, press),
        Stroke::new(1.5, EDGE),
        StrokeKind::Inside,
    );
    let c = pos2(rect.left() + 19.0 - 2.0 * hover, rect.center().y);
    let s = Stroke::new(2.1, INK);
    p.line_segment([c + vec2(4.0, -5.5), c + vec2(-1.5, 0.0)], s);
    p.line_segment([c + vec2(-1.5, 0.0), c + vec2(4.0, 5.5)], s);
    p.galley(
        pos2(rect.left() + 31.0, rect.center().y - galley.size().y / 2.0),
        galley,
        INK,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---------------------------------------------------------------------------- toggles

/// iOS-style switch.
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let enabled = ui.is_enabled();
    let (rect, mut resp) = ui.allocate_exact_size(
        vec2(46.0, 26.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    // The knob slides and the track changes colour together.
    let t = motion::flag(ui.ctx(), resp.id.with("on"), *on, motion::BASE);
    let (hover, _) = hover_press(ui, &resp, enabled);
    let p = ui.painter();
    let line = mix(mix(MUTED, INK, hover * 0.5), INK, t);
    p.rect(
        rect,
        CornerRadius::same(13),
        mix(IDLE, INK, t),
        Stroke::new(1.5, line),
        StrokeKind::Inside,
    );
    let x = egui::lerp(rect.left() + 13.0..=rect.right() - 13.0, t);
    p.circle(pos2(x, rect.center().y), 9.0, KEY, Stroke::new(1.5, line));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A small keycap checkbox: fills with ink and shows a tick when on.
pub fn check(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(22.0, 24.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = motion::flag(ui.ctx(), resp.id.with("on"), *on, motion::BASE);
    let (hover, press) = hover_press(ui, &resp, true);
    let top = Rect::from_min_size(rect.min + vec2(0.0, 1.5 * press), vec2(22.0, 22.0));
    let p = ui.painter();
    p.rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, 2.0), vec2(22.0, 22.0)),
        CornerRadius::same(6),
        mix(EDGE, INK, t),
    );
    p.rect(
        top,
        CornerRadius::same(6),
        mix(mix(KEY, HOVER, hover), INK, t),
        Stroke::new(1.5, mix(mix(MUTED, INK, hover), INK, t)),
        StrokeKind::Inside,
    );
    if t > 0.0 {
        paint_glyph(p, Glyph::Check, top.center(), 12.0, KEY.gamma_multiply(t));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A keycap chip; `on` chips are inked. With `closable` it shows a × after the label.
pub fn chip(ui: &mut Ui, label: &str, on: bool, closable: bool) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), medium(13.0), Color32::PLACEHOLDER);
    let extra = if closable { 16.0 } else { 0.0 };
    let w = galley.size().x + 20.0 + extra;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let (hover, press) = hover_press(ui, &resp, true);
    let t = motion::flag(ui.ctx(), resp.id.with("on"), on, motion::BASE);
    let top = Rect::from_min_size(rect.min + vec2(0.0, 1.5 * press), vec2(w, 26.0));
    let p = ui.painter();
    p.rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, 2.0), vec2(w, 26.0)),
        CornerRadius::same(radius::KEY),
        mix(EDGE, INK, t),
    );
    p.rect(
        top,
        CornerRadius::same(radius::KEY),
        mix(mix(KEY, HOVER, hover), INK, t),
        Stroke::new(1.0, mix(EDGE, INK, t)),
        StrokeKind::Inside,
    );
    let ink = mix(INK, KEY, t);
    p.galley(
        pos2(top.left() + 10.0, top.center().y - galley.size().y / 2.0),
        galley,
        ink,
    );
    if closable {
        paint_glyph(
            p,
            Glyph::Cross,
            pos2(top.right() - 13.0, top.center().y),
            8.0 + hover,
            mix(MUTED, ink, 0.5 + 0.5 * hover),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A pill group of icon options; the selected one is raised like a key and slides.
pub fn icon_segmented(
    ui: &mut Ui,
    id: &str,
    items: &[(Glyph, &str)],
    selected: usize,
) -> Option<usize> {
    let cell = vec2(34.0, 28.0);
    let gap = 2.0;
    let inner_w = items.len() as f32 * cell.x + (items.len().saturating_sub(1)) as f32 * gap;
    let (outer, _) = ui.allocate_exact_size(vec2(inner_w + 8.0, cell.y + 8.0), Sense::hover());
    ui.painter()
        .rect_filled(outer, CornerRadius::same(radius::CONTROL + 2), GROUND);
    let base = ui.scope_id().with(("iseg", id));
    let ctx = ui.ctx().clone();
    let target = 4.0 + selected.min(items.len().saturating_sub(1)) as f32 * (cell.x + gap);
    let px = motion::value_eps(&ctx, base.with("pill-x"), target, motion::SLOW, 0.3);
    let pill = Rect::from_min_size(outer.min + vec2(px, 4.0), cell);
    let p = ui.painter();
    p.rect_filled(
        pill.translate(vec2(0.0, 2.0)),
        CornerRadius::same(radius::KEY),
        EDGE,
    );
    p.rect(
        pill,
        CornerRadius::same(radius::KEY),
        KEY,
        Stroke::new(1.0, EDGE),
        StrokeKind::Inside,
    );
    let mut clicked = None;
    for (i, (glyph, tip)) in items.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(
                outer.left() + 4.0 + i as f32 * (cell.x + gap),
                outer.top() + 4.0,
            ),
            cell,
        );
        let resp = ui
            .interact(rect, base.with(i), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(*tip);
        let hover = motion::flag(
            &ctx,
            resp.id.with("hover"),
            resp.hovered() && i != selected,
            motion::FAST,
        );
        let p = ui.painter();
        if hover > 0.0 {
            p.rect_filled(
                rect,
                CornerRadius::same(radius::KEY),
                HOVER.gamma_multiply(hover),
            );
        }
        let color = if i == selected { INK } else { MUTED };
        paint_glyph(p, *glyph, rect.center(), 13.0, color);
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// "Label **Value** ▾" text that opens a menu. Returns the response to attach a popup to.
pub fn inline_picker(ui: &mut Ui, label: &str, value: &str, size: f32) -> Response {
    let mut job = egui::text::LayoutJob::default();
    if !label.is_empty() {
        job.append(
            &format!("{label} "),
            0.0,
            egui::TextFormat {
                font_id: body(size),
                color: MUTED,
                ..Default::default()
            },
        );
    }
    job.append(
        value,
        0.0,
        egui::TextFormat {
            font_id: bold(size),
            color: INK,
            ..Default::default()
        },
    );
    let main = ui.painter().layout_job(job);
    let text = main.size();
    let arrow_w = size * 1.1;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(text.x + arrow_w, text.y.max(30.0)), Sense::click());
    let (hover, _) = hover_press(ui, &resp, true);
    let p = ui.painter();
    let top = rect.center().y - text.y / 2.0;
    p.galley(pos2(rect.left(), top), main, INK);
    let ax = rect.left() + text.x + arrow_w * 0.55;
    // The arrow dips towards the menu it opens.
    let ay = rect.center().y + 1.0 + 1.5 * hover;
    let s = size * 0.28;
    p.add(Shape::convex_polygon(
        vec![
            pos2(ax - s, ay - s * 0.6),
            pos2(ax + s, ay - s * 0.6),
            pos2(ax, ay + s * 0.7),
        ],
        INK,
        Stroke::NONE,
    ));
    if hover > 0.0 {
        p.hline(
            rect.left()..=rect.left() + text.x,
            top + text.y,
            Stroke::new(1.0, INK.gamma_multiply(hover)),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A sidebar entry: label, optional monospace sub-line and a count chip. The highlight
/// fades in on hover and deepens when selected.
pub fn nav_item(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    glyph: Option<Glyph>,
    label: &str,
    sub: Option<&str>,
    count: Option<usize>,
    selected: bool,
) -> Response {
    let h = if sub.is_some() { 48.0 } else { 36.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let base = ui.scope_id().with(("nav", id));
    let ctx = ui.ctx();
    let down = resp.is_pointer_button_down_on();
    let hover = motion::flag(
        ctx,
        base.with("hover"),
        resp.hovered() || down,
        motion::FAST,
    );
    let sel = motion::flag(ctx, base.with("sel"), selected, motion::BASE);
    let p = ui.painter();
    let strength = hover.max(sel);
    if strength > 0.0 {
        p.rect_filled(
            rect,
            CornerRadius::same(radius::CONTROL),
            mix(HOVER, IDLE, sel).gamma_multiply(strength),
        );
    }
    if sel > 0.0 {
        let bar = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - 9.0 * sel),
            vec2(3.0, 18.0 * sel),
        );
        p.rect_filled(bar, CornerRadius::same(2), SIGNAL);
    }
    let mut x = rect.left() + 12.0;
    if let Some(g) = glyph {
        paint_glyph(
            p,
            g,
            pos2(x + 7.0, rect.top() + 18.0),
            13.0,
            if selected { INK } else { MUTED },
        );
        x += 24.0;
    }
    let count_w = count.map_or(0.0, |n| {
        let g = ui
            .painter()
            .layout_no_wrap(n.to_string(), mono(11.5), LABEL);
        let w = g.size().x + 12.0;
        let r = Rect::from_min_size(
            pos2(rect.right() - 8.0 - w, rect.top() + 18.0 - 10.0),
            vec2(w, 20.0),
        );
        p.rect(
            r,
            CornerRadius::same(radius::TINY),
            if selected { KEY } else { GROUND },
            Stroke::new(1.0, if selected { EDGE } else { Color32::TRANSPARENT }),
            StrokeKind::Inside,
        );
        p.galley(r.center() - g.size() / 2.0, g, LABEL);
        w + 12.0
    });
    let max_w = rect.right() - x - 8.0 - count_w;
    let font = if selected { bold(14.0) } else { medium(14.0) };
    let g = one_line(ui, label, font, INK, max_w);
    p.galley(pos2(x, rect.top() + 18.0 - g.size().y / 2.0), g, INK);
    if let Some(sub) = sub {
        let g = one_line(ui, sub, mono(11.0), MUTED, rect.right() - x - 10.0);
        p.galley(pos2(x, rect.top() + 33.0 - g.size().y / 2.0), g, MUTED);
    }
    resp
}

// ---------------------------------------------------------------------------- inputs & progress

/// A text field with a soft focus ring that fades in when you click into it.
pub fn text_field(ui: &mut Ui, edit: egui::TextEdit<'_>) -> Response {
    text_field_raw(ui, edit.margin(Margin::symmetric(10, 7)))
}

/// [`text_field`] keeping the caller's own margins.
fn text_field_raw(ui: &mut Ui, edit: egui::TextEdit<'_>) -> Response {
    let ring = ui.painter().add(Shape::Noop);
    let resp = ui.add(edit);
    let t = motion::flag(
        ui.ctx(),
        resp.id.with("focus"),
        resp.has_focus(),
        motion::BASE,
    );
    if t > 0.0 {
        ui.painter().set(
            ring,
            Shape::rect_stroke(
                resp.rect.expand(1.0),
                CornerRadius::same(radius::CONTROL + 2),
                Stroke::new(3.0 * t, SIGNAL.gamma_multiply(0.30 * t)),
                StrokeKind::Outside,
            ),
        );
    }
    resp
}

/// The library search box: a text field with a magnifier inside it.
pub fn search_field(ui: &mut Ui, id: egui::Id, query: &mut String, hint: &str) -> Response {
    let resp = text_field_raw(
        ui,
        egui::TextEdit::singleline(query)
            .id(id)
            .hint_text(RichText::new(hint).color(MUTED))
            .margin(Margin {
                left: 34,
                right: 10,
                top: 7,
                bottom: 7,
            })
            .desired_width(ui.available_width()),
    );
    paint_glyph(
        ui.painter(),
        Glyph::Search,
        pos2(resp.rect.left() + 17.0, resp.rect.center().y),
        13.0,
        if resp.has_focus() { INK } else { MUTED },
    );
    resp
}

/// A calm "working on it" ring: a quarter arc going round a faint track.
pub fn spinner(ui: &mut Ui) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    spinner_at(ui, rect.center(), 7.0, MUTED);
    resp
}

/// [`spinner`] painted at a given spot.
pub fn spinner_at(ui: &Ui, center: Pos2, r: f32, color: Color32) {
    if !ui.is_rect_visible(Rect::from_center_size(center, Vec2::splat(r * 2.0 + 4.0))) {
        return;
    }
    let p = ui.painter();
    p.circle_stroke(center, r, Stroke::new(2.0, color.gamma_multiply(0.22)));
    // One turn every 1.1 s.
    let start = (ui.input(|i| i.time) / 1.1).fract() as f32 * std::f32::consts::TAU;
    let points: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = start + i as f32 / 12.0 * std::f32::consts::FRAC_PI_2;
            center + r * vec2(a.cos(), a.sin())
        })
        .collect();
    p.add(Shape::line(points, Stroke::new(2.0, color)));
    ui.ctx().request_repaint_after(motion::SPINNER_FRAME);
}

/// A thin graphite progress rail with the percentage in monospace after it.
pub fn progress_bar(ui: &mut Ui, fraction: f32, width: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 18.0), Sense::hover());
    let t = motion::value(
        ui.ctx(),
        resp.id.with("p"),
        fraction.clamp(0.0, 1.0),
        motion::BASE,
    );
    let rail = Rect::from_center_size(rect.center(), vec2(width, 6.0));
    let p = ui.painter();
    p.rect_filled(rail, CornerRadius::same(3), EDGE);
    p.rect_filled(
        Rect::from_min_size(rail.min, vec2(rail.width() * t, rail.height())),
        CornerRadius::same(3),
        INK,
    );
    ui.label(
        RichText::new(format!("{:>3.0}%", fraction.clamp(0.0, 1.0) * 100.0))
            .font(mono(12.0))
            .color(INK),
    );
}

/// Toggled disclosure header ("Individual clips (12)"); returns whether it is open.
pub fn disclosure(ui: &mut Ui, id: &str, label: &str) -> bool {
    let id = ui.scope_id().with(("disclosure", id));
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), bold(14.5), INK);
    let (rect, resp) = ui.allocate_exact_size(
        vec2(ui.available_width(), galley.size().y.max(28.0)),
        Sense::click(),
    );
    if resp.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(id, open));
    }
    let t = motion::flag(ui.ctx(), id.with("t"), open, motion::BASE);
    let (hover, _) = hover_press(ui, &resp, true);
    let p = ui.painter();
    let c = pos2(rect.left() + 8.0, rect.center().y);
    // The chevron turns from → to ↓.
    let a = t * std::f32::consts::FRAC_PI_2;
    let rot = |v: Vec2| vec2(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos());
    let s = Stroke::new(2.0, INK);
    p.line_segment([c + rot(vec2(-2.5, -5.0)), c + rot(vec2(3.0, 0.0))], s);
    p.line_segment([c + rot(vec2(3.0, 0.0)), c + rot(vec2(-2.5, 5.0))], s);
    let text = galley.size();
    p.galley(
        pos2(rect.left() + 22.0, rect.center().y - text.y / 2.0),
        galley,
        INK,
    );
    if hover > 0.0 {
        p.hline(
            rect.left() + 22.0..=rect.left() + 22.0 + text.x,
            rect.center().y + text.y / 2.0,
            Stroke::new(1.0, INK.gamma_multiply(hover)),
        );
    }
    let _ = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    open
}

/// The frame used for floating notices: a key-coloured card with a soft shadow.
pub fn notice_frame(error: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(KEY)
        .stroke(Stroke::new(1.5, if error { SIGNAL } else { EDGE }))
        .corner_radius(CornerRadius::same(radius::BUTTON))
        .inner_margin(Margin::symmetric(16, 12))
        .shadow(egui::Shadow {
            offset: [0, 3],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(22),
        })
}

// ---------------------------------------------------------------------------- formatting

pub fn number(value: Option<u32>) -> String {
    value.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
}

pub fn duration(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds.filter(|s| s.is_finite()) else {
        return "—".into();
    };
    let seconds = seconds.max(0.0) as u64;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

pub fn size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / 1073741824.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1048576.0)
    }
}

pub fn map_name(map: &str) -> String {
    if map.is_empty() {
        return "Unknown map".into();
    }
    if map == "de_dust2" {
        return "Dust II".into();
    }
    map.strip_prefix("de_")
        .or_else(|| map.strip_prefix("cs_"))
        .or_else(|| map.strip_prefix("aim_"))
        .unwrap_or(map)
        .split('_')
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map(|ch| ch.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn map_art(map: &str) -> Option<&'static [u8]> {
    Some(match map {
        "de_ancient" => include_bytes!("../../assets/maps/de_ancient.png"),
        "de_anubis" => include_bytes!("../../assets/maps/de_anubis.png"),
        "de_dust2" => include_bytes!("../../assets/maps/de_dust2.png"),
        "de_inferno" => include_bytes!("../../assets/maps/de_inferno.png"),
        "de_mirage" => include_bytes!("../../assets/maps/de_mirage.png"),
        "de_nuke" => include_bytes!("../../assets/maps/de_nuke.png"),
        "de_overpass" => include_bytes!("../../assets/maps/de_overpass.png"),
        "de_train" => include_bytes!("../../assets/maps/de_train.png"),
        "de_vertigo" => include_bytes!("../../assets/maps/de_vertigo.png"),
        "de_basalt" => include_bytes!("../../assets/maps/de_basalt.png"),
        "de_brewery" => include_bytes!("../../assets/maps/de_brewery.png"),
        "de_dogtown" => include_bytes!("../../assets/maps/de_dogtown.png"),
        "de_jura" => include_bytes!("../../assets/maps/de_jura.png"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<R>(
        ctx: &egui::Context,
        time: f64,
        events: Vec<egui::Event>,
        mut add: impl FnMut(&mut Ui) -> R,
    ) -> R {
        let input = egui::RawInput {
            time: Some(time),
            events,
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
            ..Default::default()
        };
        let mut out = None;
        // Headless: no GPU takes the font atlas, so drop its upload.
        ctx.run_ui(input, |ui| out = Some(add(ui)))
            .textures_delta
            .clear();
        out.expect("the frame ran")
    }

    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn a_key_button_sinks_smoothly_and_still_clicks() {
        let ctx = egui::Context::default();
        install(&ctx);
        let draw = |time: f64, events: Vec<egui::Event>| {
            frame(&ctx, time, events, |ui| {
                button(ui, "Copy", Tone::Primary, true)
            })
        };
        let at = draw(0.0, vec![]).rect.center();
        let id = draw(0.01, vec![egui::Event::PointerMoved(at)]).id;
        draw(0.02, vec![pointer(at, true)]);
        for i in 0..40 {
            draw(0.03 + i as f64 * 0.01, vec![]);
        }
        assert_eq!(
            motion::peek(&ctx, id.with("press")),
            1.0,
            "fully down while held"
        );
        assert!(draw(0.5, vec![pointer(at, false)]).clicked());
    }

    #[test]
    fn a_disabled_key_button_does_not_click() {
        let ctx = egui::Context::default();
        install(&ctx);
        let draw = |time: f64, events: Vec<egui::Event>| {
            frame(&ctx, time, events, |ui| {
                button(ui, "Copy", Tone::Primary, false)
            })
        };
        let at = draw(0.0, vec![]).rect.center();
        draw(0.01, vec![egui::Event::PointerMoved(at)]);
        draw(0.02, vec![pointer(at, true)]);
        assert!(!draw(0.03, vec![pointer(at, false)]).clicked());
    }

    #[test]
    fn a_quick_click_toggles_switch_and_check() {
        for which in 0..2 {
            // A fresh context each time: egui's clock must not run backwards.
            let ctx = egui::Context::default();
            install(&ctx);
            let mut on = false;
            let mut draw = |time: f64, events: Vec<egui::Event>| {
                frame(&ctx, time, events, |ui| {
                    let r = if which == 0 {
                        switch(ui, &mut on)
                    } else {
                        check(ui, &mut on)
                    };
                    (r.rect.center(), r.changed())
                })
            };
            let (at, _) = draw(0.0, vec![]);
            draw(0.01, vec![egui::Event::PointerMoved(at)]);
            draw(0.02, vec![pointer(at, true)]);
            let (_, changed) = draw(0.03, vec![pointer(at, false)]);
            assert!(changed, "a 10 ms click toggles control {which}");
        }
    }

    #[test]
    fn mix_blends_and_clamps() {
        assert_eq!(mix(KEY, INK, 0.0), KEY);
        assert_eq!(mix(KEY, INK, 1.0), INK);
        assert_eq!(mix(KEY, INK, 9.0), INK);
    }
}
