use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};

use crate::core::poi::ss::usermodel::conditional_formatting_threshold::ConditionalFormattingThreshold;

/// High level representation for the Icon / Multi-State Formatting
/// component of Conditional Formatting settings
pub trait IconMultiStateFormatting: Debug {
    /// Get the Icon Set used.
    ///
    /// # Returns
    /// The icon set.
    fn get_icon_set(&self) -> IconSet;

    /// Changes the Icon Set used.
    ///
    /// If the new Icon Set has a different number of
    /// icons to the old one, you **must** update the
    /// thresholds before saving!
    ///
    /// # Arguments
    /// * `set` - The new icon set.
    fn set_icon_set(&mut self, set: IconSet);

    /// Check if only the icon should be displayed, or icon + value.
    ///
    /// # Returns
    /// `true` if only the icon is shown, `false` if icon + value are shown.
    fn is_icon_only(&self) -> bool;

    /// Control if only the icon is shown, or icon + value.
    ///
    /// # Arguments
    /// * `only` - `true` for icon only, `false` for icon + value.
    fn set_icon_only(&mut self, only: bool);

    /// Check if the icon order is reversed.
    ///
    /// # Returns
    /// `true` if icons are in reverse order, `false` otherwise.
    fn is_reversed(&self) -> bool;

    /// Set if the icon order should be reversed.
    ///
    /// # Arguments
    /// * `reversed` - `true` to reverse icon order, `false` for normal order.
    fn set_reversed(&mut self, reversed: bool);

    /// Gets the list of thresholds.
    ///
    /// # Returns
    /// Slice of thresholds.
    fn get_thresholds(&self) -> &[Box<dyn ConditionalFormattingThreshold>];

    /// Sets the list of thresholds.
    ///
    /// The number must match `IconSet::get_num_icons()` for the current icon set.
    ///
    /// # Arguments
    /// * `thresholds` - Vector of thresholds.
    ///
    /// # Panics
    /// Panics if the number of thresholds doesn't match the icon set.
    fn set_thresholds(&mut self, thresholds: Vec<Box<dyn ConditionalFormattingThreshold>>);

    /// Creates a new, empty Threshold.
    ///
    /// # Returns
    /// A new threshold.
    fn create_threshold(&self) -> Box<dyn ConditionalFormattingThreshold>;

    /// Validates the icon set configuration.
    ///
    /// # Returns
    /// `Ok(())` if valid, or an error message if invalid.
    fn validate(&self) -> Result<(), String>;
}

/// Icon set enumeration for multi-state formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Note, MS-XLS docs v20141018 say this is id=5 but seems to be id=4
    Gyr3TrafficLightsBox = 4,
    /// Green Circle / Yellow Triangle / Red Diamond.
    /// Note, MS-XLS docs v20141018 say this is id=4 but seems to be id=5
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
    /// 4-level ratings
    Ratings4 = 0xB,
    /// Green / Yellow / Red / Black traffic lights
    Gyrb4TrafficLights = 0xC,
    /// 5-level green/yellow/red arrows
    Gyyyr5Arrows = 0xD,
    /// Grey 5 arrows
    Grey5Arrows = 0xE,
    /// 5-level ratings
    Ratings5 = 0xF,
    /// 5 quarters
    Quarters5 = 0x10,
}

impl IconSet {
    /// Get the numeric ID of the icon set.
    ///
    /// # Returns
    /// Numeric ID.
    pub fn get_id(&self) -> u8 {
        *self as u8
    }

    /// Get how many icons in the set.
    ///
    /// # Returns
    /// Number of icons (3, 4, or 5).
    pub fn get_num_icons(&self) -> u8 {
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

    /// Get the system name of the icon set.
    ///
    /// # Returns
    /// System name.
    pub fn get_name(&self) -> &'static str {
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

    /// Get icon set by numeric ID.
    ///
    /// # Arguments
    /// * `id` - Numeric ID.
    ///
    /// # Returns
    /// Icon set, or `None` if ID is invalid.
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

    /// Get icon set by system name.
    ///
    /// # Arguments
    /// * `name` - System name.
    ///
    /// # Returns
    /// Icon set, or `None` if name is invalid.
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

    /// Get the display name of the icon set.
    ///
    /// # Returns
    /// Human-readable display name.
    pub fn get_display_name(&self) -> &'static str {
        match self {
            IconSet::Gyr3Arrow => "Green/Yellow/Red 3 Arrows",
            IconSet::Grey3Arrows => "Grey 3 Arrows",
            IconSet::Gyr3Flags => "Green/Yellow/Red 3 Flags",
            IconSet::Gyr3TrafficLights => "Green/Yellow/Red Traffic Lights",
            IconSet::Gyr3TrafficLightsBox => "Green/Yellow/Red Traffic Lights (Box)",
            IconSet::Gyr3Shapes => "Green/Yellow/Red 3 Shapes",
            IconSet::Gyr3SymbolsCircle => "Green/Yellow/Red 3 Symbols (Circle)",
            IconSet::Gyr3Symbols => "Green/Yellow/Red 3 Symbols",
            IconSet::Gyr4Arrows => "Green/Yellow/Red 4 Arrows",
            IconSet::Grey4Arrows => "Grey 4 Arrows",
            IconSet::Rb4TrafficLights => "Red/Black 4 Traffic Lights",
            IconSet::Ratings4 => "4 Ratings",
            IconSet::Gyrb4TrafficLights => "Green/Yellow/Red/Black 4 Traffic Lights",
            IconSet::Gyyyr5Arrows => "Green/Yellow/Red 5 Arrows",
            IconSet::Grey5Arrows => "Grey 5 Arrows",
            IconSet::Ratings5 => "5 Ratings",
            IconSet::Quarters5 => "5 Quarters",
        }
    }

    /// Check if this icon set uses color coding.
    ///
    /// # Returns
    /// `true` if the icon set uses colors, `false` if monochrome.
    pub fn is_colored(&self) -> bool {
        !matches!(
            self,
            IconSet::Grey3Arrows | IconSet::Grey4Arrows | IconSet::Grey5Arrows
        )
    }

    /// Check if this icon set uses traffic light icons.
    ///
    /// # Returns
    /// `true` if the icon set uses traffic lights.
    pub fn is_traffic_lights(&self) -> bool {
        matches!(
            self,
            IconSet::Gyr3TrafficLights
                | IconSet::Gyr3TrafficLightsBox
                | IconSet::Rb4TrafficLights
                | IconSet::Gyrb4TrafficLights
        )
    }

    /// Check if this icon set uses arrow icons.
    ///
    /// # Returns
    /// `true` if the icon set uses arrows.
    pub fn is_arrows(&self) -> bool {
        matches!(
            self,
            IconSet::Gyr3Arrow
                | IconSet::Grey3Arrows
                | IconSet::Gyr4Arrows
                | IconSet::Grey4Arrows
                | IconSet::Gyyyr5Arrows
                | IconSet::Grey5Arrows
        )
    }
}

impl Display for IconSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.get_id(), self.get_display_name())
    }
}

impl TryFrom<u8> for IconSet {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        IconSet::by_id(value).ok_or("Invalid icon set ID")
    }
}

impl From<IconSet> for u8 {
    fn from(icon_set: IconSet) -> Self {
        icon_set.get_id()
    }
}
