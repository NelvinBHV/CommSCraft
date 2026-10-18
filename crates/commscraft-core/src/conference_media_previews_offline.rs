//! Bounded local conference/media UI previews, without network, calls, screen capture or policy changes.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Error { Invalid, Duplicate, Capacity }
fn safe(s: &str) -> bool { !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control) }
fn sip(s: &str) -> bool { safe(s) && s.starts_with("sip:") && s.matches('@').count() == 1 && !s.bytes().any(|b| b.is_ascii_whitespace()) }
#[derive(Default)] pub struct QosPreview { dscp: Option<u8> }
impl QosPreview {
 pub fn stage_dscp(&mut self, value: u8) -> Result<(), Error> {
  if value > 63 { return Err(Error::Invalid); }
  self.dscp = Some(value); Ok(())
 }
 pub fn dscp(&self) -> Option<u8> { self.dscp }
}
#[derive(Default)] pub struct ConferenceJoinPreview { target: Option<String> }
impl ConferenceJoinPreview {
 pub fn stage(&mut self, uri: &str) -> Result<(), Error> {
  if !sip(uri) { return Err(Error::Invalid); }
  self.target = Some(uri.into()); Ok(())
 }
 pub fn target(&self) -> Option<&str> { self.target.as_deref() }
}
#[derive(Default)] pub struct ConferenceImPreview { drafts: Vec<String> }
impl ConferenceImPreview {
 pub fn stage(&mut self, body: &str) -> Result<(), Error> {
  if !safe(body) { return Err(Error::Invalid); }
  if self.drafts.len() >= MAX { return Err(Error::Capacity); }
  self.drafts.push(body.into()); Ok(())
 }
 pub fn drafts(&self) -> &[String] { &self.drafts }
}
#[derive(Default)] pub struct VbssPreview { source: Option<String> }
impl VbssPreview {
 pub fn stage_source(&mut self, source: &str) -> Result<(), Error> {
  if !safe(source) { return Err(Error::Invalid); }
  self.source = Some(source.into()); Ok(())
 }
 pub fn source(&self) -> Option<&str> { self.source.as_deref() }
}
#[derive(Default)] pub struct RdpSharingPreview { channels: Vec<String> }
impl RdpSharingPreview {
 pub fn stage_channel(&mut self, name: &str) -> Result<(), Error> {
  if !safe(name) || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') { return Err(Error::Invalid); }
  if self.channels.iter().any(|x| x.eq_ignore_ascii_case(name)) { return Err(Error::Duplicate); }
  if self.channels.len() >= MAX { return Err(Error::Capacity); }
  self.channels.push(name.into()); Ok(())
 }
 pub fn channels(&self) -> &[String] { &self.channels }
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn qos_dscp_rejects_out_of_range() { let mut x = QosPreview::default(); assert_eq!(x.stage_dscp(46), Ok(())); assert_eq!(x.stage_dscp(64), Err(Error::Invalid)); assert_eq!(x.dscp(), Some(46)); }
 #[test] fn conference_join_rejects_injected_uri() { let mut x = ConferenceJoinPreview::default(); assert_eq!(x.stage("sip:room@example.test"), Ok(())); assert_eq!(x.stage("sip:room@example.test\r"), Err(Error::Invalid)); }
 #[test] fn conference_im_rejects_controls_and_capacity() { let mut x = ConferenceImPreview::default(); for _ in 0..MAX { assert_eq!(x.stage("hello"), Ok(())); } assert_eq!(x.stage("more"), Err(Error::Capacity)); assert_eq!(x.stage("bad\n"), Err(Error::Invalid)); }
 #[test] fn vbss_preview_rejects_control_characters() { let mut x = VbssPreview::default(); assert_eq!(x.stage_source("Desktop"), Ok(())); assert_eq!(x.stage_source("bad\n"), Err(Error::Invalid)); }
 #[test] fn rdp_channels_reject_paths_and_duplicates() { let mut x = RdpSharingPreview::default(); assert_eq!(x.stage_channel("screen_1"), Ok(())); assert_eq!(x.stage_channel("SCREEN_1"), Err(Error::Duplicate)); assert_eq!(x.stage_channel("../secret"), Err(Error::Invalid)); }
}
