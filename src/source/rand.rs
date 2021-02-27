use crate::*;

pub(crate) struct Random;

impl PropertySource for Random {
    fn name(&self) -> String {
        "Random".to_string()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        match name {
            "random.u8" => Some(rand::random::<u8>().into_property()),
            "random.u16" => Some(rand::random::<u16>().into_property()),
            "random.u32" => Some(rand::random::<u32>().into_property()),
            "random.i8" => Some(rand::random::<i8>().into_property()),
            "random.i16" => Some(rand::random::<i16>().into_property()),
            "random.i32" => Some(rand::random::<i32>().into_property()),
            "random.i64" => Some(rand::random::<i64>().into_property()),
            _ => None,
        }
    }
    fn is_empty(&self) -> bool {
        false
    }
    fn get_keys(&self, prefix: &str) -> Vec<String> {
        match prefix {
            "random" => vec!["u8", "u16", "u32", "i8", "i16", "i32", "i64"]
                .into_iter()
                .map(|x| x.to_string())
                .collect(),
            _ => vec![],
        }
    }
    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_test() {
        let r = Random;
        let mut ok = false;
        for v in r.get_keys("random") {
            let key = &format!("random.{}", v);
            let x = i64::from_property(r.get_property(key).unwrap()).unwrap();
            let y = i64::from_property(r.get_property(key).unwrap()).unwrap();
            if x != y {
                ok = true;
            }
        }
        assert_eq!(true, ok);
    }
}
