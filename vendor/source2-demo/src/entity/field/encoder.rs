#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum FieldEncoder {
    Coord,
    SimTime,
    RuneTime,
    Normal,
    Fixed8,
    Fixed16,
    Fixed32,
    Fixed64,
    QAnglePitchYaw,
    QAnglePrecise,
}

impl FieldEncoder {
    #[inline]
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        match s {
            "coord" => Some(FieldEncoder::Coord),
            "simtime" => Some(FieldEncoder::SimTime),
            "runetime" => Some(FieldEncoder::RuneTime),
            "normal" => Some(FieldEncoder::Normal),
            "fixed8" => Some(FieldEncoder::Fixed8),
            "fixed16" => Some(FieldEncoder::Fixed16),
            "fixed32" => Some(FieldEncoder::Fixed32),
            "fixed64" => Some(FieldEncoder::Fixed64),
            "qangle_pitch_yaw" => Some(FieldEncoder::QAnglePitchYaw),
            "qangle_precise" => Some(FieldEncoder::QAnglePrecise),
            _ => None,
        }
    }

    /// Bit width of the raw fixed-width integer encoders. CS2 sends these
    /// fields as plain bits rather than varints (first seen in the
    /// 2026-11-11 update on `CCSPlayerController`).
    #[inline]
    pub(crate) fn fixed_int_bits(self) -> Option<u32> {
        match self {
            FieldEncoder::Fixed8 => Some(8),
            FieldEncoder::Fixed16 => Some(16),
            FieldEncoder::Fixed32 => Some(32),
            _ => None,
        }
    }
}
