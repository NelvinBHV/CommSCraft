//! The distributed object interfaces the client implements (PO-08): name, versions, the hashes
//! of the server and client interface, and how many client methods each version declares.
//!
//! Method indices follow the declaration order, the first method is 1 (§3.1.4.1.1, p. 28;
//! K-PO-1: the example bytes confirm the declaration order). A later version of an interface
//! appends methods, so a call with a higher index than its version declares is unknown and ends
//! the connection (§3.1.5, p. 51).

/// The kinds of distributed objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    ConnMgr,
    Meeting,
    ContentManager,
    ContentUserManager,
    UploadManager,
    UploadStream,
    Content,
    NativeFileOnly,
    Ppt,
    AnnotationContainer,
    Whiteboard,
    Poll,
    SharedNotes,
    Qna,
    SharedLinks,
}

/// One version of an interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub version: i32,
    pub server_hash: i64,
    pub client_hash: i64,
    /// Number of methods of the client interface (what the server may call on us).
    pub client_methods: u8,
}

impl Version {
    /// The hash in addProtocol: client plus server interface hash, 64-bit wrap-around
    /// (§3.2.3.1.3.1.2, p. 55; recomputed for ConnMgr, p. 95).
    pub fn sum(&self) -> i64 {
        self.client_hash.wrapping_add(self.server_hash)
    }
}

/// An interface: its DOInterface name and the versions we implement.
#[derive(Debug, Clone, Copy)]
pub struct Interface {
    pub kind: Kind,
    pub name: &'static str,
    pub versions: &'static [Version],
}

const fn v(version: i32, server_hash: i64, client_hash: i64, client_methods: u8) -> Version {
    Version {
        version,
        server_hash,
        client_hash,
        client_methods,
    }
}

/// ConnMgr client interface hash (§3.2.3.1.3, p. 55) and server interface hash (§3.3.3.1.3,
/// p. 80). Server methods: version 1, addProtocol 2, doneProtocols 3, log 4, lookup 5, ping 6.
pub const CONNMGR_CLIENT_HASH: i64 = 8_322_047_979_521_208_965;
pub const CONNMGR_SERVER_HASH: i64 = -8_221_414_758_688_209_204;

/// Hash of Meeting version 1 as the example announces it (§4.1.5, p. 96; K-PO-3: version 1's
/// interface is not printed, so this sum is taken from the example bytes, not recomputed).
pub const MEETING_V1_SUM: i64 = -2_007_473_133_263_860_314;

/// Every interface, in the order addProtocol announces them. Sources per entry:
/// ConnMgr §3.2.3.1.3/§3.3.3.1.3 (pp. 55, 80); ContentManager §3.1.4.1.4.1 (pp. 31–32);
/// Content §3.1.4.1.5.1 (pp. 33–36); Meeting §3.1.4.1.6.1 (p. 36); ContentUserManager
/// §3.1.4.1.7.1 (p. 37); UploadManager §3.1.4.1.8.1 (pp. 37–38); UploadStream (IRCStream)
/// §3.1.4.1.9.1 (p. 39); NativeFileOnlyContent §3.1.4.1.10.1 (p. 39); PptContent §3.1.4.1.11.1
/// (pp. 41–44); AnnotationContainer §3.1.4.1.12.1 (pp. 46–47); WhiteboardContent §3.1.4.1.13.1
/// (p. 47); PollContent §3.1.4.1.14.1 (p. 48); SharedNotesContent §3.1.4.1.15.1 (p. 49);
/// QnaContent §3.1.4.1.16.1 (p. 50); SharedLinksContent §3.1.4.1.17.1 (p. 51).
pub const INTERFACES: &[Interface] = &[
    Interface {
        kind: Kind::ConnMgr,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Pod.ConnMgr",
        versions: &[v(1, CONNMGR_SERVER_HASH, CONNMGR_CLIENT_HASH, 4)],
    },
    Interface {
        kind: Kind::Meeting,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Meeting",
        versions: &[v(
            2,
            7_811_924_786_664_530_844,
            2_106_930_589_629_680_263,
            4,
        )],
    },
    Interface {
        kind: Kind::ContentManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.ContentManager",
        versions: &[
            v(2, 3_800_622_354_142_801_969, -8_255_121_175_073_997_388, 8),
            v(10, 3_800_622_354_142_801_969, -5_664_441_486_369_344_972, 9),
        ],
    },
    Interface {
        kind: Kind::ContentUserManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.ContentUserManager",
        versions: &[v(
            1,
            5_320_330_165_687_787_020,
            5_320_330_165_687_787_020,
            2,
        )],
    },
    Interface {
        kind: Kind::UploadManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.UploadManager",
        versions: &[
            v(1, 4_004_400_404_121_921_234, -8_511_879_503_649_873_756, 4),
            v(2, 2_601_804_729_028_999_169, 1_079_790_469_891_676_391, 7),
        ],
    },
    Interface {
        kind: Kind::UploadStream,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Parts.IRCStream",
        versions: &[v(
            1,
            -6_716_385_024_907_738_156,
            5_963_839_780_483_567_246,
            2,
        )],
    },
    Interface {
        kind: Kind::Content,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Content",
        versions: &[
            v(1, -6_470_662_138_903_778_586, 1_113_513_223_610_002_283, 11),
            v(10, -2_530_343_413_165_516_885, 974_079_596_268_293_062, 14),
            v(20, -2_530_343_413_165_516_885, 974_079_596_268_293_062, 15),
        ],
    },
    Interface {
        kind: Kind::NativeFileOnly,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.NativeFileOnlyContent",
        versions: &[v(
            1,
            6_421_877_628_186_475_469,
            5_585_496_037_459_248_534,
            1,
        )],
    },
    Interface {
        kind: Kind::Ppt,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.PptContent",
        versions: &[
            v(2, 5_075_721_282_249_770_644, -3_775_280_164_302_319_261, 13),
            v(
                11,
                5_075_721_282_249_770_644,
                -3_775_280_164_302_319_261,
                13,
            ),
        ],
    },
    Interface {
        kind: Kind::AnnotationContainer,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.AnnotationContainer",
        versions: &[v(
            1,
            -5_714_708_003_270_970_775,
            2_571_074_477_103_256_610,
            17,
        )],
    },
    Interface {
        kind: Kind::Whiteboard,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.WhiteboardContent",
        versions: &[v(
            1,
            4_720_625_287_907_297_465,
            5_909_677_840_878_629_841,
            1,
        )],
    },
    Interface {
        kind: Kind::Poll,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.PollContent",
        versions: &[v(
            1,
            7_882_912_516_919_930_871,
            -1_572_291_151_947_590_318,
            6,
        )],
    },
    Interface {
        kind: Kind::SharedNotes,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.SharedNotesContent",
        versions: &[v(
            3,
            6_726_721_969_813_329_838,
            8_721_147_891_391_263_529,
            4,
        )],
    },
    Interface {
        kind: Kind::Qna,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.QnaContent",
        versions: &[v(1, -210_597_168_530_409_383, 473_785_100_728_654_906, 4)],
    },
    Interface {
        kind: Kind::SharedLinks,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.SharedLinksContent",
        versions: &[v(2, 7_904_618_883_769_218_068, -988_586_141_242_916_940, 7)],
    },
];

pub fn interface(kind: Kind) -> Option<&'static Interface> {
    INTERFACES.iter().find(|i| i.kind == kind)
}

pub fn by_name(name: &str) -> Option<&'static Interface> {
    INTERFACES.iter().find(|i| i.name == name)
}

/// The extended content object of a content type in cContentAdded (§3.2.4.1.3.1, p. 57; the
/// schema's content-type-type, p. 62). "Content.PptWac" has no interface in the document, and
/// the type strings of Q&A and shared links are not given: such content is shown without its
/// extension (open, needs a capture).
pub fn extended_kind(content_type: &str) -> Option<Kind> {
    match content_type {
        "Content.Ppt" => Some(Kind::Ppt),
        "Content.Whiteboard" => Some(Kind::Whiteboard),
        "Content.NativeFileOnly" => Some(Kind::NativeFileOnly),
        "Content.Poll" => Some(Kind::Poll),
        "Content.SharedNotes" => Some(Kind::SharedNotes),
        _ => None,
    }
}
