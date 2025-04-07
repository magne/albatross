# Further thoughts on the conversation

## Authentication

- The authentication process is crucial for ensuring that only authorized users can access the system.
- The use of JWT (JSON Web Tokens) is a common practice for stateless authentication in web applications.
- The system should provide a way for users to log in and obtain a token that can be used for subsequent requests.
- Users should be able to log out, which would invalidate their token.
- The system should also provide a way for users to refresh their tokens, which is important for maintaining a good user experience.
- The authentication process should be secure and protect against common vulnerabilities such as CSRF (Cross-Site Request Forgery) and XSS (Cross-Site Scripting).
- A user should be able to register, log in, and log out.
- The system should also provide a way for users to reset their passwords in case they forget them.
- The system should also provide a way for users to update their profile information, such as their email address or password.
- The system should also provide a way for users to delete their accounts if they no longer wish to use the service. This is important for user privacy and data protection. Ref. GDPR and CCPA.
- The system should also provide a way for users to verify their email addresses after registration. This is important for ensuring that the user is who they say they are and for preventing spam accounts.
- Users should be able to self-register using OAuth providers such as Google, Facebook, and Twitter. This is important for providing a good user experience and for reducing the friction of signing up for a new service.
- The system should also provide a way for users to link their accounts with OAuth providers after registration. This is important for providing a good user experience and for allowing users to use their existing accounts with other services.
- The system should also provide a way for users to unlink their accounts with OAuth providers. This is important for user privacy and data protection.

### OAuth and OpenID Connect

- OAuth is an open standard for access delegation, commonly used as a way to grant websites or applications limited access to users' information without exposing passwords.
- OpenID Connect is an authentication layer on top of OAuth 2.0 that allows clients to verify the identity of the end-user based on the authentication performed by an authorization server.
- The system should support OAuth 2.0 and OpenID Connect for authentication and authorization.

## Authorization

- The authorization process is crucial for ensuring that users can only access resources they are allowed to access.
- The system should provide a way to define roles and permissions for users.
- The system should provide a way to assign roles and permissions to users.
- The system should provide a way to check if a user has a specific role or permission before allowing them to access a resource.
- Is there a need for a role-based access control (RBAC) system?
- Is there Rust libraries for RBAC? Something like `rbac` or `casbin`?
- Tenant (Airline) administrators should be able to manage users and their roles for their tenant.
- The system should provide a way for tenant administrators to assign roles and permissions to users.

## Deployment

### Scenario 1: Self-contained deployment

- The system should be able to run as a self-contained deployment, meaning that all the necessary components (database, web server, etc.) are included in a single executable.
- This scenario should use in-memory cache, pub/sub, and queue. The database should be SQLite (Rusqulite with at least features `bundled` and `modern_sqlite`).
- All microservices should be included in the single executable.
- The system should be able to run as a single binary executable, which can be easily deployed on any platform.
- This configuration should be used for development and testing purposes, and is enabled by a feature flag.

### Scenario 2: Docker deployment

- The system should be able to run in a Docker container, which is a lightweight, portable, and self-sufficient unit that can run any application.
- This scenario should use Redis for caching and pub/sub. RabbitMQ is used for queue. The database should be PostgreSQL.
- We need Dockerfiles for each microservice.

### Scenario 3: Kubernetes deployment

- The system should be able to run in a Kubernetes cluster, which is an open-source container orchestration platform for automating deployment, scaling, and management of containerized applications.
- This scenario should use Redis for caching and pub/sub. RabbitMQ is used for queue. The database should be PostgreSQL.
- We need Helm charts for each microservice.
- Should we use the api-gateway microservice, or should this be handled by Kubernetes? If so, we need to extract the command handlers from the api-gateway microservice and put them in the respective microservices. This is important for reducing the complexity of the api-gateway microservice and for improving the performance of the system.

## Logging, monitoring, and tracing

- The system should provide a way to log important events and errors.
- The system should provide a way to monitor the health and performance of the system.
- The system should provide a way to trace requests and responses through the system.
- The system should provide a way to collect metrics and logs from the system.
- The system should provide a way to visualize the metrics and logs.
- The system should provide a way to alert the administrators when something goes wrong.
- The system should provide a way to store logs and metrics in a centralized location for analysis and troubleshooting.
- The system should provide a way to rotate logs and metrics to prevent them from consuming too much disk space.
- The system should provide a way to archive logs and metrics for long-term storage and analysis.
- The system should provide a way to delete logs and metrics after a certain period of time to comply with data retention policies.

### Implementation

- The system should use OpenTelemetry for distributed logging, tracing and metrics collection.
- The system should use Prometheus for metrics collection and visualization.
- The system should use Grafana for metrics visualization.
- The system should use ELK (Elasticsearch, Logstash, Kibana) stack for log collection, storage, and visualization.
- Or, the system should use Loki and Tempo.

## Frontend framework selection

In order to test and contrast the different frontend frameworks (React, Vue, and Svelte), we need to implement the MVP (Minimum Viable Product) using each of them. This will allow us to evaluate the performance, ease of use, and developer experience of each framework.

The MVP should be accessible using different frameworks based on the URL. For example, the URL `/react` should serve the React version of the MVP, `/vue` should serve the Vue version, and `/svelte` should serve the Svelte version. This will allow us to test and compare the different frameworks in a real-world scenario. The default should be the React version.
