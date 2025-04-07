use async_trait::async_trait;
use core_lib::{
    Aggregate, CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use proto::user::ChangePassword; // Use the Protobuf command type
use std::sync::Arc;

// Note: While ApplicationError is defined, the trait requires CoreError
// type HandlerError = ApplicationError;

pub struct ChangePasswordHandler {
    user_repository: Arc<dyn Repository<User>>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl ChangePasswordHandler {
    #[allow(dead_code)]
    pub fn new(
        user_repository: Arc<dyn Repository<User>>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user_repository,
            event_publisher,
        }
    }
}

#[async_trait]
impl CommandHandler<ChangePassword> for ChangePasswordHandler {
    // Return CoreError to match the trait definition
    async fn handle(&self, command: ChangePassword) -> Result<(), CoreError> {
        // 1. Load Aggregate
        let events = self.user_repository.load(&command.user_id).await?;
        if events.is_empty() {
            // Use specific error from ApplicationError mapped to CoreError if needed,
            // or just use CoreError::NotFound directly.
            return Err(CoreError::NotFound(format!(
                "User not found: {}",
                command.user_id
            )));
        }
        let user = User::load(events).map_err(CoreError::from)?; // Rehydrate aggregate state, map error

        // 2. Hash the new password (should happen here, before passing to aggregate)
        // Placeholder for hashing:
        let new_password_hash = format!("hashed_{}", command.new_password_hash); // Replace with actual hashing

        // 3. Create the aggregate command enum variant
        let aggregate_command = UserCommand::ChangePassword(ChangePassword {
            user_id: command.user_id.clone(),
            new_password_hash, // Pass the hash
        });

        // 4. Execute command on the loaded aggregate instance
        // Use '?' directly - relies on From<UserError> for CoreError impl in core-lib
        let resulting_events = user.handle(aggregate_command).await?;

        // 5. Save events using the repository, providing the expected version
        self.user_repository
            .save(&command.user_id, user.version(), &resulting_events)
            .await?;

        // 6. Publish events
        let topic = "user_events";
        for event in resulting_events {
            // TODO: Serialize event payload properly (e.g., using Prost)
            let payload = format!("{:?}", event).into_bytes();
            let event_type = match event {
                UserEvent::PasswordChanged(_) => "PasswordChanged",
                _ => "UnknownUserEvent", // Should not happen here
            };
            self.event_publisher
                .publish(topic, event_type, &payload)
                .await?;
        }

        Ok(())
    }
}

// TODO: Add tests for the handler
