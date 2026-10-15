#!/usr/bin/env python3
"""Schema and fail-closed v1.0 feature-parity release gate. Python stdlib only.

Validate: checks schema consistency; intended to pass during development.
Release: requires every client-facing feature verified, evidenced, and a human
signed-off interop manifest against a documented Windows reference deployment.
It cannot replace a real security review or actual interoperability tests.
"""
import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REQUIRED_AREAS = {"identity", "contacts_presence", "messaging", "voice",
                  "video_media", "meetings_sharing", "client_platform"}
STATUSES = {"not_started", "offline_subset", "in_development", "verified"}
# Frozen baseline: removing any hard parity criterion invalidates the release.
# Additional features may be added without weakening the baseline.
REQUIRED_FEATURE_IDS = frozenset("""
    accessibility app-sharing audio-call auth calendar-presence call-history call-monitor callpark
    certificate-auth certificates click-to-call codecs compliance conference-audio
    conference-gallery conference-im conference-join conference-roster contact-groups contact-layout
    contact-prepop contact-privacy contact-search contacts-availability-sort contacts-sorting
    current-conversations delegate-inbound delegate-outbound delegation devices distribution-groups
    dns-discovery e911-distribution-notify e911-sip-notify emergency enterprise-interop
    enterprise-policy exchange-contacts external-calling federation forwarding group-call-pickup
    high-volume-calls hold ice im-1to1 im-email-action im-file im-history im-importance
    im-ink-receive im-inline-photos im-multiparty im-richtext im-tabs location-routing logging
    meeting-anonymous-invite meeting-broadcast meeting-controls meeting-delegate-scheduling
    meeting-demote-all meeting-dial-in meeting-disable-im meeting-disable-video meeting-file-upload
    meeting-invite meeting-lobby meeting-meet-now meeting-mute-audience meeting-polls meeting-qa
    meeting-recording meeting-recording-artifacts meeting-remote-control meeting-scheduling
    meeting-video-spotlight mfa missed-calls modern-auth notifications onenote-notes
    outlook-contacts outlook-delegate-sync outlook-im-history packaging persistent-chat
    persistent-chat-escalate photos photos-public-url powerpoint-navigation powerpoint-sharing
    presence-alerts presence-location presence-note-edit presence-notes presence-publish
    presence-sub presentation provision pstn public-im qos recent-contacts
    recording-archive-artifacts recording-resolution register reliability response-group-calls
    response-group-management response-group-search sharing-rdp sharing-vbss sign-in-logs
    skills-sharepoint-search spellcheck srtp transfer video voicemail whiteboard windows-ui
""".split())


# Frozen independent Windows-reference acceptance scenarios. Update only with a
# reviewed comparison against a real Full Windows client and its official docs.
REQUIRED_WINDOWS_CASE_IDS = frozenset("""
    active-chat-list alternative-contact-layouts answer-delegated-call application-share call-on-behalf
    certificate-login chat-profile-images chatroom-escalation client-signin-logs complete-roster
    conference-upload configure-forwarding contact-click-to-call contact-group-edit contacts-crud
    corporate-address-book custom-location custom-note delegate-configuration demote-all desktop-share
    dial-response-group dialin-conference directory-group-expansion disable-conference-im
    disable-participant-video e911-notify-group e911-notify-sip e911-routing exchange-address-book
    exchange-im-archive external-audio-calls external-photo-url federated-im gallery-view group-im
    group-pickup high-call-volume initial-contacts ink-message-display invite-anonymous kerberos-login
    location-based-routing make-voice-call media-record-archive meet-now meeting-audio meeting-broadcast
    meeting-video message-or-email-action missed-call-alert modern-auth-flow multi-factor-auth mute-all
    onenote-meeting outlook-contact-search outlook-delegate-sync outlook-im-log parking-and-retrieval
    persistent-room-client polls ppt-navigation ppt-present presenter-controls priority-im
    privacy-relationships public-im qa-manager recent-contact-list record-av-sharing
    record-collaboration-content record-file-archive record-resolution remote-control
    response-group-directory response-team-administration schedule-for-delegator schedule-meeting
    sharepoint-skill-search sort-by-group-relationship-new sort-by-presence spell-check spotlight-video
    status-oof status-publish status-watch tabbed-conversations tag-alerts team-call-configuration
    transfer-call video-screen-sharing voice-mail-greeting whiteboard
""".split())
V1_CONTRACT_FILE = ROOT / "V1_RELEASE_CONTRACT.json"
OFFICIAL_WINDOWS_CLIENT_FEATURES = (
    "https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison"
)


def validate_v1_contract(contract):
    """Reject attempts to weaken the full-Windows-client parity contract."""
    required_dimensions = {"user_workflow", "protocol_interoperability",
                           "security", "policy_enforcement", "failure_recovery",
                           "windows_integration", "accessibility"}
    failures = []
    if contract.get("schema_version") != 1:
        failures.append("Unsupported v1 contract schema")
    if contract.get("reference") != "full-windows-desktop-client":
        failures.append("v1 contract must target Windows Full client")
    if contract.get("scope") != "all-client-visible-features-no-exclusions":
        failures.append("v1 contract scope cannot omit Windows client functions")
    if contract.get("requirements") != "all-features-and-all-reference-cases":
        failures.append("v1 contract must require all feature and reference cases")
    if contract.get("release_on_unverified") is not False:
        failures.append("v1 contract cannot allow release with unverified requirements")
    if set(contract.get("mandatory_dimensions", [])) != required_dimensions:
        failures.append("v1 contract missing mandatory behavior dimensions")
    if contract.get("primary_server") != "Skype for Business Server Subscription Edition":
        failures.append("v1 contract must cover actively supported Server Subscription Edition")
    if contract.get("official_feature_source") != OFFICIAL_WINDOWS_CLIENT_FEATURES:
        failures.append("v1 contract must retain Microsoft Windows Full feature comparison")
    return failures


def valid_build(value):
    """A pinned four-part Windows build, not a generic label or placeholder."""
    return isinstance(value, str) and re.fullmatch(r"\d+\.\d+\.\d+\.\d+", value) is not None


def valid_sha256(value):
    return isinstance(value, str) and re.fullmatch(r"[a-fA-F0-9]{64}", value) is not None


def evidence_artifacts(evidence, case_id):
    """Require actual artifact files; filenames in JSON alone are insufficient."""
    artifacts = evidence.get("raw_artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        return [f"{case_id}: missing raw test artifacts"]
    errors = []
    for relative in artifacts:
        if not isinstance(relative, str) or not relative.startswith(
                f"release/evidence/windows-reference/{case_id}/"):
            errors.append(f"{case_id}: invalid artifact path")
            continue
        path = ROOT / relative
        allowed_dir = (ROOT / "release" / "evidence" / "windows-reference" / case_id).resolve()
        if not path.resolve().is_relative_to(allowed_dir):
            errors.append(f"{case_id}: artifact path escapes case evidence directory")
        elif not path.is_file() or not path.stat().st_size:
            errors.append(f"{case_id}: raw artifact file missing or empty")
    return errors


def validate_windows_reference(baseline, manifest):
    """Make regression of independent Full Windows reference cases fail closed."""
    errors = []
    if baseline.get("schema_version") != 1:
        errors.append("Windows reference schema version is unsupported")
    if baseline.get("source_url") != OFFICIAL_WINDOWS_CLIENT_FEATURES:
        errors.append("Windows reference must retain canonical Microsoft source")
    if not isinstance(baseline.get("reference_client"), str) or not baseline["reference_client"]:
        errors.append("Missing Windows reference client description")
    rows = baseline.get("windows_scenarios")
    if not isinstance(rows, list):
        return ["Windows reference scenarios missing"]
    available_ids = {f.get("id") for f in manifest.get("features", []) if isinstance(f,dict)}
    found = set()
    for row in rows:
        if not isinstance(row,dict):
            errors.append("Windows reference case is not an object")
            continue
        case_id = row.get("id")
        if not isinstance(case_id, str) or not case_id or case_id in found:
            errors.append(f"Duplicate or invalid Windows case id: {case_id!r}")
            continue
        found.add(case_id)
        if not isinstance(row.get("scenario"),str) or not row["scenario"].strip():
            errors.append(f"{case_id}: empty test scenario")
        if not isinstance(row.get("section"),str) or not row["section"].strip():
            errors.append(f"{case_id}: missing section")
        if row.get("source_url") != OFFICIAL_WINDOWS_CLIENT_FEATURES:
            errors.append(f"{case_id}: missing Microsoft feature comparison")
        mapped = row.get("feature_ids")
        if not isinstance(mapped,list) or not mapped or any(x not in available_ids for x in mapped):
            errors.append(f"{case_id}: missing or unknown mapped features")
    missing = REQUIRED_WINDOWS_CASE_IDS - found
    if missing:
        errors.append(f"Missing Windows Full client reference cases: {sorted(missing)}")
    return errors


def check_windows_reference_evidence(baseline, signoff):
    """No 1.0 without case-specific real Windows/server cross-client evidence."""
    errors=[]
    build = signoff.get("windows_client_build") if isinstance(signoff,dict) else None
    server = signoff.get("server_version") if isinstance(signoff,dict) else None
    if not valid_build(build) or baseline.get("reference_windows_build") != build:
        errors.append("Reference Windows client build is not pinned/matches no signoff")
    if not valid_sha256(signoff.get("reference_executable_sha256")):
        errors.append("Reference Windows executable SHA-256 is missing or invalid")
    if not isinstance(signoff.get("windows_os_build"), str) or not signoff["windows_os_build"].strip():
        errors.append("Reference Windows OS build is missing")
    if not signoff.get("windows_feature_inventory_reviewed"):
        errors.append("Windows-specific feature inventory has not been independently reviewed")
    if not signoff.get("parity_differences_resolved"):
        errors.append("Behavioral parity differences not resolved/reviewed")
    for row in baseline.get("windows_scenarios", []):
        case_id = row["id"]
        filename = ROOT / "release" / "evidence" / "windows-reference" / (case_id+".json")
        if not filename.is_file():
            errors.append(f"{case_id}: missing independent Windows-client interop case evidence")
            continue
        try:
            evidence = read_json(filename)
        except (ValueError, OSError):
            errors.append(f"{case_id}: unreadable case evidence")
            continue
        if (evidence.get("case_id") != case_id
                or evidence.get("result") != "pass"
                or evidence.get("mode") != "real-server-cross-client"
                or evidence.get("windows_client_build") != build
                or evidence.get("server_version") != server
                or not evidence.get("executed_at")
                or not evidence.get("reviewer")
                or evidence.get("reference_executable_sha256") != signoff.get("reference_executable_sha256")
                or evidence.get("windows_os_build") != signoff.get("windows_os_build")):
            errors.append(f"{case_id}: invalid or unverified real-client test evidence")
        if evidence.get("interop_directions") != {"windows_to_rust": "pass", "rust_to_windows": "pass"}:
            errors.append(f"{case_id}: both Windows/Rust interop directions require passes")
        if evidence.get("policy_checks") != {"enabled": "pass", "disabled": "pass"}:
            errors.append(f"{case_id}: enabled and disabled policy behavior require passes")
        if evidence.get("failure_recovery") != "pass":
            errors.append(f"{case_id}: failure and recovery behavior unverified")
        errors.extend(evidence_artifacts(evidence, case_id))
    return errors

def read_json(path):
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def validate(manifest):
    errors = []
    if manifest.get("schema_version") != 1:
        errors.append("Unsupported feature manifest schema")
    features = manifest.get("features")
    if not isinstance(features, list) or len(features) < 60:
        return ["Expected a substantial list of Windows client features"]
    ids = set()
    areas = set()
    for index, feature in enumerate(features):
        if not isinstance(feature, dict):
            errors.append(f"features[{index}] must be an object")
            continue
        fid = feature.get("id")
        if not isinstance(fid, str) or not fid or fid in ids:
            errors.append(f"Duplicate or invalid id: {fid!r}")
        else:
            ids.add(fid)
        area = feature.get("area")
        areas.add(area)
        if area not in REQUIRED_AREAS:
            errors.append(f"{fid}: unknown area {area}")
        if feature.get("status") not in STATUSES:
            errors.append(f"{fid}: unknown status")
        for field in ("name", "acceptance", "scope_note"):
            if not isinstance(feature.get(field), str) or not feature[field].strip():
                errors.append(f"{fid}: missing {field}")
        for field in ("specifications", "unit_evidence", "interop_evidence"):
            if not isinstance(feature.get(field), list) or (
                    field == "specifications" and not feature[field]):
                errors.append(f"{fid}: invalid {field}")
    missing = REQUIRED_FEATURE_IDS - ids
    if missing:
        errors.append(f"Missing mandatory Windows parity IDs: {sorted(missing)}")
    if areas != REQUIRED_AREAS:
        errors.append(f"Missing feature areas: {sorted(REQUIRED_AREAS - areas)}")
    return errors


def check_release(manifest):
    errors = []
    for feature in manifest["features"]:
        fid = feature["id"]
        if feature["status"] != "verified":
            errors.append(f"{fid}: {feature['status']} (not verified)")
            continue
        for kind in ("unit_evidence", "interop_evidence"):
            evidence = feature[kind]
            if not evidence or any(not isinstance(e, str)
                    or not e.startswith("release/evidence/")
                    or not (ROOT / e).is_file() for e in evidence):
                errors.append(f"{fid}: missing existing {kind} artifacts")
    baseline_file = ROOT / "WINDOWS_REFERENCE_CASES.json"
    if baseline_file.is_file():
        baseline = read_json(baseline_file)
    else:
        errors.append("Missing mandatory Windows reference baseline")
        baseline = {"windows_scenarios": [], "reference_windows_build": None}
    signoff_file = ROOT / "release" / "interop-signoff.json"
    if not signoff_file.is_file():
        errors.append("Missing release/interop-signoff.json (authorized real-server test signoff)")
    else:
        signoff = read_json(signoff_file)
        errors.extend(check_windows_reference_evidence(baseline, signoff))
        for key in ("windows_client_build", "server_version", "test_date",
                    "tester", "test_environment", "security_review", "approved"):
            if not signoff.get(key):
                errors.append(f"Release signoff missing {key}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--release", action="store_true",
                        help="Enforce ALL features + real-server signoff for 1.0")
    args = parser.parse_args()
    manifest = read_json(ROOT / "FEATURE_PARITY.json")
    errors = validate(manifest)
    if not V1_CONTRACT_FILE.is_file():
        errors.append("Mandatory Windows 1.0 contract missing")
    else:
        errors += validate_v1_contract(read_json(V1_CONTRACT_FILE))
    reference_path = ROOT / "WINDOWS_REFERENCE_CASES.json"
    if not reference_path.is_file():
        errors.append("Mandatory Windows reference baseline missing")
    else:
        errors += validate_windows_reference(read_json(reference_path), manifest)
    if args.release and not errors:
        errors += check_release(manifest)
    if errors:
        print("RELEASE GATE FAILED" if args.release else "FEATURE MANIFEST INVALID")
        for err in errors[:18]:
            print(" -", err)
        if len(errors) > 18:
            print(f" ... and {len(errors)-18} more failures")
        return 1
    if args.release:
        print("Release evidence and signoff manifest present. Independent verification still required.")
    else:
        statuses = {}
        for f in manifest["features"]:
            statuses[f["status"]] = statuses.get(f["status"], 0) + 1
        print(f"Feature manifest valid: {len(manifest['features'])} requirements; {statuses}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
