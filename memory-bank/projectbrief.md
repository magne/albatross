# Project Brief

* **Project Name:** [Albatross / Axiom Air / OpsCore VA / Waypoint VA Suite - Needs Final Decision]
* **Core Goal:** Develop a web-based, multi-tenant Virtual Airline (VA) Management Platform using modern technologies (Rust/Axum backend, ES/CQRS, chosen frontend stack) suitable for both local/personal use and scalable cloud deployment.
* **Key Features:**
  * Airline Administration (Profile, Branding, Hubs, Staff)
  * Pilot Management (Registration, Profiles, PIREPs, Ranks, Awards)
  * Fleet Management (Aircraft, Assignments, potentially Maintenance)
  * Route & Schedule Management
  * Flight Tracking & PIREP Validation (Manual & potentially ACARS)
  * Financial Simulation (Revenue, Expenses)
  * Community Features (potential)
  * Multi-tenancy (secure data isolation per VA)
* **Target Audience:** Flight simulator enthusiasts participating in or running Virtual Airlines.
* **Scope:** Initial focus on core VA management features (MVP). Advanced features like deep ACARS integration, complex financial modeling, or extensive community tools are potential future phases. Designed to be deployable in three distinct models (single executable demo, Docker Compose local/self-host, Kubernetes cloud/SaaS).
* **Success Metrics:** (Initial thoughts - refine later)
  * Successful implementation of core MVP features.
  * Ability to deploy and run reliably in all three target deployment models.
  * Positive feedback from initial users/testers within the VA community.
  * Clear and maintainable codebase adhering to chosen architectural patterns.
