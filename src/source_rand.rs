use crate::{Property, PropertySource, SubKeys};

pub(crate) struct Random;

impl PropertySource for Random {
    fn name(&self) -> &str {
        "Random"
    }
    fn get_property(&self, name: &str) -> Option<Property<'_>> {
        match name {
            "random.u8" => Some(Property::I(rand::random::<u8>() as i64)),
            "random.u16" => Some(Property::I(rand::random::<u16>() as i64)),
            "random.u32" => Some(Property::I(rand::random::<u32>() as i64)),
            "random.i8" => Some(Property::I(rand::random::<i8>() as i64)),
            "random.i16" => Some(Property::I(rand::random::<i16>() as i64)),
            "random.i32" => Some(Property::I(rand::random::<i32>() as i64)),
            "random.i64" => Some(Property::I(rand::random::<i64>())),
            _ => None,
        }
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn sub_keys<'a>(&'a self, prefix: &str, sub_keys: &mut SubKeys<'a>) {
        if prefix == "random" {
            sub_keys.insert("u8");
            sub_keys.insert("u16");
            sub_keys.insert("u32");
            sub_keys.insert("i8");
            sub_keys.insert("i16");
            sub_keys.insert("i32");
            sub_keys.insert("i64");
        }
    }
}
