//! In-memory UI drafts only: no SIP transport, file upload, desktop sharing or server contact.
const LIMIT:usize=64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum Error {Invalid,Duplicate,Capacity,Missing}
fn safe(s:&str)->bool{!s.is_empty()&&s.len()<=128&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{safe(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|x|x.is_ascii_whitespace())}
#[derive(Default)]pub struct MeetingFilesDraft{items:Vec<String>}
impl MeetingFilesDraft{
 pub fn stage(&mut self,id:&str)->Result<(),Error>{
  if !safe(id)||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'){return Err(Error::Invalid);}
  if self.items.iter().any(|x|x==id){return Err(Error::Duplicate);}
  if self.items.len()>=LIMIT{return Err(Error::Capacity);}
  self.items.push(id.into());Ok(())
 }
 pub fn ids(&self)->&[String]{&self.items}
}
#[derive(Default)]pub struct MeetNowDraft{peers:Vec<String>}
impl MeetNowDraft{
 pub fn add(&mut self,uri:&str)->Result<(),Error>{
  if !sip(uri){return Err(Error::Invalid);}
  if self.peers.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
  if self.peers.len()>=LIMIT{return Err(Error::Capacity);}
  self.peers.push(uri.into());Ok(())
 }
 pub fn peers(&self)->&[String]{&self.peers}
}
#[derive(Default)]pub struct PowerPointNavigationDraft{current:u16,total:u16}
impl PowerPointNavigationDraft{
 pub fn set_total(&mut self,total:u16)->Result<(),Error>{if total==0||total>2048{return Err(Error::Invalid);}self.total=total;self.current=1;Ok(())}
 pub fn go_to(&mut self,page:u16)->Result<(),Error>{if page==0||page>self.total{return Err(Error::Invalid);}self.current=page;Ok(())}
 pub fn position(&self)->(u16,u16){(self.current,self.total)}
}
#[derive(Default)]pub struct AnonymousInviteDraft{labels:Vec<String>}
impl AnonymousInviteDraft{
 pub fn add_label(&mut self,label:&str)->Result<(),Error>{
  if !safe(label){return Err(Error::Invalid);}
  if self.labels.iter().any(|x|x==label){return Err(Error::Duplicate);}
  if self.labels.len()>=LIMIT{return Err(Error::Capacity);}
  self.labels.push(label.into());Ok(())
 }
 pub fn labels(&self)->&[String]{&self.labels}
}
#[derive(Default)]pub struct MeetingDialInDraft{number:Option<String>}
impl MeetingDialInDraft{
 pub fn stage(&mut self,number:&str)->Result<(),Error>{
  if !(3..=20).contains(&number.len())||!number.bytes().enumerate().all(|(i,b)|b.is_ascii_digit()||(i==0&&b==b'+')){return Err(Error::Invalid);}
  self.number=Some(number.into());Ok(())
 }
 pub fn number(&self)->Option<&str>{self.number.as_deref()}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn upload_stage_rejects_path_and_duplicate(){let mut x=MeetingFilesDraft::default();assert_eq!(x.stage("file_1"),Ok(()));assert_eq!(x.stage("../secret"),Err(Error::Invalid));assert_eq!(x.stage("file_1"),Err(Error::Duplicate));}
 #[test]fn meet_now_stage_rejects_control_and_duplicate(){let mut x=MeetNowDraft::default();assert_eq!(x.add("sip:a@example.test"),Ok(()));assert_eq!(x.add("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.add("sip:a@example.test\n"),Err(Error::Invalid));}
 #[test]fn powerpoint_navigation_bounds_pages(){let mut x=PowerPointNavigationDraft::default();assert_eq!(x.go_to(1),Err(Error::Invalid));assert_eq!(x.set_total(5),Ok(()));assert_eq!(x.go_to(6),Err(Error::Invalid));assert_eq!(x.go_to(3),Ok(()));assert_eq!(x.position(),(3,5));}
 #[test]fn anonymous_invite_label_rejects_controls(){let mut x=AnonymousInviteDraft::default();assert_eq!(x.add_label("Guest"),Ok(()));assert_eq!(x.add_label("Guest"),Err(Error::Duplicate));assert_eq!(x.add_label("bad\r"),Err(Error::Invalid));}
 #[test]fn dial_in_stage_rejects_command_injection(){let mut x=MeetingDialInDraft::default();assert_eq!(x.stage("+49123456"),Ok(()));assert_eq!(x.stage("12;cmd"),Err(Error::Invalid));assert_eq!(x.number(),Some("+49123456"));}
}
