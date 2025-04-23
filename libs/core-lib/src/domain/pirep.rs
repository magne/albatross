use crate::{Command, CoreError, DomainEvent, Event};
use cqrs_es::Aggregate;
use proto::pirep::{PirepSubmitted, SubmitPirep};
use std::time::{SystemTime, UNIX_EPOCH};

// --- PIREP Aggregate ---

#[derive(Debug, Default, Clone)]
pub struct Pirep {
    id: String,
    version: usize,
    tenant_id: String,
    user_id: String,
    aircraft_id: String,
    departure_icao: String,
    arrival_icao: String,
    flight_number: String,
    flight_time_hours: f64,
    remarks: String,
    // status: PirepStatus, // Add status later (e.g., Pending, Approved, Rejected)
}

// --- Commands ---

#[derive(Debug, Clone)]
pub enum PirepCommand {
    Submit(SubmitPirep),
}

impl Command for SubmitPirep {}

// --- Events ---

#[derive(Debug, Clone, PartialEq)]
pub enum PirepEvent {
    Submitted(PirepSubmitted),
}

impl DomainEvent for PirepEvent {
    fn event_type(&self) -> String {
        match self {
            PirepEvent::Submitted(_) => "PirepSubmitted".to_string(),
        }
    }

    fn event_version(&self) -> String {
        "0.1.0".to_string()
    }
}

impl Event for PirepSubmitted {}

// --- Errors ---

#[derive(thiserror::Error, Debug)]
pub enum PirepError {
    #[error("Core Error: {0}")]
    Core(#[from] CoreError),
    #[error("PIREP already submitted (ID: {0})")]
    AlreadyExists(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    // Add other PIREP-specific errors if needed
}

// --- Aggregate Implementation ---

impl Aggregate for Pirep {
    const TYPE: &'static str = "pirep";
    type Command = PirepCommand;
    type Event = PirepEvent; // Event is the concrete type here
    type Error = PirepError;
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
            PirepEvent::Submitted(PirepSubmitted {
                // Corrected casing
                pirep_id,
                tenant_id,
                user_id,
                aircraft_id,
                departure_icao,
                arrival_icao,
                flight_number,
                flight_time_hours,
                remarks,
                ..
            }) => {
                self.id = pirep_id;
                self.tenant_id = tenant_id;
                self.user_id = user_id;
                self.aircraft_id = aircraft_id;
                self.departure_icao = departure_icao;
                self.arrival_icao = arrival_icao;
                self.flight_number = flight_number;
                self.flight_time_hours = flight_time_hours;
                self.remarks = remarks;
                // self.status = PirepStatus::Pending; // Set initial status
            }
        }
        self.version += 1;
    }

    /// Handle commands and produce events.
    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            PirepCommand::Submit(cmd) => self.handle_submit(cmd).await,
        }
    }
}

impl Pirep {
    // --- Command Handlers ---

    async fn handle_submit(&self, command: SubmitPirep) -> Result<Vec<PirepEvent>, PirepError> {
        // Validate command input
        if command.pirep_id.is_empty()
            || command.tenant_id.is_empty()
            || command.user_id.is_empty()
            || command.aircraft_id.is_empty()
            || command.departure_icao.is_empty()
            || command.arrival_icao.is_empty()
        {
            return Err(PirepError::InvalidInput("Missing required fields".into()));
        }
        if command.flight_time_hours <= 0.0 {
            return Err(PirepError::InvalidInput(
                "Flight time must be positive".into(),
            ));
        }

        // Check business rules (e.g., prevent duplicate submission)
        if self.version > 0 {
            return Err(PirepError::AlreadyExists(self.id.clone()));
        }

        // Create the event
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        let event = PirepSubmitted {
            // Corrected casing
            pirep_id: command.pirep_id,
            tenant_id: command.tenant_id,
            user_id: command.user_id,
            aircraft_id: command.aircraft_id,
            departure_icao: command.departure_icao,
            arrival_icao: command.arrival_icao,
            flight_number: command.flight_number,
            flight_time_hours: command.flight_time_hours,
            remarks: command.remarks,
            timestamp,
        };

        Ok(vec![PirepEvent::Submitted(event)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cqrs_es::Aggregate;
    // Use the corrected casing from the import
    use proto::pirep::{PirepSubmitted, SubmitPirep};

    #[tokio::test]
    async fn test_submit_pirep_command() {
        let aggregate = Pirep::default();
        let command = PirepCommand::Submit(SubmitPirep {
            // Corrected casing
            pirep_id: "pirep-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-1".to_string(),
            departure_icao: "EKCH".to_string(),
            arrival_icao: "EGLL".to_string(),
            flight_number: "VA123".to_string(),
            flight_time_hours: 2.5,
            remarks: "Smooth flight".to_string(),
        });

        let result = aggregate.handle(command.clone(), &()).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        #[allow(clippy::match_single_binding)]
        match &events[0] {
            PirepEvent::Submitted(PirepSubmitted {
                pirep_id,
                tenant_id,
                user_id,
                flight_time_hours,
                ..
            }) => {
                let PirepCommand::Submit(command) = command;
                assert_eq!(pirep_id, &command.pirep_id);
                assert_eq!(tenant_id, &command.tenant_id);
                assert_eq!(user_id, &command.user_id);
                assert_eq!(*flight_time_hours, command.flight_time_hours);
            }
        }
    }

    #[tokio::test]
    async fn test_submit_pirep_already_exists() {
        let mut aggregate = Pirep::default();
        aggregate.apply(PirepEvent::Submitted(PirepSubmitted {
            // Corrected casing
            pirep_id: "pirep-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-1".to_string(),
            departure_icao: "EKCH".to_string(),
            arrival_icao: "EGLL".to_string(),
            flight_number: "VA123".to_string(),
            flight_time_hours: 2.5,
            remarks: "Smooth flight".to_string(),
            timestamp: "0".to_string(),
        }));

        let command = PirepCommand::Submit(SubmitPirep {
            // Corrected casing
            pirep_id: "pirep-2".to_string(), // Different ID, but aggregate exists
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-2".to_string(),
            departure_icao: "EGLL".to_string(),
            arrival_icao: "EKCH".to_string(),
            flight_number: "VA456".to_string(),
            flight_time_hours: 2.1,
            remarks: "Return flight".to_string(),
        });

        let result = aggregate.handle(command, &()).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            PirepError::AlreadyExists(id) => assert_eq!(id, "pirep-1"),
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_submit_pirep_invalid_input() {
        let aggregate = Pirep::default();

        // Empty ID
        let command_no_id = PirepCommand::Submit(SubmitPirep {
            // Corrected casing
            pirep_id: "".to_string(), // Empty
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-1".to_string(),
            departure_icao: "EKCH".to_string(),
            arrival_icao: "EGLL".to_string(),
            flight_number: "VA123".to_string(),
            flight_time_hours: 2.5,
            remarks: "Smooth flight".to_string(),
        });
        let result_no_id = aggregate.handle(command_no_id, &()).await;
        assert!(result_no_id.is_err());
        match result_no_id.err().unwrap() {
            PirepError::InvalidInput(msg) => assert!(msg.contains("Missing required fields")),
            _ => panic!("Expected InvalidInput error for empty ID"),
        }

        // Zero flight time
        let command_zero_time = PirepCommand::Submit(SubmitPirep {
            // Corrected casing
            pirep_id: "pirep-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-1".to_string(),
            departure_icao: "EKCH".to_string(),
            arrival_icao: "EGLL".to_string(),
            flight_number: "VA123".to_string(),
            flight_time_hours: 0.0, // Invalid
            remarks: "Smooth flight".to_string(),
        });
        let result_zero_time = aggregate.handle(command_zero_time, &()).await;
        assert!(result_zero_time.is_err());
        match result_zero_time.err().unwrap() {
            PirepError::InvalidInput(msg) => assert!(msg.contains("Flight time must be positive")),
            _ => panic!("Expected InvalidInput error for zero flight time"),
        }
    }

    #[test]
    fn test_apply_pirep_submitted() {
        let mut aggregate = Pirep::default();
        let event = PirepSubmitted {
            // Corrected casing
            pirep_id: "pirep-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            user_id: "user-1".to_string(),
            aircraft_id: "ac-1".to_string(),
            departure_icao: "EKCH".to_string(),
            arrival_icao: "EGLL".to_string(),
            flight_number: "VA123".to_string(),
            flight_time_hours: 2.5,
            remarks: "Smooth flight".to_string(),
            timestamp: "12345".to_string(),
        };

        assert_eq!(aggregate.version(), 0);
        aggregate.apply(PirepEvent::Submitted(event.clone()));

        assert_eq!(aggregate.version(), 1);
        assert_eq!(aggregate.id, event.pirep_id);
        assert_eq!(aggregate.tenant_id, event.tenant_id);
        assert_eq!(aggregate.user_id, event.user_id);
        assert_eq!(aggregate.flight_time_hours, event.flight_time_hours);
    }
}
