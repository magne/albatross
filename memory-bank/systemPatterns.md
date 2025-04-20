# System Patterns

* **Architecture Overview:** Event Sourcing (ES) and Command Query Responsibility Segregation (CQRS) based system. Planned transition to a Microservices architecture for backend components to ensure scalability and modularity. Multi-tenant design supporting independent Virtual Airlines. Client-Server model with a web frontend interacting with a backend API.
* **Key Technical Decisions:**
  * ES/CQRS: Chosen for auditability, temporal queries, and handling complex state transitions inherent in airline operations.
  * Microservices: Planned for backend scalability, independent deployment, and technology flexibility per service (though initially Rust/Axum for all).
  * PostgreSQL as Event Store: Pragmatic choice balancing maturity, querying capabilities (for potential event stream analysis), and operational familiarity over specialized Event Stores initially. Requires careful implementation of optimistic concurrency.
  * RabbitMQ as Event Bus: Mature, reliable message broker for distributing events between services (command handlers to projection workers) using competing consumers pattern. Chosen over Kafka initially for potentially simpler operations ("Scenario B").
  * Redis for Cache/PubSub: Standard choice for low-latency caching of read models and facilitating real-time notifications via backend Pub/Sub. Also used for API key authentication lookups.
  * Shared Database (Initially): Using a single PostgreSQL instance for both the Event Store and Read Models, logically separated (different tables/schemas). Multi-tenancy handled via `tenant_id` filtering.
  * Protobuf for Events/Commands: Chosen for strong schema evolution support, performance, and type safety. Events stored as binary (`bytea`) in Postgres.
* **Design Patterns:**
  * **Event Sourcing:** Core pattern. State is derived from a sequence of immutable events.
  * **CQRS:** Separating command handling (state changes) from query handling (reading state). Commands modify aggregates; queries read optimized projections (read models).
  * **Aggregate:** Encapsulates state and business logic (e.g., Airline, Pilot, Flight), processing commands and emitting events.
  * **Repository:** Abstracting data access for aggregates (loading events, appending events).
  * **Projection:** Subscribes to events (via RabbitMQ) and updates specific read models (in Postgres) optimized for querying.
  * **Competing Consumers:** Multiple instances of projection workers subscribe to RabbitMQ queues to process events in parallel.
  * **Pub/Sub:** Used internally (via Redis) for broadcasting change notifications to connected backend instances for WebSocket/SSE updates.
  * **Unit of Work (Implicit):** Command handlers typically form a unit of work: load aggregate, execute command, save events, publish events. Cache invalidation (e.g., for API keys) occurs after successful event persistence/publishing.
  * **Cache-Aside Pattern:** Used for read models and API key authentication data. Application attempts to read from cache; on miss, reads from source (DB/Aggregate logic) and populates cache.
  * **API Key Authentication (Cache-Based):** Plain text keys are used as cache keys for fast authentication lookups. A separate cache entry maps `key_id` to `plain_key` to facilitate revocation.
* **Component Relationships:** (Simplified View - will evolve with microservices)

    ```mermaid
    graph TD
        subgraph Frontend
            F[Web UI - React + Tailwind CSS v4]
        end

        subgraph Backend API / Gateway [Backend API / Gateway - Axum]
            API(API Endpoints - REST/GraphQL)
            AuthMW(Auth Middleware - API Key)
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
        API -- Requests --> AuthMW;
        AuthMW -- Authenticated --> CmdH;
        AuthMW -- Authenticated --> QueryH;
        AuthMW -- Checks --> Cache;
        API --> CmdH;
        API --> QueryH;
        CmdH --> Agg;
        Agg --> ES_DB;
        CmdH --> MQ;
        CmdH -- Invalidate/Update --> Cache; # e.g., API Key Revocation/Generation
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
  * **Command Processing:** API Request -> Command Handler -> Load Aggregate (from ES_DB) -> Validate Command -> Emit Events -> Save Events (to ES_DB) -> Publish Events (to MQ) -> [Optional: Invalidate/Update Cache].
  * **Query Processing:** API Request -> Query Handler -> Read Read Model (from RM_DB / Cache) -> Return Data.
  * **Projection Update:** Event Published (to MQ) -> Projection Worker Consumes -> Update Read Model (in RM_DB) -> Update Cache -> Publish Notification (to Redis PubSub).
  * **Real-time UI Update:** Notification (Redis PubSub) -> Backend API Instance -> Forward to relevant WebSocket/SSE Client -> Frontend UI Update.
  * **API Key Authentication:** API Request (with Bearer Token) -> Auth Middleware -> Lookup Plain Key in Cache -> On Hit: Deserialize User Context, Proceed -> On Miss/Error: Return 401.
* **Data Management:**
  * **Events:** Primary source of truth. Stored immutably in PostgreSQL (as binary `bytea` using Protobuf serialization) with stream ID (including `tenant_id`), version number. Append-only.
  * **Read Models:** Denormalized projections optimized for specific queries. Stored in PostgreSQL relational tables. Updated asynchronously by projections. Includes `tenant_id` for filtering. Eventually consistent.
  * **Cache:** Frequently accessed read model data cached in Redis for performance. Also stores API key authentication data (`plain_key -> AuthenticatedUser`) and revocation lookup data (`keyid_{key_id} -> plain_key`).
  * **State:** Aggregate state is rebuilt from events when processing commands. Application state (sessions) potentially in Redis or JWTs.
  * **Multi-Tenancy:** Logical separation via `tenant_id` in event streams, event metadata, and all read model tables. Strict filtering applied at all data access points.
