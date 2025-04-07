# Helm Charts for Albatross Infrastructure

This directory is intended to hold configurations or references for deploying the core infrastructure components (PostgreSQL, RabbitMQ, Redis) using Helm charts in a Kubernetes environment (Model 3 deployment).

It is recommended to use stable, community-maintained Helm charts, such as those provided by Bitnami:

*   **PostgreSQL:** [https://github.com/bitnami/charts/tree/main/bitnami/postgresql](https://github.com/bitnami/charts/tree/main/bitnami/postgresql)
*   **RabbitMQ:** [https://github.com/bitnami/charts/tree/main/bitnami/rabbitmq](https://github.com/bitnami/charts/tree/main/bitnami/rabbitmq)
*   **Redis:** [https://github.com/bitnami/charts/tree/main/bitnami/redis](https://github.com/bitnami/charts/tree/main/bitnami/redis)

Custom `values.yaml` files can be added here later to configure specific deployments of these charts (e.g., for different environments like staging or production).
