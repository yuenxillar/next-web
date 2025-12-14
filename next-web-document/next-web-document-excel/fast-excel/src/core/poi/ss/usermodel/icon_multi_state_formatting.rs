use crate::core::poi::ss::usermodel::conditional_formatting_threshold::ConditionalFormattingThreshold;

/// High level representation for the Icon / Multi-State Formatting
/// component of Conditional Formatting settings
pub trait IconMultiStateFormatting {
    /// Gets the Icon Set used
    fn get_icon_set(&self) -> IconSet;

    /// Changes the Icon Set used
    ///
    /// # Note
    /// If the new Icon Set has a different number of icons to the old one,
    /// you **must** update the thresholds before saving!
    fn set_icon_set(&mut self, set: IconSet);

    /// Should Icon + Value be displayed, or only the Icon?
    fn is_icon_only(&self) -> bool;

    /// Control if only the Icon is shown, or Icon + Value
    fn set_icon_only(&mut self, only: bool);

    fn is_reversed(&self) -> bool;
    fn set_reversed(&mut self, reversed: bool);

    /// Gets the list of thresholds
    fn get_thresholds(&self) -> &[&dyn ConditionalFormattingThreshold];

    /// Sets the thresholds. The number must match `IconSet::num` for the current `icon_set()`
    fn set_thresholds(&mut self, thresholds: Vec<Box<dyn ConditionalFormattingThreshold>>);

    /// Creates a new, empty Threshold
    fn create_threshold(&mut self) -> Box<dyn ConditionalFormattingThreshold>;
}

/// Icon Set enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IconSet {
    /// Green Up / Yellow Side / Red Down arrows
    Gyr3Arrow = 0,
    /// Grey Up / Side / Down arrows
    Grey3Arrows = 1,
    /// Green / Yellow / Red flags
    Gyr3Flags = 2,
    /// Green / Yellow / Red traffic lights (no background). Default
    Gyr3TrafficLights = 3,
    /// Green / Yellow / Red traffic lights on a black square background.
    /// Note: MS-XLS docs v20141018 say this is id=5 but seems to be id=4
    Gyr3TrafficLightsBox = 4,
    /// Green Circle / Yellow Triangle / Red Diamond.
    /// Note: MS-XLS docs v20141018 say this is id=4 but seems to be id=5
    Gyr3Shapes = 5,
    /// Green Tick / Yellow ! / Red Cross on a circle background
    Gyr3SymbolsCircle = 6,
    /// Green Tick / Yellow ! / Red Cross (no background)
    Gyr3Symbols = 7,
    /// Green Up / Yellow NE / Yellow SE / Red Down arrows
    Gyr4Arrows = 8,
    /// Grey Up / NE / SE / Down arrows
    Grey4Arrows = 9,
    /// Red / Light Red / Grey / Black traffic lights
    Rb4TrafficLights = 0xA,
    Ratings4 = 0xB,
    /// Green / Yellow / Red / Black traffic lights
    Gyrb4TrafficLights = 0xC,
    Gyyyr5Arrows = 0xD,
    Grey5Arrows = 0xE,
    Ratings5 = 0xF,
    Quarters5 = 0x10,
}

impl IconSet {
    /// Numeric ID of the icon set
    pub fn id(&self) -> u8 {
        *self as u8
    }

    /// How many icons in the set
    pub fn num(&self) -> u8 {
        match self {
            IconSet::Gyr3Arrow
            | IconSet::Grey3Arrows
            | IconSet::Gyr3Flags
            | IconSet::Gyr3TrafficLights
            | IconSet::Gyr3TrafficLightsBox
            | IconSet::Gyr3Shapes
            | IconSet::Gyr3SymbolsCircle
            | IconSet::Gyr3Symbols => 3,

            IconSet::Gyr4Arrows
            | IconSet::Grey4Arrows
            | IconSet::Rb4TrafficLights
            | IconSet::Ratings4
            | IconSet::Gyrb4TrafficLights => 4,

            IconSet::Gyyyr5Arrows
            | IconSet::Grey5Arrows
            | IconSet::Ratings5
            | IconSet::Quarters5 => 5,
        }
    }

    /// Name (system) of the set
    pub fn name(&self) -> &'static str {
        match self {
            IconSet::Gyr3Arrow => "3Arrows",
            IconSet::Grey3Arrows => "3ArrowsGray",
            IconSet::Gyr3Flags => "3Flags",
            IconSet::Gyr3TrafficLights => "3TrafficLights1",
            IconSet::Gyr3TrafficLightsBox => "3TrafficLights2",
            IconSet::Gyr3Shapes => "3Signs",
            IconSet::Gyr3SymbolsCircle => "3Symbols",
            IconSet::Gyr3Symbols => "3Symbols2",
            IconSet::Gyr4Arrows => "4Arrows",
            IconSet::Grey4Arrows => "4ArrowsGray",
            IconSet::Rb4TrafficLights => "4RedToBlack",
            IconSet::Ratings4 => "4Rating",
            IconSet::Gyrb4TrafficLights => "4TrafficLights",
            IconSet::Gyyyr5Arrows => "5Arrows",
            IconSet::Grey5Arrows => "5ArrowsGray",
            IconSet::Ratings5 => "5Rating",
            IconSet::Quarters5 => "5Quarters",
        }
    }

    /// Gets IconSet by its numeric ID
    pub fn by_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(IconSet::Gyr3Arrow),
            1 => Some(IconSet::Grey3Arrows),
            2 => Some(IconSet::Gyr3Flags),
            3 => Some(IconSet::Gyr3TrafficLights),
            4 => Some(IconSet::Gyr3TrafficLightsBox),
            5 => Some(IconSet::Gyr3Shapes),
            6 => Some(IconSet::Gyr3SymbolsCircle),
            7 => Some(IconSet::Gyr3Symbols),
            8 => Some(IconSet::Gyr4Arrows),
            9 => Some(IconSet::Grey4Arrows),
            0xA => Some(IconSet::Rb4TrafficLights),
            0xB => Some(IconSet::Ratings4),
            0xC => Some(IconSet::Gyrb4TrafficLights),
            0xD => Some(IconSet::Gyyyr5Arrows),
            0xE => Some(IconSet::Grey5Arrows),
            0xF => Some(IconSet::Ratings5),
            0x10 => Some(IconSet::Quarters5),
            _ => None,
        }
    }

    /// Gets IconSet by its name
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "3Arrows" => Some(IconSet::Gyr3Arrow),
            "3ArrowsGray" => Some(IconSet::Grey3Arrows),
            "3Flags" => Some(IconSet::Gyr3Flags),
            "3TrafficLights1" => Some(IconSet::Gyr3TrafficLights),
            "3TrafficLights2" => Some(IconSet::Gyr3TrafficLightsBox),
            "3Signs" => Some(IconSet::Gyr3Shapes),
            "3Symbols" => Some(IconSet::Gyr3SymbolsCircle),
            "3Symbols2" => Some(IconSet::Gyr3Symbols),
            "4Arrows" => Some(IconSet::Gyr4Arrows),
            "4ArrowsGray" => Some(IconSet::Grey4Arrows),
            "4RedToBlack" => Some(IconSet::Rb4TrafficLights),
            "4Rating" => Some(IconSet::Ratings4),
            "4TrafficLights" => Some(IconSet::Gyrb4TrafficLights),
            "5Arrows" => Some(IconSet::Gyyyr5Arrows),
            "5ArrowsGray" => Some(IconSet::Grey5Arrows),
            "5Rating" => Some(IconSet::Ratings5),
            "5Quarters" => Some(IconSet::Quarters5),
            _ => None,
        }
    }
}

impl std::fmt::Display for IconSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.id(), self.name())
    }
}
