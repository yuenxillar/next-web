

pub trait CollectionName: Send + Sync {

    fn col_name() -> &'static str;
}