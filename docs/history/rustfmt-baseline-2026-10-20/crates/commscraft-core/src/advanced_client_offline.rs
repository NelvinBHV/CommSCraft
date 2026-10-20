//! Safe, in-memory preview states only. No servers, calls, media or authentication.
const LIMIT:usize=64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum Error{Invalid,Duplicate,Capacity,Missing}
fn safe(s:&str)->bool{!s.is_empty()&&s.len()<=128&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{safe(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|b|b.is_ascii_whitespace())}
#[derive(Default)]pub struct DelegatePreview{delegates:Vec<String>}
impl DelegatePreview{
 pub fn add(&mut self,uri:&str)->Result<(),Error>{if !sip(uri){return Err(Error::Invalid);}
        if self.delegates.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
        if self.delegates.len()>=LIMIT{return Err(Error::Capacity);}self.delegates.push(uri.into());Ok(())}
 pub fn entries(&self)->&[String]{&self.delegates}
}
#[derive(Default)]pub struct GroupPickupPreview{groups:Vec<String>}
impl GroupPickupPreview{
 pub fn add(&mut self,name:&str)->Result<(),Error>{if !safe(name){return Err(Error::Invalid);}
        if self.groups.iter().any(|x|x.eq_ignore_ascii_case(name)){return Err(Error::Duplicate);}
        if self.groups.len()>=LIMIT{return Err(Error::Capacity);}self.groups.push(name.into());Ok(())}
 pub fn names(&self)->&[String]{&self.groups}
}
#[derive(Default)]pub struct BroadcastPreview{audience:Vec<String>}
impl BroadcastPreview{
 pub fn stage(&mut self,uri:&str)->Result<(),Error>{if !sip(uri){return Err(Error::Invalid);}
        if self.audience.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
        if self.audience.len()>=LIMIT{return Err(Error::Capacity);}self.audience.push(uri.into());Ok(())}
 pub fn audience(&self)->&[String]{&self.audience}
}
#[derive(Default)]pub struct VideoSpotlightPreview{spotlight:Option<String>}
impl VideoSpotlightPreview{
 pub fn select(&mut self,uri:&str)->Result<(),Error>{if !sip(uri){return Err(Error::Invalid);}self.spotlight=Some(uri.into());Ok(())}
 pub fn clear(&mut self){self.spotlight=None;}
 pub fn selected(&self)->Option<&str>{self.spotlight.as_deref()}
}
#[derive(Default)]pub struct CallLocationPreview{label:Option<String>}
impl CallLocationPreview{
 pub fn set(&mut self,label:&str)->Result<(),Error>{if !safe(label){return Err(Error::Invalid);}self.label=Some(label.into());Ok(())}
 pub fn label(&self)->Option<&str>{self.label.as_deref()}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn delegate_preview_rejects_duplicate_and_controls(){let mut x=DelegatePreview::default();assert_eq!(x.add("sip:a@example.test"),Ok(()));assert_eq!(x.add("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.add("sip:a@example.test\r"),Err(Error::Invalid));}
 #[test]fn pickup_preview_rejects_duplicate_and_controls(){let mut x=GroupPickupPreview::default();assert_eq!(x.add("Helpdesk"),Ok(()));assert_eq!(x.add("helpdesk"),Err(Error::Duplicate));assert_eq!(x.add("bad\n"),Err(Error::Invalid));}
 #[test]fn broadcast_preview_rejects_bad_addresses(){let mut x=BroadcastPreview::default();assert_eq!(x.stage("sip:a@example.test"),Ok(()));assert_eq!(x.stage("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.stage("sip:a@example.test\n"),Err(Error::Invalid));}
 #[test]fn video_spotlight_preview_rejects_invalid_selection(){let mut x=VideoSpotlightPreview::default();assert_eq!(x.select("sip:a@example.test"),Ok(()));assert_eq!(x.select("sip:bad@example.test\r"),Err(Error::Invalid));x.clear();assert_eq!(x.selected(),None);}
 #[test]fn location_routing_preview_rejects_controls(){let mut x=CallLocationPreview::default();assert_eq!(x.set("Office"),Ok(()));assert_eq!(x.set("bad\n"),Err(Error::Invalid));assert_eq!(x.label(),Some("Office"));}
}
