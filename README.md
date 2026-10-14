# Hedwig CNMS

**Centralized Notification Management System**

Hedwig is a backend notification management system that allows external applications to submit notifications through a secure REST API. It manages notification processing, email and webhook delivery, retry handling, and delivery status tracking.

## Project Objectives

- Provide a centralized notification service for external applications.
- Support email delivery through SMTP.
- Support webhook delivery through HTTP requests.
- Process notification batches asynchronously.
- Store notifications and delivery jobs persistently.
- Handle temporary delivery failures through retry mechanisms.
- Track notification status and delivery attempts.
- Secure API access using HMAC-SHA256 authentication.

## Technology Stack

| Component            | Technology     |
| -------------------- | -------------- |
| Programming language | Rust           |
| Web framework        | Axum           |
| Asynchronous runtime | Tokio          |
| Database             | MySQL          |
| Database access      | SQLx           |
| Email delivery       | Lettre         |
| SMTP server          | MailEnable     |
| Webhook delivery     | Reqwest        |
| API documentation    | OpenAPI        |
| Version control      | Git and GitHub |

## Core Features

### 1. Notification Submission

- Accept individual notifications and notification batches.
- Validate request data before processing.
- Return an accepted response after notification and delivery jobs have been persisted.

### 2. Email Delivery

- Send email notifications through SMTP.
- Record SMTP responses and delivery attempts.
- Retry eligible temporary failures.

### 3. Webhook Delivery

- Send JSON payloads to configured webhook endpoints.
- Handle HTTP responses and timeouts.
- Validate destination URLs and mitigate server-side request forgery (SSRF).
- Retry webhook failures independently of email delivery.

### 4. Batch Management

- Submit multiple notifications in a single request.
- Support idempotency to prevent duplicate processing of repeated submissions.
- Retrieve batch summaries and notification statuses.

### 5. Delivery Tracking

- Track notification and delivery-job states.
- Store delivery-attempt history.
- Support cancellation of eligible queued notifications.

### 6. Security and Reliability

- Authenticate requests using HMAC-SHA256 signatures.
- Validate request timestamps and nonces to mitigate replay attacks.
- Protect application credentials.
- Use persistent storage, job locking, and retry scheduling.
- Expose health-check endpoints.

## Planned API Endpoints

| Method | Endpoint                                       | Purpose                                                               |
| ------ | ---------------------------------------------- | --------------------------------------------------------------------- |
| POST   | `/api/v1/notifications`                        | Submit a notification                                                 |
| POST   | `/api/v1/batches`                              | Submit a notification batch                                           |
| GET    | `/api/v1/batches/{batch_id}`                   | Retrieve batch status                                                 |
| GET    | `/api/v1/batches/{batch_id}/notifications`     | List batch notifications                                              |
| GET    | `/api/v1/notifications/{notification_id}`      | Retrieve notification status                                          |
| POST   | `/api/v1/notifications/{id}/cancel`            | Cancel an eligible notification                                       |
| POST   | `/api/v1/applications`                         | Register an application through an authorized administrative workflow |
| POST   | `/api/v1/applications/{id}/rotate-credentials` | Rotate application credentials                                        |
| GET    | `/health/live`                                 | Check whether the service is running                                  |
| GET    | `/health/ready`                                | Check whether required dependencies are ready                         |

These endpoints are planned interfaces and should be updated as implementation progresses.

## Notification Statuses

| Status          | Meaning                                                                               |
| --------------- | ------------------------------------------------------------------------------------- |
| `queued`        | Waiting to be processed                                                               |
| `processing`    | Currently being processed                                                             |
| `retry_wait`    | Waiting for a retry                                                                   |
| `succeeded`     | Delivery operation completed successfully according to the channel's success criteria |
| `smtp_accepted` | SMTP server accepted the email for processing                                         |
| `dead`          | No more automatic retries will be attempted                                           |
| `cancelled`     | Cancelled before delivery began                                                       |
| `uncertain`     | The final delivery outcome cannot be determined reliably                              |

An SMTP acceptance response does not guarantee inbox delivery. Exactly-once delivery cannot be guaranteed in all failure scenarios.

## Database Design

The planned core tables are:

- `applications` — registered client applications.
- `application_credentials` — application authentication credentials.
- `notification_batches` — submitted notification batches.
- `notifications` — notification content and metadata.
- `delivery_jobs` — individual email and webhook delivery jobs.
- `delivery_attempts` — records of delivery attempts and outcomes.
- `webhook_endpoints` — configured webhook destinations.
- `audit_logs` — important security and administrative events.

Internal database records use numeric IDs where appropriate. Public resource identifiers use UUIDs.

## Team Work Division

The project is developed by five developers working in parallel.

| Developer   | Responsibility                         | Main Deliverables                                                             |
| ----------- | -------------------------------------- | ----------------------------------------------------------------------------- |
| Developer 1 | API and Authentication                 | Axum routes, HMAC authentication, request validation, OpenAPI                 |
| Developer 2 | Database and Persistent Queue          | MySQL schema, SQLx repositories, queue locking, retries, recovery             |
| Developer 3 | Email Delivery                         | Lettre integration, SMTP sending, email results and error handling            |
| Developer 4 | Webhook Delivery                       | HTTP delivery, endpoint validation, SSRF protections, independent retries     |
| Developer 5 | Batch Processing and Status Management | Batch submission, idempotency, status queries, cancellation, delivery history |

All developers are responsible for testing their own modules, reviewing code, resolving integration issues, and documenting their work.

## Git Workflow

Each developer works on a separate feature branch.

| Developer   | Branch                     |
| ----------- | -------------------------- |
| Developer 1 | `feature/api-auth`         |
| Developer 2 | `feature/database-queue`   |
| Developer 3 | `feature/email-delivery`   |
| Developer 4 | `feature/webhook-delivery` |
| Developer 5 | `feature/batch-status`     |

### Workflow

1. Pull the latest changes from the integration branch.
2. Create or switch to your assigned feature branch.
3. Implement one small task at a time.
4. Run the relevant checks and tests.
5. Commit changes with descriptive messages.
6. Push your feature branch to GitHub.
7. Create a pull request targeting the integration branch.
8. Review and resolve feedback before merging.

## Development Principles

- Keep email and webhook delivery independent.
- Persist jobs before returning a successful batch-acceptance response.
- Use database transactions where multiple related records must be created atomically.
- Implement safe queue locking and recovery for interrupted workers.
- Apply bounded retries with backoff and jitter.
- Use idempotency keys for batch submissions.
- Validate all external inputs.
- Protect webhook requests against SSRF.
- Avoid logging credentials, secrets, or sensitive notification content.
- Write automated tests alongside implementation.
- Document changes to shared interfaces and database schemas.

## Project Scope

Hedwig is a backend service. It does not provide a user-facing frontend.

The application that originates a notification is responsible for generating and verifying OTPs or other business-specific verification codes. Hedwig is responsible only for securely accepting and delivering the supplied message.

## Project Status

**Status:** In development.

Features, endpoints, database schemas, and setup instructions will be updated as development progresses.

