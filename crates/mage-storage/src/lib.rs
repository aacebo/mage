use sqlx::PgPool;

pub mod actors;
pub mod annotations;
pub mod artifacts;
pub mod chats;
pub mod events;
pub mod logs;
pub mod messages;
mod pagination;
mod search;
pub mod tasks;

pub use pagination::*;
pub use search::*;

pub struct Storage<'a> {
    _actors: actors::ActorStorage<'a>,
    _chats: chats::ChatStorage<'a>,
    _messages: messages::MessageStorage<'a>,
    _annotations: annotations::AnnotationStorage<'a>,
    _artifacts: artifacts::ArtifactStorage<'a>,
    _tasks: tasks::TaskStorage<'a>,
    _events: events::EventStorage<'a>,
    _logs: logs::LogStorage<'a>,
}

impl<'a> Storage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            _actors: actors::ActorStorage::new(pool),
            _chats: chats::ChatStorage::new(pool),
            _messages: messages::MessageStorage::new(pool),
            _annotations: annotations::AnnotationStorage::new(pool),
            _artifacts: artifacts::ArtifactStorage::new(pool),
            _tasks: tasks::TaskStorage::new(pool),
            _events: events::EventStorage::new(pool),
            _logs: logs::LogStorage::new(pool),
        }
    }

    pub fn actors(&self) -> &actors::ActorStorage<'a> {
        &self._actors
    }

    pub fn chats(&self) -> &chats::ChatStorage<'a> {
        &self._chats
    }

    pub fn messages(&self) -> &messages::MessageStorage<'a> {
        &self._messages
    }

    pub fn annotations(&self) -> &annotations::AnnotationStorage<'a> {
        &self._annotations
    }

    pub fn artifacts(&self) -> &artifacts::ArtifactStorage<'a> {
        &self._artifacts
    }

    pub fn tasks(&self) -> &tasks::TaskStorage<'a> {
        &self._tasks
    }

    pub fn events(&self) -> &events::EventStorage<'a> {
        &self._events
    }

    pub fn logs(&self) -> &logs::LogStorage<'a> {
        &self._logs
    }
}
