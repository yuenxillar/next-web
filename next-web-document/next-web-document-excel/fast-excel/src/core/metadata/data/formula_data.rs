#[derive(Debug, Clone, Default)]
pub struct FormulaData {
    formula_value: Option<String>,
}

impl FormulaData {
    pub fn new(formula_value: String) -> Self {
        FormulaData {
            formula_value: Some(formula_value),
        }
    }

    pub fn get_formula_value(&self) -> Option<&str> {
        self.formula_value.as_deref()
    }

    pub fn set_formula_value(&mut self, formula_value: impl ToString) {
        self.formula_value = Some(formula_value.to_string());
    }
}
