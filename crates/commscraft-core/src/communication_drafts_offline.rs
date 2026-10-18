//! Offline-only client data drafts; no enterprise synchronization or protocol operations.
const LIMIT:usize=64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum Error{Invalid,Duplicate,Capacity,Missing}
fn valid(s:&str)->bool{!s.is_empty()&&s.len()<=256&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{valid(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|b|b.is_ascii_whitespace())}
#[derive(Default)]pub struct PresenceNoteEditor{draft:Option<String>}
impl PresenceNoteEditor{
 pub fn edit(&mut self,note:&str)->Result<(),Error>{if !valid(note){return Err(Error::Invalid);}self.draft=Some(note.into());Ok(())}
 pub fn discard(&mut self){self.draft=None;}
 pub fn pending(&self)->Option<&str>{self.draft.as_deref()}
}
#[derive(Default)]pub struct PublicPhotoUrlDraft{url:Option<String>}
impl PublicPhotoUrlDraft{
 pub fn stage(&mut self,url:&str)->Result<(),Error>{
  if !valid(url)||!url.starts_with("https://")||url.len()>256||url.contains('@')||url.contains('#'){return Err(Error::Invalid);}
  self.url=Some(url.into());Ok(())
 }
 pub fn staged(&self)->Option<&str>{self.url.as_deref()}
}
#[derive(Default)]pub struct InlineImageDraft{image_ids:Vec<String>}
impl InlineImageDraft{
 pub fn add(&mut self,id:&str)->Result<(),Error>{
  if !valid(id)||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'){return Err(Error::Invalid);}
  if self.image_ids.iter().any(|x|x==id){return Err(Error::Duplicate);}
  if self.image_ids.len()>=LIMIT{return Err(Error::Capacity);}
  self.image_ids.push(id.into());Ok(())
 }
 pub fn ids(&self)->&[String]{&self.image_ids}
}
#[derive(Default)]pub struct ClickToCallDraft{number:Option<String>}
impl ClickToCallDraft{
 pub fn stage(&mut self,number:&str)->Result<(),Error>{
  if !(3..=20).contains(&number.len())||!number.bytes().enumerate().all(|(i,b)|b.is_ascii_digit()||(i==0&&b==b'+')){return Err(Error::Invalid);}
  self.number=Some(number.into());Ok(())
 }
 pub fn number(&self)->Option<&str>{self.number.as_deref()}
}
#[derive(Default)]pub struct EmailActionDraft{recipients:Vec<String>}
impl EmailActionDraft{
 pub fn add(&mut self,uri:&str)->Result<(),Error>{
  if !sip(uri){return Err(Error::Invalid);}
  if self.recipients.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(Error::Duplicate);}
  if self.recipients.len()>=LIMIT{return Err(Error::Capacity);}
  self.recipients.push(uri.into());Ok(())
 }
 pub fn recipients(&self)->&[String]{&self.recipients}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn note_editor_rejects_injection(){let mut x=PresenceNoteEditor::default();assert_eq!(x.edit("Working"),Ok(()));assert_eq!(x.edit("bad\n"),Err(Error::Invalid));x.discard();assert_eq!(x.pending(),None);}
 #[test]fn photo_url_requires_https_and_no_userinfo(){let mut x=PublicPhotoUrlDraft::default();assert_eq!(x.stage("https://example.test/p.jpg"),Ok(()));assert_eq!(x.stage("http://example.test/p.jpg"),Err(Error::Invalid));assert_eq!(x.stage("https://u@host.test/p"),Err(Error::Invalid));}
 #[test]fn inline_image_ids_reject_control_and_duplicates(){let mut x=InlineImageDraft::default();assert_eq!(x.add("image-1"),Ok(()));assert_eq!(x.add("image-1"),Err(Error::Duplicate));assert_eq!(x.add("bad/../"),Err(Error::Invalid));}
 #[test]fn dial_draft_rejects_invalid_numbers(){let mut x=ClickToCallDraft::default();assert_eq!(x.stage("+49123456"),Ok(()));assert_eq!(x.stage("12;DROP"),Err(Error::Invalid));assert_eq!(x.number(),Some("+49123456"));}
 #[test]fn email_action_rejects_duplicate_and_bad_uri(){let mut x=EmailActionDraft::default();assert_eq!(x.add("sip:a@example.test"),Ok(()));assert_eq!(x.add("sip:A@example.test"),Err(Error::Duplicate));assert_eq!(x.add("sip:a@example.test\r"),Err(Error::Invalid));}
}
