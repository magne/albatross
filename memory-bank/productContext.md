# Product Context

* **Problem Solved:** Many Virtual Airlines (VAs) rely on disparate tools, outdated platforms, or significant manual effort to manage their operations (pilots, flights, fleet, finances). Existing platforms might be closed-source, expensive, lack modern features, or not be easily self-hostable/customizable.
* **User Needs:**
  * VA Admins/Staff: Need a centralized, efficient platform to manage airline branding, pilots, fleet, routes, schedules, finances, and PIREPs. Require tools for administration and oversight.
  * VA Pilots: Need a platform to register, log flights (PIREPs), track progress (ranks, hours), view schedules, and potentially interact with the VA community.
* **Core Functionality:** From a user perspective, the platform allows VA staff to set up and manage their airline's virtual operations. Pilots can join a VA, file flight reports, and track their virtual career. The system validates flights, updates pilot stats, manages the fleet, and potentially simulates basic economics. Multi-tenancy ensures each VA operates independently.
* **User Experience Goals:**
  * **Intuitive:** Easy to navigate for both admins and pilots.
  * **Efficient:** Streamline common tasks like PIREP filing and validation.
  * **Modern:** Clean, responsive UI (using Tailwind CSS).
  * **Flexible:** Adaptable to different VA sizes and operational styles.
  * **Reliable:** Stable operation, especially regarding flight data logging.
* **Assumptions:**
  * Users are familiar with flight simulation concepts and the general idea of Virtual Airlines.
  * Users will have varying technical abilities.
  * VAs have diverse operational models (casual to highly structured).
  * Demand exists for a modern, potentially self-hostable, open-source (with commercial restrictions) VA platform.
