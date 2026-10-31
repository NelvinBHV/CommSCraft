use super::*;

fn p(s: &str) -> Result<Element, Error> {
    parse(s, &Limits::default())
}

#[test]
fn parses_a_protocol_document() {
    let doc = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- comment -->
<provisioningGroupList xmlns="http://schemas.microsoft.com/2006/09/sip/provisioninggrouplist" subnet="172.24.32.0">
  <provisioningGroup name="ServerConfiguration" />
  <x:extra xmlns:x="urn:x" a='1'>a &lt;b&gt; &amp; &#65;&#x42; <![CDATA[<raw>&amp;]]></x:extra>
</provisioningGroupList>"#;
    let root = p(doc).unwrap();
    assert_eq!(root.name, "provisioningGroupList");
    assert_eq!(
        root.namespace.as_deref(),
        Some("http://schemas.microsoft.com/2006/09/sip/provisioninggrouplist")
    );
    assert_eq!(root.attr("subnet"), Some("172.24.32.0"));
    let g = root.child("provisioningGroup").unwrap();
    assert_eq!(g.attr("name"), Some("ServerConfiguration"));
    assert_eq!(
        g.namespace, root.namespace,
        "default namespace is inherited"
    );
    let x = root.child("extra").unwrap();
    assert_eq!(x.namespace.as_deref(), Some("urn:x"));
    assert_eq!(x.attr("a"), Some("1"));
    assert_eq!(x.text(), "a <b> & AB <raw>&amp;");
}

#[test]
fn dtd_and_entities_are_refused() {
    let laughs = r#"<?xml version="1.0"?>
<!DOCTYPE lolz [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;">]>
<lolz>&lol2;</lolz>"#;
    assert_eq!(p(laughs), Err(Error::DtdNotAllowed));
    let xxe = r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><foo>&xxe;</foo>"#;
    assert_eq!(p(xxe), Err(Error::DtdNotAllowed));
    assert_eq!(p("<a>&xxe;</a>"), Err(Error::UnknownEntity));
    assert_eq!(p("<a>&#0;</a>"), Err(Error::BadCharReference));
    assert_eq!(p("<a>&#xD800;</a>"), Err(Error::BadCharReference));
    assert_eq!(p("<a b='&#1;'/>"), Err(Error::BadCharReference));
}

#[test]
fn well_formedness_is_enforced() {
    assert_eq!(p("<a><b></a></b>"), Err(Error::MismatchedTag));
    assert!(
        matches!(p("<a></ a>"), Err(Error::Syntax(_))),
        "space after </ (K-RG-4)"
    );
    assert_eq!(p("<a x='1' x='2'/>"), Err(Error::DuplicateAttribute));
    assert_eq!(p("<p:a/>"), Err(Error::UnboundPrefix));
    assert_eq!(p("<a p:x='1'/>"), Err(Error::UnboundPrefix));
    assert_eq!(p("<a/><b/>"), Err(Error::TrailingContent));
    assert_eq!(p("<a/>text"), Err(Error::TrailingContent));
    assert!(matches!(p("text<a/>"), Err(Error::Syntax(_))));
    assert_eq!(p("<a>"), Err(Error::Unterminated));
    assert_eq!(p(""), Err(Error::NoRoot));
    assert!(matches!(p("<a>]]></a>"), Err(Error::Syntax(_))));
    assert!(matches!(p("<a><!-- x -- y --></a>"), Err(Error::Syntax(_))));
    assert!(matches!(p("<a b=1/>"), Err(Error::Syntax(_))));
    assert!(matches!(p("<a b='1'c='2'/>"), Err(Error::Syntax(_))));
    assert!(matches!(p("<a>\u{0}</a>"), Err(Error::Syntax(_))));
    assert!(p("<a xmlns:p='urn:p' p:x='1'/>").is_ok());
}

#[test]
fn limits_hold() {
    let deep = "<a>".repeat(65) + &"</a>".repeat(65);
    assert_eq!(p(&deep), Err(Error::TooDeep));
    let ok = "<a>".repeat(64) + &"</a>".repeat(64);
    assert!(p(&ok).is_ok());
    let wide = format!("<a>{}</a>", "<b/>".repeat(10));
    let small = Limits {
        max_nodes: 5,
        ..Limits::default()
    };
    assert_eq!(parse(&wide, &small), Err(Error::TooManyNodes));
    let attrs: String = (0..65).map(|i| format!(" a{i}='x'")).collect();
    assert_eq!(p(&format!("<a{attrs}/>")), Err(Error::TooManyAttributes));
    assert_eq!(
        p(&format!("<{}/>", "n".repeat(300))),
        Err(Error::NameTooLong)
    );
    let tiny = Limits {
        max_input: 10,
        ..Limits::default()
    };
    assert_eq!(parse("<a>0123456789</a>", &tiny), Err(Error::TooLarge));
}

#[test]
fn escape_round_trips() {
    let s = "a<b>&\"'\u{1}c";
    let doc = format!("<a v=\"{}\">{}</a>", escape(s), escape(s));
    let e = p(&doc).unwrap();
    assert_eq!(e.attr("v"), Some("a<b>&\"'c"));
    assert_eq!(e.text(), "a<b>&\"'c");
}

#[test]
fn mutated_documents_never_panic() {
    let seed = r#"<?xml version="1.0"?><r xmlns="urn:a" xmlns:b="urn:b"><b:c d="1&amp;2">t&#x41;<![CDATA[x]]></b:c><!-- c --><e/></r>"#;
    let mut s: u64 = 0x1234_5678_9abc_def1;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let pieces = [
        "<", ">", "/", "&", ";", "\"", "'", "<!", "]]>", "<?", "-->", "xmlns:", ":", "&#", "\u{e9}",
    ];
    for _ in 0..20_000 {
        let mut d = seed.to_string();
        for _ in 0..(rnd() % 5 + 1) {
            let mut pos = (rnd() as usize) % (d.len() + 1);
            while !d.is_char_boundary(pos) {
                pos -= 1;
            }
            match rnd() % 3 {
                0 => d.insert_str(pos, pieces[(rnd() as usize) % pieces.len()]),
                1 => d.truncate(pos),
                _ => {
                    let mut end = (pos + (rnd() as usize % 4)).min(d.len());
                    while !d.is_char_boundary(end) {
                        end += 1;
                    }
                    d.replace_range(pos..end, "");
                }
            }
        }
        let _ = p(&d);
    }
}
