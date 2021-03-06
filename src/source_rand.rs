use crate::{Key, Property, PropertySource, SubKeys};

pub(crate) struct Random;

impl PropertySource for Random {
    fn name(&self) -> &str {
        "Random"
    }

    #[inline]
    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        match key.as_str() {
            "random.u8" => Some(Property::I(rand::random::<u8>() as i64)),
            "random.u16" => Some(Property::I(rand::random::<u16>() as i64)),
            "random.u32" => Some(Property::I(rand::random::<u32>() as i64)),
            "random.u64" => Some(Property::O(rand::random::<u64>().to_string())),
            "random.u128" => Some(Property::O(rand::random::<u128>().to_string())),
            "random.i8" => Some(Property::I(rand::random::<i8>() as i64)),
            "random.i16" => Some(Property::I(rand::random::<i16>() as i64)),
            "random.i32" => Some(Property::I(rand::random::<i32>() as i64)),
            "random.i64" => Some(Property::I(rand::random::<i64>())),
            "random.i128" => Some(Property::O(rand::random::<i128>().to_string())),
            "random.usize" => Some(Property::O(rand::random::<usize>().to_string())),
            "random.isize" => Some(Property::O(rand::random::<isize>().to_string())),
            _ => None,
        }
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn get_sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        if key.as_str() == "random" {
            sub_keys.insert("u8");
            sub_keys.insert("u16");
            sub_keys.insert("u32");
            sub_keys.insert("u64");
            sub_keys.insert("u128");
            sub_keys.insert("usize");
            sub_keys.insert("i8");
            sub_keys.insert("i16");
            sub_keys.insert("i32");
            sub_keys.insert("i64");
            sub_keys.insert("i128");
            sub_keys.insert("isize");
        }
    }
}
