use crate::*;

pub(crate) struct Random;

impl PropertySource for Random {
    fn name(&self) -> String {
        "Random".to_string()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        match name {
            "random.u8" => Some(rand::random::<u8>().into()),
            "random.u16" => Some(rand::random::<u16>().into()),
            "random.u32" => Some(rand::random::<u32>().into()),
            "random.i8" => Some(rand::random::<i8>().into()),
            "random.i16" => Some(rand::random::<i16>().into()),
            "random.i32" => Some(rand::random::<i32>().into()),
            "random.i64" => Some(rand::random::<i64>().into()),
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
    use std::convert::TryInto;

    #[test]
    fn random_test() {
        let r = Random;
        let mut ok = false;
        for v in r.get_keys("random") {
            let key = &format!("random.{}", v);
            let x: i64 = r.get_property(key).unwrap().try_into().unwrap();
            let y = r.get_property(key).unwrap().try_into().unwrap();
            if x != y {
                ok = true;
            }
        }
        assert_eq!(true, ok);
    }
}
