use enum_repr::EnumRepr;

#[EnumRepr(type = "usize")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceLevel {
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
