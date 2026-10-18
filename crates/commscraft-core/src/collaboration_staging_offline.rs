//! Offline display and metadata prototypes only; no enterprise traffic or actual media.
const LIMIT:usize=64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum Error{Invalid,Duplicate,Capacity,Missing}
fn safe(s:&str)->bool{!s.is_empty()&&s.len()<=128&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{safe(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|b|b.is_ascii_whitespace())}
#[derive(Default)]pub struct ChatEscalationDraft{target:Option<String>}
impl ChatEscalationDraft{
 pub fn stage(&mut self,uri:&str)->Result<(),Error>{if !sip(uri){return Err(Error::Invalid);}self.target=Some(uri.into());Ok(())}
 pub fn target(&self)->Option<&str>{self.target.as_deref()}
}
#[derive(Default)]pub struct InkReceiveDraft{strokes:Vec<Vec<(i16,i16)>>}
impl InkReceiveDraft{
 pub fn append(&mut self,points:&[(i16,i16)])->Result<(),Error>{
  if points.is_empty()||points.len()>LIMIT{return Err(Error::Invalid);}
  if self.strokes.len()>=LIMIT{return Err(Error::Capacity);}
  self.strokes.push(points.to_vec());Ok(())
 }
 pub fn len(&self)->usize{self.strokes.len()}
 pub fn is_empty(&self)->bool{self.strokes.is_empty()}
}
#[derive(Default)]pub struct ScreenShareDraft{window:Option<String>}
impl ScreenShareDraft{
 pub fn stage_window(&mut self,label:&str)->Result<(),Error>{if !safe(label){return Err(Error::Invalid);}self.window=Some(label.into());Ok(())}
 pub fn window(&self)->Option<&str>{self.window.as_deref()}
}
#[derive(Default)]pub struct ResponseGroupManagementDraft{groups:Vec<String>}
impl ResponseGroupManagementDraft{
 pub fn stage(&mut self,label:&str)->Result<(),Error>{
  if !safe(label){return Err(Error::Invalid);}
  if self.groups.iter().any(|x|x.eq_ignore_ascii_case(label)){return Err(Error::Duplicate);}
  if self.groups.len()>=LIMIT{return Err(Error::Capacity);}
  self.groups.push(label.into());Ok(())
 }
 pub fn groups(&self)->&[String]{&self.groups}
}
#[derive(Default)]pub struct PowerPointShareDraft{pages:Option<u16>}
impl PowerPointShareDraft{
 pub fn stage(&mut self,pages:u16)->Result<(),Error>{if pages==0||pages>2048{return Err(Error::Invalid);}self.pages=Some(pages);Ok(())}
 pub fn pages(&self)->Option<u16>{self.pages}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn persistent_chat_escalation_rejects_injection(){let mut x=ChatEscalationDraft::default();assert_eq!(x.stage("sip:a@example.test"),Ok(()));assert_eq!(x.stage("sip:a@example.test\r"),Err(Error::Invalid));}
 #[test]fn ink_receive_rejects_empty_and_excess_points(){let mut x=InkReceiveDraft::default();assert_eq!(x.append(&[]),Err(Error::Invalid));assert_eq!(x.append(&[(1,2)]),Ok(()));assert_eq!(x.len(),1);assert_eq!(x.append(&vec![(0,0);65]),Err(Error::Invalid));}
 #[test]fn app_sharing_rejects_control_characters(){let mut x=ScreenShareDraft::default();assert_eq!(x.stage_window("Window A"),Ok(()));assert_eq!(x.stage_window("bad\n"),Err(Error::Invalid));}
 #[test]fn response_group_management_rejects_duplicate_and_controls(){let mut x=ResponseGroupManagementDraft::default();assert_eq!(x.stage("Helpdesk"),Ok(()));assert_eq!(x.stage("helpdesk"),Err(Error::Duplicate));assert_eq!(x.stage("bad\n"),Err(Error::Invalid));}
 #[test]fn powerpoint_sharing_bounds_slide_count(){let mut x=PowerPointShareDraft::default();assert_eq!(x.stage(0),Err(Error::Invalid));assert_eq!(x.stage(4),Ok(()));assert_eq!(x.stage(2049),Err(Error::Invalid));assert_eq!(x.pages(),Some(4));}
}
