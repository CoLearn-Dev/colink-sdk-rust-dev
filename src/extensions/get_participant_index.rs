use crate::colink_proto::*;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl crate::application::CoLink {
    pub fn get_participant_index(&self, participants: &[Participant]) -> Result<usize, Error> {
        for (i, participant) in participants.iter().enumerate() {
            if participant.user_id == self.get_user_id()? {
                return Ok(i);
            }
        }
        Err("User not found in participants.")?
    }
}
