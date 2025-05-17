use num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};

#[derive(Copy, Clone, Debug, TryFromPrimitive, IntoPrimitive, PartialEq, Eq)]
#[repr(u8)]
/// A telnet command.
pub enum Cmd {
    /// End of Record
    EOR = 239,
    /// End of subnegotiation parameters.
    SE = 240,
    /// No operation.
    NOP = 241,
    /// Data Mark.  The data stream portion of a Synch.  This should always be
    /// accompanied by a TCP Urgent notification.
    DM = 242,
    /// NVT character BRK.
    Break = 243,
    /// The function IP.
    IP = 244,
    /// The function AO.
    AO = 245,
    /// The function AYT.
    AYT = 246,
    /// The function EC.
    EC = 247,
    /// The function EL.
    EL = 248,
    /// The GA signal.
    GA = 249,
    /// Indicates that what follows is subnegotiation of the indicated option.
    SB = 250,
    /// Indicates the desire to begin performing, or confirmation that you are
    /// now performing, the indicated option.
    WILL = 251,
    /// Indicates the refusal to perform, or continue performing, the indicated
    /// option.
    WONT = 252,
    /// Indicates the request that the other party perform, or confirmation that
    /// you are expecting the other party to perform, the indicated option.
    DO = 253,
    /// Indicates the demand that the other party stop performing, or
    /// confirmation that you are no longer expecting the other party to perform,
    /// the indicated option.
    DONT = 254,
    /// Data Byte 255.
    IAC = 255,
}

impl PartialEq<Cmd> for u8 {
    fn eq(&self, other: &Cmd) -> bool {
        *self == *other as u8
    }
}

impl PartialEq<u8> for Cmd {
    fn eq(&self, other: &u8) -> bool {
        *self as u8 == *other
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
/// A telnent option.
pub enum Opt {
    /// A recognized telnet option.
    Known(KnownOpt),
    /// A valid, but unrecognized option.
    Unknown(u8),
}

impl From<u8> for Opt {
    fn from(other: u8) -> Self {
        match KnownOpt::try_from_primitive(other) {
            Ok(opt) => Opt::Known(opt),
            Err(_) => Opt::Unknown(other),
        }
    }
}

impl From<Opt> for u8 {
    fn from(other: Opt) -> u8 {
        match other {
            Opt::Known(opt) => opt as u8,
            Opt::Unknown(opt) => opt,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
#[allow(non_camel_case_types)]
/// A known telnet option.
#[allow(missing_docs)]
pub enum KnownOpt {
    TRANSMIT_BINARY = 0,
    ECHO = 1,
    RECONNECTION = 2,
    SUPPRESS_GA = 3,
    MESSAGE_SIZE = 4,
    STATUS = 5,
    TIMING_MARK = 6,
    RC_TRANS = 7,
    LINE_WIDTH = 8,
    PAGE_SIZE = 9,
    NAOCRD = 10,
    NAOHTS = 11,
    NAOHTD = 12,
    NAOFFD = 13,
    NAOVTS = 14,
    NAOVTD = 15,
    NAOLFD = 16,
    EXTENDED_ASCII = 17,
    LOGOUT = 18,
    BYTE_MACRO = 19,
    DATA_ENTRY_TERMINAL = 20,
    SUPDUP = 21,
    SUPDUP_OUT = 22,
    SEND_LOC = 23,
    TERMINAL_TYPE = 24,
    EOR = 25,
    TACACS = 26,
    OUT_MARK = 27,
    TERM_LOC_NO = 28,
    TN3270 = 29,
    X3_PAD = 30,
    NAWS = 31,
    TERMINAL_SPEED = 32,
    TOGGLE_FLOW_CONTROL = 33,
    LINEMODE = 34,
    AUTHENTICATION = 37,
    ENCRYPTION = 38,
    NEW_ENVIRONMENT = 39,
    TN2370E = 40,
    XAUTH = 41,
    CHARSET = 42,
    RSP = 43,
    CPC = 44,
    SLE = 45,
    STARTTLS = 46,
    KERMIT = 47,
    SEND_URL = 48,
    FORWARD_X = 49,
    MSDP = 69,
    MSSP = 70,
    MCCPV1 = 85,
    MCCPV2 = 86,
    MSP = 90,
    MXP = 91,
    XMP = 93,
    LOGON = 138,
    SSPI_LOGON = 139,
    HEARTBEAT = 140,
    ATCP = 200,
    GMCP = 201,
    EOL = 255,
}

impl PartialEq<KnownOpt> for u8 {
    fn eq(&self, other: &KnownOpt) -> bool {
        *self == *other as u8
    }
}

impl PartialEq<u8> for KnownOpt {
    fn eq(&self, other: &u8) -> bool {
        *self as u8 == *other
    }
}

impl PartialEq<Opt> for u8 {
    fn eq(&self, other: &Opt) -> bool {
        *self == u8::from(*other)
    }
}

impl PartialEq<u8> for Opt {
    fn eq(&self, other: &u8) -> bool {
        u8::from(*self) == *other
    }
}
