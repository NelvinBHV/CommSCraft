//! Emergency calls with location (function 39; [MS-SIPRE] 20250520 §2.2.23, §3.17.5.1, example
//! §4.14 – RE-67; [MS-SIPREGE] §2.2.2.5.7 – RG-51; [MS-E911WS] – E9-01 … E9-07): the
//! locationPolicy from provisioning decides which dial strings are emergency numbers; the location
//! comes from the location information service (the client asks it over HTTPS) or from the user
//! (`Manual`). An emergency call publishes a time-bound routing preamble with `e911active` (all
//! forwarding suspended), then sends the INVITE with `Priority: emergency`, `Supported:
//! geolocation`, a `Geolocation` header and the PIDF-LO as the last MIME part. After the call the
//! user's own routing rules are published again.
//!
//! Security by design: an emergency call is never refused for a missing location (it then goes
//! without PIDF-LO); everything in the PIDF-LO is escaped by its builder; only dial strings from
//! the policy take the emergency path.

use commscraft_lis as lis;

use crate::call::{ExtraPart, Extras};
use crate::{pres, sdp, Engine, Event, Output, Random};

/// Lifetime of the time-bound `e911active` routing publication (design: one hour; the
/// specification fixes none, §3.17.5.1).
pub(crate) const E911_ROUTING_SECONDS: u32 = 3600;

impl<R: Random> Engine<R> {
    /// The emergency dial string a dialled number stands for, if the location policy enables
    /// enhanced emergency services (RG-51).
    pub fn emergency_dial(&self, dialled: &str) -> Option<String> {
        self.location_policy
            .as_ref()
            .and_then(|p| p.emergency(dialled))
            .map(str::to_string)
    }

    /// `SetLocation`: from the location information service or typed by the user.
    pub(crate) fn set_location(&mut self, loc: lis::Location, out: &mut Vec<Output>) {
        out.push(Output::Event(Event::Location(Box::new(loc.clone()))));
        self.location = Some(loc);
    }

    /// `EmergencyCall` (RE-67; §3.17.5.1, p. 80).
    pub(crate) fn emergency_call(
        &mut self,
        dialled: &str,
        local: sdp::Local,
        policy: sdp::Encryption,
        now: u64,
        out: &mut Vec<Output>,
    ) {
        let Some(dial) = self.emergency_dial(dialled) else {
            out.push(Output::Event(Event::Notice(format!(
                "{dialled} is not an emergency number of the location policy"
            ))));
            return;
        };
        let Some(uri) = self.phone_uri(&dial) else {
            out.push(Output::Event(Event::Notice(
                "not a usable dial string".into(),
            )));
            return;
        };
        let aor = self.cfg.aor.clone();
        let cid = lis::content_id(&aor);
        let mut extra = vec![
            ("Priority".to_string(), "emergency".to_string()),
            ("Supported".to_string(), "geolocation".to_string()),
        ];
        let ext = self
            .location_policy
            .as_ref()
            .map(|p| lis::Extension {
                conference_uri: p.conference_uri.clone(),
                conference_mode: p.conference_mode.clone(),
                tag_id: p.tag_id.clone(),
            })
            .unwrap_or_default();
        let part = self
            .location
            .as_ref()
            .and_then(|loc| lis::pidf_lo(&aor, loc, &ext, now).ok())
            .map(|body| ExtraPart {
                content_type: lis::PIDF_TYPE.to_string(),
                content_id: cid.clone(),
                body,
            });
        if part.is_some() {
            // RFC 6442 §4.1: the cid URL names the Content-ID of the location part (K-RE-3).
            extra.push((
                "Geolocation".to_string(),
                format!("<cid:{cid}>;inserted-by=\"{aor}\""),
            ));
        }
        let with = if part.is_some() {
            "with location"
        } else {
            "WITHOUT location (none known)"
        };
        // §3.17.5.1: compose the INVITE, publish the time-bound e911active preamble (all
        // forwarding suspended), send the INVITE – without waiting for the publication, so the
        // emergency call is never delayed. The routing is published only once the call exists
        // (issue #67: no suspended forwarding without an emergency call to end it).
        let mut call_out = Vec::new();
        let started = self.start_call_with(
            &uri,
            local,
            policy,
            Extras {
                headers: &extra,
                part: part.as_ref(),
                emergency: true,
            },
            now,
            &mut call_out,
        );
        if let Some(id) = started {
            let mut rules = self.routing_rules.clone();
            rules.e911_active = true;
            self.publish_routing(rules, now, out);
            if !self.emergency_calls.contains(&id) {
                self.emergency_calls.push(id);
            }
            out.extend(call_out);
            out.push(Output::Event(Event::Notice(format!(
                "emergency call {id} to {dial}, {with}"
            ))));
        } else {
            out.extend(call_out);
            out.push(Output::Event(Event::Notice(format!(
                "EMERGENCY CALL to {dial} COULD NOT BE STARTED"
            ))));
        }
    }

    /// After the last emergency call: the user's own routing rules again (static). While any
    /// emergency call exists the suspension stays (issue #70).
    pub(crate) fn emergency_tick(&mut self, now: u64, out: &mut Vec<Output>) {
        if self.emergency_calls.is_empty() {
            return;
        }
        let calls = &self.calls;
        self.emergency_calls.retain(|id| calls.contains_key(id));
        if self.emergency_calls.is_empty() {
            self.restore_routing(now, out);
        }
    }

    /// The user's rules again after the emergency suspension (or a notice when they are not
    /// known, O-RE-3). If the registration is lost before this runs, the time-bound
    /// publication ends on the server after [`E911_ROUTING_SECONDS`].
    pub(crate) fn restore_routing(&mut self, now: u64, out: &mut Vec<Output>) {
        if self.routing_known {
            let rules: pres::Forwarding = self.routing_rules.clone();
            self.publish_routing(rules, now, out);
        } else {
            // O-RE-3: rules set elsewhere are not known here and cannot be restored.
            out.push(Output::Event(Event::Notice(
                "forwarding was suspended for the emergency call; check your forwarding settings"
                    .into(),
            )));
        }
    }
}
