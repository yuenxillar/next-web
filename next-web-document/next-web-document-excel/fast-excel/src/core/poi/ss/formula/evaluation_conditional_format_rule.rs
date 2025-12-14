// use std::cmp::Ordering;
// use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
// use std::hash::{Hash, Hasher};
// use std::sync::Arc;

// use next_web_core::util::locale::Locale;

// use crate::core::poi::ss::formula::operator::OperatorEnum;
// use crate::core::poi::ss::formula::workbook_evaluator::{
//     CellType, NumberEval, ValueEval, WorkbookEvaluator,
// };
// use crate::core::poi::ss::usermodel::cell::Cell;
// use crate::core::poi::ss::usermodel::condition_filter_type::ConditionFilterType;
// use crate::core::poi::ss::usermodel::condition_type::ConditionType;
// use crate::core::poi::ss::usermodel::conditional_formatting::ConditionalFormatting;
// use crate::core::poi::ss::usermodel::conditional_formatting_rule::ConditionalFormattingRule;
// use crate::core::poi::ss::usermodel::excel_number_format::ExcelNumberFormat;
// use crate::core::poi::ss::util::cell_range_address::CellRangeAddress;
// use crate::core::poi::ss::util::cell_reference::CellReference;

// /// Abstracted and cached version of a Conditional Format rule for use with a
// /// `ConditionalFormattingEvaluator`. This references a rule, its owning
// /// `ConditionalFormatting`, its priority order (lower index = higher priority in Excel),
// /// and the information needed to evaluate the rule for a given cell.
// ///
// /// Having this all combined and cached avoids repeated access calls to the
// /// underlying structural objects, XSSF CT* objects and HSSF raw byte structures.
// /// Those objects can be referenced from here. This object will be out of sync if
// /// anything modifies the referenced structures' evaluation properties.
// ///
// /// The assumption is that consuming applications will read the display properties once and
// /// create whatever style objects they need, caching those at the application level.
// /// Thus this class only caches values needed for evaluation, not display.
// pub(crate) struct EvaluationConditionalFormatRule {
//     workbook_evaluator: Arc<WorkbookEvaluator>,
//     sheet: Arc<dyn Sheet>,
//     formatting: Arc<dyn ConditionalFormatting>,
//     rule: Arc<dyn ConditionalFormattingRule>,

//     /// Cached values
//     regions: Vec<CellRangeAddress>,
//     top_left_region: Option<CellRangeAddress>,

//     /// Depending on the rule type, it may want to know about certain values in the region when evaluating `matches()`,
//     /// such as top 10, unique, duplicate, average, etc. This collection stores those if needed so they are not repeatedly calculated
//     meaningful_region_values: HashMap<CellRangeAddress, HashSet<ValueAndFormat>>,

//     priority: i32,
//     formatting_index: i32,
//     rule_index: i32,
//     formula1: Option<String>,
//     formula2: Option<String>,
//     text: Option<String>,
//     /// Cached for performance, used with cell text comparisons, which are case insensitive and need to be Locale aware (contains, starts with, etc.)
//     lower_text: Option<String>,

//     operator: OperatorEnum,
//     type_: ConditionType,
//     /// Cached for performance, to avoid reading the XMLBean every time a conditionally formatted cell is rendered
//     number_format: Option<ExcelNumberFormat>,
//     /// Cached for performance, used to format numeric cells for string comparisons. See Bug #61764 for explanation
//     decimal_text_format: Arc<DecimalFormat>,
// }

// impl EvaluationConditionalFormatRule {
//     /// Creates a new evaluation conditional format rule.
//     ///
//     /// # Arguments
//     /// * `formatting_index` - for priority, zero based
//     /// * `rule_index` - for priority, zero based, if this is an HSSF rule. Unused for XSSF rules
//     /// * `regions` - could be read from formatting, but every call creates new objects in a new array.
//     ///               this allows calling it once per formatting instance, and re-using the array.
//     pub(crate) fn new(
//         workbook_evaluator: Arc<WorkbookEvaluator>,
//         sheet: Arc<dyn Sheet>,
//         formatting: Arc<ConditionalFormatting>,
//         formatting_index: i32,
//         rule: Arc<ConditionalFormattingRule>,
//         rule_index: i32,
//         regions: Vec<CellRangeAddress>,
//     ) -> Self {
//         let mut top_left_region: Option<CellRangeAddress> = None;

//         for region in &regions {
//             if top_left_region.is_none() {
//                 top_left_region = Some(region.clone());
//             } else if region.get_first_column()
//                 < top_left_region.as_ref().unwrap().get_first_column()
//                 || region.get_first_row() < top_left_region.as_ref().unwrap().get_first_row()
//             {
//                 top_left_region = Some(region.clone());
//             }
//         }

//         let formula1 = rule.get_formula1();
//         let formula2 = rule.get_formula2();
//         let text = rule.get_text();
//         let lower_text = text.as_ref().map(|t| t.to_lowercase());

//         let number_format = rule.get_number_format();

//         let operator = OperatorEnum::from(rule.get_comparison_operation());
//         let type_ = rule.get_condition_type();

//         let decimal_text_format = Arc::new(DecimalFormat::new("0", Locale::EnUs));
//         decimal_text_format.set_maximum_fraction_digits(340); // DecimalFormat::DOUBLE_FRACTION_DIGITS

//         Self {
//             workbook_evaluator,
//             sheet,
//             formatting,
//             rule,
//             regions,
//             top_left_region,
//             meaningful_region_values: HashMap::new(),
//             priority: rule.get_priority(),
//             formatting_index,
//             rule_index,
//             formula1,
//             formula2,
//             text,
//             lower_text,
//             operator,
//             type_,
//             number_format,
//             decimal_text_format,
//         }
//     }

//     /// Returns the sheet.
//     pub(crate) fn get_sheet(&self) -> &Arc<dyn Sheet> {
//         &self.sheet
//     }

//     /// Returns the formatting.
//     pub(crate) fn get_formatting(&self) -> &Arc<ConditionalFormatting> {
//         &self.formatting
//     }

//     /// Returns conditional formatting index.
//     pub(crate) fn get_formatting_index(&self) -> i32 {
//         self.formatting_index
//     }

//     /// Returns Excel number format string to apply to matching cells, or None to keep the cell default.
//     pub(crate) fn get_number_format(&self) -> Option<&ExcelNumberFormat> {
//         self.number_format.as_ref()
//     }

//     /// Returns the rule.
//     pub(crate) fn get_rule(&self) -> &Arc<ConditionalFormattingRule> {
//         &self.rule
//     }

//     /// Returns rule index.
//     pub(crate) fn get_rule_index(&self) -> i32 {
//         self.rule_index
//     }

//     /// Returns the regions.
//     pub(crate) fn get_regions(&self) -> &[CellRangeAddress] {
//         &self.regions
//     }

//     /// Returns the priority.
//     pub(crate) fn get_priority(&self) -> i32 {
//         self.priority
//     }

//     /// Returns the formula1.
//     pub(crate) fn get_formula1(&self) -> Option<&str> {
//         self.formula1.as_deref()
//     }

//     /// Returns the formula2.
//     pub(crate) fn get_formula2(&self) -> Option<&str> {
//         self.formula2.as_deref()
//     }

//     /// Returns condition text if any, or None.
//     pub(crate) fn get_text(&self) -> Option<&str> {
//         self.text.as_deref()
//     }

//     /// Returns the operator.
//     pub(crate) fn get_operator(&self) -> OperatorEnum {
//         self.operator
//     }

//     /// Returns the type.
//     pub(crate) fn get_type(&self) -> ConditionType {
//         self.type_
//     }

//     /// Returns true if this rule evaluates to true for the given cell.
//     pub(crate) fn matches(&self, ref_: &CellReference) -> bool {
//         // First check that it is in one of the regions defined for this format
//         let mut region: Option<&CellRangeAddress> = None;
//         for r in &self.regions {
//             if r.is_in_range(ref_) {
//                 region = Some(r);
//                 break;
//             }
//         }

//         if region.is_none() {
//             // Cell not in range of this rule
//             return false;
//         }

//         let region = region.unwrap();
//         let rule_type = self.rule.get_condition_type();

//         // These rules apply to all cells in a region. Specific condition criteria
//         // may specify no special formatting for that value partition, but that's display logic
//         if rule_type == ConditionType::COLOR_SCALE
//             || rule_type == ConditionType::DATA_BAR
//             || rule_type == ConditionType::ICON_SET
//         {
//             return true;
//         }

//         let cell = {
//             let row = self.sheet.get_row(ref_.get_row());
//             row.and_then(|r| r.get_cell(ref_.get_col()))
//         };

//         if rule_type == ConditionType::CELL_VALUE_IS {
//             // Undefined cells never match a VALUE_IS condition
//             if cell.is_none() {
//                 return false;
//             }
//             return self.check_value(cell.as_ref().unwrap(), &self.top_left_region.unwrap());
//         }

//         if rule_type == ConditionType::FORMULA {
//             return self.check_formula(ref_, &self.top_left_region.unwrap());
//         }

//         if rule_type == ConditionType::FILTER {
//             return self.check_filter(cell.as_ref(), ref_, &self.top_left_region.unwrap());
//         }

//         // TODO: anything else, we don't handle yet, such as top 10
//         false
//     }

//     /// Check if the value of the cell is valid or not for the formatting rule.
//     fn check_value(&self, cell: &dyn Cell, region: &CellRangeAddress) -> bool {
//         if DataValidationEvaluator::is_type(cell, CellType::BLANK)
//             || DataValidationEvaluator::is_type(cell, CellType::ERROR)
//             || (DataValidationEvaluator::is_type(cell, CellType::STRING)
//                 && (cell.get_string_cell_value().is_empty() || cell.get_string_cell_value() == ""))
//         {
//             return false;
//         }

//         let formula1 = match self.rule.get_formula1() {
//             Some(f) => f,
//             None => return false,
//         };

//         let eval = self.unwrap_eval(self.workbook_evaluator.evaluate(
//             &formula1,
//             &ConditionalFormattingEvaluator::get_ref(cell),
//             region,
//         ));

//         let mut eval2 = ValueEval::BlankEval;
//         if let Some(f2) = self.rule.get_formula2() {
//             if !f2.is_empty() {
//                 eval2 = self.unwrap_eval(self.workbook_evaluator.evaluate(
//                     &f2,
//                     &ConditionalFormattingEvaluator::get_ref(cell),
//                     region,
//                 ));
//             }
//         }

//         // We assume the cell has been evaluated, and the current formula value stored
//         if DataValidationEvaluator::is_type(cell, CellType::BOOLEAN)
//             && (matches!(eval, ValueEval::BlankEval) || matches!(eval, ValueEval::BoolEval(_)))
//             && (matches!(eval2, ValueEval::BlankEval) || matches!(eval2, ValueEval::BoolEval(_)))
//         {
//             let bool_val = cell.get_boolean_cell_value();
//             let eval_bool = if let ValueEval::BoolEval(b) = &eval {
//                 Some(b.get_boolean_value())
//             } else {
//                 None
//             };
//             let eval2_bool = if let ValueEval::BoolEval(b) = &eval2 {
//                 Some(b.get_boolean_value())
//             } else {
//                 None
//             };
//             return self.operator.is_valid_bool(bool_val, eval_bool, eval2_bool);
//         }

//         if DataValidationEvaluator::is_type(cell, CellType::NUMERIC)
//             && (matches!(eval, ValueEval::BlankEval) || matches!(eval, ValueEval::NumberEval(_)))
//             && (matches!(eval2, ValueEval::BlankEval) || matches!(eval2, ValueEval::NumberEval(_)))
//         {
//             let num_val = cell.get_numeric_cell_value();
//             let eval_num = if let ValueEval::NumberEval(n) = &eval {
//                 Some(n.get_number_value())
//             } else {
//                 None
//             };
//             let eval2_num = if let ValueEval::NumberEval(n) = &eval2 {
//                 Some(n.get_number_value())
//             } else {
//                 None
//             };
//             return self.operator.is_valid_number(num_val, eval_num, eval2_num);
//         }

//         if DataValidationEvaluator::is_type(cell, CellType::STRING)
//             && (matches!(eval, ValueEval::BlankEval) || matches!(eval, ValueEval::StringEval(_)))
//             && (matches!(eval2, ValueEval::BlankEval) || matches!(eval2, ValueEval::StringEval(_)))
//         {
//             let str_val = cell.get_string_cell_value();
//             let eval_str = if let ValueEval::StringEval(s) = &eval {
//                 Some(s.get_string_value())
//             } else {
//                 None
//             };
//             let eval2_str = if let ValueEval::StringEval(s) = &eval2 {
//                 Some(s.get_string_value())
//             } else {
//                 None
//             };
//             return self.operator.is_valid_string(&str_val, eval_str, eval2_str);
//         }

//         self.operator.is_valid_for_incompatible_types()
//     }

//     fn unwrap_eval(&self, eval: ValueEval) -> ValueEval {
//         let mut comp = eval;

//         while let ValueEval::RefEval(ref_eval) = comp {
//             comp = ref_eval.get_inner_value_eval(ref_eval.get_first_sheet_index());
//         }

//         comp
//     }

//     /// Check formula using the same rules as Data Validation evaluations.
//     fn check_formula(&self, ref_: &CellReference, region: &CellRangeAddress) -> bool {
//         let formula1 = match self.rule.get_formula1() {
//             Some(f) => f,
//             None => return false,
//         };

//         let comp = self.unwrap_eval(self.workbook_evaluator.evaluate(
//             &formula1,
//             Some(ref_),
//             region,
//         ));

//         // Copied for now from DataValidationEvaluator.ValidationEnum.FORMULA#isValidValue()
//         match comp {
//             ValueEval::BlankEval => true,
//             ValueEval::ErrorEval(_) => false,
//             ValueEval::BoolEval(b) => b.get_boolean_value(),
//             ValueEval::NumberEval(n) => n.get_number_value() != 0.0,
//             _ => false, // anything else is false, such as text
//         }
//     }

//     fn check_filter(
//         &self,
//         cell: Option<&dyn Cell>,
//         ref_: &CellReference,
//         region: &CellRangeAddress,
//     ) -> bool {
//         let filter_type = match self.rule.get_condition_filter_type() {
//             Some(ft) => ft,
//             None => return false,
//         };

//         let cv = self.get_cell_value(cell);

//         match filter_type {
//             ConditionFilterType::FILTER => false, // We don't evaluate HSSF filters yet
//             ConditionFilterType::TOP_10 => {
//                 // From testing, Excel only operates on numbers and dates (which are stored as numbers) in the range.
//                 // Numbers stored as text are ignored, but numbers formatted as text are treated as numbers.

//                 if !cv.is_number() {
//                     return false;
//                 }

//                 self.get_meaningful_values(region, false, |values| self.evaluate_top10(values))
//                     .contains(&cv)
//             }
//             ConditionFilterType::UNIQUE_VALUES => {
//                 // Per Excel help, "duplicate" means matching value AND format
//                 self.get_meaningful_values(region, true, |values| {
//                     self.evaluate_unique_values(values)
//                 })
//                 .contains(&cv)
//             }
//             ConditionFilterType::DUPLICATE_VALUES => {
//                 // Per Excel help, "duplicate" means matching value AND format
//                 self.get_meaningful_values(region, true, |values| {
//                     self.evaluate_duplicate_values(values)
//                 })
//                 .contains(&cv)
//             }
//             ConditionFilterType::ABOVE_AVERAGE => {
//                 // From testing, Excel only operates on numbers and dates (which are stored as numbers) in the range.
//                 // Numbers stored as text are ignored, but numbers formatted as text are treated as numbers.

//                 let conf = match self.rule.get_filter_configuration() {
//                     Some(c) => c,
//                     None => return false,
//                 };

//                 let values_vec: Vec<ValueAndFormat> = self
//                     .get_meaningful_values(region, false, |values| {
//                         self.evaluate_above_average(values)
//                     })
//                     .into_iter()
//                     .collect();

//                 if values_vec.len() < 2 {
//                     return false;
//                 }

//                 let val = if cv.is_number() { cv.value } else { None };
//                 if val.is_none() {
//                     return false;
//                 }

//                 let val = val.unwrap();
//                 let avg = values_vec[0].value.unwrap();
//                 let std_dev = values_vec[1].value.unwrap();

//                 // Use StdDev, aboveAverage, equalAverage to find:
//                 // comparison value and operator type

//                 let comp = if conf.get_std_dev() > 0.0 {
//                     avg + (if conf.get_above_average() { 1.0 } else { -1.0 })
//                         * std_dev
//                         * conf.get_std_dev()
//                 } else {
//                     avg
//                 };

//                 let op = if conf.get_above_average() {
//                     if conf.get_equal_average() {
//                         OperatorEnum::GREATER_OR_EQUAL
//                     } else {
//                         OperatorEnum::GREATER_THAN
//                     }
//                 } else {
//                     if conf.get_equal_average() {
//                         OperatorEnum::LESS_OR_EQUAL
//                     } else {
//                         OperatorEnum::LESS_THAN
//                     }
//                 };

//                 op.is_valid_number(val, Some(comp), None)
//             }
//             ConditionFilterType::CONTAINS_TEXT => {
//                 // Implemented both by a cfRule "text" attribute and a formula. Use the text.
//                 if let (Some(text), Some(lower_text)) = (&self.text, &self.lower_text) {
//                     cv.to_string().to_lowercase().contains(lower_text)
//                 } else {
//                     false
//                 }
//             }
//             ConditionFilterType::NOT_CONTAINS_TEXT => {
//                 // Implemented both by a cfRule "text" attribute and a formula. Use the text.
//                 if let (Some(text), Some(lower_text)) = (&self.text, &self.lower_text) {
//                     !cv.to_string().to_lowercase().contains(lower_text)
//                 } else {
//                     true
//                 }
//             }
//             ConditionFilterType::BEGINS_WITH => {
//                 // Implemented both by a cfRule "text" attribute and a formula. Use the text.
//                 if let Some(lower_text) = &self.lower_text {
//                     cv.to_string().to_lowercase().starts_with(lower_text)
//                 } else {
//                     false
//                 }
//             }
//             ConditionFilterType::ENDS_WITH => {
//                 // Implemented both by a cfRule "text" attribute and a formula. Use the text.
//                 if let Some(lower_text) = &self.lower_text {
//                     cv.to_string().to_lowercase().ends_with(lower_text)
//                 } else {
//                     false
//                 }
//             }
//             ConditionFilterType::CONTAINS_BLANKS => {
//                 match cv.get_string() {
//                     Ok(v) => StringUtil::is_blank(&v),
//                     Err(_) => false, // Not a valid string value, and not a blank cell (that's checked earlier)
//                 }
//             }
//             ConditionFilterType::NOT_CONTAINS_BLANKS => {
//                 match cv.get_string() {
//                     Ok(v) => StringUtil::is_not_blank(&v),
//                     Err(_) => true, // Not a valid string value, but not blank
//                 }
//             }
//             ConditionFilterType::CONTAINS_ERRORS => cell.map_or(false, |c| {
//                 DataValidationEvaluator::is_type(c, CellType::ERROR)
//             }),
//             ConditionFilterType::NOT_CONTAINS_ERRORS => cell.map_or(true, |c| {
//                 !DataValidationEvaluator::is_type(c, CellType::ERROR)
//             }),
//             ConditionFilterType::TIME_PERIOD => {
//                 // Implemented both by a cfRule "text" attribute and a formula. Use the formula.
//                 self.check_formula(ref_, region)
//             }
//             _ => false,
//         }
//     }

//     fn evaluate_top10(&self, all_values: Vec<ValueAndFormat>) -> HashSet<ValueAndFormat> {
//         let conf = match self.rule.get_filter_configuration() {
//             Some(c) => c,
//             None => return HashSet::new(),
//         };

//         let mut sorted_values = all_values;

//         if !conf.get_bottom() {
//             sorted_values.sort_by(|a, b| b.cmp(a)); // Reverse order
//         } else {
//             sorted_values.sort();
//         }

//         let mut limit = conf.get_rank() as usize;
//         if conf.get_percent() {
//             limit = sorted_values.len() * limit / 100;
//         }

//         if sorted_values.len() <= limit {
//             return sorted_values.into_iter().collect();
//         }

//         sorted_values.into_iter().take(limit).collect()
//     }

//     fn evaluate_unique_values(&self, all_values: Vec<ValueAndFormat>) -> HashSet<ValueAndFormat> {
//         let mut sorted_values = all_values;
//         sorted_values.sort();

//         let mut unique = HashSet::new();
//         let mut i = 0;

//         while i < sorted_values.len() {
//             let v = &sorted_values[i];

//             // Skip this if the current value matches the next one, or is the last one and matches the previous one
//             if (i < sorted_values.len() - 1 && v == &sorted_values[i + 1])
//                 || (i > 0 && i == sorted_values.len() - 1 && v == &sorted_values[i - 1])
//             {
//                 // Current value matches next value, skip both
//                 i += 2;
//                 continue;
//             }

//             unique.insert(v.clone());
//             i += 1;
//         }

//         unique
//     }

//     fn evaluate_duplicate_values(
//         &self,
//         all_values: Vec<ValueAndFormat>,
//     ) -> HashSet<ValueAndFormat> {
//         let mut sorted_values = all_values;
//         sorted_values.sort();

//         let mut dup = HashSet::new();
//         let mut i = 0;

//         while i < sorted_values.len() {
//             let v = &sorted_values[i];

//             // Skip this if the current value matches the next one, or is the last one and matches the previous one
//             if (i < sorted_values.len() - 1 && v == &sorted_values[i + 1])
//                 || (i > 0 && i == sorted_values.len() - 1 && v == &sorted_values[i - 1])
//             {
//                 // Current value matches next value, add one
//                 dup.insert(v.clone());
//                 i += 2;
//                 continue;
//             }

//             i += 1;
//         }

//         dup
//     }

//     fn evaluate_above_average(&self, all_values: Vec<ValueAndFormat>) -> HashSet<ValueAndFormat> {
//         let mut total = 0.0;
//         let mut pop = Vec::with_capacity(all_values.len());

//         for v in &all_values {
//             if let Some(num) = v.value {
//                 total += num;
//                 pop.push(ValueEval::NumberEval(NumberEval::new(num)));
//             }
//         }

//         let mut avg_set = HashSet::new();
//         let avg = if all_values.is_empty() {
//             0.0
//         } else {
//             total / all_values.len() as f64
//         };
//         avg_set.insert(ValueAndFormat::new_number(
//             avg,
//             None,
//             self.decimal_text_format.clone(),
//         ));

//         let std_dev = if all_values.len() <= 1 {
//             0.0
//         } else {
//             // Assuming AggregateFunction::STDEV is available
//             let result = AggregateFunction::STDEV.evaluate(&pop, 0, 0);
//             if let ValueEval::NumberEval(n) = result {
//                 n.get_number_value()
//             } else {
//                 0.0
//             }
//         };

//         avg_set.insert(ValueAndFormat::new_number(
//             std_dev,
//             None,
//             self.decimal_text_format.clone(),
//         ));
//         avg_set
//     }

//     /// From testing, Excel only operates on numbers and dates (which are stored as numbers) in the range.
//     /// Numbers stored as text are ignored, but numbers formatted as text are treated as numbers.
//     ///
//     /// # Arguments
//     /// * `func` - instances evaluate the values for a region and return the positive matches for the function type.
//     ///
//     /// # Returns
//     /// The meaningful values in the range of cells specified
//     fn get_meaningful_values<F>(
//         &mut self,
//         region: &CellRangeAddress,
//         with_text: bool,
//         func: F,
//     ) -> &HashSet<ValueAndFormat>
//     where
//         F: FnOnce(Vec<ValueAndFormat>) -> HashSet<ValueAndFormat>,
//     {
//         use std::collections::hash_map::Entry;

//         match self.meaningful_region_values.entry(region.clone()) {
//             Entry::Occupied(entry) => entry.into_mut(),
//             Entry::Vacant(entry) => {
//                 let mut all_values = Vec::with_capacity(
//                     (region.get_last_column() - region.get_first_column() + 1) as usize
//                         * (region.get_last_row() - region.get_first_row() + 1) as usize,
//                 );

//                 for r in region.get_first_row()..=region.get_last_row() {
//                     let row = match self.sheet.get_row(r) {
//                         Some(row) => row,
//                         None => continue,
//                     };

//                     for c in region.get_first_column()..=region.get_last_column() {
//                         if let Some(cell) = row.get_cell(c) {
//                             let cv = self.get_cell_value(Some(cell));
//                             if with_text || cv.is_number() {
//                                 all_values.push(cv);
//                             }
//                         }
//                     }
//                 }

//                 let values = func(all_values);
//                 entry.insert(values)
//             }
//         }
//     }

//     fn get_cell_value(&self, cell: Option<&dyn Cell>) -> ValueAndFormat {
//         if let Some(cell) = cell {
//             let format = cell.get_cell_style().get_data_format_string();
//             let mut cell_type = cell.get_cell_type();

//             if cell_type == CellType::FORMULA {
//                 cell_type = cell.get_cached_formula_result_type();
//             }

//             match cell_type {
//                 CellType::NUMERIC => ValueAndFormat::new_number(
//                     cell.get_numeric_cell_value(),
//                     Some(format),
//                     self.decimal_text_format.clone(),
//                 ),
//                 CellType::STRING | CellType::BOOLEAN => {
//                     ValueAndFormat::new_string(cell.get_string_cell_value(), Some(format))
//                 }
//                 _ => ValueAndFormat::new_string("".to_string(), Some("".to_string())),
//             }
//         } else {
//             ValueAndFormat::new_string("".to_string(), Some("".to_string()))
//         }
//     }
// }

// impl PartialEq for EvaluationConditionalFormatRule {
//     /// Defined as equal sheet name and formatting and rule indexes
//     fn eq(&self, other: &Self) -> bool {
//         self.sheet
//             .get_sheet_name()
//             .eq_ignore_ascii_case(&other.sheet.get_sheet_name())
//             && self.formatting_index == other.formatting_index
//             && self.rule_index == other.rule_index
//     }
// }

// impl Eq for EvaluationConditionalFormatRule {}

// impl PartialOrd for EvaluationConditionalFormatRule {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl Ord for EvaluationConditionalFormatRule {
//     /// Per Excel Help, XSSF rule priority is sheet-wide, not just within the owning ConditionalFormatting object.
//     /// This can be seen by creating 4 rules applying to two different ranges and examining the XML.
//     ///
//     /// HSSF priority is based on definition/persistence order.
//     ///
//     /// # Returns
//     /// Comparison based on sheet name, formatting index, and rule priority
//     fn cmp(&self, other: &Self) -> Ordering {
//         let cmp = self
//             .sheet
//             .get_sheet_name()
//             .to_lowercase()
//             .cmp(&other.sheet.get_sheet_name().to_lowercase());

//         if cmp != Ordering::Equal {
//             return cmp;
//         }

//         let cmp = self.priority.cmp(&other.priority);
//         if cmp != Ordering::Equal {
//             return cmp;
//         }

//         let cmp = self.formatting_index.cmp(&other.formatting_index);
//         if cmp != Ordering::Equal {
//             return cmp;
//         }

//         self.rule_index.cmp(&other.rule_index)
//     }
// }

// impl Hash for EvaluationConditionalFormatRule {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.sheet.get_sheet_name().to_lowercase().hash(state);
//         self.formatting_index.hash(state);
//         self.rule_index.hash(state);
//     }
// }

// /// Note: this class has a natural ordering that is inconsistent with equals.
// #[derive(Clone)]
// pub(crate) struct ValueAndFormat {
//     value: Option<f64>,
//     string: Option<String>,
//     format: Option<String>,
//     decimal_text_format: Option<Arc<DecimalFormat>>,
// }

// impl ValueAndFormat {
//     pub(crate) fn new_number(
//         value: f64,
//         format: Option<String>,
//         decimal_text_format: Arc<DecimalFormat>,
//     ) -> Self {
//         Self {
//             value: Some(value),
//             string: None,
//             format,
//             decimal_text_format: Some(decimal_text_format),
//         }
//     }

//     pub(crate) fn new_string(string: String, format: Option<String>) -> Self {
//         Self {
//             value: None,
//             string: Some(string),
//             format,
//             decimal_text_format: None,
//         }
//     }

//     pub(crate) fn is_number(&self) -> bool {
//         self.value.is_some()
//     }

//     pub(crate) fn get_value(&self) -> Option<f64> {
//         self.value
//     }

//     pub(crate) fn get_string(&self) -> Result<&str, &'static str> {
//         self.string.as_deref().ok_or("Not a string value")
//     }

//     pub(crate) fn to_string(&self) -> String {
//         if let Some(value) = self.value {
//             if let Some(df) = &self.decimal_text_format {
//                 df.format(value)
//             } else {
//                 value.to_string()
//             }
//         } else if let Some(string) = &self.string {
//             string.clone()
//         } else {
//             String::new()
//         }
//     }
// }

// impl PartialEq for ValueAndFormat {
//     fn eq(&self, other: &Self) -> bool {
//         self.value == other.value && self.format == other.format && self.string == other.string
//     }
// }

// impl Eq for ValueAndFormat {}

// impl PartialOrd for ValueAndFormat {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

// impl Ord for ValueAndFormat {
//     /// Note: this class has a natural ordering that is inconsistent with equals.
//     /// # Returns
//     /// Value comparison
//     fn cmp(&self, other: &Self) -> Ordering {
//         match (self.value, other.value) {
//             (None, Some(_)) => Ordering::Greater,
//             (Some(_), None) => Ordering::Less,
//             (Some(a), Some(b)) => {
//                 let cmp = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
//                 if cmp != Ordering::Equal {
//                     return cmp;
//                 }
//             }
//             (None, None) => {}
//         }

//         match (&self.string, &other.string) {
//             (None, Some(_)) => Ordering::Greater,
//             (Some(_), None) => Ordering::Less,
//             (Some(a), Some(b)) => a.cmp(b),
//             (None, None) => Ordering::Equal,
//         }
//     }
// }

// impl Hash for ValueAndFormat {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.value.map(|v| v.to_bits()).hash(state);
//         self.string.hash(state);
//         self.format.hash(state);
//     }
// }
