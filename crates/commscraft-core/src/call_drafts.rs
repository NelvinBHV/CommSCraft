//! Offline drafts for call routing and conferencing. No real SIP, media or policy enforcement.
const MAX: usize = 64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }
fn sip(s: &str) -> bool {
    s.len() <= 256 && s.starts_with("sip:") && s.matches('@').count()==1
        && !s.chars().any(char::is_control) && !s.bytes().any(|b|b.is_ascii_whitespace())
}
#[derive(Default)]
pub struct ForwardDraft { destination: Option<String> }
impl ForwardDraft {
    pub fn set(&mut self, destination: &str)->Result<(),DraftError>{
        if !sip(destination){return Err(DraftError::Invalid);}
        self.destination=Some(destination.into());Ok(())
    }
    pub fn clear(&mut self){self.destination=None;}
    pub fn destination(&self)->Option<&str>{self.destination.as_deref()}
}
#[derive(Default)]
pub struct TransferDraft { target: Option<String>, committed: bool }
impl TransferDraft {
    pub fn prepare(&mut self,target:&str)->Result<(),DraftError>{
        if self.committed||!sip(target){return Err(DraftError::Invalid);}
        self.target=Some(target.into());Ok(())
    }
    pub fn commit(&mut self)->Result<(),DraftError>{
        if self.target.is_none()||self.committed{return Err(DraftError::Invalid);}
        self.committed=true;Ok(())
    }
    pub fn committed(&self)->bool{self.committed}
}
#[derive(Default)]
pub struct ConferenceRosterDraft { participants: Vec<String> }
impl ConferenceRosterDraft {
    pub fn add(&mut self,uri:&str)->Result<(),DraftError>{
        if !sip(uri){return Err(DraftError::Invalid);}
        if self.participants.iter().any(|x|x.eq_ignore_ascii_case(uri)){return Err(DraftError::Duplicate);}
        if self.participants.len()>=MAX{return Err(DraftError::Capacity);}
        self.participants.push(uri.into());Ok(())
    }
    pub fn remove(&mut self,uri:&str)->Result<(),DraftError>{
        let i=self.participants.iter().position(|x|x.eq_ignore_ascii_case(uri)).ok_or(DraftError::Missing)?;
        self.participants.remove(i);Ok(())
    }
    pub fn members(&self)->&[String]{&self.participants}
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum DeviceKind { Microphone, Speaker, Camera }
#[derive(Default)]
pub struct DeviceSelectionDraft { choices: Vec<(DeviceKind,String)> }
impl DeviceSelectionDraft {
    pub fn select(&mut self,kind:DeviceKind,label:&str)->Result<(),DraftError>{
        if label.is_empty()||label.len()>128||label.chars().any(char::is_control){return Err(DraftError::Invalid);}
        if let Some(item)=self.choices.iter_mut().find(|(k,_)|*k==kind){item.1=label.into();}
        else{self.choices.push((kind,label.into()));}
        Ok(())
    }
    pub fn selected(&self,kind:DeviceKind)->Option<&str>{self.choices.iter().find(|(k,_)|*k==kind).map(|x|x.1.as_str())}
}
#[derive(Default)]
pub struct CallHistoryDraft { entries:Vec<String> }
impl CallHistoryDraft {
    pub fn record(&mut self,uri:&str)->Result<(),DraftError>{
        if !sip(uri){return Err(DraftError::Invalid);}
        if self.entries.len()>=MAX{return Err(DraftError::Capacity);}
        self.entries.push(uri.into());Ok(())
    }
    pub fn clear(&mut self){self.entries.clear();}
    pub fn entries(&self)->&[String]{&self.entries}
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn forwarding_and_transfer_fail_closed(){
    let mut f=ForwardDraft::default();assert_eq!(f.set("sip:a@example.com"),Ok(()));
    assert_eq!(f.set("sip:x@example.com\r"),Err(DraftError::Invalid));
    assert_eq!(f.destination(),Some("sip:a@example.com"));f.clear();assert_eq!(f.destination(),None);
    let mut t=TransferDraft::default();assert_eq!(t.commit(),Err(DraftError::Invalid));
    assert_eq!(t.prepare("sip:b@example.com"),Ok(()));assert_eq!(t.commit(),Ok(()));
    assert_eq!(t.prepare("sip:c@example.com"),Err(DraftError::Invalid));
 }
 #[test] fn roster_and_devices_are_bounded(){
    let mut c=ConferenceRosterDraft::default();
    assert_eq!(c.add("sip:a@example.com"),Ok(()));assert_eq!(c.add("sip:A@example.com"),Err(DraftError::Duplicate));
    assert_eq!(c.remove("sip:a@example.com"),Ok(()));assert_eq!(c.remove("sip:a@example.com"),Err(DraftError::Missing));
    let mut d=DeviceSelectionDraft::default();assert_eq!(d.select(DeviceKind::Microphone,"USB Mic"),Ok(()));
    assert_eq!(d.select(DeviceKind::Speaker,"bad\0"),Err(DraftError::Invalid));
    assert_eq!(d.selected(DeviceKind::Microphone),Some("USB Mic"));
 }
 #[test] fn history_rejects_injection_and_overflow(){
    let mut h=CallHistoryDraft::default();assert_eq!(h.record("sip:a@example.com\n"),Err(DraftError::Invalid));
    for _ in 0..MAX{h.record("sip:a@example.com").unwrap();}
    assert_eq!(h.record("sip:a@example.com"),Err(DraftError::Capacity));h.clear();assert!(h.entries().is_empty());
 }
}
