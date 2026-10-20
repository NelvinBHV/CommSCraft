//! Local UI staging only. No authorization, enterprise servers, conferences, recordings or media.
const LIMIT: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Error { Invalid, Duplicate, Capacity }
fn safe(s: &str) -> bool { !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control) }
fn id(s: &str) -> bool { safe(s) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') }
#[derive(Default)] pub struct PresentationDraft { slide_count: Option<u16> }
impl PresentationDraft {
 pub fn stage_slide_count(&mut self, value: u16) -> Result<(), Error> {
  if value == 0 || value > 4096 { return Err(Error::Invalid); }
  self.slide_count = Some(value); Ok(())
 }
 pub fn slide_count(&self) -> Option<u16> { self.slide_count }
}
#[derive(Default)] pub struct MeetingControlsDraft { raised_hands: Vec<String> }
impl MeetingControlsDraft {
 pub fn raise_hand(&mut self, attendee: &str) -> Result<(), Error> {
  if !id(attendee) { return Err(Error::Invalid); }
  if self.raised_hands.iter().any(|x| x.eq_ignore_ascii_case(attendee)) { return Err(Error::Duplicate); }
  if self.raised_hands.len() >= LIMIT { return Err(Error::Capacity); }
  self.raised_hands.push(attendee.into()); Ok(())
 }
 pub fn hands(&self) -> &[String] { &self.raised_hands }
}
#[derive(Default)] pub struct EnterpriseInteropChecklist { checks: Vec<String> }
impl EnterpriseInteropChecklist {
 pub fn stage_check(&mut self, check: &str) -> Result<(), Error> {
  if !id(check) { return Err(Error::Invalid); }
  if self.checks.iter().any(|x| x == check) { return Err(Error::Duplicate); }
  if self.checks.len() >= LIMIT { return Err(Error::Capacity); }
  self.checks.push(check.into()); Ok(())
 }
 pub fn checks(&self) -> &[String] { &self.checks }
}
#[derive(Default)] pub struct ModernAuthDisplayDraft { provider: Option<String> }
impl ModernAuthDisplayDraft {
 pub fn stage_provider(&mut self, provider: &str) -> Result<(), Error> {
  if !id(provider) { return Err(Error::Invalid); }
  self.provider = Some(provider.into()); Ok(())
 }
 pub fn provider(&self) -> Option<&str> { self.provider.as_deref() }
}
#[derive(Default)] pub struct ResponseGroupCallDraft { queue_names: Vec<String> }
impl ResponseGroupCallDraft {
 pub fn stage_queue(&mut self, queue: &str) -> Result<(), Error> {
  if !id(queue) { return Err(Error::Invalid); }
  if self.queue_names.iter().any(|x| x.eq_ignore_ascii_case(queue)) { return Err(Error::Duplicate); }
  if self.queue_names.len() >= LIMIT { return Err(Error::Capacity); }
  self.queue_names.push(queue.into()); Ok(())
 }
 pub fn queues(&self) -> &[String] { &self.queue_names }
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn presentation_rejects_zero_and_oversized_deck() { let mut x = PresentationDraft::default(); assert_eq!(x.stage_slide_count(12), Ok(())); assert_eq!(x.stage_slide_count(0), Err(Error::Invalid)); assert_eq!(x.stage_slide_count(4097), Err(Error::Invalid)); assert_eq!(x.slide_count(), Some(12)); }
 #[test] fn meeting_controls_rejects_duplicate_and_invalid_attendees() { let mut x = MeetingControlsDraft::default(); assert_eq!(x.raise_hand("attendee_1"), Ok(())); assert_eq!(x.raise_hand("ATTENDEE_1"), Err(Error::Duplicate)); assert_eq!(x.raise_hand("bad\r"), Err(Error::Invalid)); }
 #[test] fn enterprise_checklist_rejects_paths_and_duplicates() { let mut x = EnterpriseInteropChecklist::default(); assert_eq!(x.stage_check("case_1"), Ok(())); assert_eq!(x.stage_check("../secrets"), Err(Error::Invalid)); assert_eq!(x.stage_check("case_1"), Err(Error::Duplicate)); }
 #[test] fn modern_auth_display_rejects_controls() { let mut x = ModernAuthDisplayDraft::default(); assert_eq!(x.stage_provider("identity_provider"), Ok(())); assert_eq!(x.stage_provider("bad\n"), Err(Error::Invalid)); assert_eq!(x.provider(), Some("identity_provider")); }
 #[test] fn response_group_calls_rejects_duplicate_and_control() { let mut x = ResponseGroupCallDraft::default(); assert_eq!(x.stage_queue("queue_1"), Ok(())); assert_eq!(x.stage_queue("QUEUE_1"), Err(Error::Duplicate)); assert_eq!(x.stage_queue("bad\n"), Err(Error::Invalid)); }
}
