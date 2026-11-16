use super::*;

const ROOM1: &str = "ma-chan://example.com/93489432-b6be-4c67-932f-09e39a162072";

/// A client whose next envid is `envid` (the examples' values).
fn client(envid: u64) -> Client {
    Client::new(
        "sip:johns@example.com",
        "example.com",
        "CommSCraft",
        (envid - 1) << 1,
    )
    .unwrap()
}

/// §4.1 (p. 73): getserverinfo and its reply.
#[test]
fn server_info_example() {
    let mut c = client(6_698_699_123_101_735_678);
    let body = c.command(&Command::ServerInfo, 0).unwrap();
    assert_eq!(
        body,
        format!("<xccos ver=\"1\" envid=\"6698699123101735678\" xmlns=\"{NS}\"><cmd id=\"cmd:getserverinfo\" seqid=\"1\"><data><sib domain=\"example.com\" infoType=\"1783\" clientVersion=\"CommSCraft\" /></data></cmd></xccos>")
    );
    let reply = r#"<xccos xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" ver="1" envid="1472189363123229190" xmlns="urn:parlano:xml:ns:xccos">
  <rpl id="rpl:getserverinfo" seqid="1">
    <commandid seqid="1" envid="6698699123101735678" />
    <resp code="200">SUCCESS_OK</resp>
    <data>
      <sib infoType="251" serverTime="2011-10-27T21:05:48.4345771Z" searchLimit="999" messageSizeLimit="8000" storySizeLimit="16000" rootUri="ma-cat://example.com/cf724a9b-4595-4556-809d-b7846c4c4320" dbVersion="de64325e-d4ab-44d5-9ea8-55785b7c8a3b" serverVersion="5.0.7853.0" />
    </data>
  </rpl>
</xccos>"#;
    assert_eq!(
        c.on_body(reply).unwrap(),
        [Event::ServerInfo {
            name: String::new(),
            management_url: String::new(),
            message_limit: 8000
        }]
    );
    assert_eq!(c.pending(), 0);
    // The same reply again matches nothing pending and is dropped (§3.1.2.1.5 step 4).
    assert!(c.on_body(reply).unwrap().is_empty());
}

/// §4.2 (pp. 73–74): bjoin of two rooms; the reply names the rooms, the users and who is in
/// which room, and an rpl:bccontext under the same commandid carries the backchat.
#[test]
fn batch_join_example() {
    let mut c = client(6_698_699_123_101_735_680);
    let body = c
        .command(
            &Command::BatchJoin {
                rooms: vec![
                    "93489432-b6be-4c67-932f-09e39a162072".into(),
                    "6e43547f-9152-46e3-ad76-82197694fdb9".into(),
                ],
            },
            0,
        )
        .unwrap();
    assert!(body.contains("<cmd id=\"cmd:bjoin\" seqid=\"1\"><data><chanid key=\"0\" value=\"93489432-b6be-4c67-932f-09e39a162072,6e43547f-9152-46e3-ad76-82197694fdb9\" domain=\"example.com\" /></data></cmd>"), "{body}");
    let reply = r#"<xccos ver="1" envid="1472189363123229193" xmlns="urn:parlano:xml:ns:xccos">
 <rpl id="rpl:bjoin" seqid="1">
   <commandid seqid="1" envid="6698699123101735680" />
   <resp code="200">SUCCESS_OK</resp>
   <data>
     <chanib name="Projects" description="Various public projects" parent="ma-cat://example.com/2642ebba-f56a-4891-9b92-3991eb865c92" uri="ma-chan://example.com/93489432-b6be-4c67-932f-09e39a162072" overridemembers="false" behavior="NORMAL" topic="" disabled="false">
        <aib key="3456" value="0,1,2" />
        <aib key="11652" value="1" />
        <info id="urn:parlano:ma:info:ucnt">1</info>
     </chanib>
     <chanib name="Top Secret" description="Top Secret stuff" uri="ma-chan://example.com/6e43547f-9152-46e3-ad76-82197694fdb9" topic="" disabled="false">
        <aib key="3456" value="0,2" />
     </chanib>
     <uib uri="sip:johnsm@example.com" type="5" uname="John Smith" dispname="John Smith" id="0"><perms defined="1" inherited="1" inheriting="true" /></uib>
     <uib uri="sip:janedoe@example.com" type="5" uname="Jane Doe" dispname="Jane Doe" id="1"/>
     <uib uri="sip:johndoe@example.com" type="5" uname="John Doe" dispname="John Doe" id="2"/>
   </data>
 </rpl>
 <rpl id="rpl:bccontext" seqid="2">
   <commandid seqid="1" envid="6698699123101735680" />
   <resp code="200">SUCCESS_OK</resp>
   <data>
     <chanib uri="ma-chan://example.com/93489432-b6be-4c67-932f-09e39a162072" overridemembers="false" behavior="UNSET" topic="" disabled="false">
         <msg id="grpchat" chanUri="ma-chan://example.com/93489432-b6be-4c67-932f-09e39a162072" author="sip:janedoe@example.com" authdisp="Jane Doe" alert="false" chatId="1" ts="2011-10-24T21:23:33.887Z">
           <chat>Test</chat>
         </msg>
      </chanib>
      <cnt value="1" over="false" />
      <status>1</status>
    </data>
  </rpl>
</xccos>"#;
    let events = c.on_body(reply).unwrap();
    // The bjoin reply: users are read before the rooms, wherever they stand.
    let Event::Joined(rooms) = &events[0] else {
        panic!("{events:?}")
    };
    assert_eq!(rooms.len(), 2);
    assert_eq!(rooms[0].name, "Projects");
    assert_eq!(rooms[0].uri, ROOM1);
    // The users stand after the rooms; they are read first, so aib "0,1,2" resolves.
    assert_eq!(
        rooms[0].participants,
        [
            "sip:johnsm@example.com",
            "sip:janedoe@example.com",
            "sip:johndoe@example.com"
        ]
    );
    assert_eq!(
        rooms[1].participants,
        ["sip:johnsm@example.com", "sip:johndoe@example.com"]
    );
    let Event::History(history) = &events[1] else {
        panic!("{events:?}")
    };
    assert_eq!(history[0].messages[0].text, "Test");
    assert_eq!(history[0].messages[0].author_name, "Jane Doe");
    assert_eq!(c.last_chat_id(ROOM1), Some(1));
    assert_eq!(c.pending(), 0);
}

/// §4.3 (p. 74): the most recent history of a room.
#[test]
fn history_example() {
    let mut c = client(6_698_699_123_101_735_704);
    let room = "ma-chan://example.com/2a1a367c-5c14-4215-b5ae-d04eacb3b203";
    let body = c
        .command(
            &Command::History {
                room: room.into(),
                last: 100,
            },
            0,
        )
        .unwrap();
    assert!(body.contains("<cmd id=\"cmd:bccontext\" seqid=\"1\"><data><chanib uri=\"ma-chan://example.com/2a1a367c-5c14-4215-b5ae-d04eacb3b203\" overridemembers=\"false\" behavior=\"UNSET\" disabled=\"false\" /><bcq><last cnt=\"100\" /></bcq></data></cmd>"));
    let reply = r#"<xccos ver="1" envid="1472189363123229193" xmlns="urn:parlano:xml:ns:xccos">
  <rpl id="rpl:bccontext" seqid="1">
    <commandid seqid="1" envid="6698699123101735704" />
    <resp code="200">SUCCESS_OK</resp>
    <data>
      <chanib uri="ma-chan://example.com/2a1a367c-5c14-4215-b5ae-d04eacb3b203" overridemembers="false" behavior="UNSET" topic="" disabled="false">
        <msg id="grpchat" chanUri="ma-chan://example.com/2a1a367c-5c14-4215-b5ae-d04eacb3b203" author="sip:johndoe@example.com" authdisp="John Doe" alert="true" chatId="78" ts="2011-10-21T21:52:47.233Z"><chat>Let's meet!</chat></msg>
        <msg id="grpchat" chanUri="ma-chan://example.com/2a1a367c-5c14-4215-b5ae-d04eacb3b203" author="sip:janedoe@example.com" authdisp="Jane Doe" alert="false" chatId="79" ts="2011-10-22T00:21:06.993Z"><chat>Where?</chat><rtf>{\urtf1 Where?\par}</rtf></msg>
      </chanib>
      <cnt value="2" over="false" />
    </data>
  </rpl>
</xccos>"#;
    let events = c.on_body(reply).unwrap();
    let Event::History(rooms) = &events[0] else {
        panic!()
    };
    let texts: Vec<_> = rooms[0].messages.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, ["Let's meet!", "Where?"]);
    assert!(rooms[0].messages[0].alert);
    assert_eq!(c.last_chat_id(room), Some(79));
}

/// §4.6 (p. 76): sending a chat, and the server's echo with the originatingMessageId; a
/// message of somebody else arrives the same way.
#[test]
fn chat_example() {
    let mut c = client(6_698_699_123_101_735_682);
    let body = c
        .chat(ROOM1, "Hello, World!", "2011-10-27T21:09:48.3368091Z")
        .unwrap();
    assert_eq!(
        body,
        format!("<xccos ver=\"1\" envid=\"6698699123101735682\" xmlns=\"{NS}\"><grpchat id=\"grpchat\" seqid=\"1\" chanUri=\"{ROOM1}\" author=\"sip:johns@example.com\" authdisp=\"\" alert=\"false\" chatId=\"0\" ts=\"2011-10-27T21:09:48.3368091Z\"><originatingMessageId seqid=\"1\" envid=\"6698699123101735682\" /><chat>Hello, World!</chat></grpchat></xccos>")
    );
    let echo = r#"<xccos ver="1" envid="1472189363123229226" xmlns="urn:parlano:xml:ns:xccos">
  <grpchat id="grpchat" seqid="1" chanUri="ma-chan://example.com/93489432-b6be-4c67-932f-09e39a162072" author="sip:johnsm@example.com" authdisp="John Smith" alert="false" chatId="227" ts="2011-10-27T21:09:50.247Z">
    <originatingMessageId seqid="1" envid="6698699123101735682" />
    <chat>Hello, World!</chat>
  </grpchat>
</xccos>"#;
    let events = c.on_body(echo).unwrap();
    assert!(
        matches!(&events[0], Event::Message(m) if m.ours && m.chat_id == 227 && m.text == "Hello, World!")
    );
    assert!(c.chat(ROOM1, "", "t").is_err());
    assert!(
        c.chat(ROOM1, &"x".repeat(8001), "t").is_err(),
        "messageSizeLimit"
    );
    assert!(c.chat("ma-chan://example.com/../x", "hi", "t").is_err());
    // Text is escaped.
    let b = c.chat(ROOM1, "<b>&", "t").unwrap();
    assert!(b.contains("<chat>&lt;b&gt;&amp;</chat>"));
}

/// requri (§3.1.3, p. 46), a failure reply, an error primitive, a wrong reply id and the
/// transaction timer (§3.1.2.1, pp. 44–45).
#[test]
fn replies_errors_and_timeouts() {
    let mut c = client(10);
    c.command(&Command::RequestUri, 0).unwrap();
    let ok = format!("<xccos ver=\"1\" envid=\"5\" xmlns=\"{NS}\"><rpl id=\"rpl:requri\" seqid=\"1\"><commandid seqid=\"1\" envid=\"10\"/><resp code=\"200\">SUCCESS_OK</resp><data><uib uri=\"sip:chatsrv@example.com;gruu;opaque=srvr:x\"/></data></rpl></xccos>");
    assert_eq!(
        c.on_body(&ok).unwrap(),
        [Event::ChannelServer(
            "sip:chatsrv@example.com;gruu;opaque=srvr:x".into()
        )]
    );
    // getassociations answered with a wrong reply id: dropped, still pending.
    c.command(
        &Command::Associations {
            manager: false,
            max: 100,
        },
        0,
    )
    .unwrap();
    let wrong = format!("<xccos ver=\"1\" envid=\"6\" xmlns=\"{NS}\"><rpl id=\"rpl:join\" seqid=\"1\"><commandid seqid=\"1\" envid=\"11\"/><resp code=\"200\"/></rpl></xccos>");
    assert!(c.on_body(&wrong).unwrap().is_empty());
    assert_eq!(c.pending(), 1);
    let fail = format!("<xccos ver=\"1\" envid=\"7\" xmlns=\"{NS}\"><rpl id=\"rpl:getassociations\" seqid=\"1\"><commandid seqid=\"1\" envid=\"11\"/><resp code=\"403\">FORBIDDEN</resp></rpl></xccos>");
    assert_eq!(
        c.on_body(&fail).unwrap(),
        [Event::Failed {
            command: "getassociations".into(),
            code: 403
        }]
    );
    c.command(&Command::ServerInfo, 1000).unwrap();
    let err = format!("<xccos ver=\"1\" envid=\"8\" xmlns=\"{NS}\"><err id=\"error\" seqid=\"1\"><commandid seqid=\"1\" envid=\"12\"/><resp code=\"400\"/></err></xccos>");
    assert_eq!(
        c.on_body(&err).unwrap(),
        [Event::Failed {
            command: "getserverinfo".into(),
            code: 400
        }]
    );
    c.command(&Command::RequestUri, 1000).unwrap();
    assert!(c.tick(10_999).is_empty());
    assert_eq!(
        c.tick(11_000),
        [Event::Failed {
            command: "requri".into(),
            code: 408
        }]
    );
}

/// Hostile input: wrong documents, bad values, too many pending commands.
#[test]
fn hostile_input() {
    let mut c = client(10);
    assert_eq!(c.on_body("<foo/>"), Err(Error::WrongDocument));
    assert!(c.on_body("<xccos").is_err());
    assert!(c
        .on_body(&format!(
            "<!DOCTYPE x [<!ENTITY a \"b\">]><xccos xmlns=\"{NS}\"/>"
        ))
        .is_err());
    assert!(Client::new("sip:a@b", "bad domain", "v", 1).is_err());
    assert!(Client::new("mailto:a@b", "example.com", "v", 1).is_err());
    assert!(c
        .command(
            &Command::BatchJoin {
                rooms: vec!["x\"/><evil".into()]
            },
            0
        )
        .is_err());
    assert!(c.command(&Command::BatchJoin { rooms: vec![] }, 0).is_err());
    for _ in 0..MAX_PENDING {
        c.command(&Command::RequestUri, 0).unwrap();
    }
    assert_eq!(
        c.command(&Command::RequestUri, 0),
        Err(Error::TooManyPending)
    );
    // A chat in a room whose URI is not a room is ignored.
    let odd = format!("<xccos ver=\"1\" envid=\"1\" xmlns=\"{NS}\"><grpchat id=\"grpchat\" seqid=\"1\" chanUri=\"http://x\" chatId=\"1\"><chat>hi</chat></grpchat></xccos>");
    assert!(c.on_body(&odd).unwrap().is_empty());
}
