pub mod print_setup_constants {
    /// Printer's default paper size
    const PRINTER_DEFAULT_PAPERSIZE: i16 = 0;
    /// US Letter 8 1/2 x 11 in
    const LETTER_PAPERSIZE: i16 = 1;
    /// US Letter Small 8 1/2 x 11 in
    const LETTER_SMALL_PAGESIZE: i16 = 2;
    /// US Tabloid 11 x 17 in
    const TABLOID_PAPERSIZE: i16 = 3;
    /// US Ledger 17 x 11 in
    const LEDGER_PAPERSIZE: i16 = 4;
    /// US Legal 8 1/2 x 14 in
    const LEGAL_PAPERSIZE: i16 = 5;
    /// US Statement 5 1/2 x 8 1/2 in
    const STATEMENT_PAPERSIZE: i16 = 6;
    /// US Executive 7 1/4 x 10 1/2 in
    const EXECUTIVE_PAPERSIZE: i16 = 7;
    /// A3 - 297x420 mm
    const A3_PAPERSIZE: i16 = 8;
    /// A4 - 210x297 mm
    const A4_PAPERSIZE: i16 = 9;
    /// A4 Small - 210x297 mm
    const A4_SMALL_PAPERSIZE: i16 = 10;
    /// A5 - 148x210 mm
    const A5_PAPERSIZE: i16 = 11;
    /// B4 (JIS) 250x354 mm
    const B4_PAPERSIZE: i16 = 12;
    /// B5 (JIS) 182x257 mm
    const B5_PAPERSIZE: i16 = 13;
    /// Folio 8 1/2 x 13 in
    const FOLIO8_PAPERSIZE: i16 = 14;
    /// Quarto 215x275 mm
    const QUARTO_PAPERSIZE: i16 = 15;
    /// 10 x 14 in
    const TEN_BY_FOURTEEN_PAPERSIZE: i16 = 16;
    /// 11 x 17 in
    const ELEVEN_BY_SEVENTEEN_PAPERSIZE: i16 = 17;
    /// US Note 8 1/2 x 11 in
    const NOTE8_PAPERSIZE: i16 = 18;
    /// US Envelope #9 3 7/8 x 8 7/8
    const ENVELOPE_9_PAPERSIZE: i16 = 19;
    /// US Envelope #10 4 1/8 x 9 1/2
    const ENVELOPE_10_PAPERSIZE: i16 = 20;
    /// Envelope DL 110x220 mm
    const ENVELOPE_DL_PAPERSIZE: i16 = 27;
    /// Envelope C5 162x229 mm
    const ENVELOPE_CS_PAPERSIZE: i16 = 28;
    /// Envelope C5 162x229 mm
    const ENVELOPE_C5_PAPERSIZE: i16 = 28;
    /// Envelope C3 324x458 mm
    const ENVELOPE_C3_PAPERSIZE: i16 = 29;
    /// Envelope C4 229x324 mm
    const ENVELOPE_C4_PAPERSIZE: i16 = 30;
    /// Envelope C6 114x162 mm
    const ENVELOPE_C6_PAPERSIZE: i16 = 31;
    /// Envelope Monarch
    const ENVELOPE_MONARCH_PAPERSIZE: i16 = 37;
    /// A4 Extra - 9.27 x 12.69 in
    const A4_EXTRA_PAPERSIZE: i16 = 53;
    /// A4 Transverse - 210x297 mm
    const A4_TRANSVERSE_PAPERSIZE: i16 = 55;
    /// A4 Plus - 210x330 mm
    const A4_PLUS_PAPERSIZE: i16 = 60;
    /// US Letter Rotated 11 x 8 1/2 in
    const LETTER_ROTATED_PAPERSIZE: i16 = 75;
    /// A4 Rotated - 297x210 mm
    const A4_ROTATED_PAPERSIZE: i16 = 77;
}

/// Represents print setup settings for a worksheet.
pub trait PrintSetup {
    /// Set the paper size.
    ///
    /// # Arguments
    /// * `size` - The paper size.
    fn set_paper_size(&mut self, size: i16);

    /// Set the scale.
    ///
    /// # Arguments
    /// * `scale` - The scale to use.
    fn set_scale(&mut self, scale: i16);

    /// Set the page numbering start.
    ///
    /// # Arguments
    /// * `start` - The page numbering start.
    fn set_page_start(&mut self, start: i16);

    /// Set the number of pages wide to fit the sheet in.
    ///
    /// # Arguments
    /// * `width` - The number of pages.
    fn set_fit_width(&mut self, width: i16);

    /// Set the number of pages high to fit the sheet in.
    ///
    /// # Arguments
    /// * `height` - The number of pages.
    fn set_fit_height(&mut self, height: i16);

    /// Set whether to go left to right or top down in ordering.
    ///
    /// # Arguments
    /// * `ltor` - Left to right.
    fn set_left_to_right(&mut self, ltor: bool);

    /// Set whether to print in landscape.
    ///
    /// # Arguments
    /// * `ls` - Landscape.
    fn set_landscape(&mut self, ls: bool);

    /// Valid settings. I'm not for sure.
    ///
    /// # Arguments
    /// * `valid` - Valid.
    fn set_valid_settings(&mut self, valid: bool);

    /// Set whether it is black and white.
    ///
    /// # Arguments
    /// * `mono` - Black and white.
    fn set_no_color(&mut self, mono: bool);

    /// Set whether it is in draft mode.
    ///
    /// # Arguments
    /// * `d` - Draft.
    fn set_draft(&mut self, d: bool);

    /// Print the include notes.
    ///
    /// # Arguments
    /// * `printnotes` - Print the notes.
    fn set_notes(&mut self, printnotes: bool);

    /// Set no orientation. ?
    ///
    /// # Arguments
    /// * `orientation` - Orientation.
    fn set_no_orientation(&mut self, orientation: bool);

    /// Set whether to use page start.
    ///
    /// # Arguments
    /// * `page` - Use page start.
    fn set_use_page(&mut self, page: bool);

    /// Sets the horizontal resolution.
    ///
    /// # Arguments
    /// * `resolution` - Horizontal resolution.
    fn set_h_resolution(&mut self, resolution: i16);

    /// Sets the vertical resolution.
    ///
    /// # Arguments
    /// * `resolution` - Vertical resolution.
    fn set_v_resolution(&mut self, resolution: i16);

    /// Sets the header margin.
    ///
    /// # Arguments
    /// * `headermargin` - Header margin.
    fn set_header_margin(&mut self, headermargin: f64);

    /// Sets the footer margin.
    ///
    /// # Arguments
    /// * `footermargin` - Footer margin.
    fn set_footer_margin(&mut self, footermargin: f64);

    /// Sets the number of copies.
    ///
    /// # Arguments
    /// * `copies` - Number of copies.
    fn set_copies(&mut self, copies: i16);

    /// Returns the paper size.
    ///
    /// # Returns
    /// Paper size.
    fn get_paper_size(&self) -> i16;

    /// Returns the scale.
    ///
    /// # Returns
    /// Scale.
    fn get_scale(&self) -> i16;

    /// Returns the page start.
    ///
    /// # Returns
    /// Page start.
    fn get_page_start(&self) -> i16;

    /// Returns the number of pages wide to fit sheet in.
    ///
    /// # Returns
    /// Number of pages wide to fit sheet in.
    fn get_fit_width(&self) -> i16;

    /// Returns the number of pages high to fit the sheet in.
    ///
    /// # Returns
    /// Number of pages high to fit the sheet in.
    fn get_fit_height(&self) -> i16;

    /// Returns the left to right print order.
    ///
    /// # Returns
    /// Left to right print order.
    fn get_left_to_right(&self) -> bool;

    /// Returns the landscape mode.
    ///
    /// # Returns
    /// Landscape mode.
    fn get_landscape(&self) -> bool;

    /// Returns the valid settings.
    ///
    /// # Returns
    /// Valid settings.
    fn get_valid_settings(&self) -> bool;

    /// Returns the black and white setting.
    ///
    /// # Returns
    /// Black and white setting.
    fn get_no_color(&self) -> bool;

    /// Returns the draft mode.
    ///
    /// # Returns
    /// Draft mode.
    fn get_draft(&self) -> bool;

    /// Returns the print notes.
    ///
    /// # Returns
    /// Print notes.
    fn get_notes(&self) -> bool;

    /// Returns the no orientation.
    ///
    /// # Returns
    /// No orientation.
    fn get_no_orientation(&self) -> bool;

    /// Returns the use page numbers.
    ///
    /// # Returns
    /// Use page numbers.
    fn get_use_page(&self) -> bool;

    /// Returns the horizontal resolution.
    ///
    /// # Returns
    /// Horizontal resolution.
    fn get_h_resolution(&self) -> i16;

    /// Returns the vertical resolution.
    ///
    /// # Returns
    /// Vertical resolution.
    fn get_v_resolution(&self) -> i16;

    /// Returns the header margin.
    ///
    /// # Returns
    /// Header margin.
    fn get_header_margin(&self) -> f64;

    /// Returns the footer margin.
    ///
    /// # Returns
    /// Footer margin.
    fn get_footer_margin(&self) -> f64;

    /// Returns the number of copies.
    ///
    /// # Returns
    /// Number of copies.
    fn get_copies(&self) -> i16;
}
