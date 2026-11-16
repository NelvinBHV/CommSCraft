//! The distributed object interfaces the client implements (PO-08): name, versions, the hashes
//! of the server and client interface, and the methods of each version as the document
//! declares them.
//!
//! Method indices (K-PO-1): the document says once "the first method is 1, then each
//! declaration" (§3.1.4.1.1, p. 28) and once "the alphabetic order, compared ordinally"
//! (§3.1.4.1.2.1, p. 30). The ConnMgr examples follow the declaration (version 1, addProtocol 2,
//! doneProtocols 3, lookup 5; pp. 94–96); the server calls the application objects in ordinal
//! alphabetical order (`tested-in-lab`: ContentManager version 10 calls cContentAdded with 2,
//! as cAdvertiseContentCreationManifest sorts first although it is declared last). So ConnMgr
//! counts in declaration order and every other interface alphabetically. A method the
//! negotiated version does not declare ends the connection (§3.1.5, p. 51).

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
    /// The client interface's methods (what the server calls on us), as declared.
    pub client: &'static [&'static str],
    /// The server interface's methods we call, as declared (empty where we call none).
    pub server: &'static [&'static str],
}

impl Version {
    /// The hash in addProtocol: client plus server interface hash, 64-bit wrap-around
    /// (§3.2.3.1.3.1.2, p. 55; recomputed for ConnMgr, p. 95; the server announces exactly
    /// these sums, `tested-in-lab`).
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

fn ordered(kind: Kind, methods: &'static [&'static str]) -> Vec<&'static str> {
    let mut m = methods.to_vec();
    if kind != Kind::ConnMgr {
        // Ordinal comparison: the bytes of the names.
        m.sort_unstable();
    }
    m
}

/// The name of the client method with `index` (1-based) in this version.
pub fn client_method(kind: Kind, v: &Version, index: u8) -> Option<&'static str> {
    let i = usize::from(index).checked_sub(1)?;
    ordered(kind, v.client).get(i).copied()
}

/// The index of a server method we call.
pub fn server_index(kind: Kind, v: &Version, name: &str) -> Option<u8> {
    let i = ordered(kind, v.server).iter().position(|m| *m == name)?;
    u8::try_from(i.checked_add(1)?).ok()
}

/// ConnMgr client interface hash (§3.2.3.1.3, p. 55) and server interface hash (§3.3.3.1.3,
/// p. 80).
pub const CONNMGR_CLIENT_HASH: i64 = 8_322_047_979_521_208_965;
pub const CONNMGR_SERVER_HASH: i64 = -8_221_414_758_688_209_204;

/// K-PO-4 (`tested-in-lab`): for most interfaces the server announces exactly the sums of the
/// printed hashes – which confirms the table –, but two differ:
/// - ContentUserManager: the document prints the same hash for the server and the client
///   interface; the server hash is confirmed by the Connect example (§4.3, p. 97), so the client
///   hash is the misprint, and the server's sum gives 3 286 152 179 062 983 692.
/// - ContentManager version 10: the server's sum is the value printed as the client hash, and
///   its Connect carries the server hash −7 533 840 286 919 557 416 (the document repeats
///   version 2's); the client hash is their difference, 1 869 398 800 550 212 444.
///
/// Meeting version 1 is not announced: its hashes are not printed, and the example's sum
/// (§4.1.5, p. 96) is not what the server announces.
pub const CONTENT_USER_MANAGER_CLIENT_HASH: i64 = 3_286_152_179_062_983_692;

const CONTENT_V1: &[&str] = &[
    "cConnectCompleted",
    "cForceSync",
    "cSetCreationTime",
    "cSetLastUsedTime",
    "cSetNativeFileInfo",
    "cSetOwnerId",
    "cSetPresentInfo",
    "cSetPresentationOrder",
    "cSetTitle",
    "cSetTitleComplete",
    "cSetVisibility",
];
const CONTENT_V10: &[&str] = &[
    "cConnectCompleted",
    "cForceSync",
    "cSetCreationTime",
    "cSetLastUsedTime",
    "cSetNativeFileInfo",
    "cSetOwnerId",
    "cSetPresentInfo",
    "cSetPresentationOrder",
    "cSetTitle",
    "cSetTitleComplete",
    "cSetVisibility",
    "cSetOriginalFileUrl",
    "cSetViewingUrl",
    "cSetRecordingUrl",
];
const CONTENT_V20: &[&str] = &[
    "cConnectCompleted",
    "cForceSync",
    "cSetCreationTime",
    "cSetLastUsedTime",
    "cSetNativeFileInfo",
    "cSetOwnerId",
    "cSetPresentInfo",
    "cSetPresentationOrder",
    "cSetTitle",
    "cSetTitleComplete",
    "cSetVisibility",
    "cSetOriginalFileUrl",
    "cSetViewingUrl",
    "cSetRecordingUrl",
    "cSetSecondaryViewingUrl",
];
const CONTENT_MANAGER_V2: &[&str] = &[
    "cContentAdded",
    "cContentCreated",
    "cContentCreationFailed",
    "cContentRemoved",
    "cReserveTitleCompleted",
    "cSetActiveContent",
    "cSetActivePresenter",
    "cTitleReleased",
];
const CONTENT_MANAGER_V10: &[&str] = &[
    "cContentAdded",
    "cContentCreated",
    "cContentCreationFailed",
    "cContentRemoved",
    "cReserveTitleCompleted",
    "cSetActiveContent",
    "cSetActivePresenter",
    "cTitleReleased",
    "cAdvertiseContentCreationManifest",
];
const UPLOAD_MANAGER_V1: &[&str] = &[
    "cAcceptUpload",
    "cRejectUpload",
    "cSetAvailableSpace",
    "cUploadFinished",
];
const UPLOAD_MANAGER_V2: &[&str] = &[
    "cAcceptUpload",
    "cRejectUpload",
    "cSetAvailableSpace",
    "cUploadFinished",
    "cAcceptWebUpload",
    "cAcceptWebDownload",
    "cRejectWebDownload",
];
const PPT: &[&str] = &[
    "cConnectCompleted",
    "cErrorAddingResource",
    "cSetFullPptFileInfo",
    "cSetLocation",
    "cSetNotesInfo",
    "cSetPartialPptFileInfo",
    "cSetPreviewThumbnailFileInfo",
    "cSetResourceErrorCode",
    "cSetSlideCount",
    "cSetSlideDhtmlMarkupInfo",
    "cSetSlideDhtmlResourceInfo",
    "cSetSlideJpegInfo",
    "cSetThumbnailFileInfo",
];
const ANNOTATIONS: &[&str] = &[
    "cAddAnnotationBatch",
    "cChangePropertiesBatch",
    "cChangeTextBatch",
    "cClearAnnotations",
    "cErrorAddAnnotation",
    "cErrorChangeProperties",
    "cErrorChangePropertyForGroup",
    "cErrorChangeText",
    "cErrorClearAnnotations",
    "cErrorRemoveAnnotation",
    "cErrorRemoveAnnotations",
    "cErrorSetTelepointer",
    "cRemoveAnnotation",
    "cRemoveAnnotations",
    "cSetAnnotationConstraints",
    "cSetImageFileInfo",
    "cSetTelepointer",
];

const fn v(
    version: i32,
    server_hash: i64,
    client_hash: i64,
    client: &'static [&'static str],
) -> Version {
    Version {
        version,
        server_hash,
        client_hash,
        client,
        server: &[],
    }
}

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
        versions: &[Version {
            version: 1,
            server_hash: CONNMGR_SERVER_HASH,
            client_hash: CONNMGR_CLIENT_HASH,
            client: &["version", "addProtocol", "doneProtocols", "ping"],
            server: &[
                "version",
                "addProtocol",
                "doneProtocols",
                "log",
                "lookup",
                "ping",
            ],
        }],
    },
    Interface {
        kind: Kind::Meeting,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Meeting",
        versions: &[v(
            2,
            7_811_924_786_664_530_844,
            2_106_930_589_629_680_263,
            &["cMeetingReady", "cSetInfo", "cSetServerTime", "cSetUrlBase"],
        )],
    },
    Interface {
        kind: Kind::ContentManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.ContentManager",
        versions: &[
            v(
                2,
                3_800_622_354_142_801_969,
                -8_255_121_175_073_997_388,
                CONTENT_MANAGER_V2,
            ),
            v(
                10,
                -7_533_840_286_919_557_416,
                1_869_398_800_550_212_444,
                CONTENT_MANAGER_V10,
            ),
        ],
    },
    Interface {
        kind: Kind::ContentUserManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.ContentUserManager",
        versions: &[v(
            1,
            5_320_330_165_687_787_020,
            CONTENT_USER_MANAGER_CLIENT_HASH,
            &["cUsersAdded", "cUsersRemoved"],
        )],
    },
    Interface {
        kind: Kind::UploadManager,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.UploadManager",
        versions: &[
            v(
                1,
                4_004_400_404_121_921_234,
                -8_511_879_503_649_873_756,
                UPLOAD_MANAGER_V1,
            ),
            v(
                2,
                2_601_804_729_028_999_169,
                1_079_790_469_891_676_391,
                UPLOAD_MANAGER_V2,
            ),
        ],
    },
    Interface {
        kind: Kind::UploadStream,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Parts.IRCStream",
        versions: &[v(
            1,
            -6_716_385_024_907_738_156,
            5_963_839_780_483_567_246,
            &["cDisconnect", "cWriteComplete"],
        )],
    },
    Interface {
        kind: Kind::Content,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.Content",
        versions: &[
            v(
                1,
                -6_470_662_138_903_778_586,
                1_113_513_223_610_002_283,
                CONTENT_V1,
            ),
            v(
                10,
                -2_530_343_413_165_516_885,
                974_079_596_268_293_062,
                CONTENT_V10,
            ),
            v(
                20,
                -2_530_343_413_165_516_885,
                974_079_596_268_293_062,
                CONTENT_V20,
            ),
        ],
    },
    Interface {
        kind: Kind::NativeFileOnly,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.NativeFileOnlyContent",
        versions: &[v(
            1,
            6_421_877_628_186_475_469,
            5_585_496_037_459_248_534,
            &["cConnectCompleted"],
        )],
    },
    Interface {
        kind: Kind::Ppt,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.PptContent",
        versions: &[
            v(
                2,
                5_075_721_282_249_770_644,
                -3_775_280_164_302_319_261,
                PPT,
            ),
            v(
                11,
                5_075_721_282_249_770_644,
                -3_775_280_164_302_319_261,
                PPT,
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
            ANNOTATIONS,
        )],
    },
    Interface {
        kind: Kind::Whiteboard,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.WhiteboardContent",
        versions: &[v(
            1,
            4_720_625_287_907_297_465,
            5_909_677_840_878_629_841,
            &["cConnectCompleted"],
        )],
    },
    Interface {
        kind: Kind::Poll,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.PollContent",
        versions: &[Version {
            version: 1,
            server_hash: 7_882_912_516_919_930_871,
            client_hash: -1_572_291_151_947_590_318,
            client: &[
                "cConnectCompleted",
                "cSetLocalVote",
                "cSetOpenState",
                "cSetQuestion",
                "cSetResults",
                "cSetResultsState",
            ],
            server: &[
                "sClearVotes",
                "sModify",
                "sSetOpenState",
                "sSetResultsState",
                "sVote",
            ],
        }],
    },
    Interface {
        kind: Kind::SharedNotes,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.SharedNotesContent",
        versions: &[v(
            3,
            6_726_721_969_813_329_838,
            8_721_147_891_391_263_529,
            &[
                "cConnectCompleted",
                "cSetNotesId",
                "cSetNotesTitles",
                "cSetErrorCode",
            ],
        )],
    },
    Interface {
        kind: Kind::Qna,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.QnaContent",
        versions: &[v(
            1,
            -210_597_168_530_409_383,
            473_785_100_728_654_906,
            &[
                "cSetOpenState",
                "cSetQuestionsCount",
                "cPutBlob",
                "cConnectCompleted",
            ],
        )],
    },
    Interface {
        kind: Kind::SharedLinks,
        name: "Microsoft.Rtc.Server.DataMCU.Meeting.SharedLinksContent",
        versions: &[v(
            2,
            7_904_618_883_769_218_068,
            -988_586_141_242_916_940,
            &[
                "cConnectCompleted",
                "cSetViewURL",
                "cSetEditURL",
                "cSetActive",
                "cSetInactive",
                "cSetUniqueId",
                "cSetState",
            ],
        )],
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
