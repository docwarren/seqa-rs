use std::any::Any;

pub trait ToLines: Any + Send + Sync {
    fn to_lines(&self) -> Vec<String>;
}
