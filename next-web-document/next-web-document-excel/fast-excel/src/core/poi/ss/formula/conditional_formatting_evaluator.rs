use std::collections::HashMap;
use std::sync::Arc;

// use crate::core::poi::ss::formula::evaluation_conditional_format_rule::EvaluationConditionalFormatRule;
use crate::core::poi::ss::formula::workbook_evaluator::WorkbookEvaluator;
use crate::core::poi::ss::formula::workbook_evaluator_provider::WorkbookEvaluatorProvider;
use crate::core::poi::ss::usermodel::cell::Cell;
use crate::core::poi::ss::usermodel::conditional_formatting::ConditionalFormatting;
use crate::core::poi::ss::usermodel::conditional_formatting_rule::ConditionalFormattingRule;
use crate::core::poi::ss::usermodel::row::Row;
use crate::core::poi::ss::usermodel::sheet::Sheet;
use crate::core::poi::ss::usermodel::workbook::Workbook;
use crate::core::poi::ss::util::cell_reference::CellReference;

/// Evaluates Conditional Formatting constraints.
///
/// For performance reasons, this class keeps a cache of all previously evaluated rules and cells.
/// Be sure to call `clear_all_cached_formats()` if any conditional formats are modified, added, or deleted,
/// and `clear_all_cached_values()` whenever cell values change.
pub struct ConditionalFormattingEvaluator<W> {
    workbook_evaluator: Arc<WorkbookEvaluator>,
    workbook: W,

    /// All the underlying structures, for both HSSF and XSSF, repeatedly go to the raw bytes/XML for the
    /// different pieces used in the ConditionalFormatting* structures.  That's highly inefficient,
    /// and can cause significant lag when checking formats for large workbooks.
    ///
    /// Instead we need a cached version that is discarded when definitions change.
    ///
    /// Sheets don't implement equals, and since its an interface,
    /// there's no guarantee instances won't be recreated on the fly by some implementation.
    /// So we use sheet name.
    formats: HashMap<String, Vec<Arc<EvaluationConditionalFormatRule>>>,

    /// Evaluating rules for cells in their region(s) is expensive, so we want to cache them,
    /// and empty/reevaluate the cache when values change.
    ///
    /// Rule lists are in priority order, as evaluated by Excel (smallest priority # for XSSF, definition order for HSSF)
    ///
    /// CellReference implements equals().
    values: HashMap<CellReference, Vec<Arc<EvaluationConditionalFormatRule>>>,
}

impl<S> ConditionalFormattingEvaluator<S> {
    pub fn new(workbook: Arc<dyn Workbook>, provider: Arc<dyn WorkbookEvaluatorProvider>) -> Self {
        ConditionalFormattingEvaluator {
            workbook_evaluator: provider.get_workbook_evaluator(),
            workbook,
            formats: Default::default(),
            values: Default::default(),
        }
    }

    /// Get the workbook evaluator
    pub fn get_workbook_evaluator(&self) -> Arc<WorkbookEvaluator> {
        self.workbook_evaluator.clone()
    }

    /// Call this whenever rules are added, reordered, or removed, or a rule formula is changed
    /// (not the formula inputs but the formula expression itself)
    pub fn clear_all_cached_formats(&self) {
        let mut formats = self.formats.write().unwrap();
        formats.clear();
    }

    /// Call this whenever cell values change in the workbook, so conditional formats are re-evaluated
    /// for all cells.
    ///
    /// TODO: eventually this should work like `EvaluationCache::notify_update_cell`
    /// and only clear values that need recalculation based on the formula dependency tree.
    pub fn clear_all_cached_values(&self) {
        let mut values = self.values.write().unwrap();
        values.clear();
    }

    /// Lazy load by sheet since reading can be expensive
    ///
    /// # Arguments
    /// * `sheet` - The sheet to look at
    ///
    /// # Returns
    /// * Unmodifiable list of rules
    fn get_rules(&self, sheet: &dyn Sheet) -> Vec<Arc<EvaluationConditionalFormatRule>> {
        let sheet_name = sheet.get_sheet_name().to_string();

        // Check cache with read lock first
        {
            let formats = self.formats.read().unwrap();
            if let Some(rules) = formats.get(&sheet_name) {
                return rules.clone();
            }
        }

        // Compute rules if not in cache
        let scf = sheet.get_sheet_conditional_formatting();
        let count = scf.get_num_conditional_formattings();
        let mut rules = Vec::with_capacity(count);

        for i in 0..count {
            let f = scf.get_conditional_formatting_at(i);
            let regions = f.get_formatting_ranges();

            for r in 0..f.get_number_of_rules() {
                let rule = f.get_rule(r);
                let eval_rule = EvaluationConditionalFormatRule::new(
                    self.workbook_evaluator.clone(),
                    sheet,
                    f.clone(),
                    i,
                    rule.clone(),
                    r,
                    regions.clone(),
                );
                rules.push(Arc::new(eval_rule));
            }
        }

        // Need them in formatting and priority order so logic works right
        rules.sort();

        // Store in cache
        let mut formats = self.formats.write().unwrap();
        formats.insert(sheet_name, rules.clone());

        rules
    }

    /// This checks all applicable `ConditionalFormattingRule`s for the cell's sheet,
    /// in defined "priority" order, returning the matches if any.  This is a property currently
    /// not exposed from `CTCfRule` in `XSSFConditionalFormattingRule`.
    ///
    /// Most cells will have zero or one applied rule, but it is possible to define multiple rules
    /// that apply at the same time to the same cell, thus the List result.
    ///
    /// Note that to properly apply conditional rules, care must be taken to offset the base
    /// formula by the relative position of the current cell, or the wrong value is checked.
    /// This is handled by `WorkbookEvaluator::evaluate`.
    ///
    /// If the cell exists and is a formula cell, its cached value may be used for rule evaluation, so
    /// make sure it is up to date.  If values have changed, it is best to call
    /// `FormulaEvaluator::evaluate_formula_cell` or `FormulaEvaluator::evaluate_all` first,
    /// or the wrong conditional results may be returned.
    ///
    /// # Arguments
    /// * `cell_ref` - NOTE: if no sheet name is specified, this uses the workbook active sheet
    ///
    /// # Returns
    /// * Unmodifiable List of `EvaluationConditionalFormatRule`s that apply to the current cell value,
    ///   in priority order, as evaluated by Excel (smallest priority # for XSSF, definition order for HSSF),
    ///   or empty vector if none apply
    pub fn get_conditional_formatting_for_cell(
        &self,
        cell_ref: &CellReference,
    ) -> Vec<Arc<EvaluationConditionalFormatRule>> {
        // Check cache first
        {
            let values = self.values.read().unwrap();
            if let Some(rules) = values.get(cell_ref) {
                return rules.clone();
            }
        }

        // Compute and cache them
        let mut rules = Vec::new();

        let sheet_name = cell_ref.get_sheet_name();
        let sheet = if let Some(name) = sheet_name {
            self.workbook.get_sheet(name)
        } else {
            self.workbook
                .get_sheet_at(self.workbook.get_active_sheet_index())
        };

        let sheet_ref = match sheet {
            Some(s) => s,
            None => return Vec::new(),
        };

        /*
         * Per Excel help:
         * https://support.office.com/en-us/article/Manage-conditional-formatting-rule-precedence-e09711a3-48df-4bcb-b82c-9d8b8b22463d#__toc269129417
         * stopIfTrue is true for all rules from HSSF files, and an explicit value for XSSF files.
         * thus the explicit ordering of the rule lists in `get_formatting_rules_for_sheet`
         */
        let mut stop_if_true = false;
        for rule in self.get_rules(sheet_ref) {
            if stop_if_true {
                continue; // a previous rule matched and wants no more evaluations
            }

            if rule.matches(cell_ref) {
                rules.push(rule.clone());
                stop_if_true = rule.get_rule().get_stop_if_true();
            }
        }

        rules.sort();

        // Store in cache
        let mut values = self.values.write().unwrap();
        values.insert(cell_ref.clone(), rules.clone());

        rules
    }

    /// This checks all applicable `ConditionalFormattingRule`s for the cell's sheet,
    /// in defined "priority" order, returning the matches if any.  This is a property currently
    /// not exposed from `CTCfRule` in `XSSFConditionalFormattingRule`.
    ///
    /// Most cells will have zero or one applied rule, but it is possible to define multiple rules
    /// that apply at the same time to the same cell, thus the List result.
    ///
    /// Note that to properly apply conditional rules, care must be taken to offset the base
    /// formula by the relative position of the current cell, or the wrong value is checked.
    /// This is handled by `WorkbookEvaluator::evaluate`.
    ///
    /// If the cell exists and is a formula cell, its cached value may be used for rule evaluation, so
    /// make sure it is up to date.  If values have changed, it is best to call
    /// `FormulaEvaluator::evaluate_formula_cell` or `FormulaEvaluator::evaluate_all` first,
    /// or the wrong conditional results may be returned.
    ///
    /// # Arguments
    /// * `cell` - The cell to look for
    ///
    /// # Returns
    /// * Unmodifiable List of `EvaluationConditionalFormatRule`s that apply to the current cell value,
    ///   in priority order, as evaluated by Excel (smallest priority # for XSSF, definition order for HSSF),
    ///   or empty vector if none apply
    pub fn get_conditional_formatting_for_cell_ref(
        &self,
        cell: &dyn Cell,
    ) -> Vec<Arc<EvaluationConditionalFormatRule>> {
        let cell_ref = Self::get_ref(cell);
        self.get_conditional_formatting_for_cell(&cell_ref)
    }

    /// Create a CellReference from a Cell
    pub fn get_ref(cell: &dyn Cell) -> CellReference {
        CellReference::from_sheet_and_indices(
            cell.get_sheet().get_sheet_name().to_string(),
            cell.get_row_index(),
            cell.get_column_index(),
            false,
            false,
        )
    }

    /// Retrieve all formatting rules for the sheet with the given name.
    ///
    /// # Arguments
    /// * `sheet_name` - The name of the sheet to look at
    ///
    /// # Returns
    /// * Unmodifiable list of all Conditional format rules for the given sheet, if any
    pub fn get_format_rules_for_sheet_name(
        &self,
        sheet_name: &str,
    ) -> Vec<Arc<EvaluationConditionalFormatRule>> {
        match self.workbook.get_sheet(sheet_name) {
            Some(sheet) => self.get_format_rules_for_sheet(sheet),
            None => Vec::new(),
        }
    }

    /// Retrieve all formatting rules for the given sheet.
    ///
    /// # Arguments
    /// * `sheet` - The sheet to look at
    ///
    /// # Returns
    /// * Unmodifiable list of all Conditional format rules for the given sheet, if any
    pub fn get_format_rules_for_sheet(
        &self,
        sheet: &dyn Sheet,
    ) -> Vec<Arc<EvaluationConditionalFormatRule>> {
        self.get_rules(sheet)
    }

    /// Conditional formatting rules can apply only to cells in the sheet to which they are attached.
    /// The POI data model does not have a back-reference to the owning sheet, so it must be passed in separately.
    ///
    /// We could overload this with convenience methods taking a sheet name and sheet index as well.
    ///
    /// # Arguments
    /// * `sheet` - containing the rule
    /// * `conditional_formatting_index` - of the `ConditionalFormatting` instance in the sheet's array
    /// * `rule_index` - of the `ConditionalFormattingRule` instance within the `ConditionalFormatting`
    ///
    /// # Returns
    /// * Unmodifiable List of all cells in the rule's region matching the rule's condition
    pub fn get_matching_cells_by_index(
        &self,
        sheet: &dyn Sheet,
        conditional_formatting_index: usize,
        rule_index: usize,
    ) -> Vec<Arc<dyn Cell>> {
        for rule in self.get_rules(sheet) {
            if rule.get_sheet().get_sheet_name() == sheet.get_sheet_name()
                && rule.get_formatting_index() == conditional_formatting_index
                && rule.get_rule_index() == rule_index
            {
                return self.get_matching_cells(&rule);
            }
        }
        Vec::new()
    }

    /// Retrieve all cells where the given formatting rule evaluates to true.
    ///
    /// # Arguments
    /// * `rule` - The rule to look at
    ///
    /// # Returns
    /// * Unmodifiable List of all cells in the rule's region matching the rule's condition
    pub fn get_matching_cells(&self, rule: &EvaluationConditionalFormatRule) -> Vec<Arc<dyn Cell>> {
        let mut cells = Vec::new();
        let sheet = rule.get_sheet();

        for region in rule.get_regions() {
            for r in region.get_first_row()..=region.get_last_row() {
                let row = sheet.get_row(r);
                if row.is_none() {
                    continue; // no cells to check
                }
                let row = row.unwrap();

                for c in region.get_first_column()..=region.get_last_column() {
                    let cell = row.get_cell(c);
                    if cell.is_none() {
                        continue;
                    }
                    let cell = cell.unwrap();

                    let cell_rules = self.get_conditional_formatting_for_cell_ref(&*cell);
                    if cell_rules.iter().any(|cr| Arc::ptr_eq(cr, rule)) {
                        cells.push(cell);
                    }
                }
            }
        }
        cells
    }
}
