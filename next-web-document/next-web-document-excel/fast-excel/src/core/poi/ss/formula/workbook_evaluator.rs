use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use tracing::{debug, error, warn};

use crate::core::poi::ss::formula::evaluation_name::EvaluationName;
use crate::core::poi::ss::formula::ptg::name_ptg::NamePtg;
use crate::core::poi::ss::formula::ptg::{Ptg, PtgExt};

/// Evaluates formula cells.
/// For performance reasons, this class keeps a cache of all previously calculated intermediate
/// cell values.  Be sure to call `clear_all_cached_result_values()` if any workbook cells are changed between
/// calls to evaluate methods on this class.
pub struct WorkbookEvaluator {
    workbook: Arc<dyn EvaluationWorkbook>,
    cache: Arc<RwLock<EvaluationCache>>,
    workbook_idx: usize,
    evaluation_listener: Option<Arc<dyn IEvaluationListener>>,
    sheet_indexes_by_sheet: RwLock<HashMap<usize, usize>>, // Using sheet ID as key
    sheet_indexes_by_name: RwLock<HashMap<String, usize>>,
    // collaborating_workbook_environment: Arc<RwLock<CollaboratingWorkbooksEnvironment>>,
    stability_classifier: Option<Arc<dyn IStabilityClassifier>>,
    udf_finder: Option<Arc<dyn AggregatingUDFFinder>>,
    ignore_missing_workbooks: bool,
    debug_evaluation_output_for_next_eval: bool,
}

impl WorkbookEvaluator {
    /// Create a new WorkbookEvaluator
    ///
    /// # Arguments
    /// * `workbook` - The evaluation workbook
    /// * `stability_classifier` - Stability classifier for caching decisions
    /// * `udf_finder` - User-defined function finder (pass `None` for default)
    pub fn new(
        workbook: Arc<dyn EvaluationWorkbook>,
        stability_classifier: Option<Arc<dyn IStabilityClassifier>>,
        udf_finder: Option<Arc<dyn AggregatingUDFFinder>>,
    ) -> Self {
        Self::with_listener(workbook, None, stability_classifier, udf_finder)
    }

    /// Create a new WorkbookEvaluator with evaluation listener
    pub fn with_listener(
        workbook: Arc<dyn EvaluationWorkbook>,
        evaluation_listener: Option<Arc<dyn IEvaluationListener>>,
        stability_classifier: Option<Arc<dyn IStabilityClassifier>>,
        udf_finder: Option<Arc<dyn AggregatingUDFFinder>>,
    ) -> Self {
        WorkbookEvaluator {
            workbook: workbook.clone(),
            cache: Arc::new(RwLock::new(EvaluationCache::new(
                evaluation_listener.clone(),
            ))),
            workbook_idx: 0,
            evaluation_listener,
            sheet_indexes_by_sheet: RwLock::new(HashMap::new()),
            sheet_indexes_by_name: RwLock::new(HashMap::new()),
            // collaborating_workbook_environment: Arc::new(RwLock::new(
            //     CollaboratingWorkbooksEnvironment::empty(),
            // )),
            stability_classifier,
            udf_finder: if let Some(finder) = udf_finder {
                Some(finder)
            } else if let Some(default) = workbook.get_udf_finder() {
                Some(default)
            } else {
                None
            },
            ignore_missing_workbooks: false,
            debug_evaluation_output_for_next_eval: false,
        }
    }

    /// Get sheet name for debug use
    pub fn get_sheet_name(&self, sheet_index: usize) -> Option<String> {
        self.workbook.get_sheet_name(sheet_index)
    }

    /// Get evaluation sheet
    pub fn get_sheet(&self, sheet_index: usize) -> Option<Arc<dyn EvaluationSheet>> {
        self.workbook.get_sheet(sheet_index)
    }

    /// Get the evaluation workbook
    pub fn get_workbook(&self) -> Arc<dyn EvaluationWorkbook> {
        self.workbook.clone()
    }

    /// Get name by name and sheet index
    pub fn get_name(&self, name: &str, sheet_index: usize) -> Option<Arc<dyn EvaluationName>> {
        // self.workbook.get_name(name, sheet_index)
        todo!()
    }

    /// Attach to collaborating workbooks environment
    pub fn attach_to_environment(
        &mut self,
        // collaborating_workbooks_environment: Arc<RwLock<CollaboratingWorkbooksEnvironment>>,
        cache: Arc<RwLock<EvaluationCache>>,
        workbook_idx: usize,
    ) {
        // self.collaborating_workbook_environment = collaborating_workbooks_environment;
        self.cache = cache;
        self.workbook_idx = workbook_idx;
    }

    /// Get evaluation listener
    pub fn get_evaluation_listener(&self) -> Option<Arc<dyn IEvaluationListener>> {
        self.evaluation_listener.clone()
    }

    /// Clear all cached result values
    /// Should be called whenever there are changes to input cells in the evaluated workbook.
    pub fn clear_all_cached_result_values(&self) {
        self.cache.write().unwrap().clear();
        self.sheet_indexes_by_sheet.write().unwrap().clear();
        self.workbook.clear_all_cached_result_values();
    }

    /// Notify update cell
    pub fn notify_update_cell(&self, cell: &dyn EvaluationCell) {
        let sheet_index = self.get_sheet_index_by_cell(cell);
        self.cache
            .write()
            .unwrap()
            .notify_update_cell(self.workbook_idx, sheet_index, cell);
    }

    /// Notify delete cell
    pub fn notify_delete_cell(&self, cell: &dyn EvaluationCell) {
        let sheet_index = self.get_sheet_index_by_cell(cell);
        self.cache
            .write()
            .unwrap()
            .notify_delete_cell(self.workbook_idx, sheet_index, cell);
    }

    fn get_sheet_index_by_cell(&self, cell: &dyn EvaluationCell) -> usize {
        let sheet = cell.get_sheet();
        // Simplified implementation - would need proper sheet identification
        0
    }

    /// Evaluate a formula cell
    pub fn evaluate(&self, src_cell: &dyn EvaluationCell) -> Result<Arc<dyn ValueEval>, String> {
        let sheet_index = 0; // Simplified
        let tracker = EvaluationTracker::new(self.cache.clone());

        self.evaluate_any(
            src_cell,
            sheet_index,
            src_cell.get_row_index(),
            src_cell.get_column_index(),
            tracker,
        )
    }

    /// Get sheet index by name (case-insensitive)
    pub fn get_sheet_index(&self, sheet_name: &str) -> Option<usize> {
        // Check cache first
        {
            let cache = self.sheet_indexes_by_name.read().unwrap();
            if let Some(&idx) = cache.get(sheet_name) {
                return Some(idx);
            }
        }

        // Get from workbook
        let idx = self.workbook.get_sheet_index(sheet_name);
        if let Some(idx) = idx {
            self.sheet_indexes_by_name
                .write()
                .unwrap()
                .insert(sheet_name.to_string(), idx);
            Some(idx)
        } else {
            None
        }
    }

    /// Evaluate any cell (formula or non-formula)
    fn evaluate_any(
        &self,
        src_cell: &dyn EvaluationCell,
        sheet_index: usize,
        row_index: usize,
        column_index: usize,
        tracker: EvaluationTracker,
    ) -> Result<Arc<dyn ValueEval>, String> {
        // Check if cell dependency should be recorded
        let should_cell_dependency_be_recorded = self
            .stability_classifier
            .as_ref()
            .map_or(true, |classifier| {
                !classifier.is_cell_final(sheet_index, row_index, column_index)
            });

        // Handle non-formula cells
        if src_cell.get_cell_type() != CellType::Formula {
            let result = Self::get_value_from_non_formula_cell(src_cell);
            if should_cell_dependency_be_recorded {
                tracker.accept_plain_value_dependency(
                    self.workbook.clone(),
                    self.workbook_idx,
                    sheet_index,
                    row_index,
                    column_index,
                    result.clone(),
                );
            }
            return Ok(result);
        }

        // Handle formula cells
        let cache_entry = self
            .cache
            .write()
            .unwrap()
            .get_or_create_formula_cell_entry(src_cell);

        if should_cell_dependency_be_recorded || cache_entry.is_input_sensitive() {
            tracker.accept_formula_dependency(cache_entry.clone());
        }

        let result = if cache_entry.get_value().is_none() {
            // Check for circular reference
            if !tracker.start_evaluate(cache_entry.clone()) {
                return Ok(Arc::new(ErrorEval::CircularRefError));
            }

            let result = self.evaluate_formula_cell(
                src_cell,
                sheet_index,
                row_index,
                column_index,
                &tracker,
            )?;

            tracker.update_cache_result(result.clone());
            tracker.end_evaluate(cache_entry.clone());

            result
        } else {
            // Cache hit
            if let Some(listener) = &self.evaluation_listener {
                listener.on_cache_hit(
                    sheet_index,
                    row_index,
                    column_index,
                    cache_entry.get_value().as_ref().unwrap(),
                );
            }
            cache_entry.get_value().unwrap().clone()
        };

        Ok(result)
    }

    /// Get value from non-formula cell
    fn get_value_from_non_formula_cell(cell: &dyn EvaluationCell) -> Arc<dyn ValueEval> {
        match cell.get_cell_type() {
            CellType::Numeric => Arc::new(NumberEval::new(cell.get_numeric_cell_value())),
            CellType::String => Arc::new(StringEval::new(cell.get_string_cell_value())),
            CellType::Boolean => Arc::new(BoolEval::new(cell.get_boolean_cell_value())),
            CellType::Blank => Arc::new(BlankEval),
            CellType::Error => Arc::new(ErrorEval::new(cell.get_error_cell_value())),
            CellType::Formula => panic!("Unexpected formula cell type"),
        }
    }

    /// Evaluate formula cell
    fn evaluate_formula_cell(
        &self,
        src_cell: &dyn EvaluationCell,
        sheet_index: usize,
        row_index: usize,
        column_index: usize,
        tracker: &EvaluationTracker,
    ) -> Result<Arc<dyn ValueEval>, String> {
        // let ptgs = self.workbook.get_formula_tokens(src_cell);
        // let ec = OperationEvaluationContext::new(
        //     self,
        //     self.workbook.clone(),
        //     sheet_index,
        //     row_index,
        //     column_index,
        //     tracker.clone(),
        // );

        // // Notify evaluation start
        // if let Some(listener) = &self.evaluation_listener {
        //     // Would need cache entry reference
        // }

        // let result = self.evaluate_formula(&ec, &ptgs)?;

        // // Notify evaluation end
        // if let Some(listener) = &self.evaluation_listener {
        //     // Would need cache entry reference
        // }

        // Ok(result)
        //
        todo!()
    }

    /// Evaluate formula from PTG tokens
    fn evaluate_formula(
        &self,
        ec: &OperationEvaluationContext,
        ptgs: &[&dyn PtgExt],
    ) -> Result<Arc<dyn ValueEval>, String> {
        // let mut stack = Vec::new();

        // for (i, ptg) in ptgs.iter().enumerate() {
        //     // Handle AttrPtg special cases
        //     if let Some(attr_ptg) = ptg.as_any().downcast_ref::<AttrPtg>() {
        //         if attr_ptg.is_sum() {
        //             // Replace with SUM function
        //             // Continue with FuncVarPtg::SUM
        //             continue;
        //         }
        //         if attr_ptg.is_optimized_choose() {
        //             // Handle CHOOSE optimization
        //             continue;
        //         }
        //         if attr_ptg.is_optimized_if() {
        //             // Handle IF optimization
        //             continue;
        //         }
        //         if attr_ptg.is_skip() {
        //             // Handle skip
        //             continue;
        //         }
        //     }

        //     // Handle control PTGs
        //     if ptg.is_control_ptg() {
        //         continue;
        //     }

        //     // Handle specific PTG types
        //     if ptg.is_union_ptg() {
        //         let v2 = stack.pop().unwrap();
        //         let v1 = stack.pop().unwrap();
        //         stack.push(Arc::new(RefListEval::new(v1, v2)));
        //         continue;
        //     }

        //     let op_result = if let Some(operation_ptg) = ptg.as_any().downcast_ref::<OperationPtg>()
        //     {
        //         // Handle operation PTG
        //         let num_ops = operation_ptg.get_number_of_operands();
        //         let mut ops = Vec::with_capacity(num_ops);

        //         // Pop operands in reverse order
        //         for _ in 0..num_ops {
        //             ops.push(stack.pop().unwrap());
        //         }
        //         ops.reverse();

        //         // Check for array mode
        //         let mut area_arg = false;
        //         for op in &ops {
        //             if op.as_any().is::<AreaEval>() {
        //                 area_arg = true;
        //                 break;
        //             }
        //         }

        //         // Evaluate operation
        //         OperationEvaluatorFactory::evaluate(operation_ptg, &ops, ec)?
        //     } else {
        //         // Get eval for non-operation PTG
        //         self.get_eval_for_ptg(ptg.as_ref(), ec)?
        //     };

        //     stack.push(op_result);
        // }

        // if stack.len() != 1 {
        //     return Err("Evaluation stack not empty".to_string());
        // }

        // let value = stack.pop().unwrap();

        // let result = if ec.is_single_value() {
        //     Self::dereference_result(&*value, ec)?
        // } else {
        //     value
        // };

        // Ok(result)
        todo!()
    }

    /// Get eval for PTG
    fn get_eval_for_ptg(
        &self,
        ptg: &dyn PtgExt,
        ec: &OperationEvaluationContext,
    ) -> Result<Arc<dyn ValueEval>, String> {
        // Handle different PTG types
        // if let Some(name_ptg) = ptg.as_any().downcast_ref::<NamePtg>() {
        //     // Named ranges, macro functions
        //     let name_record = self.workbook.get_name(name_ptg);
        //     self.get_eval_for_name_record(&name_record, ec)
        // } else if let Some(int_ptg) = ptg.as_any().downcast_ref::<IntPtg>() {
        //     Ok(Arc::new(NumberEval::new(int_ptg.get_value() as f64)))
        // } else if let Some(number_ptg) = ptg.as_any().downcast_ref::<NumberPtg>() {
        //     Ok(Arc::new(NumberEval::new(number_ptg.get_value())))
        // } else if let Some(string_ptg) = ptg.as_any().downcast_ref::<StringPtg>() {
        //     Ok(Arc::new(StringEval::new(string_ptg.get_value())))
        // } else if let Some(bool_ptg) = ptg.as_any().downcast_ref::<BoolPtg>() {
        //     Ok(Arc::new(BoolEval::new(bool_ptg.get_value())))
        // } else if let Some(ref_ptg) = ptg.as_any().downcast_ref::<RefPtg>() {
        //     ec.get_ref_eval(ref_ptg.get_row(), ref_ptg.get_column())
        // } else if let Some(area_ptg) = ptg.as_any().downcast_ref::<AreaPtg>() {
        //     ec.get_area_eval(
        //         area_ptg.get_first_row(),
        //         area_ptg.get_first_column(),
        //         area_ptg.get_last_row(),
        //         area_ptg.get_last_column(),
        //     )
        // } else {
        //     Err(format!("Unexpected PTG class: {:?}", ptg))
        // }
        todo!()
    }

    /// Get eval for name record
    fn get_eval_for_name_record(
        &self,
        name_record: &dyn EvaluationName,
        ec: &OperationEvaluationContext,
    ) -> Result<Arc<dyn ValueEval>, String> {
        todo!()
        // if name_record.is_function_name() {
        //     Ok(Arc::new(FunctionNameEval::new(name_record.get_name_text())))
        // } else if name_record.has_formula() {
        //     self.evaluate_name_formula(name_record.get_name_definition(), ec)
        // } else {
        //     Err(format!(
        //         "Don't know how to evaluate name '{}'",
        //         name_record.get_name_text()
        //     ))
        // }
    }

    /// Evaluate name formula
    fn evaluate_name_formula(
        &self,
        ptgs: &[&dyn PtgExt],
        ec: &OperationEvaluationContext,
    ) -> Result<Arc<dyn ValueEval>, String> {
        if ptgs.len() == 1 {
            // Handle single PTG
            return self.get_eval_for_ptg(ptgs[0], ec);
        }

        // Create new context for formula evaluation
        let any_value_context = OperationEvaluationContext::new(
            self,
            ec.get_workbook().clone(),
            ec.get_sheet_index(),
            ec.get_row_index(),
            ec.get_column_index(),
            EvaluationTracker::new(self.cache.clone()),
            false,
        );

        self.evaluate_formula(&any_value_context, ptgs)
    }

    /// Dereference result
    fn dereference_result(
        eval_result: &dyn ValueEval,
        ec: &OperationEvaluationContext,
    ) -> Result<Arc<dyn ValueEval>, String> {
        todo!()
        // if let Some(area_eval) = eval_result.as_any().downcast_ref::<AreaEval>() {
        //     // Check if part of array formula
        //     if ec.is_part_of_array_formula() {
        //         let element =
        //             OperandResolver::get_element_from_array(area_eval, ec.get_eval_cell()?)?;
        //         Ok(element)
        //     } else {
        //         OperandResolver::get_single_value(
        //             eval_result,
        //             ec.get_row_index(),
        //             ec.get_column_index(),
        //         )
        //     }
        // } else {
        //     OperandResolver::get_single_value(
        //         eval_result,
        //         ec.get_row_index(),
        //         ec.get_column_index(),
        //     )
        // }
    }

    /// Find user-defined function
    pub fn find_user_defined_function(
        &self,
        function_name: &str,
    ) -> Option<Arc<dyn FreeRefFunction>> {
        self.udf_finder
            .as_ref()
            .and_then(|finder| finder.find_function(function_name))
    }

    /// Set ignore missing workbooks flag
    pub fn set_ignore_missing_workbooks(&mut self, ignore: bool) {
        self.ignore_missing_workbooks = ignore;
    }

    /// Get ignore missing workbooks flag
    pub fn is_ignore_missing_workbooks(&self) -> bool {
        self.ignore_missing_workbooks
    }

    /// Set debug evaluation output flag
    pub fn set_debug_evaluation_output_for_next_eval(&mut self, value: bool) {
        self.debug_evaluation_output_for_next_eval = value;
    }

    /// Get debug evaluation output flag
    pub fn is_debug_evaluation_output_for_next_eval(&self) -> bool {
        self.debug_evaluation_output_for_next_eval
    }
}

// Supporting types and traits
pub trait EvaluationWorkbook: Send + Sync {
    fn get_sheet_name(&self, sheet_index: usize) -> Option<String>;
    fn get_sheet(&self, sheet_index: usize) -> Option<Arc<dyn EvaluationSheet>>;
    fn get_name(&self, name_ptg: &NamePtg) -> Arc<dyn EvaluationName>;
    fn clear_all_cached_result_values(&self);
    fn get_formula_tokens(&self, cell: &dyn EvaluationCell) -> Vec<Ptg>;
    fn get_sheet_index(&self, sheet_name: &str) -> Option<usize>;
    fn get_udf_finder(&self) -> Option<Arc<dyn AggregatingUDFFinder>>;
}

pub trait EvaluationSheet: Send + Sync {
    fn get_cell(&self, row_index: usize, column_index: usize) -> Option<Arc<dyn EvaluationCell>>;
}

pub trait EvaluationCell: Send + Sync {
    fn get_cell_type(&self) -> CellType;
    fn get_row_index(&self) -> usize;
    fn get_column_index(&self) -> usize;
    fn get_sheet(&self) -> Arc<dyn EvaluationSheet>;
    fn get_numeric_cell_value(&self) -> f64;
    fn get_string_cell_value(&self) -> String;
    fn get_boolean_cell_value(&self) -> bool;
    fn get_error_cell_value(&self) -> u8;
    fn is_part_of_array_formula(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Numeric,
    String,
    Boolean,
    Blank,
    Error,
    Formula,
}

pub trait IEvaluationListener: Send + Sync {
    fn on_cache_hit(
        &self,
        sheet_index: usize,
        row_index: usize,
        column_index: usize,
        value: &dyn ValueEval,
    );
    fn on_start_evaluate(&self, cell: &dyn EvaluationCell, cache_entry: &FormulaCellCacheEntry);
    fn on_end_evaluate(&self, cache_entry: &FormulaCellCacheEntry, result: &dyn ValueEval);
}

pub trait IStabilityClassifier: Send + Sync {
    fn is_cell_final(&self, sheet_index: usize, row_index: usize, column_index: usize) -> bool;
}

pub trait AggregatingUDFFinder: Send + Sync {
    fn find_function(&self, name: &str) -> Option<Arc<dyn FreeRefFunction>>;
    fn add(&mut self, finder: Arc<dyn UDFFinder>);
}

pub trait UDFFinder: Send + Sync {
    fn find_function(&self, name: &str) -> Option<Arc<dyn FreeRefFunction>>;
}

pub trait FreeRefFunction: Send + Sync {
    fn evaluate(
        &self,
        args: &[Arc<dyn ValueEval>],
        ec: &OperationEvaluationContext,
    ) -> Result<Arc<dyn ValueEval>, String>;
}

pub trait ValueEval: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct NumberEval(f64);
pub struct StringEval(String);
pub struct BoolEval(bool);
pub struct BlankEval;
pub struct ErrorEval(u8);

impl ValueEval for NumberEval {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Similarly for other ValueEval implementations...

pub struct OperationEvaluationContext {
    // Context for operation evaluation
}

impl OperationEvaluationContext {
    pub fn new(
        evaluator: &WorkbookEvaluator,
        workbook: Arc<dyn EvaluationWorkbook>,
        sheet_index: usize,
        row_index: usize,
        column_index: usize,
        tracker: EvaluationTracker,
        single_value: bool,
    ) -> Self {
        // Implementation...
        Self {}
    }

    pub fn is_single_value(&self) -> bool {
        true
    }
    pub fn is_part_of_array_formula(&self) -> bool {
        false
    }
    pub fn get_workbook(&self) -> Arc<dyn EvaluationWorkbook> {
        unimplemented!()
    }
    pub fn get_sheet_index(&self) -> usize {
        0
    }
    pub fn get_row_index(&self) -> usize {
        0
    }
    pub fn get_column_index(&self) -> usize {
        0
    }
    pub fn get_eval_cell(&self) -> Result<Arc<dyn EvaluationCell>, String> {
        unimplemented!()
    }

    pub fn get_ref_eval(&self, row: usize, column: usize) -> Result<Arc<dyn ValueEval>, String> {
        // Implementation...
        Ok(Arc::new(BlankEval))
    }

    pub fn get_area_eval(
        &self,
        first_row: usize,
        first_column: usize,
        last_row: usize,
        last_column: usize,
    ) -> Result<Arc<dyn ValueEval>, String> {
        // Implementation...
        Ok(Arc::new(BlankEval))
    }
}

pub struct EvaluationTracker {
    cache: Arc<RwLock<EvaluationCache>>,
}

impl EvaluationTracker {
    pub fn new(cache: Arc<RwLock<EvaluationCache>>) -> Self {
        Self { cache }
    }

    pub fn accept_plain_value_dependency(
        &self,
        workbook: Arc<dyn EvaluationWorkbook>,
        workbook_idx: usize,
        sheet_index: usize,
        row_index: usize,
        column_index: usize,
        value: Arc<dyn ValueEval>,
    ) {
        // Implementation...
    }

    pub fn accept_formula_dependency(&self, cache_entry: Arc<FormulaCellCacheEntry>) {
        // Implementation...
    }

    pub fn start_evaluate(&self, cache_entry: Arc<FormulaCellCacheEntry>) -> bool {
        // Check for circular reference
        true
    }

    pub fn update_cache_result(&self, result: Arc<dyn ValueEval>) {
        // Implementation...
    }

    pub fn end_evaluate(&self, cache_entry: Arc<FormulaCellCacheEntry>) {
        // Implementation...
    }
}

pub struct EvaluationCache {
    // Cache implementation
}

impl EvaluationCache {
    pub fn new(listener: Option<Arc<dyn IEvaluationListener>>) -> Self {
        Self {}
    }

    pub fn clear(&mut self) {}

    pub fn notify_update_cell(
        &mut self,
        workbook_idx: usize,
        sheet_index: usize,
        cell: &dyn EvaluationCell,
    ) {
    }

    pub fn notify_delete_cell(
        &mut self,
        workbook_idx: usize,
        sheet_index: usize,
        cell: &dyn EvaluationCell,
    ) {
    }

    pub fn get_or_create_formula_cell_entry(
        &mut self,
        cell: &dyn EvaluationCell,
    ) -> Arc<FormulaCellCacheEntry> {
        Arc::new(FormulaCellCacheEntry::new())
    }
}

pub struct FormulaCellCacheEntry {
    value: Option<Arc<dyn ValueEval>>,
    input_sensitive: bool,
}

impl FormulaCellCacheEntry {
    pub fn new() -> Self {
        Self {
            value: None,
            input_sensitive: false,
        }
    }

    pub fn get_value(&self) -> &Option<Arc<dyn ValueEval>> {
        &self.value
    }

    pub fn is_input_sensitive(&self) -> bool {
        self.input_sensitive
    }
}
