# System Patterns

* **Architecture Overview:** Event Sourcing (ES) and Command Query Responsibility Segregation (CQRS) based system. Planned transition to a Microservices architecture for backend components to ensure scalability and modularity. Multi-tenant design supporting independent Virtual Airlines. Client-Server model with a web frontend interacting with a backend API.
* **Key Technical Decisions:**
  * ES/CQRS: Chosen for auditability, temporal queries, and handling complex state transitions inherent in airline operations.
  * Microservices: Planned for backend scalability, independent deployment, and technology flexibility per service (though initially Rust/Axum for all).
  * PostgreSQL as Event Store: Pragmatic choice balancing maturity, querying capabilities (for potential event stream analysis), and operational familiarity over specialized Event Stores initially. Requires careful implementation of optimistic concurrency.
  * RabbitMQ as Event Bus: Mature, reliable message broker for distributing events between services (command handlers to projection workers) using competing consumers pattern. Chosen over Kafka initially for potentially simpler operations ("Scenario B").
  * Redis for Cache/PubSub: Standard choice for low-latency caching of read models and facilitating real-time notifications via backend Pub/Sub.
  * Shared Database (Initially): Using a single PostgreSQL instance for both the Event Store and Read Models, logically separated (different tables/schemas). Multi-tenancy handled via `tenant_id` filtering.
  * Protobuf for Events/Commands: Recommended for strong schema evolution support, performance, and type safety.
* **Design Patterns:**
  * **Event Sourcing:** Core pattern. State is derived from a sequence of immutable events.
  * **CQRS:** Separating command handling (state changes) from query handling (reading state). Commands modify aggregates; queries read optimized projections (read models).
  * **Aggregate:** Encapsulates state and business logic (e.g., Airline, Pilot, Flight), processing commands and emitting events.
  * **Repository:** Abstracting data access for aggregates (loading events, appending events).
  * **Projection:** Subscribes to events (via RabbitMQ) and updates specific read models (in Postgres) optimized for querying.
  * **Competing Consumers:** Multiple instances of projection workers subscribe to RabbitMQ queues to process events in parallel.
  * **Pub/Sub:** Used internally (via Redis) for broadcasting change notifications to connected backend instances for WebSocket/SSE updates.
  * **Unit of Work (Implicit):** Command handlers typically form a unit of work: load aggregate, execute command, save events, publish events.
* **Component Relationships:** (Simplified View - will evolve with microservices)

    ```mermaid
    graph TD
        subgraph Frontend
            F[Web UI - React/Vue/Svelte + Tailwind CSS]
        end

        subgraph Backend API / Gateway [Backend API / Gateway - Axum]
            API(API Endpoints - REST/GraphQL)
            CmdH(Command Handlers)
            QueryH(Query Handlers)
        end

        subgraph Core Services / Microservices [Core Services / Microservices - Axum]
            Agg(Aggregates - Airline, Pilot, etc.)
            ProjW(Projection Workers)
        end

        subgraph Infrastructure
            ES_DB[(PostgreSQL - Event Store)]
            RM_DB[(PostgreSQL - Read Models)]
            MQ(RabbitMQ - Event Bus)
            Cache(Redis - Cache / PubSub)
        end

        F --> API;
        API --> CmdH;
        API --> QueryH;
        CmdH --> Agg;
        Agg --> ES_DB;
        CmdH --> MQ;
        QueryH --> RM_DB;
        QueryH --> Cache;
        MQ --> ProjW;
        ProjW --> RM_DB;
        ProjW --> Cache;
        ProjW -->|Notify| Cache(Redis PubSub);
        Cache -->|Notify| API;
        API -->|WS/SSE| F;
    ```

* **Critical Implementation Paths:**
  * **Command Processing:** API Request -> Command Handler -> Load Aggregate (from ES_DB) -> Validate Command -> Emit Events -> Save Events (to ES_DB) -> Publish Events (to MQ).
  * **Query Processing:** API Request -> Query Handler -> Read Read Model (from RM_DB / Cache) -> Return Data.
  * **Projection Update:** Event Published (to MQ) -> Projection Worker Consumes -> Update Read Model (in RM_DB) -> Update Cache -> Publish Notification (to Redis PubSub).
  * **Real-time UI Update:** Notification (Redis PubSub) -> Backend API Instance -> Forward to relevant WebSocket/SSE Client -> Frontend UI Update.
* **Data Management:**
  * **Events:** Primary source of truth. Stored immutably in PostgreSQL (JSONB or Protobuf bytes) with stream ID (including `tenant_id`), version number. Append-only.
  * **Read Models:** Denormalized projections optimized for specific queries. Stored in PostgreSQL relational tables. Updated asynchronously by projections. Includes `tenant_id` for filtering. Eventually consistent.
  * **Cache:** Frequently accessed read model data cached in Redis for performance.
  * **State:** Aggregate state is rebuilt from events when processing commands. Application state (sessions) potentially in Redis or JWTs.
  * **Multi-Tenancy:** Logical separation via `tenant_id` in event streams, event metadata, and all read model tables. Strict filtering applied at all data access points.
