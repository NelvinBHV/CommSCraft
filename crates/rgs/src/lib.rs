//! The response group agent (function 47; WS-01): the Response Group Service web service
//! ([MS-RGSWS] 20250520) tells a user whether they are an agent, lists their agent groups with
//! the calls waiting, signs them in and out of groups and lists the response groups they serve.
//!
//! SOAP 1.1 over HTTPS (the WSDL's `soap:` binding, §6, p. 37; §2.1, p. 12) at the URL of in-band
//! provisioning (`responseGroupServiceInternalURL`/`…ExternalURL`, [MS-SIPREGE] §2.2.2.5.1),
//! authenticated as [MS-OCAUTHWS] describes (§1.5, p. 11): the service's metadata name a
//! web-ticket endpoint only (`tested-in-lab`), so the caller wraps [`body`] in an envelope with a
//! web ticket (`ocauthws::webticket`).
//!
//! Security by design: group IDs are taken and sent only as GUIDs; answers are read with the
//! bounded XML reader, at most [`MAX_GROUPS`] groups and [`MAX_ENTRIES`] response groups; texts
//! are stripped of control characters and bounded.

use commscraft_xml::{Element, Limits};

pub const CONTENT_TYPE: &str = "text/xml; charset=utf-8";
const NS_SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const NS_RGS: &str = "http://schemas.microsoft.com/acd/2007/12/Microsoft.Rtc.Acd.Clients.Proxy";
/// Most groups, response groups and agents per group kept from an answer (design).
pub const MAX_GROUPS: usize = 256;
pub const MAX_ENTRIES: usize = 256;
pub const MAX_AGENTS: usize = 512;
const MAX_TEXT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadGroupId,
    Xml(commscraft_xml::Error),
    WrongDocument(&'static str),
    /// SOAP fault from the server (fault string, printable).
    Fault(String),
}

/// The operations of §3.1.4 (pp. 16–33).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    IsAgent,
    GetAgent,
    GetGroups,
    GetActiveResponseGroups,
    SignIn(String),
    SignOut(String),
}

impl Op {
    pub fn name(&self) -> &'static str {
        match self {
            Op::IsAgent => "IsAgent",
            Op::GetAgent => "GetAgent",
            Op::GetGroups => "GetGroups",
            Op::GetActiveResponseGroups => "GetActiveResponseGroups",
            Op::SignIn(_) => "SignIn",
            Op::SignOut(_) => "SignOut",
        }
    }

    /// The `soapAction` of the operation (§6, pp. 39–40).
    pub fn soap_action(&self) -> String {
        format!("\"{NS_RGS}/{}\"", self.name())
    }
}

/// A GUID in its usual form (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`), lower case.
pub fn guid(s: &str) -> Option<String> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split('-').collect();
    let ok = parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()));
    ok.then(|| s.to_ascii_lowercase())
}

/// The SOAP request of an operation (examples §4.1–4.3, pp. 34–35).
pub fn request(op: &Op) -> Result<String, Error> {
    let inner = body(op)?;
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><soap:Envelope xmlns:soap=\"{NS_SOAP}\"><soap:Body>{inner}</soap:Body></soap:Envelope>"
    ))
}

/// The SOAP body content of an operation (for an envelope with a web ticket, [MS-OCAUTHWS]).
pub fn body(op: &Op) -> Result<String, Error> {
    Ok(match op {
        Op::SignIn(g) | Op::SignOut(g) => {
            let g = guid(g).ok_or(Error::BadGroupId)?;
            format!(
                "<{0} xmlns=\"{NS_RGS}\"><groupId>{g}</groupId></{0}>",
                op.name()
            )
        }
        _ => format!("<{} xmlns=\"{NS_RGS}\"/>", op.name()),
    })
}

fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEXT)
        .collect::<String>()
        .trim()
        .to_string()
}

/// The `<Op>Result` element of an answer.
fn result<'a>(root: &'a Element, op: &Op) -> Result<Option<&'a Element>, Error> {
    if root.name != "Envelope" || root.namespace.as_deref() != Some(NS_SOAP) {
        return Err(Error::WrongDocument("soap:Envelope"));
    }
    let body = root
        .child("Body")
        .ok_or(Error::WrongDocument("soap:Body"))?;
    if let Some(f) = body.child("Fault") {
        let text = f.child("faultstring").map(|s| s.text()).unwrap_or_default();
        return Err(Error::Fault(printable(&text)));
    }
    let r = body
        .child(&format!("{}Response", op.name()))
        .filter(|r| r.namespace.as_deref() == Some(NS_RGS))
        .ok_or(Error::WrongDocument("operation response"))?;
    Ok(r.child(&format!("{}Result", op.name())))
}

fn parse(xml: &str) -> Result<Element, Error> {
    commscraft_xml::parse(
        xml,
        &Limits {
            max_input: 1024 * 1024,
            ..Limits::default()
        },
    )
    .map_err(Error::Xml)
}

fn text(e: &Element, name: &str) -> Option<String> {
    e.child(name)
        .map(|c| printable(&c.text()))
        .filter(|t| !t.is_empty())
}

fn flag(e: &Element, name: &str) -> bool {
    e.child(name).map(|c| c.text().trim().to_string()) == Some("true".into())
}

fn number(e: &Element, name: &str) -> u32 {
    e.child(name)
        .and_then(|c| c.text().trim().parse().ok())
        .unwrap_or(0)
}

/// The boolean result of IsAgent, SignIn and SignOut (§3.1.4.1, §3.1.4.4, §3.1.4.5).
pub fn parse_bool(xml: &str, op: &Op) -> Result<bool, Error> {
    let root = parse(xml)?;
    let r = result(&root, op)?.ok_or(Error::WrongDocument("result"))?;
    match r.text().trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(Error::WrongDocument("boolean")),
    }
}

/// An agent (AcdAgent, §2.2.4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub id: Option<String>,
    pub sip_address: Option<String>,
    pub display_name: Option<String>,
}

fn agent(e: &Element) -> Agent {
    Agent {
        id: e.child("Id").and_then(|i| guid(&i.text())),
        sip_address: text(e, "SipAddress"),
        display_name: text(e, "DisplayName"),
    }
}

/// The user as agent (GetAgent, §3.1.4.2); `None` when the user is no agent.
pub fn parse_agent(xml: &str) -> Result<Option<Agent>, Error> {
    let root = parse(xml)?;
    Ok(result(&root, &Op::GetAgent)?.map(agent))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInState {
    SignedIn,
    SignedOut,
    Unknown,
}

/// An agent group (AcdGroup, §2.2.4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub agents: u32,
    /// The agent may sign in and out (a formal group); an informal group signs in by presence.
    pub can_sign_in: bool,
    pub state: SignInState,
    pub calls_waiting: u32,
    /// Longest waiting time, in seconds.
    pub longest_wait: u32,
    pub members: Vec<Agent>,
}

/// The user's agent groups (GetGroups, §3.1.4.3); a group without a usable ID is left out.
pub fn parse_groups(xml: &str) -> Result<Vec<Group>, Error> {
    let root = parse(xml)?;
    let Some(r) = result(&root, &Op::GetGroups)? else {
        return Ok(Vec::new());
    };
    Ok(r.children_named("AcdGroup")
        .filter_map(|g| {
            Some(Group {
                id: guid(&g.child("Id")?.text())?,
                name: text(g, "Name").unwrap_or_default(),
                agents: number(g, "NumberOfAgents"),
                can_sign_in: flag(g, "CanSignIn"),
                state: match g.child("SignInState").map(|s| s.text()).as_deref() {
                    Some(s) if s.trim() == "SignedIn" => SignInState::SignedIn,
                    Some(s) if s.trim() == "SignedOut" => SignInState::SignedOut,
                    _ => SignInState::Unknown,
                },
                calls_waiting: number(g, "NumberOfCallsWaiting"),
                longest_wait: number(g, "LongestWaitingTime"),
                members: g
                    .child("AllAgents")
                    .map(|a| {
                        a.children_named("AcdAgent")
                            .take(MAX_AGENTS)
                            .map(agent)
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .take(MAX_GROUPS)
        .collect())
}

/// A response group the user serves (ResponseGroupEntry, §2.2.4.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseGroup {
    pub uri: String,
    pub display_name: Option<String>,
    /// Calls of agents are anonymized (the caller does not see the agent).
    pub anonymized: bool,
    /// The agent may call out on behalf of the response group.
    pub outbound_allowed: bool,
}

/// The active response groups (GetActiveResponseGroups, §3.1.4.8; example §4.1, p. 34); an
/// entry without a SIP URI is left out.
pub fn parse_response_groups(xml: &str) -> Result<Vec<ResponseGroup>, Error> {
    let root = parse(xml)?;
    let Some(r) = result(&root, &Op::GetActiveResponseGroups)? else {
        return Ok(Vec::new());
    };
    Ok(r.children_named("ResponseGroupEntry")
        .filter_map(|e| {
            let uri = text(e, "Uri")?;
            let ok = uri.starts_with("sip:")
                && uri.contains('@')
                && uri
                    .chars()
                    .all(|c| c.is_ascii_graphic() && !"<>\"'&,".contains(c));
            ok.then(|| ResponseGroup {
                uri,
                display_name: text(e, "DisplayName"),
                anonymized: flag(e, "IsAnonymized"),
                outbound_allowed: flag(e, "IsOutboundAllowed"),
            })
        })
        .take(MAX_ENTRIES)
        .collect())
}

#[cfg(test)]
mod tests;
