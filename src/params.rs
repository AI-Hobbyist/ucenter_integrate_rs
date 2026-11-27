//! 表单参数构造器。数组参数按 PHP 约定编码为 `key[]=v1&key[]=v2`。

#[derive(Debug, Clone, Default)]
pub struct Params {
    pairs: Vec<(String, String)>,
}

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置一个参数。
    pub fn set(mut self, key: &str, value: impl ToString) -> Self {
        self.pairs.push((key.to_string(), value.to_string()));
        self
    }

    /// 仅在有值时设置参数。
    pub fn opt<T: ToString>(self, key: &str, value: Option<T>) -> Self {
        match value {
            Some(v) => self.set(key, v),
            None => self,
        }
    }

    /// 为真时设置 `key=1`。
    pub fn flag(self, key: &str, enabled: bool) -> Self {
        if enabled { self.set(key, 1) } else { self }
    }

    /// 设置数组参数 `key[]`。
    pub fn list<I, T>(mut self, key: &str, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: ToString,
    {
        let name = format!("{key}[]");
        for value in values {
            self.pairs.push((name.clone(), value.to_string()));
        }
        self
    }

    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_optional_flag_and_list_params() {
        let params = Params::new()
            .set("uid", 7)
            .opt::<i64>("page", None)
            .opt("pagesize", Some(10))
            .flag("isuid", true)
            .flag("nolog", false)
            .list("pmids", [1, 2]);
        let pairs: Vec<(&str, &str)> = params
            .pairs()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("uid", "7"),
                ("pagesize", "10"),
                ("isuid", "1"),
                ("pmids[]", "1"),
                ("pmids[]", "2"),
            ]
        );
    }
}
