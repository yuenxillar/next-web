use crate::core::poi::ss::usermodel::color::Color;

/// Represents a XSSF-style color (based on either a XSSFColor or a ExtendedColor)
pub trait ExtendedColor: Color {
    /// Set the color from an RGB color
    ///
    /// # Arguments
    /// * `red` - Red component (0-255)
    /// * `green` - Green component (0-255)
    /// * `blue` - Blue component (0-255)
    /// * `alpha` - Alpha component (0-255, optional)
    fn set_color(&mut self, red: u8, green: u8, blue: u8, alpha: Option<u8>) {
        let rgb = match alpha {
            Some(a) => vec![a, red, green, blue],
            None => vec![red, green, blue],
        };
        self.set_rgb(&rgb);
    }

    /// Check if the color is automatic
    fn is_auto(&self) -> bool;

    /// Check if the color is indexed
    fn is_indexed(&self) -> bool;

    /// Check if the color is RGB / ARGB
    fn is_rgb(&self) -> bool;

    /// Check if the color is from a Theme
    fn is_themed(&self) -> bool;

    /// Get indexed color index value, if `is_indexed()` is true
    fn get_index(&self) -> Option<u16>;

    /// Get index of Theme color, if `is_themed()` is true
    fn get_theme(&self) -> Option<u32>;

    /// Get Standard Red Green Blue color value (RGB) bytes.
    /// If there was an A (Alpha) value, it will be stripped.
    fn get_rgb(&self) -> Option<Vec<u8>>;

    /// Get Standard Alpha Red Green Blue color value (ARGB) bytes.
    fn get_argb(&self) -> Option<Vec<u8>>;

    /// Get RGB or ARGB bytes as stored internally
    fn get_stored_rgb(&self) -> Option<Vec<u8>>;

    /// Set the Red Green Blue or Alpha Red Green Blue
    ///
    /// # Arguments
    /// * `rgb` - RGB or ARGB bytes
    fn set_rgb(&mut self, rgb: &[u8]);

    /// Get RGB or ARGB bytes, either stored or by index
    fn get_rgb_or_argb(&self) -> Option<Vec<u8>> {
        if self.is_indexed() {
            if let Some(index) = self.get_index() {
                if index > 0 {
                    if let Some(rgb) = self.get_indexed_rgb() {
                        return Some(rgb);
                    }
                }
            }
        }

        // Grab the colour
        self.get_stored_rgb()
    }

    /// Get index color RGB bytes, if `is_indexed()` == true, None if not indexed or index is invalid
    fn get_indexed_rgb(&self) -> Option<Vec<u8>>;

    /// Get Standard Red Green Blue color value (RGB) bytes with applied tint.
    /// Alpha values are ignored.
    fn get_rgb_with_tint(&self) -> Option<Vec<u8>> {
        let mut rgb = self.get_stored_rgb()?;

        // Remove alpha channel if present
        if rgb.len() == 4 {
            rgb = vec![rgb[1], rgb[2], rgb[3]];
        }

        let tint = self.get_tint();
        for i in 0..rgb.len() {
            rgb[i] = self.apply_tint(rgb[i] as u32, tint);
        }

        Some(rgb)
    }

    /// Get the ARGB value in hex string format, eg "FF00FF00".
    /// Works for both regular and indexed colours.
    fn get_argb_hex(&self) -> Option<String> {
        let rgb = self.get_argb()?;

        let mut sb = String::with_capacity(rgb.len() * 2);
        for &c in &rgb {
            sb.push_str(&format!("{:02X}", c));
        }

        Some(sb)
    }

    /// Set the ARGB value from hex format, eg "FF0077FF".
    /// Only works for regular (non-indexed) colours
    ///
    /// # Arguments
    /// * `argb` - color ARGB hex string
    ///
    /// # Errors
    /// * Returns error if the string format is invalid
    fn set_argb_hex(&mut self, argb: &str) -> Result<(), String> {
        if argb.len() != 6 && argb.len() != 8 {
            return Err("Must be of the form 112233 or FFEEDDCC".to_string());
        }

        let mut rgb = Vec::with_capacity(argb.len() / 2);
        for i in 0..(argb.len() / 2) {
            let start = i * 2;
            let end = start + 2;
            let part = &argb[start..end];
            let byte =
                u8::from_str_radix(part, 16).map_err(|e| format!("Invalid hex value: {}", e))?;
            rgb.push(byte);
        }

        self.set_rgb(&rgb);
        Ok(())
    }

    /// Apply tint to a luminance value
    ///
    /// # Arguments
    /// * `lum` - Luminance value (0-255)
    /// * `tint` - Tint value (-1.0 to 1.0)
    ///
    /// # Returns
    /// * Tinted luminance value
    fn apply_tint(&self, lum: u32, tint: f64) -> u8 {
        if tint > 0.0 {
            (lum as f64 * (1.0 - tint) + (255.0 - 255.0 * (1.0 - tint))) as u8
        } else if tint < 0.0 {
            (lum as f64 * (1.0 + tint)) as u8
        } else {
            lum as u8
        }
    }

    /// Get the tint value applied to the color.
    ///
    /// If tint is supplied, then it is applied to the RGB value of the color to determine the final
    /// color applied.
    ///
    /// The tint value is stored as a double from -1.0 .. 1.0, where -1.0 means 100% darken and
    /// 1.0 means 100% lighten. Also, 0.0 means no change.
    ///
    /// In loading the RGB value, it is converted to HLS where HLS values are (0..HLSMAX), where
    /// HLSMAX is currently 255.
    ///
    /// Examples of how to apply tint to color:
    ///
    /// If (tint < 0)
    /// Lum' = Lum * (1.0 + tint)
    ///
    /// For example: Lum = 200; tint = -0.5; Darken 50%
    /// Lum' = 200 * (0.5) => 100
    ///
    /// For example: Lum = 200; tint = -1.0; Darken 100% (make black)
    /// Lum' = 200 * (1.0-1.0) => 0
    ///
    /// If (tint > 0)
    /// Lum' = Lum * (1.0-tint) + (HLSMAX - HLSMAX * (1.0-tint))
    ///
    /// For example: Lum = 100; tint = 0.75; Lighten 75%
    /// Lum' = 100 * (1-0.75) + (255 - 255 * (1-0.75))
    ///      = 100 * 0.25 + (255 - 255 * 0.25)
    ///      = 25 + (255 - 63) = 25 + 192 = 217
    ///
    /// For example: Lum = 100; tint = 1.0; Lighten 100% (make white)
    /// Lum' = 100 * (1-1) + (255 - 255 * (1-1))
    ///      = 100 * 0 + (255 - 255 * 0)
    ///      = 0 + (255 - 0) = 255
    fn get_tint(&self) -> f64;

    /// Set the tint value applied to the color.
    ///
    /// If tint is supplied, then it is applied to the RGB value of the color to determine the final
    /// color applied.
    ///
    /// The tint value is stored as a double from -1.0 .. 1.0, where -1.0 means 100% darken and
    /// 1.0 means 100% lighten. Also, 0.0 means no change.
    ///
    /// In loading the RGB value, it is converted to HLS where HLS values are (0..HLSMAX), where
    /// HLSMAX is currently 255.
    ///
    /// # Arguments
    /// * `tint` - Tint value (-1.0 to 1.0)
    fn set_tint(&mut self, tint: f64);
}
