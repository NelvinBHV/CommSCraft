//! Local-only client usability drafts. None publishes to a server or enforces enterprise policy.
const MAX: usize = 128;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }
fn clean(s:&str)->bool{!s.is_empty()&&s.len()<=256&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{clean(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|x|x.is_ascii_whitespace())}
#[derive(Default)]
pub struct PresenceAlertsDraft { watchers:Vec<String> }
impl PresenceAlertsDraft{
 pub fn follow(&mut self,uri:&str)->Result<(),DraftError>{if !sip(uri){return Err(DraftError::Invalid)}
 if self.watchers.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(DraftError::Duplicate)}
 if self.watchers.len()>=MAX{return Err(DraftError::Capacity)}self.watchers.push(uri.into());Ok(())}
 pub fn watchers(&self)->&[String]{&self.watchers}
}
#[derive(Default)]
pub struct TabbedConversationsDraft { tabs:Vec<String>,active:Option<usize> }
impl TabbedConversationsDraft{
 pub fn open(&mut self,uri:&str)->Result<(),DraftError>{if !sip(uri){return Err(DraftError::Invalid)}
 if self.tabs.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(DraftError::Duplicate)}
 if self.tabs.len()>=MAX{return Err(DraftError::Capacity)}self.tabs.push(uri.into());self.active=Some(self.tabs.len()-1);Ok(())}
 pub fn active(&self)->Option<&str>{self.active.and_then(|i|self.tabs.get(i).map(String::as_str))}
 pub fn close(&mut self,uri:&str)->Result<(),DraftError>{let i=self.tabs.iter().position(|x|x.eq_ignore_ascii_case(uri)).ok_or(DraftError::Missing)?;self.tabs.remove(i);self.active=if self.tabs.is_empty(){None}else{Some(0)};Ok(())}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Urgency { Normal, High }
pub struct MessageImportanceDraft { pub recipient:String,pub text:String,pub urgency:Urgency }
impl MessageImportanceDraft{
 pub fn new(uri:&str,text:&str,urgency:Urgency)->Result<Self,DraftError>{if !sip(uri)||!clean(text){return Err(DraftError::Invalid)}Ok(Self{recipient:uri.into(),text:text.into(),urgency})}
}
#[derive(Default)]
pub struct NotificationDraft { items:Vec<String> }
impl NotificationDraft{
 pub fn push(&mut self,message:&str)->Result<(),DraftError>{if !clean(message){return Err(DraftError::Invalid)}
 if self.items.len()>=MAX{return Err(DraftError::Capacity)}self.items.push(message.into());Ok(())}
 pub fn dismiss_all(&mut self){self.items.clear();}
 pub fn len(&self)->usize{self.items.len()}
 pub fn is_empty(&self)->bool{self.items.is_empty()}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LobbyState{Waiting,Admitted,Denied}
pub struct MeetingLobbyDraft { pub state: LobbyState }
impl Default for MeetingLobbyDraft{fn default()->Self{Self{state:LobbyState::Waiting}}}
impl MeetingLobbyDraft{
 pub fn admit(&mut self)->Result<(),DraftError>{if self.state!=LobbyState::Waiting{return Err(DraftError::Invalid)}self.state=LobbyState::Admitted;Ok(())}
 pub fn deny(&mut self)->Result<(),DraftError>{if self.state!=LobbyState::Waiting{return Err(DraftError::Invalid)}self.state=LobbyState::Denied;Ok(())}
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test] fn alerts_reject_duplicate_and_injection(){let mut x=PresenceAlertsDraft::default();assert_eq!(x.follow("sip:a@example.com"),Ok(()));assert_eq!(x.follow("sip:A@example.com"),Err(DraftError::Duplicate));assert_eq!(x.follow("sip:a@example.com\r"),Err(DraftError::Invalid));}
 #[test] fn tabs_reject_duplicate_and_close_cleanly(){let mut x=TabbedConversationsDraft::default();assert_eq!(x.open("sip:a@example.com"),Ok(()));assert_eq!(x.open("sip:a@example.com"),Err(DraftError::Duplicate));assert_eq!(x.close("sip:a@example.com"),Ok(()));assert_eq!(x.active(),None);}
 #[test] fn importance_rejects_control_characters(){assert!(MessageImportanceDraft::new("sip:a@example.com","Urgent",Urgency::High).is_ok());assert!(MessageImportanceDraft::new("sip:a@example.com","bad\n",Urgency::Normal).is_err());}
 #[test] fn notifications_are_bounded(){let mut n=NotificationDraft::default();for _ in 0..MAX{n.push("hello").unwrap();}assert_eq!(n.push("extra"),Err(DraftError::Capacity));n.dismiss_all();assert_eq!(n.len(),0);}
 #[test] fn lobby_is_terminal(){let mut m=MeetingLobbyDraft::default();assert_eq!(m.admit(),Ok(()));assert_eq!(m.deny(),Err(DraftError::Invalid));}
}
