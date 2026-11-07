//! Privacy relationships and blocking (function 12).
//!
//! Evidence: file 07 (PR-03, PR-20, PR-60), file 39 (OB-05). Sources: [MS-PRES] 20250520
//! §1.3.1.3.4, §2.2.2.5.2–3 (pp. 46–48), §4.9; [MS-SIPREGE] 20250819 §4.2.2 (manifest with the
//! block rules for container 32000, p. 100 ff.); [MS-SIPRE] 20250520 §3.9.5 (block flag: the proxy
//! answers 480); Lync SDK schema "Blocked Contacts container": the blocking routing instance.
//!
//! Security by design: blocking takes effect in two places, at the server (the routing instance)
//! and here (no call or conversation from a blocked user or domain reaches the user, with the
//! server's answer, so the blocked person cannot tell). Membership changes name the container
//! versions; a refused change is not repeated, the containers are read again.

use super::*;

/// How long a change waits for the changed containers before they are fetched anyway (seconds).
const EDIT_WAIT: u64 = 10;

impl<R: Random> Engine<R> {
    /// The users with a set relationship.
    pub fn relationships(&self) -> Vec<(pres::Relationship, Vec<String>)> {
        pres::Relationship::ALL
            .into_iter()
            .map(|r| {
                let users = self
                    .containers
                    .iter()
                    .filter_map(|c| c.get(&r.container()))
                    .flat_map(|c| c.members.iter())
                    .filter(|m| m.kind == "user")
                    .filter_map(|m| m.value.clone())
                    .collect();
                (r, users)
            })
            .collect()
    }

    /// `SetRelationship`: out of the other relationship containers and into the new one, in one
    /// request. Blocking also publishes the blocking routing instance if it is not there yet.
    pub(crate) fn set_relationship(
        &mut self,
        uri: &str,
        relationship: Option<pres::Relationship>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let notice = |out: &mut Vec<Output>, text: &str| {
            out.push(Output::Event(Event::Notice(text.into())));
        };
        let Ok(user) = pres::member_value(uri) else {
            notice(out, "not a usable SIP address");
            return;
        };
        let Some(containers) = &self.containers else {
            notice(out, "the privacy relationships are not loaded yet");
            return;
        };
        if self.privacy_edit_until.is_some() {
            notice(out, "a relationship change is still under way");
            return;
        }
        let target = relationship.map(pres::Relationship::container);
        let mut changes = Vec::new();
        for r in pres::Relationship::ALL {
            let id = r.container();
            let current = containers.get(&id);
            let member = current.is_some_and(|c| {
                c.members.iter().any(|m| {
                    m.kind == "user"
                        && m.value
                            .as_deref()
                            .is_some_and(|v| v.eq_ignore_ascii_case(&user))
                })
            });
            if member != (target == Some(id)) {
                changes.push(pres::MemberChange {
                    container: id,
                    version: current.map(|c| c.version).unwrap_or(0),
                    add: !member,
                    user: user.clone(),
                });
            }
        }
        if changes.is_empty() {
            notice(out, "the relationship is already set");
            return;
        }
        let Ok(body) = pres::set_container_members_body(&changes) else {
            notice(out, "not a usable SIP address");
            return;
        };
        if relationship == Some(pres::Relationship::Blocked) && self.block_version.is_none() {
            self.publish_block(now, out);
        }
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", pres::SET_CONTAINER_MEMBERS_TYPE);
        m.body = body.into_bytes();
        self.privacy_edit_until = Some(now.saturating_add(EDIT_WAIT));
        self.send_request(m, Purpose::SetMembers, now, out);
    }

    /// The blocking routing instance in the Blocked Contacts container.
    pub(crate) fn publish_block(&mut self, now: u64, out: &mut Vec<Output>) {
        let version = self.block_version.unwrap_or(0);
        let Ok(body) = pres::publish_block_routing_body(&self.cfg.aor, version) else {
            return;
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", "application/msrtc-category-publish+xml");
        m.body = body.into_bytes();
        self.send_request(m, Purpose::PublishBlock, now, out);
    }

    /// Containers of a self document: a notification may list only the changed ones.
    pub(crate) fn on_containers(&mut self, list: Vec<pres::Container>) {
        let known = self.containers.get_or_insert_with(BTreeMap::new);
        for c in list {
            known.insert(c.id, c);
        }
        self.privacy_edit_until = None;
    }

    /// Whether an INVITE comes from a member of the Blocked Contacts container, by user or domain.
    pub(crate) fn from_blocked_user(&self, req: &Message) -> bool {
        let Some(blocked) = self
            .containers
            .as_ref()
            .and_then(|c| c.get(&pres::BLOCKED_CONTAINER))
        else {
            return false;
        };
        let Some(from) = req
            .single("From")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .and_then(|n| pres::member_value(&n.uri).ok())
        else {
            return false;
        };
        let domain = from.rsplit('@').next().unwrap_or("");
        blocked.members.iter().any(|m| {
            let value = m.value.as_deref().unwrap_or("");
            match m.kind.as_str() {
                "user" => value.trim_start_matches("sip:").eq_ignore_ascii_case(&from),
                "domain" => value.eq_ignore_ascii_case(domain),
                _ => false,
            }
        })
    }

    /// Refused, for example because of a change elsewhere: read the containers again.
    pub(crate) fn on_set_members_failed(&mut self, code: u16, now: u64, out: &mut Vec<Output>) {
        self.privacy_edit_until = None;
        out.push(Output::Event(Event::Notice(format!(
            "relationship not changed ({code})"
        ))));
        self.fetch_self_document(now, out);
    }

    /// No changed containers arrived: fetch them, the next change needs their versions.
    pub(crate) fn privacy_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        if self.privacy_edit_until.is_some_and(|until| now >= until) {
            self.privacy_edit_until = None;
            self.fetch_self_document(now, out);
        }
    }

    fn fetch_self_document(&mut self, now: u64, out: &mut Vec<Output>) {
        let index = self
            .subs
            .iter()
            .position(|s| matches!(s.kind, SubKind::SelfRoaming));
        if let Some(index) = index {
            self.resubscribe_now(index, now, out);
        }
    }
}
