use macros::From;
use macros::Into;
use serde::Deserialize;
use serde::Serialize;
use specta::Type;

#[derive(Clone, Copy, Debug, Deserialize, Eq, From, Into, PartialEq, Serialize, Type)]
#[specta(type = String)]
pub(crate) struct Timestamp(jiff::Timestamp);
