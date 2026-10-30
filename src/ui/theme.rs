use eframe::egui::*;

pub const BG: Color32 = Color32::from_rgb(11, 13, 17);
pub const SURFACE: Color32 = Color32::from_rgb(18, 21, 28);
pub const CARD: Color32 = Color32::from_rgb(24, 28, 37);
pub const HOVER: Color32 = Color32::from_rgb(31, 36, 47);
pub const BORDER: Color32 = Color32::from_rgb(35, 41, 54);
pub const TEXT: Color32 = Color32::from_rgb(231, 234, 240);
pub const SECONDARY: Color32 = Color32::from_rgb(180, 186, 198);
pub const MUTED: Color32 = Color32::from_rgb(108, 115, 130);
pub const ACCENT: Color32 = Color32::from_rgb(245, 158, 11);
pub const SOFT: Color32 = Color32::from_rgb(62, 46, 16);
pub const RED: Color32 = Color32::from_rgb(239, 100, 100);
pub const BLUE: Color32 = Color32::from_rgb(96, 165, 250);

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    #[cfg(windows)]
    for (name, file, family) in [
        (
            "Segoe UI",
            r"C:\Windows\Fonts\segoeui.ttf",
            FontFamily::Proportional,
        ),
        (
            "Consolas",
            r"C:\Windows\Fonts\consola.ttf",
            FontFamily::Monospace,
        ),
    ] {
        if let Ok(bytes) = std::fs::read(file) {
            fonts
                .font_data
                .insert(name.into(), FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.into());
        }
    }
    #[cfg(windows)]
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        fonts
            .font_data
            .insert("Segoe UI Symbol".into(), FontData::from_owned(bytes).into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("Segoe UI Symbol".into());
        }
    }
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    let mut style = Style {
        text_styles: [
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(12.5)),
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Heading, FontId::proportional(19.0)),
            (TextStyle::Monospace, FontId::monospace(11.5)),
        ]
        .into(),
        ..Default::default()
    };
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(12.0, 7.0);
    style.spacing.interact_size.y = 30.0;
    style.visuals = Visuals::dark();
    style.visuals.panel_fill = SURFACE;
    style.visuals.window_fill = CARD;
    style.visuals.extreme_bg_color = BG;
    style.visuals.faint_bg_color = SURFACE;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = SOFT;
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    for visuals in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        visuals.corner_radius = CornerRadius::same(6);
        visuals.bg_stroke = Stroke::new(1.0, BORDER);
        visuals.fg_stroke = Stroke::new(1.0, SECONDARY);
    }
    style.visuals.widgets.inactive.bg_fill = CARD;
    style.visuals.widgets.inactive.weak_bg_fill = CARD;
    style.visuals.widgets.hovered.bg_fill = HOVER;
    style.visuals.widgets.hovered.weak_bg_fill = HOVER;
    style.visuals.widgets.active.bg_fill = SOFT;
    ctx.set_theme(Theme::Dark);
    ctx.set_style_of(Theme::Dark, style);
}

pub fn card() -> Frame {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(12)
}

pub fn primary(text: impl Into<String>) -> Button<'static> {
    Button::new(RichText::new(text.into()).color(BG).strong())
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .min_size(vec2(100.0, 36.0))
}

pub fn eyebrow(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(10.0).color(MUTED));
}
pub fn small(ui: &mut Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).size(11.0).color(MUTED));
}
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
