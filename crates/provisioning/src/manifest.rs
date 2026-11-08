//! The container manifest of the publication grammar (RG-45; [MS-SIPREGE] 20250819 §2.2.2.5.4,
//! pp. 33–48, and §2.2.2.5.10, pp. 59–71; example §4.2.2, pp. 102–104): which presence
//! containers exist and which members they must have. "The UAC SHOULD create containers and its
//! members based on the constraints defined in the manifest" (p. 44): without the member
//! `sameEnterprise` in a visible container, nobody in the company sees the user's presence –
//! they get the blocked, empty view ([MS-PRES] 20250520 §1.3.1.3.4, p. 20).
//!
//! Only members by network type are handled here (`sameEnterprise`, `federated`, `publicCloud`);
//! members by role (buddy, delegate) follow the contact list and delegates, members by user or
//! domain are the user's own choices.
//!
//! Security by design: the manifest comes from the server over the signed channel but is still
//! bounded – at most [`MAX_CONTAINERS`] containers and [`MAX_MEMBERS`] member rules, container IDs
//! up to 65535; anything else is dropped. A manifest never removes a member here.

use commscraft_xml::Element;

/// Bounds of a manifest (design).
pub const MAX_CONTAINERS: usize = 64;
pub const MAX_MEMBERS: usize = 64;
/// The network member types handled ([MS-SIPREGE] p. 45, sourceNetwork values).
pub const NETWORK_TYPES: [&str; 3] = ["sameEnterprise", "federated", "publicCloud"];

/// How often a member must occur (p. 46).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occurrence {
    /// "one": the member must be there.
    One,
    /// "zeroOrOne", "zero" (to be ignored by a client) or not given: nothing to add.
    Optional,
}

/// One member rule by network type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRule {
    pub kind: String,
    /// The containers the member may be in.
    pub allowed: Vec<u32>,
    pub default_container: Option<u32>,
    pub occurrence: Occurrence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerManifest {
    /// Container IDs with their visible flag.
    pub containers: Vec<(u32, bool)>,
    pub members: Vec<MemberRule>,
}

/// A member to add: its network type and the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMember {
    pub kind: String,
    pub container: u32,
}

fn container_id(e: &Element) -> Option<u32> {
    e.attr("id")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|id| *id <= 65_535)
}

fn flag(e: &Element, name: &str) -> bool {
    // The schema has it as a child element (example p. 104); the prose calls it an attribute.
    e.child(name)
        .map(|c| c.text())
        .or_else(|| e.attr(name).map(str::to_string))
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

impl ContainerManifest {
    /// Reads the first `containerManifest` of a `containerManifestList`.
    pub fn parse(list: &Element) -> Option<ContainerManifest> {
        let m = list.child("containerManifest")?;
        let containers: Vec<(u32, bool)> = m
            .child("containers")?
            .children_named("container")
            .filter_map(|c| Some((container_id(c)?, flag(c, "visible"))))
            .take(MAX_CONTAINERS)
            .collect();
        let visible: Vec<u32> = containers
            .iter()
            .filter(|(_, v)| *v)
            .map(|(id, _)| *id)
            .collect();
        let mut members = Vec::new();
        for r in m
            .child("members")?
            .children_named("member")
            .take(MAX_MEMBERS)
        {
            // Members by role take precedence and are not ours to create (p. 45).
            let Some(kind) = r.attr("type").filter(|_| r.attr("role").is_none()) else {
                continue;
            };
            let Some(kind) = NETWORK_TYPES.iter().find(|t| t.eq_ignore_ascii_case(kind)) else {
                continue;
            };
            let allowed = match r.child("allowedContainers") {
                Some(a) if a.attr("type") == Some("custom") => a
                    .children_named("container")
                    .filter_map(container_id)
                    .take(MAX_CONTAINERS)
                    .collect(),
                Some(a) if a.attr("type") == Some("visibleContainers") => visible.clone(),
                _ => Vec::new(),
            };
            let occurrence = match r.child("occurrenceConstraint").and_then(|o| o.attr("type")) {
                Some(t) if t.eq_ignore_ascii_case("one") => Occurrence::One,
                _ => Occurrence::Optional,
            };
            members.push(MemberRule {
                kind: (*kind).to_string(),
                allowed,
                default_container: r.child("defaultContainer").and_then(container_id),
                occurrence,
            });
        }
        Some(ContainerManifest {
            containers,
            members,
        })
    }

    /// The network members that must be there but are in none of their allowed containers,
    /// each with its default container (p. 46: the member "MAY be added" there "if the
    /// constraint is not satisfied"). `present(container, kind)` tells whether a container has
    /// a member of that type now. A default container outside the allowed ones, container 0
    /// (its membership cannot change, [MS-PRES] §2.2.2.5.2) or one the manifest does not list is
    /// never used.
    pub fn missing(&self, present: impl Fn(u32, &str) -> bool) -> Vec<MissingMember> {
        self.members
            .iter()
            .filter(|r| r.occurrence == Occurrence::One)
            .filter(|r| !r.allowed.iter().any(|c| present(*c, &r.kind)))
            .filter_map(|r| {
                let c = r.default_container?;
                let known = self.containers.iter().any(|(id, _)| *id == c);
                (c != 0 && known && r.allowed.contains(&c)).then(|| MissingMember {
                    kind: r.kind.clone(),
                    container: c,
                })
            })
            .collect()
    }
}
