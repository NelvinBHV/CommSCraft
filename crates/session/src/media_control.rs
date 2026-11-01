//! Video control through SIP INFO (function 31): `application/media_control+xml` of
//! draft-levin-mmusic-xml-media-control-12 (IE-07) with the `picture_freeze` extension of
//! [MS-XMLMC] 20250520 (PL-50).
//!
//! Documented: the body is carried in a SIP INFO ([RFC2976]; XMLSMC §8); a `vc_primitive` holds
//! one `to_encoder` command – `picture_fast_update` (send a full picture, XMLSMC §4) or
//! `picture_freeze` (stop sending video, keep RTCP; only sent by a central video processor,
//! XMLMC §1.6, §3.1.5.1); a well-formed `picture_freeze` is never answered with an error
//! (§3.1.5.2); an INFO with a body only gets a 200 OK (XMLSMC §6). A `stream_id` is not
//! interpreted (XMLMC §3.2.1.1: MCUs leave it out).
//!
//! Security by design: the body is parsed with tight limits (it is a few elements); unknown
//! elements are ignored; the number of commands taken from one body is bounded; nothing of
//! the body is echoed back (we send no `general_error` – the draft makes it optional and it
//! would reflect peer input).

use commscraft_xml::{Element, Limits};

/// The MIME type of the body (XMLSMC §8, §9.1).
pub const CONTENT_TYPE: &str = "application/media_control+xml";
/// At most this many commands are taken from one body.
const MAX_COMMANDS: usize = 8;

/// A command to our video encoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoControl {
    /// Send a full picture as soon as possible (and resume sending after a freeze).
    FastUpdate,
    /// Stop sending video until the next fast update (from an MCU only).
    Freeze,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Xml(commscraft_xml::Error),
    /// The root element is not `media_control`.
    NotMediaControl,
}

fn limits() -> Limits {
    Limits {
        max_input: 4096,
        max_depth: 8,
        max_attributes: 8,
        max_nodes: 64,
        ..Limits::default()
    }
}

/// Whether a Content-Type header value names a media control body (parameters such as
/// `charset` are allowed, XMLSMC §9.1).
pub fn is_media_control(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|t| t.trim().eq_ignore_ascii_case(CONTENT_TYPE))
}

/// The commands of a body, in order; unknown commands are skipped, a body without any is an
/// empty list (e.g. a `general_error` report).
pub fn parse(body: &str) -> Result<Vec<VideoControl>, Error> {
    let root = commscraft_xml::parse(body, &limits()).map_err(Error::Xml)?;
    if root.name != "media_control" {
        return Err(Error::NotMediaControl);
    }
    let commands = root
        .children_named("vc_primitive")
        .filter_map(|p| p.child("to_encoder"))
        .flat_map(Element::elements)
        .filter_map(|c| match c.name.as_str() {
            "picture_fast_update" => Some(VideoControl::FastUpdate),
            "picture_freeze" => Some(VideoControl::Freeze),
            _ => None,
        })
        .take(MAX_COMMANDS)
        .collect();
    Ok(commands)
}

/// The body of a fast update request (XMLSMC §7.1).
pub fn fast_update() -> String {
    "<?xml version=\"1.0\" encoding=\"utf-8\" ?>\r\n<media_control>\r\n  <vc_primitive>\r\n    \
     <to_encoder>\r\n      <picture_fast_update/>\r\n    </to_encoder>\r\n  </vc_primitive>\r\n\
     </media_control>\r\n"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// XMLSMC §7.1 and XMLMC §4: the documented examples.
    #[test]
    fn documented_examples() {
        assert_eq!(parse(&fast_update()), Ok(vec![VideoControl::FastUpdate]));
        let freeze = "<?xml version=\"1.0\" encoding=\"utf-8\" ?>\n<media_control>\n<vc_primitive>\n\
                      <to_encoder>\n<picture_freeze/>\n</to_encoder>\n</vc_primitive>\n</media_control>";
        assert_eq!(parse(freeze), Ok(vec![VideoControl::Freeze]));
        // XMLSMC §7.2: an error report carries no command.
        let error = "<media_control><general_error>Parsing error</general_error></media_control>";
        assert_eq!(parse(error), Ok(vec![]));
        // A stream_id is not interpreted (XMLMC §3.2.1.1); unknown commands are skipped.
        let mixed = "<media_control><vc_primitive><to_encoder><zoom/></to_encoder></vc_primitive>\
                     <vc_primitive><to_encoder><picture_fast_update/></to_encoder>\
                     <stream_id>7</stream_id></vc_primitive></media_control>";
        assert_eq!(parse(mixed), Ok(vec![VideoControl::FastUpdate]));
    }

    /// Hostile bodies: other roots, entities, oversized and deep input, many commands.
    #[test]
    fn hostile_bodies() {
        assert_eq!(parse("<other/>"), Err(Error::NotMediaControl));
        assert!(parse("").is_err());
        assert!(parse("<media_control>").is_err());
        assert!(parse(
            "<!DOCTYPE m [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><media_control>&x;</media_control>"
        )
        .is_err());
        assert!(parse(&format!(
            "<media_control>{}</media_control>",
            " ".repeat(5000)
        ))
        .is_err());
        assert!(parse(&format!("{}{}", "<a>".repeat(20), "</a>".repeat(20))).is_err());
        let many = format!(
            "<media_control>{}</media_control>",
            "<vc_primitive><to_encoder><picture_fast_update/></to_encoder></vc_primitive>"
                .repeat(12)
        );
        // Bounded by the node limit or by MAX_COMMANDS.
        assert!(parse(&many).map_or(true, |c| c.len() <= MAX_COMMANDS));
    }

    #[test]
    fn content_type() {
        assert!(is_media_control("application/media_control+xml"));
        assert!(is_media_control(
            "Application/Media_Control+XML; charset=utf-8"
        ));
        assert!(!is_media_control("application/sdp"));
        assert!(!is_media_control("application/media_control+xmlx"));
    }
}
