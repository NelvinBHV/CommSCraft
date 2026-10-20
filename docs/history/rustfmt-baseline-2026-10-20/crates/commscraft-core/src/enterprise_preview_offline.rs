//! Bounded in-memory integration previews, with no third-party access or network requests.
const MAX:usize=64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum Error{Invalid,Duplicate,Capacity}
fn safe(s:&str)->bool{!s.is_empty()&&s.len()<=128&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{safe(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|b|b.is_ascii_whitespace())}
#[derive(Default)]pub struct ExchangeContactsPreview{contacts:Vec<String>}
impl ExchangeContactsPreview{
 pub fn stage(&mut self,uri:&str)->Result<(),Error>{
  if !sip(uri){return Err(Error::Invalid);}
  if self.contacts.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
  if self.contacts.len()>=MAX{return Err(Error::Capacity);}
  self.contacts.push(uri.into());Ok(())
 }
 pub fn contacts(&self)->&[String]{&self.contacts}
}
#[derive(Default)]pub struct SharePointSearchPreview{queries:Vec<String>}
impl SharePointSearchPreview{
 pub fn stage_query(&mut self,query:&str)->Result<(),Error>{
  if !safe(query){return Err(Error::Invalid);}
  if self.queries.len()>=MAX{return Err(Error::Capacity);}
  self.queries.push(query.into());Ok(())
 }
 pub fn queries(&self)->&[String]{&self.queries}
}
#[derive(Default)]pub struct ResponseGroupSearchPreview{terms:Vec<String>}
impl ResponseGroupSearchPreview{
 pub fn stage(&mut self,term:&str)->Result<(),Error>{
  if !safe(term){return Err(Error::Invalid);}
  if self.terms.len()>=MAX{return Err(Error::Capacity);}
  self.terms.push(term.into());Ok(())
 }
 pub fn terms(&self)->&[String]{&self.terms}
}
#[derive(Default)]pub struct OutlookHistoryPreview{conversation_ids:Vec<String>}
impl OutlookHistoryPreview{
 pub fn stage(&mut self,id:&str)->Result<(),Error>{
  if !safe(id)||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'){return Err(Error::Invalid);}
  if self.conversation_ids.iter().any(|x|x==id){return Err(Error::Duplicate);}
  if self.conversation_ids.len()>=MAX{return Err(Error::Capacity);}
  self.conversation_ids.push(id.into());Ok(())
 }
 pub fn ids(&self)->&[String]{&self.conversation_ids}
}
#[derive(Default)]pub struct DelegateSyncPreview{delegates:Vec<String>}
impl DelegateSyncPreview{
 pub fn stage(&mut self,uri:&str)->Result<(),Error>{
  if !sip(uri){return Err(Error::Invalid);}
  if self.delegates.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
  if self.delegates.len()>=MAX{return Err(Error::Capacity);}
  self.delegates.push(uri.into());Ok(())
 }
 pub fn delegates(&self)->&[String]{&self.delegates}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn exchange_contacts_preview_rejects_invalid_and_duplicate(){let mut x=ExchangeContactsPreview::default();assert_eq!(x.stage("sip:a@example.test"),Ok(()));assert_eq!(x.stage("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.stage("sip:bad@example.test\n"),Err(Error::Invalid));}
 #[test]fn sharepoint_preview_rejects_controls(){let mut x=SharePointSearchPreview::default();assert_eq!(x.stage_query("calendar"),Ok(()));assert_eq!(x.stage_query("secret\r"),Err(Error::Invalid));}
 #[test]fn response_group_preview_rejects_controls(){let mut x=ResponseGroupSearchPreview::default();assert_eq!(x.stage("helpdesk"),Ok(()));assert_eq!(x.stage("bad\n"),Err(Error::Invalid));}
 #[test]fn outlook_history_preview_rejects_paths(){let mut x=OutlookHistoryPreview::default();assert_eq!(x.stage("thread_1"),Ok(()));assert_eq!(x.stage("../private"),Err(Error::Invalid));assert_eq!(x.stage("thread_1"),Err(Error::Duplicate));}
 #[test]fn delegate_sync_preview_rejects_duplicate_and_controls(){let mut x=DelegateSyncPreview::default();assert_eq!(x.stage("sip:a@example.test"),Ok(()));assert_eq!(x.stage("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.stage("sip:bad@example.test\r"),Err(Error::Invalid));}
}
