pub struct PersistentRememberMeToken {
    username: String,
    series: String,
    token_value: String,

    date: i64,
}

impl PersistentRememberMeToken {
    pub fn new(
        username: impl Into<String>,
        series: impl Into<String>,
        token_value: impl Into<String>,
        date: i64,
    ) -> Self {
        Self {
            username: username.into(),
            series: series.into(),
            token_value: token_value.into(),
            date,
        }
    }

    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn get_series(&self) -> &str {
        &self.series
    }

    pub fn get_token_value(&self) -> &str {
        &self.token_value
    }

    pub fn get_date(&self) -> i64 {
        self.date
    }
}
