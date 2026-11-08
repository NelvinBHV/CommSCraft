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
/// How often a session tries to set up the members the container manifest requires.
const MANIFEST_TRIES: u8 = 2;
/// New subscribers reported per session: anyone can subscribe, so they cannot flood the user.
const MAX_TOLD: usize = 256;

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

    /// RG-45: the network members the server's container manifest requires but the user's
    /// containers lack (for example `sameEnterprise` in container 200 – without it nobody in the
    /// company sees the user's presence), added once both are known; at most twice a session.
    pub(crate) fn apply_manifest(&mut self, now: u64, out: &mut Vec<Output>) {
        if self.manifest_tries >= MANIFEST_TRIES || self.privacy_edit_until.is_some() {
            return;
        }
        let (Some(manifest), Some(containers)) = (&self.container_manifest, &self.containers)
        else {
            return;
        };
        let missing = manifest.missing(|id, kind| {
            containers
                .get(&id)
                .is_some_and(|c| c.members.iter().any(|m| m.kind.eq_ignore_ascii_case(kind)))
        });
        if missing.is_empty() {
            self.manifest_tries = MANIFEST_TRIES;
            return;
        }
        let changes: Vec<(u32, u32, &str)> = missing
            .iter()
            .map(|m| {
                let version = containers.get(&m.container).map_or(0, |c| c.version);
                (m.container, version, m.kind.as_str())
            })
            .collect();
        let Ok(body) = pres::add_network_members_body(&changes) else {
            self.manifest_tries = MANIFEST_TRIES;
            return;
        };
        let who: Vec<&str> = missing
            .iter()
            .map(|m| match m.kind.as_str() {
                "sameEnterprise" => "everyone in your company",
                "federated" => "federated users",
                _ => "public IM users",
            })
            .collect();
        self.manifest_tries += 1;
        out.push(Output::Event(Event::Notice(format!(
            "your presence becomes visible as the server's rules ask: {}",
            who.join(", ")
        ))));
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", pres::SET_CONTAINER_MEMBERS_TYPE);
        m.body = body.into_bytes();
        self.privacy_edit_until = Some(now.saturating_add(EDIT_WAIT));
        self.send_request(m, Purpose::ManifestMembers, now, out);
    }

    /// Refused (for example a version conflict): the containers are read again and the
    /// manifest is applied once more to what they say.
    pub(crate) fn on_manifest_members_failed(
        &mut self,
        code: u16,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        self.privacy_edit_until = None;
        if self.manifest_tries >= MANIFEST_TRIES {
            out.push(Output::Event(Event::Notice(format!(
                "others may not see your presence: the server refused the visibility rules ({code})"
            ))));
        }
        self.fetch_self_document(now, out);
    }

    /// Whether an INVITE comes from a member of the Blocked Contacts container, by user or domain.
    pub(crate) fn invite_is_blocked(&self, req: &Message) -> bool {
        req.single("From")
            .ok()
            .flatten()
            .and_then(|v| commscraft_sip_msg::parse_name_addr(v).ok())
            .is_some_and(|n| self.user_is_blocked(&n.uri))
    }

    fn user_is_blocked(&self, uri: &str) -> bool {
        let Some(blocked) = self
            .containers
            .as_ref()
            .and_then(|c| c.get(&pres::BLOCKED_CONTAINER))
        else {
            return false;
        };
        let Ok(user) = pres::member_value(uri) else {
            return false;
        };
        let domain = user.rsplit('@').next().unwrap_or("");
        blocked.members.iter().any(|m| {
            let value = m.value.as_deref().unwrap_or("");
            match m.kind.as_str() {
                "user" => value.trim_start_matches("sip:").eq_ignore_ascii_case(&user),
                "domain" => value.eq_ignore_ascii_case(domain),
                _ => false,
            }
        })
    }

    /// The subscriber list. Each unacknowledged subscriber is reported once per session; it stays
    /// unacknowledged until the user decides, so that it comes back at the next sign-in or on
    /// another device (§1.3.1.4). A blocked user is not reported (`blockRule subscriberPrompt`,
    /// [MS-SIPREGE] §4.2.2) and acknowledged right away.
    pub(crate) fn on_subscribers(
        &mut self,
        list: Vec<pres::Subscriber>,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        for s in list.into_iter().filter(|s| !s.acknowledged) {
            if self.told_subscribers.len() >= MAX_TOLD || self.told_subscribers.contains(&s.uri) {
                continue;
            }
            self.told_subscribers.insert(s.uri.clone());
            if self.user_is_blocked(&s.uri) {
                self.acknowledge_subscriber(&s.uri, now, out);
            } else {
                out.push(Output::Event(Event::NewSubscriber {
                    uri: s.uri,
                    name: s.name,
                }));
            }
        }
    }

    /// `AcknowledgeSubscriber`: one subscriber per request (PR-19).
    pub(crate) fn acknowledge_subscriber(&mut self, uri: &str, now: u64, out: &mut Vec<Output>) {
        let Ok(body) = pres::set_subscriber_body(uri) else {
            out.push(Output::Event(Event::Notice(
                "not a usable SIP address".into(),
            )));
            return;
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", pres::SET_SUBSCRIBER_TYPE);
        m.body = body.into_bytes();
        self.send_request(m, Purpose::AcknowledgeSubscriber, now, out);
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
