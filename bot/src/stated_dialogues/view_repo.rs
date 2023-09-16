use std::sync::Arc;

use sec_store::repository::RecordsRepository;

use crate::stated_dialogues::CtxResult;

use super::DialContext;

pub struct ViewRepo<T: RecordsRepository> {
    repo: Arc<T>,
}

impl <T>ViewRepo<T> where T: RecordsRepository {
    pub fn new(repo: T) -> Self {
        ViewRepo { repo: Arc::new(repo) }
    }
}

impl <T>DialContext for ViewRepo<T> where T: RecordsRepository {
    fn init(&mut self) -> anyhow::Result<Vec<super::CtxResult>> {
        Ok(vec![CtxResult::Messages(vec!["!!!!".into()])])
    }

    fn shutdown(&mut self) -> anyhow::Result<Vec<super::CtxResult>> {
        todo!()
    }

    fn handle_select(&mut self, select: super::Select) -> anyhow::Result<Vec<super::CtxResult>> {
        todo!()
    }

    fn handle_message(&mut self, input: super::Message) -> anyhow::Result<Vec<super::CtxResult>> {
        todo!()
    }

    fn handle_command(&mut self, command: super::Message) -> anyhow::Result<Vec<super::CtxResult>> {
        todo!()
    }

    fn remember_sent_messages(&mut self, msg_ids: Vec<super::MessageId>) {
        todo!()
    }
}
