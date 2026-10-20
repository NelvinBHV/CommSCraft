//! Offline-only local client previews: not a network, compliance or security implementation.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error { Invalid, Duplicate, Capacity, Missing }
fn safe(s: &str) -> bool { !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control) }
fn sip(s: &str) -> bool { safe(s) && s.starts_with("sip:") && s.matches('@').count() == 1 && !s.bytes().any(|b| b.is_ascii_whitespace()) }
#[derive(Default)]
pub struct CallParkPreview { labels: Vec<String> }
impl CallParkPreview {
 pub fn stage(&mut self, label: &str) -> Result<(), Error> {
  if !safe(label) { return Err(Error::Invalid); }
  if self.labels.iter().any(|x| x.eq_ignore_ascii_case(label)) { return Err(Error::Duplicate); }
  if self.labels.len() >= MAX { return Err(Error::Capacity); }
  self.labels.push(label.into()); Ok(())
 }
 pub fn labels(&self) -> &[String] { &self.labels }
}
#[derive(Default)]
pub struct VoicemailPreview { items: Vec<String> }
impl VoicemailPreview {
 pub fn add_metadata(&mut self, id: &str) -> Result<(), Error> {
  if !safe(id) || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') { return Err(Error::Invalid); }
  if self.items.iter().any(|x| x == id) { return Err(Error::Duplicate); }
  if self.items.len() >= MAX { return Err(Error::Capacity); }
  self.items.push(id.into()); Ok(())
 }
 pub fn items(&self) -> &[String] { &self.items }
}
#[derive(Default)]
pub struct MeetingAudioPreview { muted: bool, volume: u8 }
impl MeetingAudioPreview {
 pub fn set_muted(&mut self, value: bool) { self.muted = value; }
 pub fn set_volume(&mut self, value: u8) -> Result<(), Error> {
  if value > 100 { return Err(Error::Invalid); }
  self.volume = value; Ok(())
 }
 pub fn settings(&self) -> (bool, u8) { (self.muted, self.volume) }
}
#[derive(Default)]
pub struct ComplianceLabelPreview { labels: Vec<String> }
impl ComplianceLabelPreview {
 pub fn stage(&mut self, label: &str) -> Result<(), Error> {
  if !safe(label) { return Err(Error::Invalid); }
  if self.labels.iter().any(|x| x.eq_ignore_ascii_case(label)) { return Err(Error::Duplicate); }
  if self.labels.len() >= MAX { return Err(Error::Capacity); }
  self.labels.push(label.into()); Ok(())
 }
 pub fn labels(&self) -> &[String] { &self.labels }
}
#[derive(Default)]
pub struct PublicImPreview { recipients: Vec<String> }
impl PublicImPreview {
 pub fn add(&mut self, uri: &str) -> Result<(), Error> {
  if !sip(uri) { return Err(Error::Invalid); }
  if self.recipients.iter().any(|x| x.eq_ignore_ascii_case(uri)) { return Err(Error::Duplicate); }
  if self.recipients.len() >= MAX { return Err(Error::Capacity); }
  self.recipients.push(uri.into()); Ok(())
 }
 pub fn recipients(&self) -> &[String] { &self.recipients }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn park_labels_reject_duplicate_and_controls() {
  let mut x = CallParkPreview::default();
  assert_eq!(x.stage("slot 1"), Ok(())); assert_eq!(x.stage("SLOT 1"), Err(Error::Duplicate));
  assert_eq!(x.stage("bad\n"), Err(Error::Invalid));
 }
 #[test] fn voicemail_metadata_rejects_paths_and_duplicates() {
  let mut x = VoicemailPreview::default();
  assert_eq!(x.add_metadata("msg_01"), Ok(())); assert_eq!(x.add_metadata("../secret"), Err(Error::Invalid));
  assert_eq!(x.add_metadata("msg_01"), Err(Error::Duplicate));
 }
 #[test] fn meeting_audio_rejects_out_of_range_volume() {
  let mut x = MeetingAudioPreview::default();
  x.set_muted(true); assert_eq!(x.set_volume(75), Ok(()));
  assert_eq!(x.set_volume(101), Err(Error::Invalid)); assert_eq!(x.settings(), (true, 75));
 }
 #[test] fn compliance_labels_reject_control_and_duplicate() {
  let mut x = ComplianceLabelPreview::default();
  assert_eq!(x.stage("Internal"), Ok(())); assert_eq!(x.stage("internal"), Err(Error::Duplicate));
  assert_eq!(x.stage("bad\r"), Err(Error::Invalid));
 }
 #[test] fn public_im_preview_rejects_bad_addresses() {
  let mut x = PublicImPreview::default();
  assert_eq!(x.add("sip:a@example.test"), Ok(())); assert_eq!(x.add("sip:A@example.test"), Err(Error::Duplicate));
  assert_eq!(x.add("sip:a@example.test\n"), Err(Error::Invalid));
 }
}
