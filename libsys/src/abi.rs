use enum_repr::EnumRepr;

#[EnumRepr(type = "usize")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemCall {
    // I/O
    Read = 1,
    Write = 2,
    Open = 3,
    Close = 4,
    FileStatus = 5,
    Ioctl = 6,
    Select = 7,
    Access = 8,
    ReadDirectory = 9,
    GetUserId = 10,
    GetGroupId = 11,
    DuplicateFd = 12,
    SetUserId = 13,
    SetGroupId = 14,
    SetCurrentDirectory = 15,
    GetCurrentDirectory = 16,
    Seek = 17,
    MapMemory = 18,
    UnmapMemory = 19,

    // Process manipulation
    Fork = 32,
    Clone = 33,
    Exec = 34,
    Exit = 35,
    WaitPid = 36,
    WaitTid = 37,
    GetPid = 38,
    GetTid = 39,
    Sleep = 40,
    SetSignalEntry = 41,
    SignalReturn = 42,
    SendSignal = 43,
    Yield = 44,
    GetSid = 45,
    GetPgid = 46,
    GetPpid = 47,
    SetSid = 48,
    SetPgid = 49,
    // System
    GetCpuTime = 64,
    Mount = 65,
    // Debugging
    DebugTrace = 128
}
