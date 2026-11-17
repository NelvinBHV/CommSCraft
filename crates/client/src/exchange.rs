//! The user's own Exchange data for the interfaces: one EWS call (`ews_call`) and the
//! appointments and automatic replies (function 16: GetUserAvailability, [MS-OXWAVLS]
//! 20250520; GetUserOofSettings, [MS-OXWOOF] 20250520), as the server's calendar settings allow
//! ([MS-SIPREGE] §2.2.2.5.6, pp. 50–52: `DisableCalendarPresence` – no calendar data is loaded,
//! `DisablePresenceNote` – the out-of-office message is not loaded).
//!
//! Security by design: the EWS host gets the NTLM authentication only in the user's SIP domain or
//! a trusted domain; TLS is verified; the answers are read by the bounded parsers of `ews`.

use crate::directory::{ntlm_post, HttpNtlm};
use crate::setup::Trust;
use commscraft_ews::calendar::{self as cal, CalendarEvent, Oof};
use commscraft_net::TlsClient;
use commscraft_presence::{ContactEdit, ContactList};
use commscraft_provisioning::CalendarPolicy;

/// One EWS call at the user's EWS URL; the answer's text (a SOAP fault comes with 500).
#[allow(clippy::too_many_arguments)]
pub fn ews_call(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    (name, action): (&str, &str),
    body: &str,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let base = settings
        .https_url(&["InternalEwsUrl", "ExternalEwsUrl"])
        .ok_or("Exchange named no web services URL")?;
    let url = commscraft_url::parse_https(base).ok_or("unusable EWS URL")?;
    let own = aor.rsplit('@').next().unwrap_or_default();
    if !commscraft_url::in_domain(&url.host, own) && !trust.trusts(&url.host) {
        return Err(format!(
            "{} is outside your SIP domain and not trusted",
            url.host
        ));
    }
    log(&format!("  POST {base} ({name})"));
    let r = ntlm_post(
        tls,
        &url,
        &[
            ("Content-Type", commscraft_ews::CONTENT_TYPE),
            ("SOAPAction", action),
        ],
        body,
        user_agent,
        ntlm,
    )?;
    match r.status {
        401 => Err("not authorized at Exchange (401)".into()),
        200 | 500 => Ok(String::from_utf8_lossy(&r.body).to_string()),
        s => Err(format!("HTTP {s}")),
    }
}

/// The user's appointments from the start of today for four days, and the automatic replies.
/// Each part may be switched off by the server or fail on its own; both failing is an error.
#[allow(clippy::too_many_arguments)]
pub fn appointments(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    policy: &CalendarPolicy,
    now: u64,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(Option<Vec<CalendarEvent>>, Option<Oof>), String> {
    let mailbox = aor.trim_start_matches("sip:").to_ascii_lowercase();
    let (start, end) = cal::window(now);
    let events = if policy.disable_calendar {
        None
    } else {
        let request =
            cal::availability_request(&mailbox, start, end).map_err(|e| format!("{e:?}"))?;
        Some(
            ews_call(
                tls,
                trust,
                aor,
                settings,
                ("GetUserAvailability", cal::GET_USER_AVAILABILITY_ACTION),
                &request,
                user_agent,
                ntlm,
                log,
            )
            .and_then(|x| cal::parse_availability(&x, start).map_err(|e| format!("{e:?}")))
            .map(|a| a.events),
        )
    };
    let oof = if policy.disable_oof_note {
        None
    } else {
        let request = cal::oof_request(&mailbox).map_err(|e| format!("{e:?}"))?;
        Some(
            ews_call(
                tls,
                trust,
                aor,
                settings,
                ("GetUserOofSettings", cal::GET_USER_OOF_SETTINGS_ACTION),
                &request,
                user_agent,
                ntlm,
                log,
            )
            .and_then(|x| cal::parse_oof(&x).map_err(|e| format!("{e:?}"))),
        )
    };
    match (events, oof) {
        (Some(Err(a)), Some(Err(o))) => Err(format!("{a}; {o}")),
        (Some(Err(a)), None) | (None, Some(Err(a))) => Err(a),
        (e, o) => Ok((e.and_then(Result::ok), o.and_then(Result::ok))),
    }
}

/// The Exchange identifiers behind the numbers the contact list from Exchange shows
/// (function 11): group number → group, contact address → contact.
#[derive(Debug, Default, Clone)]
pub struct UcsIds {
    pub groups: Vec<(u32, commscraft_ews::ucs::ItemId)>,
    pub contacts: Vec<(String, commscraft_ews::ucs::ItemId)>,
}

/// The contact list from Exchange (GetImItemList, EX-04/EX-05) in the form the session keeps:
/// groups numbered in order, contacts with a SIP address as IM address in the groups whose
/// members they are (MemberCorrelationKey, [MS-OXWSCOS] §2.2.4.2, through the contact items
/// behind the persona – K-EX-2).
pub fn contact_list_of(l: &commscraft_ews::ucs::ImList) -> (ContactList, UcsIds) {
    let mut list = ContactList::default();
    let mut ids = UcsIds::default();
    for (n, g) in (1u32..).zip(&l.groups) {
        // The kinds as the server's own contact list marks them ([MS-OXWSCOS] §2.2.4.2,
        // pp. 14–15): the other contacts are the default group, the favourites pinned; the
        // tagged contacts and distribution groups (no mail address here) are not shown as groups.
        use commscraft_ews::ucs::GroupKind as K;
        let (name, external) = match g.kind {
            K::OtherContacts => ("~".to_string(), ""),
            K::Favorites => (
                g.name.clone(),
                "<groupExtension groupType=\"pinnedGroup\"/>",
            ),
            K::Tagged => (g.name.clone(), "<groupExtension groupType=\"tagged\"/>"),
            K::DistributionGroup => (g.name.clone(), "<groupExtension groupType=\"exchangeDG\"/>"),
            K::UserGroup | K::Other => (g.name.clone(), ""),
        };
        list.groups.push(commscraft_presence::Group {
            id: n,
            name,
            external: external.to_string(),
        });
        if let Some(id) = &g.id {
            ids.groups.push((n, id.clone()));
        }
    }
    for p in &l.personas {
        let uri = p
            .im_address
            .as_deref()
            .and_then(|a| commscraft_presence::checked_sip_uri(a).ok());
        let Some(uri) = uri else {
            list.skipped.push(p.name.clone());
            continue;
        };
        let groups = (1u32..)
            .zip(&l.groups)
            .filter(|(_, g)| p.in_group(g))
            .map(|(n, _)| n)
            .collect();
        list.contacts.push(commscraft_presence::Contact {
            uri: uri.clone(),
            name: p.name.clone(),
            groups,
            subscribed: true,
            extension: false,
        });
        ids.contacts.push((uri, p.contact_id().clone()));
    }
    (list, ids)
}

/// The Exchange operations for a change of the list (EX-04): add a contact (into its first
/// group), move it (into the new groups, out of the old ones), remove it; add, rename and
/// remove a group.
pub fn ucs_ops(
    edit: &ContactEdit,
    list: &ContactList,
    ids: &UcsIds,
) -> Result<Vec<commscraft_ews::ucs::UcsOp>, String> {
    use commscraft_ews::ucs::UcsOp;
    let group = |n: u32| {
        ids.groups
            .iter()
            .find(|(g, _)| *g == n)
            .map(|(_, id)| id.clone())
            .ok_or_else(|| format!("no group {n}"))
    };
    let contact = |uri: &str| {
        ids.contacts
            .iter()
            .find(|(u, _)| u == uri)
            .map(|(_, id)| id.clone())
    };
    Ok(match edit {
        ContactEdit::Set {
            uri, name, groups, ..
        } => match (contact(uri), list.contacts.iter().find(|c| c.uri == *uri)) {
            (Some(id), Some(old)) => {
                let mut ops = Vec::new();
                for g in groups.iter().filter(|g| !old.groups.contains(g)) {
                    ops.push(UcsOp::AddContactToGroup {
                        contact: id.clone(),
                        group: group(*g)?,
                    });
                }
                for g in old.groups.iter().filter(|g| !groups.contains(g)) {
                    ops.push(UcsOp::RemoveContactFromGroup {
                        contact: id.clone(),
                        group: group(*g)?,
                    });
                }
                ops
            }
            _ => vec![UcsOp::AddNewContact {
                im_address: uri.clone(),
                name: (!name.is_empty()).then(|| name.clone()),
                group: groups.first().map(|g| group(*g)).transpose()?,
            }],
        },
        ContactEdit::Delete { uri } => vec![UcsOp::RemoveContact {
            contact: contact(uri).ok_or("not in the contact list")?,
        }],
        ContactEdit::AddGroup { name } => vec![UcsOp::AddGroup { name: name.clone() }],
        ContactEdit::RenameGroup { id, name } => vec![UcsOp::RenameGroup {
            group: group(*id)?,
            name: name.clone(),
        }],
        ContactEdit::DeleteGroup { id } => vec![UcsOp::RemoveGroup { group: group(*id)? }],
    })
}

/// The contact list from Exchange (GetImItemList) and the identifiers behind it.
pub fn contact_list(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(ContactList, UcsIds), String> {
    use commscraft_ews::ucs::{parse_im_item_list, UcsOp};
    let text = ucs_call(
        tls,
        trust,
        aor,
        settings,
        &UcsOp::GetList,
        user_agent,
        ntlm,
        log,
    )?;
    let l = parse_im_item_list(&text).map_err(|e| format!("{e:?}"))?;
    Ok(contact_list_of(&l))
}

/// A change of the contact list in Exchange (EX-04): the operations one after the other,
/// each answer checked, then the list read again.
#[allow(clippy::too_many_arguments)]
pub fn change_contact_list(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    edit: &ContactEdit,
    (list, ids): (&ContactList, &UcsIds),
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(ContactList, UcsIds), String> {
    for op in &ucs_ops(edit, list, ids)? {
        let text = ucs_call(tls, trust, aor, settings, op, user_agent, ntlm, log)?;
        commscraft_ews::ucs::parse_ucs_answer(op, &text).map_err(|e| format!("{e:?}"))?;
    }
    contact_list(tls, trust, aor, settings, user_agent, ntlm, log)
}

#[allow(clippy::too_many_arguments)]
fn ucs_call(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    op: &commscraft_ews::ucs::UcsOp,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let body = commscraft_ews::ucs::ucs_request(op).map_err(|e| format!("{e:?}"))?;
    let action = op.soap_action();
    ews_call(
        tls,
        trust,
        aor,
        settings,
        (op.name(), &action),
        &body,
        user_agent,
        ntlm,
        log,
    )
}

/// Function 49: saves a finished conversation in the Conversation History folder (CreateItem,
/// EX-09, OB-22).
#[allow(clippy::too_many_arguments)]
pub fn save_conversation(
    tls: &TlsClient,
    trust: &Trust,
    aor: &str,
    settings: &commscraft_ews::UserSettings,
    t: &commscraft_ews::history::Conversation,
    user_agent: &str,
    ntlm: &mut HttpNtlm<'_>,
    log: &mut dyn FnMut(&str),
) -> Result<(), String> {
    use commscraft_ews::history;
    let body = history::save_request(t).map_err(|e| format!("{e:?}"))?;
    let answer = ews_call(
        tls,
        trust,
        aor,
        settings,
        ("CreateItem", history::CREATE_ITEM_ACTION),
        &body,
        user_agent,
        ntlm,
        log,
    )?;
    history::parse_save_answer(&answer).map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_group_kinds_map_to_the_contact_list() {
        let xml = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body>
<GetImItemListResponse ResponseClass="Success" xmlns="http://schemas.microsoft.com/exchange/services/2006/messages">
<ResponseCode>NoError</ResponseCode><ImItemList>
 <Groups xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
  <ImGroup><DisplayName>Other Contacts</DisplayName><GroupType>IPM.DistList.MOC.OtherContacts</GroupType>
   <ExchangeStoreId Id="AAMkG1="/><MemberCorrelationKey><ItemId Id="AAMkP1="/></MemberCorrelationKey></ImGroup>
  <ImGroup><DisplayName>Favorites</DisplayName><GroupType>IPM.DistList.MOC.Favorites</GroupType>
   <ExchangeStoreId Id="AAMkG2="/><MemberCorrelationKey><ItemId Id="AAMkP1="/></MemberCorrelationKey></ImGroup>
  <ImGroup><DisplayName>Tagged</DisplayName><GroupType>IPM.DistList.MOC.Tagged</GroupType>
   <ExchangeStoreId Id="AAMkG3="/></ImGroup>
  <ImGroup><DisplayName>Team</DisplayName><GroupType>IPM.DistList.MOC.UserGroup</GroupType>
   <ExchangeStoreId Id="AAMkG4="/></ImGroup>
 </Groups>
 <Personas xmlns="http://schemas.microsoft.com/exchange/services/2006/types">
  <Persona><PersonaId Id="AAMkX1="/><DisplayName>Bob</DisplayName><ImAddress>sip:bob@beispiel.invalid</ImAddress>
   <Attributions><Attribution><Id>0</Id><SourceId Id="AAMkP1="/></Attribution></Attributions></Persona>
 </Personas></ImItemList></GetImItemListResponse></s:Body></s:Envelope>"#;
        let l = commscraft_ews::ucs::parse_im_item_list(xml).unwrap();
        let (list, ids) = contact_list_of(&l);
        use commscraft_presence::{group_extension, GroupExtension as X};
        let kinds: Vec<(String, X)> = list
            .groups
            .iter()
            .map(|g| (g.name.clone(), group_extension(&g.external)))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("~".to_string(), X::Plain),
                ("Favorites".to_string(), X::Pinned),
                ("Tagged".to_string(), X::Other),
                ("Team".to_string(), X::Plain),
            ]
        );
        assert_eq!(list.contacts[0].groups, vec![1, 2]);
        assert_eq!(ids.groups.len(), 4);
    }
}
