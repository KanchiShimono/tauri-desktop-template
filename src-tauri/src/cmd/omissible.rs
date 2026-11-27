use serde::Deserialize;
use serde::Deserializer;
use specta::Type;
use specta::Types;
use specta::datatype::DataType;

use crate::app::presence::Presence;

#[derive(Debug)]
pub(crate) struct Omissible<T>(Presence<T>);

impl<T> Default for Omissible<T> {
    fn default() -> Self {
        Self(Presence::Absent)
    }
}

impl<T> From<Omissible<T>> for Presence<T> {
    fn from(field: Omissible<T>) -> Self {
        field.0
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Omissible<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer)
            .map(Presence::Present)
            .map(Self)
    }
}

impl<T: Type> Type for Omissible<T> {
    fn definition(types: &mut Types) -> DataType {
        types.define::<T>()
    }
}
