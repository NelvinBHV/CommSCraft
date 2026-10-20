//! Local-only sketches. No meeting service, calendar integration or network transport.
const MAX: usize = 64;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Error { Invalid, Duplicate, Capacity, Missing }
fn safe(s:&str)->bool{!s.is_empty()&&s.len()<=256&&!s.chars().any(char::is_control)}
fn sip(s:&str)->bool{safe(s)&&s.starts_with("sip:")&&s.matches('@').count()==1&&!s.bytes().any(|b|b.is_ascii_whitespace())}
#[derive(Default)]
pub struct MeetingPollDraft { choices:Vec<String>,votes:Vec<usize> }
impl MeetingPollDraft {
 pub fn option(&mut self,text:&str)->Result<(),Error>{
  if !safe(text){return Err(Error::Invalid);}
  if self.choices.iter().any(|x|x.eq_ignore_ascii_case(text)){return Err(Error::Duplicate);}
  if self.choices.len()>=MAX{return Err(Error::Capacity);}
  self.choices.push(text.into());self.votes.push(0);Ok(())
 }
 pub fn vote(&mut self,index:usize)->Result<(),Error>{
  let count=self.votes.get_mut(index).ok_or(Error::Missing)?;
  *count=count.checked_add(1).ok_or(Error::Capacity)?;Ok(())
 }
 pub fn totals(&self)->&[usize]{&self.votes}
}
#[derive(Default)]
pub struct WhiteboardDraft { strokes:Vec<Vec<(i16,i16)>> }
impl WhiteboardDraft {
 pub fn stroke(&mut self,points:&[(i16,i16)])->Result<(),Error>{
  if points.is_empty()||points.len()>MAX{return Err(Error::Invalid);}
  if self.strokes.len()>=MAX{return Err(Error::Capacity);}
  self.strokes.push(points.to_vec());Ok(())
 }
 pub fn clear(&mut self){self.strokes.clear();}
 pub fn stroke_count(&self)->usize{self.strokes.len()}
}
#[derive(Default)]
pub struct MeetingScheduleDraft { start:Option<u64>,end:Option<u64> }
impl MeetingScheduleDraft {
 pub fn set(&mut self,start:u64,end:u64)->Result<(),Error>{
  if start>=end||end-start>60*60*24{return Err(Error::Invalid);}
  self.start=Some(start);self.end=Some(end);Ok(())
 }
 pub fn window(&self)->Option<(u64,u64)>{self.start.zip(self.end)}
}
#[derive(Default)]
pub struct MissedCallsDraft { callers:Vec<String> }
impl MissedCallsDraft {
 pub fn record(&mut self,uri:&str)->Result<(),Error>{
  if !sip(uri){return Err(Error::Invalid);}
  if self.callers.len()>=MAX{return Err(Error::Capacity);}
  self.callers.push(uri.into());Ok(())
 }
 pub fn dismiss(&mut self){self.callers.clear();}
 pub fn len(&self)->usize{self.callers.len()}
 pub fn is_empty(&self)->bool{self.callers.is_empty()}
}
#[derive(Default)]
pub struct OneNoteDraft { note:Option<String> }
impl OneNoteDraft {
 pub fn set(&mut self,text:&str)->Result<(),Error>{
  if !safe(text){return Err(Error::Invalid);}
  self.note=Some(text.into());Ok(())
 }
 pub fn clear(&mut self){self.note=None;}
 pub fn note(&self)->Option<&str>{self.note.as_deref()}
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test] fn polls_validate_choices_and_vote_indices(){
  let mut x=MeetingPollDraft::default();assert_eq!(x.option("Yes"),Ok(()));
  assert_eq!(x.option("yes"),Err(Error::Duplicate));
  assert_eq!(x.vote(1),Err(Error::Missing));assert_eq!(x.vote(0),Ok(()));assert_eq!(x.totals(),&[1]);
 }
 #[test] fn board_rejects_empty_and_oversized_strokes(){
  let mut x=WhiteboardDraft::default();assert_eq!(x.stroke(&[]),Err(Error::Invalid));
  assert_eq!(x.stroke(&[(1,2)]),Ok(()));assert_eq!(x.stroke_count(),1);
  x.clear();assert_eq!(x.stroke_count(),0);
 }
 #[test] fn schedule_rejects_invalid_windows(){
  let mut x=MeetingScheduleDraft::default();assert_eq!(x.set(9,9),Err(Error::Invalid));
  assert_eq!(x.set(10,9),Err(Error::Invalid));assert_eq!(x.set(10,20),Ok(()));
  assert_eq!(x.window(),Some((10,20)));
 }
 #[test] fn missed_calls_bound_and_reject_control(){
  let mut x=MissedCallsDraft::default();assert_eq!(x.record("sip:a@example.com"),Ok(()));
  assert_eq!(x.record("sip:x@example.com\r"),Err(Error::Invalid));x.dismiss();assert!(x.is_empty());
 }
 #[test] fn onenote_draft_does_not_sync(){
  let mut x=OneNoteDraft::default();assert_eq!(x.set("Notes"),Ok(()));
  assert_eq!(x.set("bad\n"),Err(Error::Invalid));x.clear();assert_eq!(x.note(),None);
 }
}
