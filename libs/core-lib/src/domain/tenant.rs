use crate::{Command, CoreError, DomainEvent, Event};
use cqrs_es::Aggregate;
use proto::tenant::{CreateTenant, TenantCreated};
use std::time::{SystemTime, UNIX_EPOCH};

// --- Tenant Aggregate ---

#[derive(Debug, Default, Clone)]
pub struct Tenant {
    id: String,
    version: usize,
    name: String,
    // Add other tenant state fields here
}

// --- Commands ---

#[derive(Debug, Clone)]
pub enum TenantCommand {
    Create(CreateTenant),
}

impl Command for CreateTenant {}

// --- Events ---

// Implement the marker trait for the event
#[derive(Debug, Clone, PartialEq)]
pub enum TenantEvent {
    Created(TenantCreated),
}

impl DomainEvent for TenantEvent {
    fn event_type(&self) -> String {
        match self {
            TenantEvent::Created(_) => "TenantCreated".to_string(),
        }
    }

    fn event_version(&self) -> String {
        "0.1.0".to_string()
    }
}

impl Event for TenantCreated {}

// --- Errors ---

#[derive(thiserror::Error, Debug)]
pub enum TenantError {
    #[error("Core Error: {0}")]
    Core(#[from] CoreError),
    #[error("Tenant already exists (ID: {0})")]
    AlreadyExists(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// --- Aggregate Implementation ---

impl Aggregate for Tenant {
    const TYPE: &'static str = "tenant";
    type Command = TenantCommand;
    type Event = TenantEvent;
    type Error = TenantError;
    type Services = ();

    fn aggregate_id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> usize {
        self.version
    }

    /// Apply state changes based on events.
    fn apply(&mut self, event: Self::Event) {
        #[allow(clippy::match_single_binding)]
        match event {
            TenantEvent::Created(TenantCreated {
                tenant_id, name, ..
            }) => {
                self.id = tenant_id;
                self.name = name;
                // Initialize other fields if necessary
            }
        }
        self.version += 1; // Increment version after applying any event
    }

    /// Handle commands and produce events.
    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            TenantCommand::Create(cmd) => self.handle_create(cmd).await,
        }
    }
}

impl Tenant {
    async fn handle_create(&self, command: CreateTenant) -> Result<Vec<TenantEvent>, TenantError> {
        // Validate command input
        if command.tenant_id.is_empty() {
            return Err(TenantError::InvalidInput(
                "Tenant ID cannot be empty".into(),
            ));
        }
        if command.name.is_empty() {
            return Err(TenantError::InvalidInput(
                "Tenant name cannot be empty".into(),
            ));
        }

        // Check business rules (e.g., prevent duplicate creation)
        if self.version > 0 {
            // If version is > 0, it means the aggregate already exists
            return Err(TenantError::AlreadyExists(self.id.clone()));
        }

        // Create the event
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string()) // Simple timestamp, consider ISO 8601 later
            .unwrap_or_default();

        let event = TenantCreated {
            tenant_id: command.tenant_id,
            name: command.name,
            timestamp,
        };

        Ok(vec![TenantEvent::Created(event)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqrs_es::Aggregate;

    #[tokio::test]
    async fn test_create_tenant_command() {
        let aggregate = Tenant::default();
        let command = TenantCommand::Create(CreateTenant {
            tenant_id: "tenant-123".to_string(),
            name: "Test VA".to_string(),
        });

        let result = aggregate.handle(command.clone(), &()).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        #[allow(clippy::match_single_binding)]
        match &events[0] {
            TenantEvent::Created(TenantCreated {
                tenant_id, name, ..
            }) => {
                let TenantCommand::Create(command) = command;
                assert_eq!(tenant_id, &command.tenant_id);
                assert_eq!(name, &command.name);
            }
        }
    }

    #[tokio::test]
    async fn test_create_tenant_already_exists() {
        let mut aggregate = Tenant::default();
        // Apply an initial event to simulate existing state
        aggregate.apply(TenantEvent::Created(TenantCreated {
            tenant_id: "tenant-123".to_string(),
            name: "Existing VA".to_string(),
            timestamp: "0".to_string(),
        }));

        let command = TenantCommand::Create(CreateTenant {
            tenant_id: "tenant-456".to_string(), // Different ID, but aggregate exists
            name: "New VA".to_string(),
        });

        let result = aggregate.handle(command, &()).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            TenantError::AlreadyExists(id) => assert_eq!(id, "tenant-123"),
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_create_tenant_invalid_input() {
        let aggregate = Tenant::default();

        // Empty ID
        let command_no_id = TenantCommand::Create(CreateTenant {
            tenant_id: "".to_string(),
            name: "Test VA".to_string(),
        });
        let result_no_id = aggregate.handle(command_no_id, &()).await;
        assert!(result_no_id.is_err());
        match result_no_id.err().unwrap() {
            TenantError::InvalidInput(msg) => assert!(msg.contains("ID cannot be empty")),
            _ => panic!("Expected InvalidInput error for empty ID"),
        }

        // Empty Name
        let command_no_name = TenantCommand::Create(CreateTenant {
            tenant_id: "tenant-123".to_string(),
            name: "".to_string(),
        });
        let result_no_name = aggregate.handle(command_no_name, &()).await;
        assert!(result_no_name.is_err());
        match result_no_name.err().unwrap() {
            TenantError::InvalidInput(msg) => assert!(msg.contains("name cannot be empty")),
            _ => panic!("Expected InvalidInput error for empty name"),
        }
    }

    #[test]
    fn test_apply_tenant_created() {
        let mut aggregate = Tenant::default();
        let event = TenantCreated {
            tenant_id: "tenant-123".to_string(),
            name: "Applied VA".to_string(),
            timestamp: "12345".to_string(),
        };

        assert_eq!(aggregate.version(), 0);
        aggregate.apply(TenantEvent::Created(event.clone()));

        assert_eq!(aggregate.version(), 1);
        assert_eq!(aggregate.id, event.tenant_id);
        assert_eq!(aggregate.name, event.name);
    }
}
