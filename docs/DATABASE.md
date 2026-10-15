\# Hedwig: Database and Persistent Queue (Developer 2)



MySQL 8.4 (InnoDB) + SQLx. Notifications, delivery jobs, attempt history and a

database-backed queue that multiple workers can use safely.



\## Setup (Windows)



1\. Install MySQL 8.4 and make sure the service is running.

2\. Create the database: `CREATE DATABASE hedwig;`

3\. Create a `.env` file in the project root (never commit it):

&#x20;  `DATABASE\_URL=mysql://USER:PASSWORD@localhost:3306/hedwig`

4\. Install the migration tool and apply migrations:

&#x20;  `cargo install sqlx-cli --no-default-features --features mysql,rustls`

&#x20;  then `sqlx migrate run`



\## Tables



| Table | Purpose |

|---|---|

| `applications` | Client applications that send notifications |

| `notification\_batches` | Group of notifications sent together |

| `notifications` | One notification (recipient, subject, body, status) |

| `delivery\_jobs` | One job per notification per channel (email / webhook) |

| `delivery\_attempts` | History of every delivery attempt |

| `audit\_logs` | Audit events (never store secrets here) |



\## Job statuses (proposed, needs team approval)



`queued`, `processing`, `succeeded`, `dead`, `cancelled`



Retries use `queued` with a future `next\_attempt\_at` (no separate `retry\_wait`).



\## Queue functions (`src/db/queue.rs`)



| Function | What it does |

|---|---|

| `claim\_next\_job(pool, worker\_id, lease\_seconds)` | Atomically claims one eligible job using `FOR UPDATE SKIP LOCKED`. Sets lease token and expiry. |

| `complete\_job(pool, job\_id, lease\_token)` | Marks job `succeeded`. Returns `false` if the lease is stale. |

| `fail\_job(pool, job\_id, lease\_token, retryable, error)` | Retryable and attempts left: back to `queued` with backoff. Otherwise `dead`. |

| `recover\_expired\_leases(pool)` | Moves jobs with expired leases back to `queued`. |

| `backoff\_seconds(attempt, base, max)` | Exponential backoff helper. |



\## Repository functions (`src/db/`)



`create\_notification`, `get\_notification\_by\_id`, `create\_delivery\_job`,

`get\_batch\_by\_id`, `record\_delivery\_attempt`.



\## Worker flow



1\. `claim\_next\_job`

2\. Do the delivery (email / webhook). Do NOT hold a DB transaction during this.

3\. `record\_delivery\_attempt`

4\. `complete\_job` on success, or `fail\_job` on failure.



`fail\_job` increments `attempt\_count`. `record\_delivery\_attempt` does not.



\## Retry policy (proposed, needs team approval)



\- Max attempts: 5 (`max\_attempts` per job)

\- Backoff: 30s base, doubles each attempt, capped at 1 hour

\- Permanent errors go straight to `dead`



\## Limitations



\- At-least-once delivery, NOT exactly-once. If a worker delivers and crashes

&#x20; before `complete\_job`, another worker may deliver again.

\- No backoff jitter yet.

\- `audit\_logs` table exists but is not written to yet.

