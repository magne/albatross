# Tech Context

* **Primary Technologies:**
  * Backend Language: Rust
  * Backend Framework: Axum
  * Frontend Language: TypeScript
  * Frontend Framework: React
  * Frontend Router: React Router
  * Frontend Styling: Tailwind CSS v4
  * Frontend Build Tool: Vite (with SWC)
  * Database: PostgreSQL (for Event Store & Read Models)
  * Message Queue: RabbitMQ
  * Cache/PubSub: Redis
  * Serialization: Protobuf (Recommended, using `prost` crate)
* **Development Environment:**
  * OS: Linux/macOS/Windows (Rust/Docker support needed)
  * Code Structure: Monorepo managed with Git. Rust projects organized within a Cargo workspace (`apps/`, `libs/`).
  * Rust Toolchain: Latest stable Rust, Cargo.
  * Node.js Package Manager: PNPM (primarily for Biome tooling).
  * Local Infrastructure: Docker Desktop (or equivalent) for running Docker Compose (`docker-compose.infra.yml` with Postgres, RabbitMQ, Redis).
  * Local Kubernetes (Optional but recommended for DX): k3s (e.g., via `k3d`) for running infrastructure via Helm charts and testing K8s deployments.
  * IDE: VS Code recommended (with Rust Analyzer, Biome extension).
  * Debugging: Hybrid approach planned - run individual microservices in IDE debugger connected to local or k3s-hosted infrastructure.
* **Build/Deployment Process:**
  * **Model 1 (Single Executable):**
    * Build: `cargo build --release --features "single_executable_mode"`. Frontend assets embedded (`rust-embed`). SQLite/in-memory channels used via feature flags.
    * Deploy: Distribute the single binary.
  * **Model 2 (Docker Compose):**
    * Build: Multi-stage Dockerfiles for Rust services. Frontend built via Vite.
    * Deploy: `docker compose -f docker-compose.infra.yml -f application.yml up`. Run locally or on any Docker host. (Note: `application.yml` not yet created).
  * **Model 3 (Kubernetes/k3s):**
    * Build: Container images pushed to a registry (e.g., Docker Hub, GHCR, cloud provider registry).
    * Deploy: Apply Kubernetes manifests or Helm charts (preferred) via `kubectl` or CI/CD pipeline (e.g., GitHub Actions, GitLab CI). Infrastructure deployed via Helm charts (self-hosted or using managed cloud services).
* **Key Dependencies:**
  * **Backend:** Axum, Tokio, Serde, SQLx/Diesel (Postgres), Lapin (RabbitMQ), Redis-rs, Prost (Protobuf), Tower/Tower-http, rust-embed.
  * **Frontend:** React, react-router, Vite, tailwindcss, @tailwindcss/vite. (Headless UI/Radix UI still recommended for components).
  * **Infrastructure:** PostgreSQL, RabbitMQ, Redis (specific versions TBD).
  * **Tooling:** Docker, Docker Compose, Kubernetes/k3s, Helm, Git, PNPM, Biome.
* **Technical Constraints:**
  * Must support multi-tenancy with strict data isolation.
  * Backend needs to be horizontally scalable (stateless services).
  * Eventual consistency inherent in CQRS projections must be handled.
  * Complexity of ES/CQRS requires careful design and testing.
  * Schema evolution for events (Protobuf helps).
  * Requires team skills in Rust, ES/CQRS, chosen frontend stack, and DevOps (Docker, K8s).
* **Tooling:**
  * Version Control: Git
  * Monorepo Management: Cargo Workspaces (for Rust).
  * Package Manager (Node): PNPM.
  * Build Tools (Core): Cargo (Rust), Vite (Frontend), Docker, Helm.
  * CI/CD: Basic GitHub Actions workflow created (`.github/workflows/ci.yml`).
  * Linting/Formatting: `cargo fmt`, `clippy` (Rust), Biome (`biomejs.dev`) (JS/TS/JSON - via PNPM script).
  * Testing: Rust's built-in test framework, Vitest (Frontend Unit - default with Vite template), Playwright (Frontend E2E - default with Vite template).
  * Infrastructure Provisioning (Cloud): Terraform/Pulumi (optional, for managed services).
  * Serialization Codegen: `protoc`, `prost-build` (if using Protobuf).
* **API Integrations:**
  * Planned: None initially for MVP.
  * Potential Future: SimBrief (flight planning), ACARS systems (various), Discord (webhooks).
