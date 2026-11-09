//! Changing the contact list and its groups.
//!
//! Evidence: file 05 (SIP-07, SIP-08), file 07 (PR-50, PR-51, PR-52). Sources: [MS-SIP] 20170601
//! §2.2.4.2 (pp. 18–19), §3.7.4.3–§3.7.4.4 (p. 44), §4.6–§4.9 (pp. 60–64), §9 (pp. 80 ff.).
//!
//! Security by design: every change names the list version it is based on (`deltaNum`), so it
//! never overwrites a change made elsewhere; a refused change is not repeated, the list is read
//! again and the user decides; one change at a time; a list kept in Exchange is not changed here;
//! an entry with data this client does not keep is not rewritten.

use super::*;

/// How long a change waits for the changed list before the list is fetched anyway (seconds).
const EDIT_WAIT: u64 = 10;

impl<R: Random> Engine<R> {
    /// PR-51: the client can reach Exchange. The contact list subscription then carries
    /// `ms-ucs` and `ms-ucs-ready` (§3.10.4, p. 134: "SIP enabled endpoints that can connect to
    /// the email server SHOULD add the ms-ucs supported header"; example §4.14.1, p. 197) and
    /// is sent again at once.
    pub fn enable_exchange_contacts(&mut self, now: u64) -> Vec<Output> {
        let mut out = Vec::new();
        if self.ucs {
            return out;
        }
        self.ucs = true;
        if let Some(i) = self.contacts_index().filter(|_| self.registered) {
            self.resubscribe_now(i, now, &mut out);
        }
        out
    }

    /// Whether the contact list is kept in Exchange (read and changed there by the client).
    pub fn contacts_in_exchange(&self) -> bool {
        self.contacts_in_exchange
    }

    /// The server moved the list: no more contact list subscription (§3.10.4: endpoints that
    /// connect to the email server keep `ms-ucs` and get 488).
    pub(crate) fn on_contacts_moved(&mut self, index: usize, out: &mut Vec<Output>) {
        if let Some(s) = self.subs.get_mut(index) {
            s.dialog = None;
            s.refresh_at = None;
            s.retry_at = None;
        }
        self.contact_edit_until = None;
        if !self.contacts_in_exchange {
            self.contacts_in_exchange = true;
            out.push(Output::Event(Event::ContactsInExchange));
        }
    }

    /// The contact list as read from Exchange (EX-04 GetImItemList): kept like a list from the
    /// server, and the presence of its contacts is subscribed.
    pub fn set_exchange_contacts(&mut self, mut list: pres::ContactList, now: u64) -> Vec<Output> {
        let mut out = Vec::new();
        list.ucs_mode = Some("migrated".into());
        let uris: Vec<String> = list.contacts.iter().map(|c| c.uri.clone()).collect();
        self.contact_list = Some(list.clone());
        out.push(Output::Event(Event::ContactList(list)));
        self.replace_presence_subscriptions(uris, now, &mut out);
        out
    }

    /// The contact list as last received.
    pub fn contact_list(&self) -> Option<&pres::ContactList> {
        self.contact_list.as_ref()
    }

    /// `EditContacts`: a SERVICE to the own AOR at the list's current delta (§3.7.4.3).
    pub(crate) fn edit_contacts(
        &mut self,
        edit: pres::ContactEdit,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let refusal = match (&self.contact_list, &edit) {
            (None, _) => Some("the contact list is not loaded yet"),
            (Some(l), _) if l.ucs_mode.as_deref() == Some("migrated") => {
                // PR-51: the list lives in Exchange; the client changes it there (EX-04).
                Some("the contact list is kept in Exchange and is changed there")
            }
            (Some(l), _) if l.delta_num.is_none() => Some("the contact list has no version"),
            (Some(l), pres::ContactEdit::Set { uri, .. })
                if l.contacts.iter().any(|c| c.uri == *uri && c.extension) =>
            {
                Some("this contact holds data that changing it here would lose")
            }
            _ if self.contact_edit_until.is_some() => Some("a contact change is still under way"),
            _ => None,
        };
        if let Some(text) = refusal {
            out.push(Output::Event(Event::Notice(text.into())));
            return;
        }
        let delta = self
            .contact_list
            .as_ref()
            .and_then(|l| l.delta_num)
            .unwrap_or(0);
        let Ok(body) = pres::contact_edit_body(&edit, delta) else {
            out.push(Output::Event(Event::Notice(
                "not a usable contact or group".into(),
            )));
            return;
        };
        let aor = self.cfg.aor.clone();
        let mut m = self.request("SERVICE", &aor, &aor);
        m.push_header("Content-Type", pres::CONTACT_EDIT_TYPE);
        m.body = body.into_bytes();
        self.contact_edit_until = Some(now.saturating_add(EDIT_WAIT));
        self.send_request(m, Purpose::ContactEdit, now, out);
    }

    /// A document of the contact subscription, in a notification or a response.
    pub(crate) fn on_contact_document(
        &mut self,
        index: usize,
        body: &str,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        match pres::contact_document_kind(body) {
            pres::ContactDocument::Full => match pres::parse_contact_list(body) {
                Ok(list) => {
                    let uris: Vec<String> = list.contacts.iter().map(|c| c.uri.clone()).collect();
                    self.contact_list = Some(list.clone());
                    self.contact_edit_until = None;
                    out.push(Output::Event(Event::ContactList(list)));
                    self.replace_presence_subscriptions(uris, now, out);
                }
                Err(e) => out.push(Output::Event(Event::Notice(format!(
                    "unusable contact list ({e:?})"
                )))),
            },
            // A change: fetch the full list again with a new subscription (MS-SIP §3.7.5.2).
            pres::ContactDocument::Delta => self.resubscribe_now(index, now, out),
            pres::ContactDocument::Other => {}
        }
    }

    fn contacts_index(&self) -> Option<usize> {
        self.subs
            .iter()
            .position(|s| matches!(s.kind, SubKind::ContactList))
    }

    /// The change is accepted; the changed list comes as a notification or in this response
    /// (§9.1, "piggy-back notify").
    pub(crate) fn on_contact_edit_answered(
        &mut self,
        resp: &Message,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        if let (Some(index), Ok(body)) = (self.contacts_index(), std::str::from_utf8(&resp.body)) {
            self.on_contact_document(index, body, now, out);
        }
    }

    /// Refused, for example because of a change elsewhere: read the list again, change nothing.
    pub(crate) fn on_contact_edit_failed(&mut self, code: u16, now: u64, out: &mut Vec<Output>) {
        self.contact_edit_until = None;
        out.push(Output::Event(Event::Notice(format!(
            "contact list not changed ({code})"
        ))));
        if let Some(index) = self.contacts_index() {
            self.resubscribe_now(index, now, out);
        }
    }

    /// No changed list arrived: fetch it, the next change needs its version.
    pub(crate) fn contact_edit_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        if self.contact_edit_until.is_some_and(|until| now >= until) {
            self.contact_edit_until = None;
            if let Some(index) = self.contacts_index() {
                self.resubscribe_now(index, now, out);
            }
        }
    }
}
