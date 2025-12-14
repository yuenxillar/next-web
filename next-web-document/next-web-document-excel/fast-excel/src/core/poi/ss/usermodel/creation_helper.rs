use crate::core::{
    metadata::csv::csv_data_format::DataFormat,
    poi::{
        common::usermodel::hyperlink_type::HyperlinkType,
        ss::{
            usermodel::{
                client_anchor::ClientAnchor, extended_color::ExtendedColor,
                formula_evaluator::FormulaEvaluator, hyperlink::Hyperlink,
                rich_text_string::RichTextString,
            },
            util::{area_reference::AreaReference, cell_reference::CellReference},
        },
    },
};

/// An object that handles instantiating concrete classes of the various instances one needs for
/// HSSF and XSSF.
/// Works around a limitation in Java where we cannot have static methods on interfaces or abstract
/// classes.
/// This allows you to get the appropriate class for a given interface, without you having to worry
/// about if you're dealing with HSSF or XSSF.
pub trait CreationHelper: Send + Sync {
    /// Creates a new RichTextString instance
    ///
    /// # Arguments
    /// * `text` - The text to initialise the RichTextString with
    ///
    /// # Returns
    /// * New RichTextString instance
    fn create_rich_text_string(&self, text: &str) -> Box<dyn RichTextString>;

    /// Creates a new DataFormat instance
    ///
    /// # Returns
    /// * New DataFormat instance
    fn create_data_format(&self) -> Box<dyn DataFormat>;

    /// Creates a new Hyperlink, of the given type
    ///
    /// # Arguments
    /// * `link_type` - The type of hyperlink to create
    ///
    /// # Returns
    /// * New Hyperlink instance
    fn create_hyperlink(&self, link_type: HyperlinkType) -> Box<dyn Hyperlink>;

    /// Creates FormulaEvaluator - an object that evaluates formula cells.
    ///
    /// # Returns
    /// * A FormulaEvaluator instance
    fn create_formula_evaluator(&self) -> Box<dyn FormulaEvaluator>;

    /// Creates a XSSF-style Color object, used for extended sheet
    /// formattings and conditional formattings
    ///
    /// # Returns
    /// * New ExtendedColor instance
    fn create_extended_color(&self) -> Box<dyn ExtendedColor>;

    /// Creates a ClientAnchor. Use this object to position drawing object in a sheet
    ///
    /// # Returns
    /// * A ClientAnchor instance
    fn create_client_anchor(&self) -> Box<dyn ClientAnchor>;

    /// Creates an AreaReference.
    ///
    /// # Arguments
    /// * `reference` - Cell reference string
    ///
    /// # Returns
    /// * An AreaReference instance
    fn create_area_reference(&self, reference: &str) -> Result<AreaReference, String>;

    /// Creates an area ref from a pair of Cell References.
    ///
    /// # Arguments
    /// * `top_left` - Top left cell reference
    /// * `bottom_right` - Bottom right cell reference
    ///
    /// # Returns
    /// * An AreaReference instance
    fn create_area_reference_from_cells(
        &self,
        top_left: &CellReference,
        bottom_right: &CellReference,
    ) -> AreaReference;
}
